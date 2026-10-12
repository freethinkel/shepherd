use serde_json::{Value, json};
use shepherd::herdr::Client;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

/// A fake herdr: answers each request line with `reply(request)`; returns the socket path.
fn fake_server(dir: &tempfile::TempDir, reply: fn(&Value) -> Vec<Value>) -> std::path::PathBuf {
    let path = dir.path().join("herdr.sock");
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let (r, mut w) = stream.into_split();
                let mut lines = BufReader::new(r).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let req: Value = serde_json::from_str(&line).unwrap();
                    for out in reply(&req) {
                        w.write_all(format!("{out}\n").as_bytes()).await.unwrap();
                    }
                }
            });
        }
    });
    path
}

#[tokio::test]
async fn call_returns_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| vec![json!({"id": req["id"], "result": {"type": "pong"}})]);
    let result = Client::at(path).call("ping", json!({})).await.unwrap();
    assert_eq!(result, json!({"type": "pong"}));
}

#[tokio::test]
async fn call_error_carries_code_and_message() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| {
        vec![json!({"id": req["id"], "error": {"code": "pane_not_found", "message": "no pane w9:p9"}})]
    });
    let err = Client::at(path).call("pane.get", json!({"pane_id": "w9:p9"})).await.unwrap_err();
    assert_eq!(err.to_string(), "pane_not_found: no pane w9:p9");
}

#[tokio::test]
async fn missing_socket_names_the_path() {
    let err = Client::at("/tmp/demo/no-such/herdr.sock".into()).call("ping", json!({})).await.unwrap_err();
    assert!(err.to_string().contains("/tmp/demo/no-such/herdr.sock"), "{err}");
}

#[tokio::test]
async fn subscription_yields_events() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| {
        assert_eq!(req["method"], "events.subscribe");
        // herdr follows an agent's status only per pane
        assert_eq!(req["params"]["subscriptions"][2], json!({"type": "pane.agent_status_changed", "pane_id": "w1:p1"}));
        vec![
            json!({"id": req["id"], "result": {"type": "subscription_started"}}),
            json!({"event": "tab.created", "data": {"type": "tab_created", "tab": {"tab_id": "w1:t2"}}}),
            json!({"event": "pane.closed", "data": {"type": "pane_closed", "pane_id": "w1:p2", "workspace_id": "w1"}}),
        ]
    });
    let mut rx = Client::at(path).subscribe(&["tab.created", "pane.closed"], &["w1:p1".into()]).await.unwrap();
    assert_eq!(rx.recv().await.unwrap()["event"], "tab.created");
    assert_eq!(rx.recv().await.unwrap()["event"], "pane.closed");
}

#[tokio::test]
async fn subscription_ends_when_server_closes() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| {
        vec![
            json!({"id": req["id"], "result": {"type": "subscription_started"}}),
            json!({"id": req["id"], "error": {"code": "events_lost", "message": "fell behind"}}),
        ]
    });
    let mut rx = Client::at(path).subscribe(&["tab.created"], &[]).await.unwrap();
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn a_missing_or_dead_socket_means_no_server() {
    let dir = tempfile::tempdir().unwrap();
    let missing = Client::at(dir.path().join("herdr.sock")).call("ping", json!({})).await.unwrap_err();
    assert!(shepherd::herdr::no_server(&missing), "{missing:#}");
    // A server that died leaves its socket file behind: connecting is refused.
    let stale = dir.path().join("stale.sock");
    drop(std::os::unix::net::UnixListener::bind(&stale).unwrap());
    let refused = Client::at(stale).call("ping", json!({})).await.unwrap_err();
    assert!(shepherd::herdr::no_server(&refused), "{refused:#}");
    // An answer that is an error is a running server.
    let path = fake_server(&dir, |req| vec![json!({"id": req["id"], "error": {"code": "x", "message": "y"}})]);
    let answered = Client::at(path).call("ping", json!({})).await.unwrap_err();
    assert!(!shepherd::herdr::no_server(&answered));
}
