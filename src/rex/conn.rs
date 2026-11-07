//! A control connection to Rex's server: requests and their responses, and its events to a callback.
use super::ws;
use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

/// The revision of the API this speaks, which the server translates to from a newer one (the doc at `/llms.txt`).
const API_REVISION: u64 = 9;
const TIMEOUT: Duration = Duration::from_secs(10);

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<anyhow::Result<Value>>>>>;

pub struct Control {
    /// The server it reached, where a pane's stream goes too.
    pub server: super::Endpoint,
    send: ws::Sender,
    pending: Pending,
    next: AtomicU64,
    lost: Arc<AtomicBool>,
}

impl Control {
    /// Connects as a new client to the first of `servers` that answers. `event` runs on the reader thread, in order,
    /// with each event's inner `payload`.
    pub fn open(servers: &[super::Endpoint], event: impl FnMut(&Value) + Send + 'static) -> anyhow::Result<Arc<Self>> {
        let mut tried = Vec::new();
        let (server, (send, mut receive)) = loop {
            let Some(server) = servers.get(tried.len()) else {
                return Err(match tried.pop() {
                    Some(last) if tried.is_empty() => last,
                    Some(last) => last.context(format!("no address of the server answered ({} tried)", servers.len())),
                    None => anyhow!("no address to reach Rex at"),
                });
            };
            match ws::connect(server, "/api/control", "rex.control.v1") {
                Ok(both) => break (server.clone(), both),
                Err(e) => tried.push(anyhow::Error::from(e).context(format!("{server:?}"))),
            }
        };
        let client = json!({"kind": "app", "name": "Shepherd", "version": env!("CARGO_PKG_VERSION")});
        send.text(&json!({"type": "hello", "api_revision": API_REVISION, "api_min_revision": API_REVISION, "client": client}).to_string())?;
        let welcome: Value = serde_json::from_slice(&receive.next().map_err(|why| anyhow!("Rex refused Shepherd: {why}"))?)?;
        if welcome["type"] != "welcome" || welcome["protocol"] != "rex.control.v1" {
            bail!("Rex answered with {}", welcome["type"]);
        }
        let (pending, lost) = (Pending::default(), Arc::new(AtomicBool::new(false)));
        let (routes, gone) = (pending.clone(), lost.clone());
        std::thread::Builder::new().name("rex-control".into()).spawn(move || read_loop(receive, routes, gone, event))?;
        Ok(Arc::new(Self { server, send, pending, next: AtomicU64::new(1), lost }))
    }

    pub fn lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }

    fn ask(&self, method: &str, payload: &Value) -> anyhow::Result<oneshot::Receiver<anyhow::Result<Value>>> {
        let id = self.next.fetch_add(1, Ordering::Relaxed).to_string();
        let (reply, answer) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), reply);
        self.send.text(&json!({"type": "request", "id": id, "method": method, "payload": payload}).to_string())?;
        Ok(answer)
    }

    pub async fn call(&self, method: &str, payload: Value) -> anyhow::Result<Value> {
        let answer = self.ask(method, &payload)?;
        match tokio::time::timeout(TIMEOUT, answer).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => bail!("lost Rex"),
            Err(_) => bail!("Rex did not answer {method}"),
        }
    }

    /// [`Self::call`] from a thread of its own.
    pub fn blocking_call(&self, method: &str, payload: Value) -> anyhow::Result<Value> {
        self.ask(method, &payload)?.blocking_recv().map_err(|_| anyhow!("lost Rex"))?
    }

    pub fn close(&self) {
        self.send.close();
    }
}

fn read_loop(mut receive: ws::Receiver, pending: Pending, lost: Arc<AtomicBool>, mut event: impl FnMut(&Value)) {
    while let Ok(message) = receive.next() {
        let Ok(message) = serde_json::from_slice::<Value>(&message) else { continue };
        match message["type"].as_str() {
            Some("response") => {
                let reply = message["id"].as_str().and_then(|id| pending.lock().unwrap().remove(id));
                if let Some(reply) = reply {
                    let _ = reply.send(answer(message));
                }
            }
            Some("event") => event(&message["payload"]),
            _ => {} // a newer server's: the API says to skip it
        }
    }
    lost.store(true, Ordering::Relaxed);
    event(&Value::Null);
    pending.lock().unwrap().clear();
}

/// A response's result, or its error as herdr's are read: `code: message`.
fn answer(mut response: Value) -> anyhow::Result<Value> {
    match response.get("error") {
        Some(error) if !error.is_null() => Err(anyhow!("{}: {}", error["code"].as_str().unwrap_or("error"), error["message"].as_str().unwrap_or(""))),
        _ => Ok(response["payload"].take()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_is_its_payload_or_its_error() {
        assert_eq!(answer(json!({"id": "1", "payload": {"a": 1}})).unwrap(), json!({"a": 1}));
        assert_eq!(answer(json!({"id": "1"})).unwrap(), Value::Null);
        let error = json!({"id": "2", "error": {"code": "not_found", "message": "block not found", "status": 404}});
        assert_eq!(answer(error).unwrap_err().to_string(), "not_found: block not found");
    }
}
