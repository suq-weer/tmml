//! NeoForge processor 执行器。
//!
//! 按文档 §3.4~3.7：
//! - 仅执行 sides 缺失或含 client 的 processor；
//! - 参数逐项传给 Java（不经过 shell），`-cp` 用平台分隔符；
//! - 支持 `{VAR}` / `'literal'` / `[g:a:v[:c][@ext]]` / `data/file` 展开；
//! - processor 串行；有 outputs 时先做「存在 + SHA-1 命中则跳过」，跑完再校验。

use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use tokio::process::Command;
use zip::ZipArchive;

use crate::{
    loader::{
        install_ctx::InstallContext,
        maven::relative_path_of,
        neoforge::profile::{DataEntry, InstallProfile, Processor},
    },
    platform::path_separator,
};

/// 数据条目里指向 installer 内部文件的形态
pub struct ProcessorRunner<'a> {
    ctx: &'a InstallContext,
    profile: &'a InstallProfile,
    installer_path: &'a Path,
    temp_dir: &'a Path,
    /// 传给处理器的原版客户端副本
    client_jar: &'a Path,
    /// 已确认可用的库：坐标 -> 本地绝对路径
    lib_paths: HashMap<String, PathBuf>,
    data_tokens: HashMap<String, String>,
    main_class_cache: HashMap<PathBuf, String>,
}

impl<'a> ProcessorRunner<'a> {
    pub fn new(
        ctx: &'a InstallContext,
        profile: &'a InstallProfile,
        installer_path: &'a Path,
        temp_dir: &'a Path,
        client_jar: &'a Path,
    ) -> Result<Self> {
        let mut lib_paths = HashMap::new();
        for lib in &profile.libraries {
            let path = match lib.artifact() {
                Some(a) => PathBuf::from(&a.path),
                None => relative_path_of(&lib.name)
                    .unwrap_or_else(|| PathBuf::from(&lib.name.replace('.', "/"))),
            };
            lib_paths.insert(lib.name.clone(), ctx.libraries_dir.join(path));
        }
        Ok(Self {
            ctx,
            profile,
            installer_path,
            temp_dir,
            client_jar,
            lib_paths,
            data_tokens: HashMap::new(),
            main_class_cache: HashMap::new(),
        })
    }

    fn base_token(&self, key: &str) -> Option<String> {
        let v = match key {
            "SIDE" => "client".to_string(),
            "MINECRAFT_VERSION" => self.ctx.minecraft_version().to_string(),
            "ROOT" => self.ctx.dot_minecraft.display().to_string(),
            "LIBRARY_DIR" => self.ctx.libraries_dir.display().to_string(),
            "INSTALLER" => self.installer_path.display().to_string(),
            "MINECRAFT_JAR" => self.client_jar.display().to_string(),
            _ => return None,
        };
        Some(v)
    }

    /// 解析 data 变量（client 侧取值）：
    /// - `'literal'` → 字面量
    /// - `[coord]` → 库文件路径
    /// - `/data/file`、`data/file` → 从 installer 内提取到临时目录
    /// - 其余 → 字面量
    fn resolve_data_value(&mut self, raw: &str) -> Result<String> {
        let t = raw.trim();
        if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
            return Ok(t[1..t.len() - 1].to_string());
        }
        if t.starts_with('[') && t.ends_with(']') {
            return Ok(self.library_path(&t[1..t.len() - 1]).display().to_string());
        }
        let entry = t.strip_prefix('/').unwrap_or(t);
        if entry.starts_with("data/") {
            return Ok(self.extract_entry(entry)?.display().to_string());
        }
        // 其它形态按字面量处理
        Ok(t.to_string())
    }

    /// 获取某个库在库目录下的绝对路径（profile 声明的 artifact 优先）
    fn library_path(&self, coordinate: &str) -> PathBuf {
        if let Some(p) = self.lib_paths.get(coordinate) {
            return p.clone();
        }
        match relative_path_of(coordinate) {
            Some(rel) => self.ctx.libraries_dir.join(rel),
            None => {
                // 未知坐标兜底：原样拼到库目录
                self.ctx.libraries_dir.join(coordinate.replace('.', "/"))
            }
        }
    }

    /// 展开单条参数/路径字符串
    fn expand(&mut self, raw: &str) -> Result<String> {
        let t = raw.trim();
        if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
            return Ok(t[1..t.len() - 1].to_string());
        }
        let mut out = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            match chars[i] {
                '{' => {
                    if let Some(end) = raw[i + 1..].find('}') {
                        let key = &raw[i + 1..i + 1 + end];
                        let value = if let Some(base) = self.base_token(key) {
                            base
                        } else if self.data_entry_of(key).is_some() {
                            self.data_value_or(key)?
                        } else {
                            // 未知变量保留原样，避免把参数改写坏
                            format!("{{{}}}", key)
                        };
                        out.push_str(&value);
                        i += 1 + end + 1;
                    } else {
                        out.push('{');
                        i += 1;
                    }
                }
                '[' => {
                    if let Some(end) = raw[i + 1..].find(']') {
                        let inner = &raw[i + 1..i + 1 + end];
                        out.push_str(&self.library_path(inner).display().to_string());
                        i += 1 + end + 1;
                    } else {
                        out.push('[');
                        i += 1;
                    }
                }
                c => {
                    out.push(c);
                    i += 1;
                }
            }
        }
        Ok(out)
    }

    fn data_entry_of(&self, key: &str) -> Option<String> {
        self.profile
            .data
            .get(key)
            .and_then(|d: &DataEntry| d.client.clone().or_else(|| d.server.clone()))
    }

    fn data_value_or(&mut self, key: &str) -> Result<String> {
        if let Some(v) = self.data_tokens.get(key) {
            return Ok(v.clone());
        }
        let entry = self
            .data_entry_of(key)
            .ok_or_else(|| anyhow!("install_profile.data 缺少变量 {}", key))?;
        let resolved = self.resolve_data_value(&entry)?;
        self.data_tokens.insert(key.to_string(), resolved.clone());
        Ok(resolved)
    }

    /// 从 installer jar 内提取一个文件到临时目录（幂等）
    fn extract_entry(&mut self, entry_path: &str) -> Result<PathBuf> {
        let normalized: String = entry_path.replace('\\', "/");
        if normalized.split('/').any(|seg| seg == "..") {
            bail!("installer 内部文件路径不合法: {}", entry_path);
        }
        let dest = self.temp_dir.join("data").join(&normalized);
        if dest.exists() {
            return Ok(dest);
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::open(self.installer_path)
            .with_context(|| format!("打开 installer 失败: {}", self.installer_path.display()))?;
        let mut archive = ZipArchive::new(file).context("解析 installer 失败")?;
        let mut found: Option<PathBuf> = None;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().replace('\\', "/");
            if name == normalized {
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                std::fs::write(&dest, &data)?;
                found = Some(dest.clone());
                break;
            }
        }
        found.ok_or_else(|| {
            anyhow!(
                "installer 内未找到文件 {}（{}）",
                entry_path,
                self.installer_path.display()
            )
        })
    }

    /// 读取 jar 的 Main-Class
    fn main_class_of(&mut self, jar: &Path) -> Result<String> {
        if let Some(c) = self.main_class_cache.get(jar) {
            return Ok(c.clone());
        }
        let file = std::fs::File::open(jar)
            .with_context(|| format!("打开 processor jar 失败: {}", jar.display()))?;
        let mut archive = ZipArchive::new(file).context("解析 processor jar 失败")?;
        let mut manifest: Option<String> = None;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            if entry.name().eq_ignore_ascii_case("META-INF/MANIFEST.MF") {
                let mut text = String::new();
                entry.read_to_string(&mut text)?;
                manifest = Some(text);
                break;
            }
        }
        let text = manifest
            .ok_or_else(|| anyhow!("processor jar 缺少 META-INF/MANIFEST.MF: {}", jar.display()))?;
        let main = text
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .find_map(|line| line.strip_prefix("Main-Class:"))
            .map(|v| v.trim().to_string())
            .ok_or_else(|| {
                anyhow!(
                    "processor jar 的 MANIFEST 里没有 Main-Class: {}",
                    jar.display()
                )
            })?;
        self.main_class_cache
            .insert(jar.to_path_buf(), main.clone());
        Ok(main)
    }

    fn is_coordinate(s: &str) -> bool {
        if s.contains('/') || s.contains('\\') {
            return false;
        }
        if s.starts_with('[') || s.starts_with('{') {
            return false;
        }
        // 排除 Windows 盘符形式 C:\...
        let bytes = s.as_bytes();
        let drive = bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/');
        !drive && s.matches(':').count() >= 2
    }

    /// 解析单个 classpath/jar 条目为本地文件（工具库在跑之前已全部就绪）
    fn resolve_entry(&mut self, item: &str) -> Result<PathBuf> {
        let expanded = self.expand(item)?;
        let path = if Self::is_coordinate(item) {
            self.library_path(&expanded)
        } else {
            PathBuf::from(&expanded)
        };
        if !path.exists() {
            bail!(
                "缺少 processor 依赖 {}（{}），请检查网络后重试",
                item,
                path.display()
            );
        }
        Ok(path)
    }

    /// 把 classpath 解析为本地文件列表（去重）
    async fn resolve_entries(&mut self, items: &[String]) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item in items {
            let path = self.resolve_entry(item)?;
            if seen.insert(path.clone()) {
                out.push(path);
            }
        }
        Ok(out)
    }

    fn expected_sha(value: &str) -> bool {
        let v = value.trim();
        v.len() == 40 && v.chars().all(|c| c.is_ascii_hexdigit())
    }

    async fn verify_outputs(&mut self, proc: &Processor) -> Result<()> {
        let Some(outputs) = &proc.outputs else {
            return Ok(());
        };
        for (key, expected_raw) in outputs {
            let target = PathBuf::from(self.expand(key)?);
            let expected = self.expand(expected_raw)?;
            if !target.exists() {
                bail!("processor 输出文件不存在: {}（{}）", target.display(), key);
            }
            if Self::expected_sha(&expected) {
                let actual = crate::loader::download::sha1_hex_file(&target)?;
                if !actual.eq_ignore_ascii_case(&expected) {
                    bail!(
                        "processor 输出校验失败 {}: 期望 {} 实际 {}",
                        target.display(),
                        expected,
                        actual
                    );
                }
            }
        }
        Ok(())
    }

    /// 输出是否已全部命中（命中则跳过执行）
    async fn outputs_already_ready(&mut self, proc: &Processor) -> Result<bool> {
        let Some(outputs) = &proc.outputs else {
            return Ok(false);
        };
        let mut ready = true;
        for (key, expected_raw) in outputs {
            let target = PathBuf::from(self.expand(key)?);
            let expected = self.expand(expected_raw)?;
            if !target.exists() {
                ready = false;
                break;
            }
            if Self::expected_sha(&expected) {
                let actual = crate::loader::download::sha1_hex_file(&target)?;
                if !actual.eq_ignore_ascii_case(&expected) {
                    let _ = std::fs::remove_file(&target);
                    ready = false;
                    break;
                }
            }
        }
        Ok(ready)
    }

    /// 串行执行所有客户端相关 processor
    pub async fn run_all(&mut self) -> Result<()> {
        let processors: Vec<Processor> = self
            .profile
            .processors
            .iter()
            .filter(|p| p.for_client())
            .cloned()
            .collect();
        for (index, proc) in processors.iter().enumerate() {
            tracing::info!(
                "执行 processor {}/{}: {}",
                index + 1,
                processors.len(),
                proc.jar
            );
            self.run_one(proc).await?;
        }
        Ok(())
    }

    async fn run_one(&mut self, proc: &Processor) -> Result<()> {
        // 1. outputs 命中则跳过
        if self.outputs_already_ready(proc).await? {
            tracing::debug!("processor 输出已就绪，跳过: {}", proc.jar);
            return Ok(());
        }

        // 2. 组装 classpath（jar 主类在列表中则不加重复）
        let mut cp_items: Vec<String> = proc.classpath.clone().unwrap_or_default();
        if !cp_items.iter().any(|c| c == &proc.jar) {
            cp_items.push(proc.jar.clone());
        }
        let classpath = self.resolve_entries(&cp_items).await?;

        // 3. 主类从 processor 自身的 jar（proc.jar）MANIFEST 读取，
        //    不能按 classpath 顺序取最后一项——classpath 可能含大量普通依赖
        let main_jar = self.resolve_entry(&proc.jar)?;
        let main_class = self.main_class_of(&main_jar)?;

        // 4. 展开参数
        let mut args = Vec::new();
        if let Some(list) = &proc.args {
            for raw in list {
                let expanded = self.expand(raw)?;
                // 输出路径所在目录可能尚不存在（如写进 libraries 的映射/产物），提前建好
                if let Some(p) = Path::new(&expanded).parent() {
                    if !p.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(p);
                    }
                }
                args.push(expanded);
            }
        }

        let cp_str = classpath
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(path_separator());

        let java = &self.ctx.java_bin;
        let mut cmd = Command::new(java);
        cmd.arg("-cp")
            .arg(&cp_str)
            .arg(&main_class)
            .args(&args)
            .current_dir(self.temp_dir);

        tracing::debug!(
            "processor 命令: {} -cp <...> {} {}",
            java,
            main_class,
            args.join(" ")
        );
        let output = cmd.output().await.map_err(|e| {
            anyhow!(
                "无法运行 processor（Java={}）: {}。请安装 Java 并在设置里指定 javaPath",
                java,
                e
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let tail = format!("{}{}", stdout, stderr)
                .lines()
                .rev()
                .take(12)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "processor {} 执行失败（退出码 {}）:\n{}",
                proc.jar,
                output.status.code().unwrap_or(-1),
                tail
            );
        }

        // 5. 输出校验
        self.verify_outputs(proc).await
    }
}
