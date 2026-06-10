// Prevents additional console window on Windows in release, safe no-op on macOS
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    portkill_lib::run()
}
