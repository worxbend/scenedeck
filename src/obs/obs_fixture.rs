//! Loopback OBS WebSocket server pieces shared by protocol tests.
//!
//! Speaks the wire format with `tokio-websockets` directly, never `obws`, so
//! fixtures exercise the real client end to end.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

pub(crate) type MockSocket = WebSocketStream<TcpStream>;

/// Bind an ephemeral loopback port for one mock OBS.
pub(crate) async fn listen() -> (TcpListener, u16) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Accept one client and complete Hello → Identify → Identified.
pub(crate) async fn accept(listener: TcpListener) -> MockSocket {
    let (tcp, _) = listener.accept().await.unwrap();
    let (_, mut socket) = ServerBuilder::new().accept(tcp).await.unwrap();
    socket
        .send(Message::text(
            json!({"op":0,"d":{"obsWebSocketVersion":"5.5.0","rpcVersion":1}}).to_string(),
        ))
        .await
        .unwrap();
    socket.next().await.unwrap().unwrap();
    socket
        .send(Message::text(
            json!({"op":2,"d":{"negotiatedRpcVersion":1}}).to_string(),
        ))
        .await
        .unwrap();
    socket
}

/// `GetVersion` response data.
pub(crate) fn version(image_formats: &[&str]) -> Value {
    json!({
        "obsStudioVersion": "31.0.0",
        "obsWebSocketVersion": "5.5.0",
        "rpcVersion": 1,
        "availableRequests": [],
        "supportedImageFormats": image_formats,
        "platform": "mock",
        "platformDescription": "mock",
    })
}
