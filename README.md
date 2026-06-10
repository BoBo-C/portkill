<div align="center">

<img src="src-tauri/icons/icon.png" width="96" alt="PortKill icon" />

# PortKill

**See every listening port. Kill the process in one click.**

No more `lsof -i :3000` → `kill -9`. It lives in your menu bar.

[![macOS](https://img.shields.io/badge/platform-macOS-black?logo=apple)](https://github.com/BoBo-C/portkill)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Vue 3](https://img.shields.io/badge/Vue-3-42b883?logo=vuedotjs&logoColor=white)](https://vuejs.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

English · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

<img src="docs/screenshot.png" width="360" alt="PortKill screenshot" />

</div>

## Why

Every frontend developer knows the drill: `Error: port 3000 is already in use`, then a trip to Google for the `lsof` incantation, then `kill -9`. PortKill turns that into a single click from the menu bar.

## Features

- ⚡ **One-click kill** — every listening TCP port with its process name, PID, and memory usage; hover and hit Kill
- 📌 **Dev ports pinned** — 3000, 5173, 8080 and friends always sort to the top
- 🎯 **Bring app to front** — right-click an entry to focus the app that owns the process (walks up the parent chain, so a `node` started from VS Code focuses VS Code)
- 🖥 **A real menu bar citizen** — non-activating panel that opens on any Space, even over fullscreen apps, never covers the menu bar, and hides when it loses focus
- 🌗 Dark mode follows the system · UI in English / 中文 / 日本語

## Install

Download the latest `.dmg` from [Releases](https://github.com/BoBo-C/portkill/releases), drag PortKill to Applications.

The app is not notarized yet. If macOS refuses to open it:

```sh
xattr -cr /Applications/PortKill.app
```

## Usage

Click the ⚡ icon in the menu bar. The list refreshes every time the panel opens (or via the refresh button).

| Action | Result |
| --- | --- |
| Click **Kill** | SIGKILL the process, list refreshes |
| Right-click a row | **Bring app to front** — focus the owning app |
| Hover a row | Full process name, PID, and bind address in the tooltip |

> Processes owned by root (or another user) can't be killed without elevated privileges — PortKill will tell you instead of failing silently.

## How it works

- Port list: `lsof -nP -iTCP -sTCP:LISTEN -F pcn`, parsed in Rust; IPv4/IPv6 duplicates deduped
- Memory: one batched `ps -o pid=,rss=` call per refresh
- Kill: `SIGTERM` first so the process gets a chance to clean up, escalating to `SIGKILL` if it's still alive after 500ms; the pid/port pair is revalidated right before signaling
- Fullscreen support: the window is swizzled to a non-activating `NSPanel` (`CanJoinAllSpaces` + `FullScreenAuxiliary`) via [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel)

## Build from source

Prerequisites: Node 18+, Rust 1.80+, Xcode Command Line Tools.

```sh
git clone https://github.com/BoBo-C/portkill.git
cd portkill
npm install
npm run tauri dev     # development
npm run tauri build   # .app + .dmg in src-tauri/target/release/bundle/
```

```
src/                    Vue frontend (App.vue, i18n.js, styles.css)
src-tauri/src/lib.rs    Tray icon, panel toggle, command registration
src-tauri/src/ports.rs  lsof / ps parsing, kill
src-tauri/src/macos_panel.rs  NSPanel swizzle, positioning, focus app
```

## License

[MIT](LICENSE)
