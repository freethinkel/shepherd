//! A Rex host on the network as a local socket. [`super::ws`] reads and writes one socket from two threads at once,
//! which a TLS session can't be split into, so the network side runs on a small runtime of its own and the WebSocket
//! gets the other end of a socket pair.
use std::io;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::{self, pki_types::ServerName};

/// As long as `rex` waits for a host by default.
const TIMEOUT: Duration = Duration::from_secs(10);

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread().worker_threads(1).thread_name("rex-tunnel").enable_all().build().expect("tokio")
});

/// A tailnet's HTTPS certificates are Let's Encrypt's, so the public roots are enough.
static TLS: LazyLock<tokio_rustls::TlsConnector> = LazyLock::new(|| {
    let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("ring has TLS 1.2 and 1.3")
        .with_root_certificates(roots)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
});

/// Connects to `host` at `port`, over TLS when `tls`, and gives the local end that carries it both ways. Returns once
/// the connection and its handshake are made, or with why they can't be.
pub fn open(host: &str, port: u16, tls: bool) -> io::Result<UnixStream> {
    let (ours, theirs) = UnixStream::pair()?;
    theirs.set_nonblocking(true)?;
    let (made, answer) = std::sync::mpsc::channel();
    let host = host.to_string();
    RUNTIME.spawn(async move {
        let opened = async {
            let local = tokio::net::UnixStream::from_std(theirs)?;
            let net = timed(tokio::time::timeout(TIMEOUT, tokio::net::TcpStream::connect((host.as_str(), port))).await)?;
            net.set_nodelay(true)?;
            io::Result::Ok((local, net))
        };
        let (local, net) = match opened.await {
            Ok(both) => both,
            Err(e) => return drop(made.send(Err(e))),
        };
        if !tls {
            let _ = made.send(Ok(()));
            return carry(local, net).await;
        }
        let name = match ServerName::try_from(host.clone()) {
            Ok(name) => name,
            Err(_) => return drop(made.send(Err(io::Error::other(format!("{host} is not a host name"))))),
        };
        match timed(tokio::time::timeout(TIMEOUT, TLS.connect(name, net)).await) {
            Ok(net) => {
                let _ = made.send(Ok(()));
                carry(local, net).await
            }
            Err(e) => drop(made.send(Err(e))),
        }
    });
    answer.recv().map_err(|_| io::Error::other("the connection's runtime is gone"))??;
    Ok(ours)
}

fn timed<T>(result: Result<io::Result<T>, tokio::time::error::Elapsed>) -> io::Result<T> {
    result.unwrap_or_else(|_| Err(io::ErrorKind::TimedOut.into()))
}

/// Until either side ends: a close from the WebSocket ends the connection, the server's end ends the WebSocket's reader.
async fn carry(mut local: tokio::net::UnixStream, mut net: impl AsyncRead + AsyncWrite + Unpin) {
    let _ = tokio::io::copy_bidirectional(&mut local, &mut net).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn bytes_cross_both_ways() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            let mut got = [0; 4];
            peer.read_exact(&mut got).unwrap();
            peer.write_all(&[got, *b"-ack"].concat()).unwrap();
        });
        let mut local = open("127.0.0.1", port, false).unwrap();
        local.write_all(b"ping").unwrap();
        let mut back = String::new();
        local.read_to_string(&mut back).unwrap();
        assert_eq!(back, "ping-ack", "and the server's close ends the local end");
    }

    /// Live, on the internet: `cargo test --lib rex::tunnel::tests::tls -- --ignored`.
    #[test]
    #[ignore]
    fn tls_carries_https() {
        let mut local = open("example.com", 443, true).unwrap();
        local.write_all(b"HEAD / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n").unwrap();
        let mut answer = String::new();
        local.read_to_string(&mut answer).unwrap();
        assert!(answer.starts_with("HTTP/1.1 "), "{answer}");
        assert!(open("self-signed.badssl.com", 443, true).is_err(), "a certificate no root vouches for is refused");
    }

    #[test]
    fn nothing_listening_is_an_error() {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // freed again
        assert!(open("127.0.0.1", port, false).is_err());
    }
}
