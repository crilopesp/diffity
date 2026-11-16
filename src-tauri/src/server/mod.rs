//! `diffity-server`: the app's commands over HTTP, so the web build runs in a browser against a remote machine.
//!
//! The browser only shows; every command runs here, with this machine's repositories, agents and `gh`.
//! The web build replaces Tauri's IPC (`window.__TAURI_INTERNALS__`, see `src-web/`) with
//! `POST /api/invoke/<command>` and a WebSocket for events, so the UI is the app's, unchanged.

mod events;
pub mod glue;
mod handler;
pub mod tauri_shim;
mod web_commands;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::agents::AgentManager;
use crate::core::store::Store;
use crate::core::{task, AppError};
use crate::github::GithubService;
use crate::AppState;

pub use handler::NOT_IN_WEB;
use events::Events;

/// A notification for the reader (`tauri_plugin_notification`), shown by the browser.
pub const NOTIFICATION_EVENT: &str = "diffity-web://notification";

pub struct ServerOptions {
    pub address: SocketAddr,
    /// The web build (`pnpm build:web` → `dist-web/`).
    pub web_dir: PathBuf,
}

pub struct Server {
    pub state: AppState,
    pub events: Events,
}

type SharedServer = Arc<Server>;

pub async fn serve(options: ServerOptions) -> anyhow::Result<()> {
    let events = Events::new();
    let state = open_state(&data_dir()?, &events)?;
    let server = Arc::new(Server { state, events });
    let index = options.web_dir.join("index.html");
    if !index.exists() {
        anyhow::bail!("{} does not exist: build the web app with `pnpm build:web`", index.display());
    }
    let app = Router::new()
        .route("/api/invoke/{command}", post(invoke))
        .route("/api/events", get(events::socket))
        .fallback_service(ServeDir::new(&options.web_dir).fallback(ServeFile::new(index)))
        .layer(CompressionLayer::new())
        .with_state(server);
    let listener = tokio::net::TcpListener::bind(options.address).await?;
    println!("Diffity at http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

/// `POST /api/invoke/<command>` with the arguments Tauri's `invoke` would get; answers what the command returns,
/// or its error, which the browser rejects with like Tauri does.
async fn invoke(State(server): State<SharedServer>, UrlPath(command): UrlPath<String>, Json(args): Json<Value>) -> Response {
    // Its own task, so the command finishes even when the browser goes away (a closed tab must not stop an agent
    // halfway, as quitting would not in the app).
    let task = tokio::spawn(async move { dispatch(&server, &command, args).await });
    let result = task.await.unwrap_or_else(|e| Err(json!(AppError::internal(format!("the command failed: {e}")))));
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let status = match error.get("code").and_then(Value::as_str) {
                Some("unsupported") => StatusCode::NOT_IMPLEMENTED,
                Some("internal") => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::UNPROCESSABLE_ENTITY,
            };
            (status, Json(error)).into_response()
        }
    }
}

async fn dispatch(server: &Server, name: &str, args: Value) -> Result<Value, Value> {
    if let Some(answer) = plugin_answer(name) {
        return Ok(answer);
    }
    match handler::find(name) {
        Some(run) => run(server, args).await,
        None => Err(json!(unsupported(name))),
    }
}

/// Every command the browser can run, for `tests/web_commands.rs`.
pub fn command_names() -> Vec<&'static str> {
    handler::names().collect()
}

/// The Tauri plugins the UI asks the server about; the ones about the browser window it answers itself.
fn plugin_answer(name: &str) -> Option<Value> {
    match name {
        "plugin:app|version" => Some(json!(env!("CARGO_PKG_VERSION"))),
        "plugin:app|tauri_version" => Some(json!("web")),
        // The server is updated where it runs, not from the browser.
        "plugin:updater|check" => Some(Value::Null),
        // The browser uses its own fonts.
        "plugin:system-fonts|get_system_fonts" => Some(json!([])),
        _ => None,
    }
}

/// The app's own data folder (Tauri's `app_data_dir`), so a server keeps the same database and MCP socket the
/// app would have on this machine.
fn data_dir() -> anyhow::Result<PathBuf> {
    let identifier = if crate::dev_build() { "com.diffity.app.dev" } else { "com.diffity.app" };
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))
    };
    Ok(base.join(identifier))
}

/// The app's `init_state` (src/lib.rs) without Tauri: events go to the browsers instead of the windows.
fn open_state(data_dir: &Path, events: &Events) -> anyhow::Result<AppState> {
    std::fs::create_dir_all(data_dir)?;
    let store = Arc::new(Store::open(data_dir.join("diffity.db")).map_err(|e| anyhow::anyhow!(e))?);
    if let Err(e) = task::fail_interrupted(&store) {
        tracing::warn!("could not reset interrupted tasks: {}", e.message);
    }
    let agents = Arc::new(AgentManager::new(store.clone(), data_dir.to_path_buf(), crate::mcp_binary_path()));
    let threads_events = events.clone();
    agents.on_threads_changed(Arc::new(move |session_id: &str| {
        threads_events.emit("threads-changed", json!({ "sessionId": session_id }));
    }));
    let bridge_agents = agents.clone();
    tokio::spawn(async move {
        if let Err(e) = bridge_agents.start_bridge().await {
            tracing::warn!("could not start the MCP socket: {}", e.message);
        }
    });
    let github = Arc::new(GithubService::new(store.clone()));
    let worktrees_root = task::default_root(crate::dev_build());
    Ok(AppState { store, agents, github, worktrees_root })
}

fn unsupported(command: &str) -> AppError {
    let message = if NOT_IN_WEB.contains(&command) {
        format!("{command} acts on the server's desktop, so the web app does not offer it")
    } else {
        format!("{command} is not available in the web app")
    };
    AppError::new("unsupported", message)
}
