mod ports;

#[cfg(target_os = "macos")]
mod macos_panel;

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

#[tauri::command]
fn list_ports() -> Result<Vec<ports::PortEntry>, String> {
    ports::list_listening_ports()
}

#[tauri::command]
fn kill_process(pid: u32, port: u16) -> Result<(), String> {
    ports::kill_pid_on_port(pid, port)
}

#[tauri::command]
fn focus_process(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos_panel::focus_pid(pid);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        Err("only supported on macOS".into())
    }
}

fn toggle_panel(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(panel) = app.get_webview_panel("main") {
            if panel.is_visible() {
                panel.order_out(None);
            } else if !macos_panel::recently_hidden() {
                // Position below the menu bar *before* showing
                macos_panel::position_panel_below_menubar(app);
                // Frontend refreshes the port list on this event
                let _ = app.emit("panel-shown", ());
                panel.show();
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(window) = app.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = app.emit("panel-shown", ());
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}

pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .invoke_handler(tauri::generate_handler![
            list_ports,
            kill_process,
            focus_process
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                // Menu bar app: no Dock icon
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                // NSWindow → non-activating NSPanel, so the panel can show
                // over fullscreen apps (CanJoinAllSpaces + FullScreenAuxiliary)
                macos_panel::swizzle_to_menubar_panel(app.app_handle());
                // Auto-hide on resign key / app switch / Space switch
                macos_panel::setup_panel_listeners(app.app_handle());
            }

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

            TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("PortKill")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_panel(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running PortKill");
}
