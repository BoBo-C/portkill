//! Converts the main window into a non-activating NSPanel so it can appear
//! over fullscreen apps. A plain NSWindow can never join a fullscreen Space,
//! no matter its level — see tauri-apps/tauri#5793, #11488.
#![allow(deprecated)]

use std::ffi::CString;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_nspanel::{
    block::ConcreteBlock,
    cocoa::{
        appkit::{NSMainMenuWindowLevel, NSWindowCollectionBehavior},
        base::{id, nil, NO},
        foundation::{NSPoint, NSRect},
    },
    objc::{class, msg_send, sel, sel_impl},
    panel_delegate, ManagerExt, WebviewWindowExt,
};

#[allow(non_upper_case_globals)]
const NSWindowStyleMaskNonActivatingPanel: i32 = 1 << 7;

/// Timestamp (ms) of the last auto-hide. Clicking the tray icon while the
/// panel is open hides it (resign key) right before the click event arrives —
/// without this guard the click would instantly re-open it.
static LAST_HIDE_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn recently_hidden() -> bool {
    now_ms().saturating_sub(LAST_HIDE_MS.load(Ordering::Relaxed)) < 300
}

pub fn swizzle_to_menubar_panel(app_handle: &AppHandle) {
    let delegate = panel_delegate!(PortKillPanelDelegate {
        window_did_resign_key
    });

    let window = app_handle.get_webview_window("main").unwrap();
    let panel = window.to_panel().unwrap();

    let handle = app_handle.clone();
    delegate.set_listener(Box::new(move |delegate_name: String| {
        if delegate_name.as_str() == "window_did_resign_key" {
            let _ = handle.emit("panel_did_resign_key", ());
        }
    }));

    // One level above the menu bar so it floats over fullscreen apps.
    // The panel is positioned *below* the menu bar (TrayBottomCenter),
    // so it never visually covers it — unlike NSPopUpMenuWindowLevel (101),
    // which drew over menu bar dropdowns.
    panel.set_level(NSMainMenuWindowLevel + 1);

    // Non-activating: the panel takes key input without activating the app
    // or disturbing the fullscreen app underneath.
    panel.set_style_mask(NSWindowStyleMaskNonActivatingPanel);

    panel.set_collection_behaviour(
        NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary,
    );

    panel.set_delegate(delegate);
}

pub fn setup_panel_listeners(app_handle: &AppHandle) {
    fn hide_panel(handle: &AppHandle) {
        // Don't hide if our own app just became frontmost
        if get_frontmost_app_pid() == app_pid() {
            return;
        }
        if let Ok(panel) = handle.get_webview_panel("main") {
            if panel.is_visible() {
                LAST_HIDE_MS.store(now_ms(), Ordering::Relaxed);
                panel.order_out(None);
            }
        }
    }

    let handle = app_handle.clone();
    app_handle.listen_any("panel_did_resign_key", move |_| {
        hide_panel(&handle);
    });

    let handle = app_handle.clone();
    let callback = Box::new(move || {
        hide_panel(&handle);
    });

    // Hide when another app is activated or the user switches Spaces
    register_workspace_listener(
        "NSWorkspaceDidActivateApplicationNotification".into(),
        callback.clone(),
    );
    register_workspace_listener(
        "NSWorkspaceActiveSpaceDidChangeNotification".into(),
        callback,
    );
}

/// Position the panel just *below* the menu bar, horizontally centered on the
/// mouse (≈ the tray icon that was clicked). Uses NSScreen.visibleFrame, which
/// excludes the menu bar, so the panel can never cover it.
pub fn position_panel_below_menubar(app_handle: &AppHandle) {
    let window = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };
    let handle: id = match window.ns_window() {
        Ok(ptr) => ptr as _,
        Err(_) => return,
    };

    unsafe {
        // AppKit global coordinates: origin bottom-left, y grows upward
        let mouse: NSPoint = msg_send![class!(NSEvent), mouseLocation];

        // Find the screen under the cursor (where the tray was clicked)
        let screens: id = msg_send![class!(NSScreen), screens];
        let count: usize = msg_send![screens, count];
        let mut screen: id = msg_send![class!(NSScreen), mainScreen];
        for i in 0..count {
            let candidate: id = msg_send![screens, objectAtIndex: i];
            let frame: NSRect = msg_send![candidate, frame];
            if mouse.x >= frame.origin.x
                && mouse.x <= frame.origin.x + frame.size.width
                && mouse.y >= frame.origin.y
                && mouse.y <= frame.origin.y + frame.size.height
            {
                screen = candidate;
                break;
            }
        }

        // visibleFrame excludes the menu bar (and Dock)
        let visible: NSRect = msg_send![screen, visibleFrame];
        let mut frame: NSRect = msg_send![handle, frame];

        // Panel top = top of visible area = bottom edge of the menu bar
        frame.origin.y = visible.origin.y + visible.size.height - frame.size.height;

        // Centered on the cursor, clamped to the screen
        let min_x = visible.origin.x;
        let max_x = visible.origin.x + visible.size.width - frame.size.width;
        frame.origin.x = (mouse.x - frame.size.width / 2.0).clamp(min_x, max_x);

        let _: () = msg_send![handle, setFrame: frame display: NO];
    }
}

fn register_workspace_listener(name: String, callback: Box<dyn Fn()>) {
    let workspace: id = unsafe { msg_send![class!(NSWorkspace), sharedWorkspace] };
    let notification_center: id = unsafe { msg_send![workspace, notificationCenter] };

    let block = ConcreteBlock::new(move |_notif: id| {
        callback();
    });
    let block = block.copy();

    let name: id =
        unsafe { msg_send![class!(NSString), stringWithCString: CString::new(name).unwrap()] };

    unsafe {
        let _: () = msg_send![
            notification_center,
            addObserverForName: name object: nil queue: nil usingBlock: block
        ];
    }
}

/// Bring the GUI app owning `pid` to the front. Listening processes are often
/// windowless children (node, vite…), so walk up the parent chain until we
/// find a pid that NSRunningApplication knows about (e.g. VS Code, iTerm).
pub fn focus_pid(pid: u32) -> Result<(), String> {
    let mut current = pid;
    for _ in 0..10 {
        let app: id = unsafe {
            msg_send![
                class!(NSRunningApplication),
                runningApplicationWithProcessIdentifier: current as i32
            ]
        };
        if app != nil {
            // 2 = NSApplicationActivationPolicyProhibited (background daemon)
            let policy: i64 = unsafe { msg_send![app, activationPolicy] };
            if policy != 2 {
                // NSApplicationActivateAllWindows | ActivateIgnoringOtherApps
                let _: bool = unsafe { msg_send![app, activateWithOptions: 3u64] };
                return Ok(());
            }
        }
        match crate::ports::parent_pid(current) {
            Some(ppid) => current = ppid,
            None => break,
        }
    }
    Err("no app with windows found for this process".into())
}

fn app_pid() -> i32 {
    let process_info: id = unsafe { msg_send![class!(NSProcessInfo), processInfo] };
    unsafe { msg_send![process_info, processIdentifier] }
}

fn get_frontmost_app_pid() -> i32 {
    let workspace: id = unsafe { msg_send![class!(NSWorkspace), sharedWorkspace] };
    let frontmost_application: id = unsafe { msg_send![workspace, frontmostApplication] };
    unsafe { msg_send![frontmost_application, processIdentifier] }
}
