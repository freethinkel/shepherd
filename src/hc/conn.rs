//! One framed connection to herdr-core through `hc connect`, as the Herdr app holds one: JSON-RPC on channel 0, a
//! stream of its own for each attached terminal and for the events (herdr-core's schema/README.md). `hc connect`
//! starts the core when it isn't running.
use anyhow::anyhow;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tokio::sync::oneshot;

pub const DATA: u8 = 0;
const CREDIT: u8 = 1;
pub const CLOSE: u8 = 2;
pub const RESIZE: u8 = 3;
pub const FACTS: u8 = 5;
const HEADER: usize = 9;
/// Nothing Shepherd asks of the core takes longer: `terminal.send` waits out its ~3 s of quiet.
const TIMEOUT: Duration = Duration::from_secs(10);

/// What a stream carries.
pub enum Item {
    Data(Vec<u8>),
    /// The core ended the stream; the payload says why (`{"reason": "exited", ...}`).
    Closed(Value),
    /// The connection to the core dropped: the program most likely runs on.
    Lost,
}

/// Where a stream's items go. Runs on the reader thread, in order.
pub type Sink = Box<dyn FnMut(Item) + Send>;

struct Pending {
    reply: oneshot::Sender<anyhow::Result<Value>>,
    /// Takes the stream the reply's `channel` opens, before any frame after the reply is read.
    sink: Option<Sink>,
}

#[derive(Default)]
struct Routes {
    pending: HashMap<u64, Pending>,
    streams: HashMap<u32, Sink>,
    lost: bool,
}

pub struct Core {
    writer: mpsc::Sender<Vec<u8>>,
    routes: Arc<Mutex<Routes>>,
    next_id: AtomicU64,
    child: Mutex<Child>,
}

fn encode(channel: u32, kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(&channel.to_le_bytes());
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn read_frame(r: &mut impl Read) -> Option<(u32, u8, Vec<u8>)> {
    let mut h = [0u8; HEADER];
    r.read_exact(&mut h).ok()?;
    let channel = u32::from_le_bytes(h[0..4].try_into().ok()?);
    let len = u32::from_le_bytes(h[5..9].try_into().ok()?) as usize;
    let mut payload = vec![0; len];
    r.read_exact(&mut payload).ok()?;
    Some((channel, h[4], payload))
}

fn reply(message: &Value) -> anyhow::Result<Value> {
    match message.get("error") {
        Some(error) => Err(anyhow!("{}: {}", error["data"]["kind"].as_str().unwrap_or("error"), error["message"].as_str().unwrap_or(""))),
        None => Ok(message["result"].clone()),
    }
}

impl Core {
    pub fn connect(binary: &str) -> anyhow::Result<Arc<Core>> {
        let mut child = crate::process::command(binary)
            .arg("connect")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("cannot run {binary}: {e}. Is herdr-core installed?"))?;
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("hc connect has no stdin"))?;
        let mut reader = BufReader::new(child.stdout.take().ok_or_else(|| anyhow!("hc connect has no stdout"))?);
        stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"hello\",\"params\":{\"framed\":true}}\n")?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        reply(&serde_json::from_str(&line).map_err(|_| anyhow!("herdr-core did not say hello"))?)?;

        // writes never block the caller, the window's thread among them
        let (writer, frames) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            while let Ok(frame) = frames.recv() {
                if stdin.write_all(&frame).is_err() {
                    break;
                }
            }
        });
        let routes = Arc::new(Mutex::new(Routes::default()));
        let (shared, credit) = (routes.clone(), writer.clone());
        std::thread::spawn(move || read_loop(&mut reader, &shared, &credit));
        Ok(Arc::new(Core { writer, routes, next_id: AtomicU64::new(1), child: Mutex::new(child) }))
    }

    pub fn lost(&self) -> bool {
        self.routes.lock().unwrap().lost
    }

    pub fn send(&self, channel: u32, kind: u8, payload: &[u8]) {
        let _ = self.writer.send(encode(channel, kind, payload));
    }

    /// More room on a stream than it starts with.
    pub fn credit(&self, channel: u32, bytes: u32) {
        self.send(channel, CREDIT, &bytes.to_le_bytes());
    }

    /// Sends a request. `sink` takes the stream its reply opens.
    pub fn ask(&self, method: &str, params: Value, sink: Option<Sink>) -> oneshot::Receiver<anyhow::Result<Value>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut routes = self.routes.lock().unwrap();
            if routes.lost {
                let _ = tx.send(Err(anyhow!("lost herdr-core")));
                return rx;
            }
            routes.pending.insert(id, Pending { reply: tx, sink });
        }
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.send(0, DATA, request.to_string().as_bytes());
        rx
    }

    pub async fn call(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        match tokio::time::timeout(TIMEOUT, self.ask(method, params, None)).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("lost herdr-core")),
            Err(_) => Err(anyhow!("{method}: herdr-core did not answer")),
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        // only the bridge ends: the core and its terminals run on
        let mut child = self.child.lock().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn read_loop(reader: &mut impl Read, routes: &Mutex<Routes>, writer: &mpsc::Sender<Vec<u8>>) {
    while let Some((channel, kind, payload)) = read_frame(reader) {
        let mut routes = routes.lock().unwrap();
        if channel == 0 {
            let Ok(message) = serde_json::from_slice::<Value>(&payload) else { continue };
            let Some(pending) = message["id"].as_u64().and_then(|id| routes.pending.remove(&id)) else { continue };
            if let (Some(sink), Some(opened)) = (pending.sink, message["result"]["channel"].as_u64()) {
                routes.streams.insert(opened as u32, sink);
            }
            let _ = pending.reply.send(reply(&message));
            continue;
        }
        match kind {
            DATA => {
                // ponytail: credit on read, not once painted, so a stalled window queues here; credit from the pane
                // to get the core's repaint instead if that ever shows
                let _ = writer.send(encode(channel, CREDIT, &(payload.len() as u32).to_le_bytes()));
                if let Some(sink) = routes.streams.get_mut(&channel) {
                    sink(Item::Data(payload));
                }
            }
            CLOSE => {
                if let Some(mut sink) = routes.streams.remove(&channel) {
                    sink(Item::Closed(serde_json::from_slice(&payload).unwrap_or(Value::Null)));
                }
            }
            _ => {}
        }
    }
    let mut routes = routes.lock().unwrap();
    routes.lost = true;
    for (_, mut sink) in routes.streams.drain() {
        sink(Item::Lost);
    }
    routes.pending.clear(); // their receivers wake with an error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let bytes = encode(7, DATA, b"hello");
        assert_eq!(bytes.len(), HEADER + 5);
        assert_eq!(read_frame(&mut bytes.as_slice()), Some((7, DATA, b"hello".to_vec())));
        assert_eq!(read_frame(&mut &bytes[..HEADER + 2]), None, "a frame cut short");
    }

    #[test]
    fn a_stream_opens_before_its_first_frame_is_read() {
        let mut wire = Vec::new();
        wire.extend(encode(0, DATA, br#"{"id":1,"result":{"channel":5}}"#));
        wire.extend(encode(5, DATA, b"first paint"));
        wire.extend(encode(5, CLOSE, br#"{"reason":"exited"}"#));
        let routes = Mutex::new(Routes::default());
        let (tx, mut rx) = oneshot::channel();
        let got = Arc::new(Mutex::new(Vec::new()));
        let log = got.clone();
        let sink: Sink = Box::new(move |item| {
            log.lock().unwrap().push(match item {
                Item::Data(bytes) => String::from_utf8(bytes).unwrap(),
                Item::Closed(why) => why["reason"].as_str().unwrap().to_string(),
                Item::Lost => "lost".into(),
            })
        });
        routes.lock().unwrap().pending.insert(1, Pending { reply: tx, sink: Some(sink) });
        let (writer, credits) = mpsc::channel();
        read_loop(&mut wire.as_slice(), &routes, &writer);
        assert_eq!(*got.lock().unwrap(), ["first paint", "exited"]);
        assert_eq!(rx.try_recv().unwrap().unwrap()["channel"], 5);
        assert_eq!(credits.try_recv().unwrap(), encode(5, CREDIT, &11u32.to_le_bytes()), "what was read is credited");
    }

    #[test]
    fn a_core_that_refuses_says_why() {
        let refused = json!({"id": 1, "error": {"code": -32000, "message": "no terminal t9", "data": {"kind": "not_found"}}});
        assert_eq!(reply(&refused).unwrap_err().to_string(), "not_found: no terminal t9");
    }
}
