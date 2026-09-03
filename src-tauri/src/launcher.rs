//! Minecraft 实例启动会话（守护进程）。
//!
//! `launch_minecraft` 校验实例后立即返回 sessionId，真正的启动流程放到独立 OS 线程：
//! 预检 Java → 解压 natives（日志写入 Island）→ 生成最终命令并打印 → 在 PTY 中拉起
//! Minecraft → 把真实终端输出实时转成 `mc-session-log` 事件（保留颜色/转义），并在
//! 进程退出后发出 `mc-session-state`（exited/error）。前端据此把 Island 置为「已停止」。

pub mod command;
pub mod logfmt;
pub mod natives;

use std::{
    collections::HashMap,
    io::Read,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, LazyLock, Mutex,
    },
    thread,
};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    appfile::dirs,
    downloader::deserializer::VersionContent,
    instance::{self, InstanceConfig},
    profile::{self, GameProfile},
    runtime,
};

use self::{
    command::{build_final_command, LaunchCommand, LaunchContext, LaunchPaths},
    natives::{extract_natives, resolve_native_tasks},
};

pub const SESSION_STATE_EVENT: &str = "mc-session-state";
pub const SESSION_LOG_EVENT: &str = "mc-session-log";

pub const LAUNCHER_NAME: &str = "tmml";

// | 事件载荷 |

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatePayload {
    pub session_id: u64,
    /// launching | running | exited | error
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionLogPayload {
    pub session_id: u64,
    /// 原始文本（可能包含 ANSI 转义，前端负责渲染）
    pub text: String,
    /// system | game
    pub kind: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LaunchReply {
    pub session_id: u64,
}

fn emit_state(
    app: &AppHandle,
    session_id: u64,
    status: &str,
    pid: Option<u32>,
    exit_code: Option<i32>,
    message: Option<String>,
) {
    let payload = SessionStatePayload {
        session_id,
        status: status.to_string(),
        pid,
        exit_code,
        message,
    };
    if let Err(e) = app.emit(SESSION_STATE_EVENT, &payload) {
        tracing::warn!("发送会话状态事件失败: {}", e);
    }
}

fn emit_log(app: &AppHandle, session_id: u64, text: String, kind: &str) {
    let payload = SessionLogPayload {
        session_id,
        text,
        kind: kind.to_string(),
    };
    if let Err(e) = app.emit(SESSION_LOG_EVENT, &payload) {
        tracing::warn!("发送会话日志事件失败: {}", e);
    }
}

// | 会话注册表 |

struct SessionHandle {
    cancel: Arc<AtomicBool>,
    killer: Arc<Mutex<Option<Box<dyn ChildKiller + Send + Sync>>>>,
}

static SESSIONS: LazyLock<Mutex<HashMap<u64, SessionHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

fn drop_session(session_id: u64) {
    if let Ok(mut reg) = SESSIONS.lock() {
        reg.remove(&session_id);
    }
}

/// 应用退出时的兜底：终止所有仍存活的会话，避免 Minecraft 变孤儿进程
pub fn kill_all_sessions() {
    let reg = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    for handle in reg.values() {
        handle.cancel.store(true, Ordering::Relaxed);
        if let Ok(mut killer) = handle.killer.lock() {
            if let Some(k) = killer.as_mut() {
                let _ = k.kill();
            }
        }
    }
}

// | 启动参数 |

struct SessionBundle {
    session_id: u64,
    cancel: Arc<AtomicBool>,
    instance_name: String,
    content: VersionContent,
    dir_name: String,
    config: InstanceConfig,
    profile: GameProfile,
    java_bin: String,
    game_dir: PathBuf,
    natives_dir: PathBuf,
    libraries_root: PathBuf,
    assets_root: PathBuf,
    log_config_path: PathBuf,
    client_jar: PathBuf,
}

// | tauri 命令 |

/// 启动指定实例：快速校验后立即返回 sessionId，后台线程推进后续流程
#[tauri::command]
pub async fn launch_minecraft(app: AppHandle, version_id: String) -> Result<LaunchReply, String> {
    let info = instance::get(&version_id)
        .map_err(|e| format!("读取实例失败: {}", e))?
        .ok_or_else(|| format!("实例 {} 不存在，请先在实例列表完成下载", version_id))?;

    let dir_name = dir_name_of(&info);
    if dir_name.is_empty() {
        return Err("实例目录名无效".to_string());
    }

    let dot_minecraft =
        dirs::dot_minecraft().map_err(|e| format!("定位 .minecraft 失败: {}", e))?;
    let game_dir = dot_minecraft.join("versions").join(&dir_name);
    let client_jar = game_dir.join(format!("{}.jar", version_id));
    if !client_jar.exists() {
        return Err(format!(
            "缺少客户端 {}.jar，请先完成该实例的下载",
            version_id
        ));
    }

    let json_path = game_dir.join(format!("{}.json", version_id));
    let raw = std::fs::read(&json_path)
        .map_err(|_| format!("缺少版本描述文件 <version>.json，请重新下载该实例"))?;
    let content: VersionContent =
        serde_json::from_slice(&raw).map_err(|e| format!("解析 <version>.json 失败: {}", e))?;

    let profile = profile::get_current().await.ok_or_else(|| {
        "尚未选择游戏档案，请先在「游戏档案管理」创建并选择一个离线档案".to_string()
    })?;

    let config = info.config.clone();
    let java_bin = config
        .java_path
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "java".to_string());

    let natives_dir = game_dir.join("natives");
    let libraries_root = dot_minecraft.join("libraries");
    let assets_root = dot_minecraft.join("assets");
    let log_config_path = assets_root
        .join("log_configs")
        .join(&content.logging.client.file.id);

    let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut reg = SESSIONS
            .lock()
            .map_err(|e| format!("会话表锁被污染: {}", e))?;
        reg.insert(
            session_id,
            SessionHandle {
                cancel: cancel.clone(),
                killer: Arc::new(Mutex::new(None)),
            },
        );
    }

    let bundle = SessionBundle {
        session_id,
        cancel,
        instance_name: info.name.clone(),
        content,
        dir_name,
        config,
        profile,
        java_bin,
        game_dir,
        natives_dir,
        libraries_root,
        assets_root,
        log_config_path,
        client_jar,
    };

    thread::Builder::new()
        .name(format!("mc-session-{}", session_id))
        .spawn(move || run_session(app, bundle))
        .map_err(|e| format!("创建启动线程失败: {}", e))?;

    tracing::info!(session_id, version_id, "已创建 Minecraft 启动会话");
    Ok(LaunchReply { session_id })
}

/// 终止指定会话（置取消位 + 杀掉进程），会话会在退出事件中把状态置为「已停止」
#[tauri::command]
pub fn stop_minecraft_session(session_id: u64) -> Result<(), String> {
    let reg = SESSIONS
        .lock()
        .map_err(|e| format!("会话表锁被污染: {}", e))?;
    if let Some(handle) = reg.get(&session_id) {
        handle.cancel.store(true, Ordering::Relaxed);
        if let Ok(mut killer) = handle.killer.lock() {
            if let Some(k) = killer.as_mut() {
                let _ = k.kill();
            }
        }
    }
    Ok(())
}

fn dir_name_of(info: &instance::InstanceInfo) -> String {
    info.path
        .strip_prefix("versions/")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty() && !s.contains('/') && !s.contains('\\'))
        .unwrap_or_else(|| info.version_id.clone())
}

// | 会话主流程 |

fn run_session(app: AppHandle, bundle: SessionBundle) {
    let session_id = bundle.session_id;
    let result = panic::catch_unwind(AssertUnwindSafe(|| run_session_inner(&app, &bundle)));
    if result.is_err() {
        tracing::error!(session_id, "启动会话线程发生 panic");
        emit_state(
            &app,
            session_id,
            "error",
            None,
            None,
            Some("启动守护线程异常终止".to_string()),
        );
    }
    drop_session(session_id);
}

fn run_session_inner(app: &AppHandle, b: &SessionBundle) {
    let sid = b.session_id;
    let mut syslog = |text: String| emit_log(app, sid, text, "system");

    emit_state(app, sid, "launching", None, None, None);
    syslog(format!("[启动] {} · {}", b.instance_name, b.content.id));
    syslog(format!("[目录] {}", b.game_dir.display()));

    if b.cancel.load(Ordering::Relaxed) {
        syslog("[系统] 启动已取消".to_string());
        emit_state(
            app,
            sid,
            "exited",
            None,
            None,
            Some("已停止（启动被取消）".to_string()),
        );
        return;
    }

    // 1. Java 预检
    if let Err(e) = preflight_java(app, b, &mut syslog) {
        syslog(format!("[错误] {}", e));
        emit_state(app, sid, "error", None, None, Some(e.to_string()));
        return;
    }

    // 2. natives 定位与解压（日志写入 Island）
    let tasks = resolve_native_tasks(&b.content, &b.libraries_root);
    if !tasks.is_empty() {
        if b.natives_dir.exists() {
            syslog(format!("[原生库] 已就绪：{}", b.natives_dir.display()));
        } else {
            let missing: Vec<String> = tasks
                .iter()
                .filter(|t| !t.jar_path.exists())
                .map(|t| t.source.clone())
                .collect();
            if !missing.is_empty() {
                let msg = format!(
                    "缺少 {} 个原生依赖 jar（可能是在架构选择修复前下载的），请重新下载该实例后再启动：\n  - {}",
                    missing.len(),
                    missing.join("\n  - ")
                );
                syslog(format!("[错误] {}", msg));
                emit_state(app, sid, "error", None, None, Some(msg));
                return;
            }
            syslog(format!(
                "[原生库] 共 {} 个依赖待解压 → {}",
                tasks.len(),
                b.natives_dir.display()
            ));
            match extract_natives(&tasks, &b.natives_dir) {
                Ok((_, lines)) => {
                    for line in lines {
                        syslog(line);
                    }
                }
                Err(e) => {
                    let msg = format!("解压原生依赖失败: {}", e);
                    syslog(format!("[错误] {}", msg));
                    emit_state(app, sid, "error", None, None, Some(msg));
                    return;
                }
            }
        }
    } else {
        syslog("[原生库] 该版本没有需要解压的原生依赖".to_string());
    }

    if b.cancel.load(Ordering::Relaxed) {
        syslog("[系统] 启动已取消".to_string());
        emit_state(
            app,
            sid,
            "exited",
            None,
            None,
            Some("已停止（启动被取消）".to_string()),
        );
        return;
    }

    // 3. 生成最终启动命令
    let paths = LaunchPaths {
        game_dir: b.game_dir.clone(),
        natives_dir: b.natives_dir.clone(),
        libraries_root: b.libraries_root.clone(),
        assets_root: b.assets_root.clone(),
        log_config_path: b.log_config_path.clone(),
        client_jar: b.client_jar.clone(),
    };
    let ctx = LaunchContext {
        content: &b.content,
        paths: &paths,
        config: &b.config,
        profile: &b.profile,
        java_bin: &b.java_bin,
        launcher_name: LAUNCHER_NAME,
        launcher_version: env!("CARGO_PKG_VERSION"),
    };
    let command = match build_final_command(&ctx) {
        Ok(cmd) => cmd,
        Err(e) => {
            let msg = format!("生成启动命令失败: {}", e);
            syslog(format!("[错误] {}", msg));
            emit_state(app, sid, "error", None, None, Some(msg));
            return;
        }
    };
    for warning in &command.warnings {
        syslog(format!("[提示] {}", warning));
    }
    syslog(format!("[命令] {}", command.display));

    // 4. PTY 拉起 Minecraft 并持续转发日志
    spawn_and_run(app, b, command);
}

fn preflight_java<F>(app: &AppHandle, b: &SessionBundle, syslog: &mut F) -> Result<(), String>
where
    F: FnMut(String),
{
    let _ = app;
    let required = b.content.java_version.major_version as i64;
    match java_major(&b.java_bin) {
        Ok(Some(major)) => {
            syslog(format!(
                "[Java] {}（主版本 {}，{} 需要 {}）",
                b.java_bin, major, b.content.id, required
            ));
            if (major as i64) < required {
                syslog(format!(
                    "[警告] Java 主版本 {} 低于 {} 要求的 {}，可能无法启动",
                    major, b.content.id, required
                ));
            }
            Ok(())
        }
        Ok(None) => {
            syslog(format!(
                "[警告] 无法解析 {} 的版本号，继续尝试启动",
                b.java_bin
            ));
            Ok(())
        }
        Err(e) => Err(format!(
            "无法运行 Java（{}）：{}。请安装 Java 或在实例配置里指定 javaPath。",
            b.java_bin, e
        )),
    }
}

/// 运行 `java -version`，返回主版本号；输出可解析但找不到版本号时返回 Ok(None)
fn java_major(java_bin: &str) -> Result<Option<u32>, String> {
    let out = std::process::Command::new(java_bin)
        .arg("-version")
        .output()
        .map_err(|e| format!("{} -version 启动失败: {}", java_bin, e))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(parse_java_major(&text))
}

/// 解析形如 `openjdk version "21.0.2"` / `java version "1.8.0_..."` 输出的主版本号
fn parse_java_major(text: &str) -> Option<u32> {
    let idx = text.find("version")?;
    let after = text[idx + "version".len()..].trim_start_matches([' ', '"', '=', '\t']);
    let num: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = num.split('.');
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        // Java 1.8 -> 主版本 8
        parts.next().and_then(|s| s.parse().ok()).or(Some(first))
    } else {
        Some(first)
    }
}

fn spawn_and_run(app: &AppHandle, b: &SessionBundle, command: LaunchCommand) {
    let sid = b.session_id;
    let syslog = |text: String| emit_log(app, sid, text, "system");

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 40,
        cols: 160,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(e) => {
            let msg = format!("创建伪终端失败: {}", e);
            syslog(format!("[错误] {}", msg));
            emit_state(app, sid, "error", None, None, Some(msg));
            return;
        }
    };
    let (master, slave) = (pair.master, pair.slave);

    let mut cb = CommandBuilder::new(&command.argv[0]);
    if command.argv.len() > 1 {
        cb.args(&command.argv[1..]);
    }
    cb.cwd(&b.game_dir);

    let mut child = match slave.spawn_command(cb) {
        Ok(child) => child,
        Err(e) => {
            let msg = format!("启动 Minecraft 进程失败: {}", e);
            syslog(format!("[错误] {}", msg));
            emit_state(app, sid, "error", None, None, Some(msg));
            return;
        }
    };
    // slave 已由子进程持有，关闭我们这一侧，保证子进程退出后 master 读到 EOF
    drop(slave);

    // 登记 kill 句柄（供停止/退出兜底），并保有一份本地副本用于读取循环
    let killer = child.clone_killer();
    {
        if let Ok(mut reg) = SESSIONS.lock() {
            if let Some(handle) = reg.get_mut(&sid) {
                *handle.killer.lock().unwrap_or_else(|e| e.into_inner()) = Some(killer);
            }
        }
    }
    // 注意：上面 move 走了 killer；读取循环改用从注册表克隆出的新 handle
    let killer = SESSIONS
        .lock()
        .ok()
        .and_then(|reg| reg.get(&sid).map(|h| h.killer.clone()))
        .unwrap_or_else(|| Arc::new(Mutex::new(None)));

    let pid = child.process_id();
    emit_state(app, sid, "running", pid, None, None);
    syslog(format!(
        "[进程] Minecraft 已启动{}",
        pid.map(|p| format!("（PID {}）", p)).unwrap_or_default()
    ));

    // 记录最后一次启动的实例（供主界面“继续启动”）
    let record = runtime::LastLaunchedInstance {
        version_id: b.content.id.clone(),
        name: b.instance_name.clone(),
        dir: b.dir_name.clone(),
    };
    tauri::async_runtime::spawn(async move {
        if let Err(e) = runtime::record_last_launched(record).await {
            tracing::warn!("记录上次启动失败: {}", e);
        }
    });

    // 读取 master 输出，实时转发（保留原始字节，含 ANSI 颜色）
    let reader = master.try_clone_reader();
    let mut stream = match reader {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("读取伪终端失败: {}", e);
            syslog(format!("[错误] {}", msg));
            let _ = child.kill();
            let _ = child.wait();
            emit_state(app, sid, "exited", pid, Some(-1), Some(msg));
            return;
        }
    };

    let mut assembler = OutAssembler::default();
    let mut killed = false;
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break, // EOF：子进程退出
            Ok(n) => {
                assembler.push(&chunk[..n], &mut |line| emit_log(app, sid, line, "game"));
            }
            Err(_) => break,
        }
        if b.cancel.load(Ordering::Relaxed) && !killed {
            killed = true;
            if let Ok(mut k) = killer.lock() {
                if let Some(k) = k.as_mut() {
                    let _ = k.kill();
                }
            }
        }
    }
    assembler.finish(&mut |line| emit_log(app, sid, line, "game"));
    drop(stream);
    drop(master);

    // 回收进程，得到退出信息
    let wait_result = child.wait();
    let (code, message) = match &wait_result {
        Ok(status) => {
            if killed {
                (
                    Some(status.exit_code() as i32),
                    "已停止（由用户终止）".to_string(),
                )
            } else if status.success() {
                (Some(0), "Minecraft 已正常退出".to_string())
            } else {
                (
                    Some(status.exit_code() as i32),
                    format!("（请不要直接上传窗口内容）Minecraft 异常退出：{}", status),
                )
            }
        }
        Err(e) => (Some(-1), format!("等待 Minecraft 退出失败: {}", e)),
    };
    syslog(format!("[进程] {}", message));
    emit_state(app, sid, "exited", pid, code, Some(message));
}

/// 组装 Minecraft 控制台输出。
///
/// 现代原版通过 log4j `LegacyXMLLayout` 输出 XML 事件流，一个事件可能跨多行，
/// 因此先把 `<log4j:Event>…</log4j:Event>` 整段组装出来，解析为可读终端行再转发；
/// 事件之外的字节（JVM 提示等）按普通行输出，避免把 XML 切得支离破碎。
#[derive(Default)]
struct OutAssembler {
    pending: Vec<u8>,
}

impl OutAssembler {
    fn push(&mut self, chunk: &[u8], emit: &mut dyn FnMut(String)) {
        self.pending.extend_from_slice(chunk);
        self.drain(emit);
    }

    /// EOF 时调用：把残留（含未闭合的事件）尽力按行输出，避免丢日志
    fn finish(&mut self, emit: &mut dyn FnMut(String)) {
        self.drain(emit);
        if !self.pending.is_empty() {
            emit_plain_lines(&self.pending, emit);
            self.pending.clear();
        }
    }

    fn drain(&mut self, emit: &mut dyn FnMut(String)) {
        let mut consumed = 0usize;
        loop {
            let Some(s) = logfmt::find_event_start(&self.pending[consumed..]) else {
                // 缓冲里没有事件起始：全部按普通行输出
                emit_plain_lines(&self.pending[consumed..], emit);
                self.pending.clear();
                return;
            };
            let abs_s = consumed + s;
            if s > 0 {
                emit_plain_lines(&self.pending[consumed..abs_s], emit);
            }
            // 从事件起始开始找结束标签；找不到说明事件跨 chunk，留待下次再组装
            let Some(after) = logfmt::find_event_end_after(&self.pending[abs_s..]) else {
                if abs_s > 0 {
                    self.pending.drain(..abs_s);
                }
                return;
            };
            let abs_e = abs_s + after;
            let block = &self.pending[abs_s..abs_e];
            if let Ok(text) = std::str::from_utf8(block) {
                if let Some(ev) = logfmt::parse_event_block(text) {
                    for line in logfmt::event_to_lines(&ev) {
                        emit(line);
                    }
                } else {
                    // 解析失败原样输出，尽量不丢内容
                    emit_plain_lines(block, emit);
                }
            }
            consumed = abs_e;
            if consumed >= self.pending.len() {
                self.pending.clear();
                return;
            }
        }
    }
}

/// 按行输出普通文本（剥掉行尾 \r；不完整行也即时输出）
fn emit_plain_lines(bytes: &[u8], emit: &mut dyn FnMut(String)) {
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'\n' {
            let mut line = bytes[start..idx].to_vec();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if !line.is_empty() {
                emit(String::from_utf8_lossy(&line).into_owned());
            }
            start = idx + 1;
        }
        idx += 1;
    }
    if start < bytes.len() {
        let mut line = bytes[start..].to_vec();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if !line.is_empty() {
            emit(String::from_utf8_lossy(&line).into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_java_major;
    use super::OutAssembler;

    #[test]
    fn java_major_parsing() {
        assert_eq!(
            parse_java_major("openjdk version \"21.0.2\" 2024-01-16"),
            Some(21)
        );
        assert_eq!(parse_java_major("java version \"1.8.0_392\""), Some(8));
        assert_eq!(parse_java_major("java version \"17.0.9\""), Some(17));
        assert_eq!(parse_java_major("no version here"), None);
    }

    const SAMPLE: &str = concat!(
        "<log4j:Event logger=\"net.minecraft.client.Minecraft\" ",
        "timestamp=\"1788355881585\" level=\"INFO\" thread=\"Render thread\">\n",
        "<log4j:Message><![CDATA[Stopping!]]></log4j:Message>\n",
        "</log4j:Event>\n",
    );

    #[test]
    fn assembler_groups_event_across_chunks() {
        let mut assembler = OutAssembler::default();
        let mut lines: Vec<String> = Vec::new();
        {
            let mut emit = |line: String| lines.push(line);
            // 刻意把事件从中间切开，分两次喂入
            let mid = 40;
            assembler.push(SAMPLE[..mid].as_bytes(), &mut emit);
            assembler.push(SAMPLE[mid..].as_bytes(), &mut emit);
            assembler.finish(&mut emit);
        }
        // 若按“行”错误切分会产生多条 XML 碎片；正确结果应是单行可读日志
        assert_eq!(lines.len(), 1, "整段事件应解析成一行：{lines:?}");
        assert!(lines[0].contains("[Render thread/INFO]: Stopping!"));
    }

    #[test]
    fn assembler_handles_end_tag_split() {
        let mut assembler = OutAssembler::default();
        let mut lines: Vec<String> = Vec::new();
        {
            let mut emit = |line: String| lines.push(line);
            // 把结束标签 </log4j:Event> 拦腰切开
            let marker = "</log4j:Event>";
            let pos = SAMPLE.find(marker).expect("包含结束标签") + 4;
            assembler.push(SAMPLE[..pos].as_bytes(), &mut emit);
            assembler.push(SAMPLE[pos..].as_bytes(), &mut emit);
            assembler.finish(&mut emit);
        }
        assert_eq!(lines.len(), 1, "结束标签被切开也应组装为一行：{lines:?}");
        assert!(lines[0].contains("[Render thread/INFO]: Stopping!"));
    }

    #[test]
    fn assembler_emits_plain_lines() {
        let mut assembler = OutAssembler::default();
        let mut lines: Vec<String> = Vec::new();
        {
            let mut emit = |line: String| lines.push(line);
            assembler.push(
                b"Picked up JAVA_TOOL_OPTIONS\nsecond line\n".as_slice(),
                &mut emit,
            );
            assembler.finish(&mut emit);
        }
        assert_eq!(
            lines,
            vec![
                "Picked up JAVA_TOOL_OPTIONS".to_string(),
                "second line".to_string()
            ]
        );
    }
}
