//! The part of Tauri that `src/commands` uses, for `diffity-server`, where the crate is also known as `tauri`
//! (see `extern crate self as tauri` in src/lib.rs). The commands compile unchanged and their events, channel
//! messages and notifications go to the browsers.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use serde_json::json;

pub use diffity_web_command::command;

use super::events::Events;

/// What a command reaches the app through; here, the browsers.
#[derive(Clone)]
pub struct AppHandle {
    pub(super) events: Events,
}

/// `State<'_, AppState>`: the server's state, borrowed for the command.
pub struct State<'r, T>(pub(super) &'r T);

impl<T> Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}

impl<'r, T> State<'r, T> {
    pub fn inner(&self) -> &'r T {
        self.0
    }
}

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub trait Emitter {
    fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), Error>;
}

impl Emitter for AppHandle {
    fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), Error> {
        let payload = serde_json::to_value(payload).map_err(|e| Error(e.to_string()))?;
        self.events.emit(event, payload);
        Ok(())
    }
}

/// The browser is not one of the app's windows, so there is never one to find.
pub trait Manager {
    fn get_webview_window(&self, label: &str) -> Option<WebviewWindow>;
}

impl Manager for AppHandle {
    fn get_webview_window(&self, _label: &str) -> Option<WebviewWindow> {
        None
    }
}

pub struct WebviewWindow;

impl WebviewWindow {
    pub fn set_badge_count(&self, _count: Option<i64>) -> Result<(), Error> {
        Ok(())
    }
}

pub mod ipc {
    use super::*;

    /// A command's stream to the browser (an agent's replies, a setup command's output), over its event socket.
    pub struct Channel<T> {
        id: u64,
        next_index: Arc<AtomicU64>,
        events: Events,
        message: PhantomData<fn(T)>,
    }

    impl<T> Clone for Channel<T> {
        fn clone(&self) -> Self {
            Self { id: self.id, next_index: self.next_index.clone(), events: self.events.clone(), message: PhantomData }
        }
    }

    impl<T: Serialize> Channel<T> {
        pub(in crate::server) fn new(id: u64, events: Events) -> Self {
            Self { id, next_index: Arc::new(AtomicU64::new(0)), events, message: PhantomData }
        }

        pub fn send(&self, message: T) -> Result<(), Error> {
            let message = serde_json::to_value(message).map_err(|e| Error(e.to_string()))?;
            // Tauri's `Channel` in the browser puts messages back in order by this index.
            let index = self.next_index.fetch_add(1, Ordering::SeqCst);
            self.events.channel(self.id, json!({ "message": message, "index": index }));
            Ok(())
        }
    }
}

/// `tauri_plugin_notification`: the notification shows in the browser, if the reader allowed it.
pub trait NotificationExt {
    fn notification(&self) -> Notifications;
}

impl NotificationExt for AppHandle {
    fn notification(&self) -> Notifications {
        Notifications { app: self.clone() }
    }
}

pub struct Notifications {
    app: AppHandle,
}

impl Notifications {
    pub fn builder(&self) -> NotificationBuilder {
        NotificationBuilder { app: self.app.clone(), title: String::new(), body: String::new() }
    }
}

pub struct NotificationBuilder {
    app: AppHandle,
    title: String,
    body: String,
}

impl NotificationBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn show(self) -> Result<(), Error> {
        self.app.emit(super::NOTIFICATION_EVENT, json!({ "title": self.title, "body": self.body }))
    }
}
