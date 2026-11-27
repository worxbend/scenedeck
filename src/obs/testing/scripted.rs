//! Scripted loopback OBS: answers requests from a script and counts them.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use super::wire::{self, MockSocket, Reply};
use crate::infra::sync::lock_or_recover;
use crate::obs::client::ObsClient;
use crate::obs::event::ObsEventStream;

/// Requests the mock OBS received, keyed by `requestType`.
#[derive(Clone, Default)]
pub(crate) struct RequestCounts(Arc<Mutex<HashMap<String, usize>>>);

impl RequestCounts {
    pub(crate) fn snapshot(&self) -> HashMap<String, usize> {
        lock_or_recover(&self.0).clone()
    }

    fn record(&self, request_type: &str) {
        *lock_or_recover(&self.0)
            .entry(request_type.to_owned())
            .or_default() += 1;
    }
}

enum Script {
    /// Each request must match the next entry; the server stops after the last.
    Ordered(VecDeque<(&'static str, Reply)>),
    /// Any order and any number of times; serves until the client disconnects.
    ByType(HashMap<&'static str, Reply>),
}

impl Script {
    fn next_reply(&mut self, request_type: &str) -> Reply {
        match self {
            Self::Ordered(steps) => {
                let (expected, reply) = steps.pop_front().unwrap();
                assert_eq!(request_type, expected);
                reply
            }
            Self::ByType(responders) => responders
                .get(request_type)
                .unwrap_or_else(|| panic!("unscripted OBS request {request_type}"))
                .clone(),
        }
    }

    fn is_exhausted(&self) -> bool {
        matches!(self, Self::Ordered(steps) if steps.is_empty())
    }

    /// A disconnect ends a by-type script; an ordered one must be used up.
    fn assert_complete(&self) {
        if let Self::Ordered(steps) = self {
            let left: Vec<_> = steps
                .iter()
                .map(|(request_type, _)| *request_type)
                .collect();
            assert!(
                left.is_empty(),
                "client disconnected before requesting {left:?}"
            );
        }
    }
}

/// Sends OBS event frames (see [`wire::event`]) to the connected client,
/// in push order, between request replies.
pub(crate) struct EventFeed(UnboundedSender<Value>);

impl EventFeed {
    pub(crate) fn push(&self, frame: Value) {
        self.0
            .send(frame)
            .expect("scripted OBS stopped before the event was pushed");
    }
}

/// A client connected to a [`ScriptedObs`], with the client's OBS event
/// stream and a feed that pushes events into it.
pub(crate) struct ScriptedSession {
    pub(crate) client: ObsClient,
    pub(crate) events: ObsEventStream,
    pub(crate) feed: EventFeed,
    /// Yields every request frame once the server stops.
    pub(crate) server: JoinHandle<Vec<Value>>,
}

/// A loopback OBS that answers `GetVersion` (with PNG support) and then
/// follows its script.
pub(crate) struct ScriptedObs {
    script: Script,
    counts: RequestCounts,
}

impl ScriptedObs {
    /// Strict order: request N must be the Nth script entry.
    pub(crate) fn ordered(script: impl IntoIterator<Item = (&'static str, Reply)>) -> Self {
        Self::new(Script::Ordered(
            std::iter::once(handshake()).chain(script).collect(),
        ))
    }

    /// Match by `requestType`; each responder answers every request of its type.
    pub(crate) fn by_type(responders: impl IntoIterator<Item = (&'static str, Reply)>) -> Self {
        Self::new(Script::ByType(
            std::iter::once(handshake()).chain(responders).collect(),
        ))
    }

    fn new(script: Script) -> Self {
        Self {
            script,
            counts: RequestCounts::default(),
        }
    }

    /// A handle that keeps counting after [`Self::connect`] consumes the server.
    pub(crate) fn counts(&self) -> RequestCounts {
        self.counts.clone()
    }

    /// Start the server and connect a real client to it. The handle yields
    /// every request frame once the server stops.
    pub(crate) async fn connect(self) -> (ObsClient, JoinHandle<Vec<Value>>) {
        let session = self.connect_with_events().await;
        (session.client, session.server)
    }

    /// [`Self::connect`], keeping the client's event stream and a feed that
    /// pushes OBS events into it.
    pub(crate) async fn connect_with_events(self) -> ScriptedSession {
        let (listener, port) = wire::listen().await;
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (feed, pushed) = unbounded_channel();
        let server = tokio::spawn(async move {
            let mut socket = wire::accept(listener).await;
            let requests = self.serve(&mut socket, pushed).await;
            // Keep the socket open until `connect` returned, even for an empty
            // script; an error only means the test already dropped the client.
            let _ = ready_rx.await;
            requests
        });
        let (client, events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
        ready_tx.send(()).unwrap();
        ScriptedSession {
            client,
            events,
            feed: EventFeed(feed),
            server,
        }
    }

    async fn serve(
        mut self,
        socket: &mut MockSocket,
        mut pushed: UnboundedReceiver<Value>,
    ) -> Vec<Value> {
        let mut requests = Vec::new();
        while !self.script.is_exhausted() {
            // A dropped feed disables its branch; requests keep being served.
            let request = tokio::select! {
                request = next_request(socket) => request,
                Some(frame) = pushed.recv() => {
                    wire::send_json(socket, &frame).await;
                    continue;
                }
            };
            let Some(request) = request else {
                self.script.assert_complete();
                break;
            };
            let request_type = request["d"]["requestType"].as_str().unwrap().to_owned();
            self.counts.record(&request_type);
            let reply = self.script.next_reply(&request_type);
            wire::send_json(socket, &wire::response(&request, &reply)).await;
            requests.push(request);
        }
        requests
    }
}

/// The `GetVersion` exchange `ObsClient::connect` performs first.
fn handshake() -> (&'static str, Reply) {
    ("GetVersion", Reply::Ok(wire::version(&["png"])))
}

/// The next request frame, or `None` once the client disconnects.
async fn next_request(socket: &mut MockSocket) -> Option<Value> {
    loop {
        let message = socket.next().await?.ok()?;
        if let Some(text) = message.as_text() {
            return Some(serde_json::from_str(text).unwrap());
        }
    }
}

/// [`ScriptedObs::ordered`] for a script where every request succeeds.
pub(crate) async fn mock_client(
    script: Vec<(&'static str, Value)>,
) -> (ObsClient, JoinHandle<Vec<Value>>) {
    let script = script
        .into_iter()
        .map(|(request_type, data)| (request_type, Reply::Ok(data)));
    ScriptedObs::ordered(script).connect().await
}

/// Withhold every ordinary response while observing all requests until EOF.
pub(crate) async fn unresponsive_client() -> (
    ObsClient,
    tokio::sync::mpsc::UnboundedReceiver<Value>,
    JoinHandle<Vec<Value>>,
) {
    let (listener, port) = wire::listen().await;
    let (seen_tx, seen_rx) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut socket = wire::accept(listener).await;
        let version = wire::read_json(&mut socket).await;
        let reply = Reply::Ok(wire::version(&[]));
        wire::send_json(&mut socket, &wire::response(&version, &reply)).await;
        let mut requests = Vec::new();
        while let Some(request) = next_request(&mut socket).await {
            // The test may stop watching early; keep recording regardless.
            let _ = seen_tx.send(request.clone());
            requests.push(request);
        }
        requests
    });
    let (client, _events) = ObsClient::connect("127.0.0.1", port, None).await.unwrap();
    (client, seen_rx, server)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn by_type_counts_each_request_type() {
        let obs = ScriptedObs::by_type([
            (
                "GetVideoSettings",
                Reply::Ok(
                    json!({"fpsNumerator":30,"fpsDenominator":1,"baseWidth":1920,"baseHeight":1080,"outputWidth":1920,"outputHeight":1080}),
                ),
            ),
            (
                "GetSourceScreenshot",
                Reply::Ok(json!({"imageData":"data:image/png;base64,fixture"})),
            ),
        ]);
        let counts = obs.counts();
        let (client, _server) = obs.connect().await;
        let not_cancelled = std::sync::atomic::AtomicBool::new(false);

        client
            .get_scene_thumbnail("Program", &not_cancelled)
            .await
            .unwrap();

        let counts = counts.snapshot();
        assert_eq!(counts["GetVersion"], 1);
        assert_eq!(counts["GetVideoSettings"], 1);
        assert_eq!(counts["GetSourceScreenshot"], 1);
    }
}
