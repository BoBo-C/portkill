<div align="center">

<img src="src-tauri/icons/icon.png" width="96" alt="PortKill 图标" />

# PortKill

**看见每一个被监听的端口,一键干掉占用它的进程。**

告别 `lsof -i :3000` → `kill -9`,它就在你的菜单栏里。

[![macOS](https://img.shields.io/badge/platform-macOS-black?logo=apple)](https://github.com/BoBo-C/portkill)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Vue 3](https://img.shields.io/badge/Vue-3-42b883?logo=vuedotjs&logoColor=white)](https://vuejs.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[English](README.md) · 简体中文 · [日本語](README.ja.md)

<img src="docs/screenshot.png" width="360" alt="PortKill 截图" />

</div>

## 为什么做它

每个前端开发者都经历过:`Error: port 3000 is already in use`,然后搜 `lsof` 命令,再 `kill -9`。PortKill 把这套流程变成菜单栏里的一次点击。

## 功能

- ⚡ **一键 kill** — 列出所有监听中的 TCP 端口及进程名、PID、内存占用,悬停点 Kill 即可
- 📌 **常用端口置顶** — 3000、5173、8080 等开发端口永远排在最前
- 🎯 **前置应用** — 右键条目可激活该进程所属的 App(沿父进程链查找,VS Code 终端里跑的 `node` 会前置 VS Code)
- 🖥 **地道的菜单栏应用** — 非激活面板,任意桌面、甚至全屏 App 之上都能弹出,不遮挡菜单栏,失焦自动隐藏
- 🌗 深色模式跟随系统 · 界面支持 English / 中文 / 日本語

## 安装

从 [Releases](https://github.com/BoBo-C/portkill/releases) 下载最新 `.dmg`,拖入「应用程序」。

应用暂未公证,如果 macOS 拒绝打开:

```sh
xattr -cr /Applications/PortKill.app
```

## 使用

点击菜单栏的 ⚡ 图标。每次打开面板自动刷新(也可点刷新按钮)。

| 操作 | 结果 |
| --- | --- |
| 点击 **结束** | SIGKILL 该进程并刷新列表 |
| 右键条目 | **前置应用** — 激活所属 App |
| 悬停条目 | tooltip 显示完整进程名、PID 和绑定地址 |

> root(或其他用户)的进程没有权限直接 kill —— PortKill 会明确提示,而不是静默失败。

## 实现原理

- 端口列表:Rust 侧解析 `lsof -nP -iTCP -sTCP:LISTEN -F pcn`,IPv4/IPv6 重复项去重
- 内存:每次刷新只调一次 `ps -o pid=,rss=` 批量获取
- Kill:直接 `SIGKILL` —— 快而干脆,对付卡死的 dev server 正合适
- 全屏支持:窗口通过 [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) 转为非激活 `NSPanel`(`CanJoinAllSpaces` + `FullScreenAuxiliary`)

## 从源码构建

依赖:Node 18+、Rust 1.80+、Xcode Command Line Tools。

```sh
git clone https://github.com/BoBo-C/portkill.git
cd portkill
npm install
npm run tauri dev     # 开发
npm run tauri build   # 产物在 src-tauri/target/release/bundle/(.app + .dmg)
```

```
src/                    Vue 前端(App.vue、i18n.js、styles.css)
src-tauri/src/lib.rs    托盘图标、面板开关、命令注册
src-tauri/src/ports.rs  lsof / ps 解析、kill
src-tauri/src/macos_panel.rs  NSPanel 转换、定位、前置应用
```

## 许可证

[MIT](LICENSE)
