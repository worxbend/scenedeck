//! Loopback OBS WebSocket server pieces shared by protocol tests.
//!
//! Speaks the wire format with `tokio-websockets` directly, never `obws`, so
//! fixtures exercise the real client end to end.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

pub(crate) type MockSocket = WebSocketStream<TcpStream>;

/// How the mock OBS answers one request.
#[derive(Clone, Debug)]
pub(crate) enum Reply {
    /// `result: true`; `{}` sends no `responseData`, as OBS does for setters.
    Ok(Value),
    /// `result: false` with an OBS request status code and comment.
    Err { code: u64, comment: String },
}

impl Reply {
    /// A rejection with the comment the fixture has always used.
    pub(crate) fn err(code: u64) -> Self {
        Self::Err {
            code,
            comment: "mock missing source".into(),
        }
    }
}

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
    accept_with_identify(listener).await.0
}

/// [`accept`], also returning the client's Identify frame for assertions.
pub(crate) async fn accept_with_identify(listener: TcpListener) -> (MockSocket, Value) {
    let (tcp, _) = listener.accept().await.unwrap();
    let (_, mut socket) = ServerBuilder::new().accept(tcp).await.unwrap();
    send_json(
        &mut socket,
        &json!({"op":0,"d":{"obsWebSocketVersion":"5.5.0","rpcVersion":1}}),
    )
    .await;
    let identify = read_json(&mut socket).await;
    send_json(&mut socket, &json!({"op":2,"d":{"negotiatedRpcVersion":1}})).await;
    (socket, identify)
}

/// Read the next frame as JSON; panics when the client closed the socket.
pub(crate) async fn read_json(socket: &mut MockSocket) -> Value {
    let message = socket.next().await.unwrap().unwrap();
    serde_json::from_str(message.as_text().unwrap()).unwrap()
}

/// Send one JSON frame.
pub(crate) async fn send_json(socket: &mut MockSocket, frame: &Value) {
    socket.send(Message::text(frame.to_string())).await.unwrap();
}

/// The `op: 7` response frame answering `request` with `reply`.
pub(crate) fn response(request: &Value, reply: &Reply) -> Value {
    let status = match reply {
        Reply::Ok(_) => json!({"result":true,"code":100}),
        Reply::Err { code, comment } => json!({"result":false,"code":code,"comment":comment}),
    };
    let mut frame = json!({"op":7,"d":{
        "requestId": request["d"]["requestId"],
        "requestType": request["d"]["requestType"],
        "requestStatus": status,
    }});
    if let Reply::Ok(data) = reply {
        if *data != json!({}) {
            frame["d"]["responseData"] = data.clone();
        }
    }
    frame
}

/// The `op: 5` event frame; `intent` is the event's subscription bit.
pub(crate) fn event(event_type: &str, intent: u32, data: Value) -> Value {
    json!({"op":5,"d":{"eventType":event_type,"eventIntent":intent,"eventData":data}})
}

/// The `requestData` the client sent in an `op: 6` request frame.
pub(crate) fn request_data(request: &Value) -> &Value {
    &request["d"]["requestData"]
}

/// A `GetInputList` row with a fixed UUID.
pub(crate) fn input(name: &str) -> Value {
    json!({"inputName":name,"inputUuid":"00000000-0000-0000-0000-000000000001","inputKind":"mock","unversionedInputKind":"mock"})
}

/// A `GetSceneItemList` row for an input source.
pub(crate) fn scene_item(id: i64, name: &str) -> Value {
    json!({"sceneItemId":id,"sceneItemIndex":0,"sourceName":name,"sourceType":"OBS_SOURCE_TYPE_INPUT","inputKind":"mock"})
}

/// A `GetSceneList` row with a fixed UUID.
pub(crate) fn scene(name: &str) -> Value {
    json!({"sceneName":name,"sceneUuid":"00000000-0000-0000-0000-000000000001","sceneIndex":0})
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
