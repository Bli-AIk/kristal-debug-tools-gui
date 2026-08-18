# kristal-debug-tools-gui

[kristal-debug-tools](https://github.com/Bli-AIk/kristal-debug-tools) 的图形界面（Deltarune 像素风），给没装 `just` 的 Windows 用户也能可视化操作 Kristal 项目：

- **启动游戏** —— 可视化填 `--encounter / --wave / --wave-force / --tp / --mercy`；检测到 `kristalI18n` 库后才显示 `--lang`
- **任务列表** —— 枚举 justfile 的 recipe 并运行（`just` 已编译进程序，无需安装）
- **章节配置** —— 从引擎 `configs/chapter*.json` 自动读取基线，图形化改 `config.kristal` 覆盖；值标签跟随 GUI 语言（JSONC 保留注释原样写入）
- **项目初始化** —— 模板项目 `start.sh --name` 图形化触发
- 游戏和任务都在**新的交互式终端窗口**里跑，输出不进 GUI

技术栈：Tauri v2 + React + TypeScript。游戏启动是 `bin/kristal-run` 的 Rust 移植（`src-tauri/src/launcher.rs`）。

## Kristal Version Support

| `kristal`                                                                                                                          | `kristal-debug-tools-gui`                                  |
| ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| [v0.10.0](https://github.com/KristalTeam/Kristal/commit/752bc0688ba97ca8a256ba9125b7e05a1ca6edbd) (`752bc068`, 2026-06-23)     | v0.1.5                                                     |
| [v0.11.0-dev](https://github.com/KristalTeam/Kristal/commit/f62afea63ccab02f468c24ac0d096bd8a2c9aa81) (`f62afea`, 2026-08-16) | v0.2.0（发布后；源码 ref 为 `feat/v0.11-dev`）             |

## 最终用户：一键运行（零工具链）

Windows / Linux 都只需要 **LÖVE 装好并进 PATH**（Git Bash 进 PATH 后构建任务也能跑）。不需要 just、Rust、Node。

| 方式         | 说明                                                                                                                                      |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 下载 Release | 去 [Releases](https://github.com/Bli-AIk/kristal-debug-tools-gui/releases) 拿对应架构（x64/arm64）的裸二进制，双击运行                    |
| `just gui`   | 项目里 `just --justfile libraries/kristal-debug-tools/justfile gui`，按引擎 `VERSION` 下载匹配的 release 到 `.tools/gui/`（SHA256 校验） |
| `gui.cmd`    | 库目录里双击 `gui.cmd`（Windows），逻辑同上                                                                                               |

`just gui` 只运行当前引擎对应的固定 release：`0.10.0 -> v0.1.5`，`0.11.0-dev -> v0.2.0`。未知引擎版本会停止并说明原因，不会请求 GitHub 的全局 `latest`。目标 release 尚未上传时请稍后重试。

`just gui-dev` 会 checkout 当前引擎对应的 GUI tag 或 `feat/v0.11-dev` 分支后从源码启动；源码目录有本地改动或无法快进时会停止，避免覆盖开发中的工作。

## 开发者

```bash
just gui-dev      # 构建 sidecar + npm run tauri dev（热更新）
just check        # cargo check
just sidecar      # 编译 kristal-run 到 src-tauri/binaries/（tauri externalBin）
just build        # 全量构建 release bundle
```

注意：`npm run tauri dev` 只编译主 bin，**任务列表依赖 kristal-run sidecar**——`just gui-dev` 会先构建它。

结构：

```
src-tauri/
  src/bin/kristal-run.rs   # 命令行 sidecar：just-task / just-dump / 游戏启动（console 子系统）
  src/launcher.rs          # bin/kristal-run 的 Rust 移植（flag 解析、project/engine 解析、love 查找）
  src/config.rs            # project `mod.json` 的 JSONC 保留编辑、章节配置、模板检测
  src/tasks.rs             # just --dump json 解析（embedded/system 两种来源）
  src/term.rs              # 新终端窗口 spawn（kitty/gnome-terminal/cmd start）
src/                       # React 前端（中英双语、DPR 缩放、章节配置页）
```

## 发布

[release-please](https://github.com/googleapis/release-please) 管理版本（`Cargo.toml` / `tauri.conf.json` / `package.json` 三处同步）。合并 release PR 后 CI 自动构建并上传：

- `kristal-debug-tools-gui-{windows,linux}-{x64,arm64}[.exe]` + `kristal-run-{windows,linux}-{x64,arm64}[.exe]`（裸二进制，launcher 按宿主架构自动选择）
- `checksums-{windows,linux}-{x64,arm64}.txt`（SHA256，下载脚本校验用）
- deb / nsis 安装包（顺手打包的，不是主推路径）

二进制未签名，SmartScreen 提示时选 "More info → Run anyway"。
