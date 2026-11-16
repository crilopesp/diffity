//! Commands whose app version lives with Tauri's windows (src/open_link.rs), answered for the browser.

/// Links the desktop app was opened with (`diffity://…`, the `diffity` command); a browser opens them as URLs.
#[tauri::command]
pub fn take_open_requests() -> Vec<serde_json::Value> {
    Vec::new()
}
