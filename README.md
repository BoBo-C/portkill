# PortKill ⚡

macOS menu bar tool for frontend developers: see every listening port, kill the process in one click. No more `lsof -i :3000` + `kill -9`.

## Features

- Menu bar icon → panel listing all listening TCP ports with process name + PID
- One-click kill (SIGKILL) per entry
- Common dev ports (3000, 5173, 8080, …) pinned to the top
- Dark mode, English/中文/日本語 UI

## Stack

Tauri 2 + Vue 3. Port info comes from `lsof -nP -iTCP -sTCP:LISTEN -F pcn` on the Rust side.

## Development

Prerequisites: Node 18+, Rust (stable, 1.80+), Xcode Command Line Tools.

```sh
npm install
npm run tauri dev
```

The app has no Dock icon (Accessory activation policy) — look for the ⚡ icon in the menu bar. The panel refreshes each time it opens and hides when it loses focus.

## Build & release

```sh
npm run tauri build
```

Outputs `.app` and `.dmg` under `src-tauri/target/release/bundle/`. For distribution outside your own machine, sign and notarize (set `APPLE_SIGNING_IDENTITY` etc. — see [Tauri macOS distribution docs](https://tauri.app/distribute/)).

## Project layout

```
src/                  Vue frontend (App.vue, i18n.js, styles.css)
src-tauri/src/lib.rs  Tray icon, panel show/hide, command registration
src-tauri/src/ports.rs  lsof parsing + kill
```

## Notes

- Processes owned by other users (or root) can't be killed without elevated privileges; the UI surfaces the error.
- IPv4/IPv6 duplicates of the same pid+port are deduped.
