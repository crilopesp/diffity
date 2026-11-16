use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::SharedServer;

/// What the app sends with `app.emit(...)` (`repo-changed`, `threads-changed`…) and over `Channel`s, delivered
/// to every browser; each one runs the callbacks it registered.
#[derive(Clone)]
pub struct Events {
    sender: broadcast::Sender<String>,
}

impl Events {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn emit(&self, event: &str, payload: Value) {
        // With no browser connected there is nobody to tell, which is fine.
        let _ = self.sender.send(json!({ "event": event, "payload": payload }).to_string());
    }

    /// A `Channel` message (`{ message, index }`) for the browser callback with this id.
    pub fn channel(&self, id: u64, payload: Value) {
        let _ = self.sender.send(json!({ "channel": id, "payload": payload }).to_string());
    }
}

/// `GET /api/events`: the browser keeps this socket open and receives every event.
pub async fn socket(ws: WebSocketUpgrade, State(server): State<SharedServer>) -> Response {
    let receiver = server.events.sender.subscribe();
    ws.on_upgrade(move |socket| forward(socket, receiver))
}

async fn forward(mut socket: WebSocket, mut receiver: broadcast::Receiver<String>) {
    loop {
        tokio::select! {
            event = receiver.recv() => match event {
                Ok(text) => {
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        return;
                    }
                }
                // A browser too slow to keep up misses some events; the next refresh catches it up.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            },
            message = socket.recv() => {
                if !matches!(message, Some(Ok(_))) {
                    return;
                }
            }
        }
    }
}
