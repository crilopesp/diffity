//! What `#[command]` (src-tauri/web-command) generates calls into: reading the browser's arguments and handing
//! the command the server's state, app handle and channels.

use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::tauri_shim::ipc::Channel;
use super::tauri_shim::{AppHandle, State};
use super::Server;
use crate::core::AppError;
use crate::AppState;

/// The command's result as JSON, or its error as the JSON Tauri would reject `invoke` with.
pub type Reply<'a> = Pin<Box<dyn Future<Output = Result<Value, Value>> + Send + 'a>>;

pub trait WebCommand {
    const NAME: &'static str;
    fn run<'a>(server: &'a Server, args: Value) -> Reply<'a>;
}

pub fn reply<T: Serialize>(result: Result<T, AppError>) -> Result<Value, Value> {
    let error = |e: AppError| serde_json::to_value(e).unwrap_or(Value::Null);
    let value = result.map_err(error)?;
    serde_json::to_value(value).map_err(|e| error(AppError::internal(e.to_string())))
}

/// One argument, by its camelCase name; a missing one reads as `null`, so an `Option` parameter can be left out.
pub fn argument<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, AppError> {
    let value = args.get(key).cloned().unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|e| AppError::invalid(format!("invalid argument `{key}`: {e}")))
}

pub fn state(server: &Server) -> State<'_, AppState> {
    State(&server.state)
}

pub fn app_handle(server: &Server) -> AppHandle {
    AppHandle { events: server.events.clone() }
}

/// The browser sends a `Channel` as `__CHANNEL__:<id>`; its messages go back over the event socket with that id.
pub fn channel<T: Serialize>(server: &Server, args: &Value, key: &str) -> Result<Channel<T>, AppError> {
    let raw: String = argument(args, key)?;
    let id = raw
        .strip_prefix("__CHANNEL__:")
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| AppError::invalid(format!("`{key}` is not a channel")))?;
    Ok(Channel::new(id, server.events.clone()))
}
