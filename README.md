<h1 align=center>Too Many Minecraft Launcher</h1>

<p align=center><sub><del><b>🥵<i>Minecraft 启动器真的太多啦~！</i></b></del></sub></p>

## 介绍

> <del>_注：目前项目仍作为空壳，还不能实现完整的 Minecraft 启动器体验_</del>
>
> 已完成启动功能，现在正在进行 UI 交互设计，完善启动器体验中。

### 正在实现

#### 后端

##### 离线登录+启动 Minecraft

- [x] 原版较新版本 Minecraft 下载
- [x] Minecraft 实例管理
- [x] 游戏档案管理
- [x] JVM 参数生成
- [x] Minecraft 启动（离线）

##### ModLoader 适配

- [ ] NeoForge 加载器安装
- [ ] Fabric 加载器安装
- [ ] Fabric Api 附带安装
- [ ] Forge 加载器安装
- [ ] 一键安装信雅互联版本

#### 前端

- [x] MC 下载进度监测
- [ ] UI 初步设计
- [ ] Modrinth api 适配
- [ ] Curseforge api 适配

## 推荐的 IDE 配置

本项目建议使用 [VS Code](https://code.visualstudio.com/) 开发，安装工作区所有的建议插件可获得最佳开发体验。

> 建议使用 `bun` 作为包管理器，且 `rustup` 1.97.1 版本以上

### 建议的插件

- [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)（Vue 支持）
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)（Tauri 支持）
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)（Rust 支持）
- [Project Actions](https://marketplace.visualstudio.com/items?itemName=Julynx.project-actions)（快速启动本地项目测试）
- [mdui](https://marketplace.visualstudio.com/items?itemName=zdhxiong.mdui)（UI 库语法联想支持）
- [TARUS](https://marketplace.visualstudio.com/items?itemName=mvoof.tarus-vscode-extension)（Tauri `invoke<>()` 关联支持）
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)（TOML 支持）

## 开源许可

除特别注明的部分外，代码均使用 GPL-v3.0 协议进行开源。

## 部分素材来源

- Minecraft 草方块与泥土：[Minecraft 中文 Wiki](https://zh.minecraft.wiki/)
