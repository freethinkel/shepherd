//! The little of WebSocket that Rex's local socket needs: a client, whole messages, no compression. It is two ends of
//! one socket, one reader and one writer that don't wait for each other, which a WebSocket crate's single object isn't.
use crate::herdr::socket::{self, Stream};
use std::io::{self, BufRead, BufReader, Read, Write};
use super::Endpoint;
use std::sync::{Arc, Mutex};

const CONTINUATION: u8 = 0;
const TEXT: u8 = 1;
const BINARY: u8 = 2;
const CLOSE: u8 = 8;
const PING: u8 = 9;
const PONG: u8 = 10;
/// Rex's largest message is 1 MiB: anything far past that is not Rex.
const MAX_MESSAGE: u64 = 64 << 20;

#[derive(Clone)]
pub struct Sender(Arc<Mutex<Stream>>);

pub struct Receiver {
    read: BufReader<Stream>,
    pong: Sender,
}

/// Opens `target` (`/api/control`) on the server at `server`, speaking `protocol`.
pub fn connect(server: &Endpoint, target: &str, protocol: &str) -> io::Result<(Sender, Receiver)> {
    let (mut stream, host) = match server {
        Endpoint::Socket(path) => (socket::connect(path)?, "localhost".to_string()),
        Endpoint::Net { host, port, tls } => (net(host, *port, *tls)?, format!("{host}:{port}")),
    };
    // the key only proves to a browser that a server understood it: any 16 bytes do, and the peer here is the user's
    write!(
        stream,
        "GET {target} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: c2hlcGhlcmQtc2hlcGhlcg==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: {protocol}\r\n\r\n"
    )?;
    let sender = Sender(Arc::new(Mutex::new(stream.try_clone()?)));
    let mut read = BufReader::new(stream);
    let mut line = String::new();
    read.read_line(&mut line)?;
    if line.split(' ').nth(1) != Some("101") {
        return Err(io::Error::other(format!("Rex refused {target}: {}", line.trim())));
    }
    while line != "\r\n" {
        line.clear();
        if read.read_line(&mut line)? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
    }
    Ok((sender.clone(), Receiver { read, pong: sender }))
}

#[cfg(unix)]
fn net(host: &str, port: u16, tls: bool) -> io::Result<Stream> {
    super::tunnel::open(host, port, tls)
}

#[cfg(not(unix))]
fn net(host: &str, _: u16, _: bool) -> io::Result<Stream> {
    Err(io::Error::other(format!("reaching Rex at {host} over the network needs macOS or Linux")))
}

impl Sender {
    pub fn text(&self, text: &str) -> io::Result<()> {
        self.frame(TEXT, text.as_bytes())
    }

    pub fn binary(&self, bytes: &[u8]) -> io::Result<()> {
        self.frame(BINARY, bytes)
    }

    /// A close frame, which ends the stream or client at once, then the socket, which ends the reader.
    pub fn close(&self) {
        let _ = self.frame(CLOSE, &1000u16.to_be_bytes());
        let _ = self.0.lock().unwrap().shutdown(std::net::Shutdown::Both);
    }

    fn frame(&self, op: u8, data: &[u8]) -> io::Result<()> {
        let mut out = Vec::with_capacity(data.len() + 14);
        out.push(0x80 | op);
        match data.len() {
            n @ 0..=125 => out.push(0x80 | n as u8),
            n @ 126..=0xffff => {
                out.push(0x80 | 126);
                out.extend((n as u16).to_be_bytes());
            }
            n => {
                out.push(0x80 | 127);
                out.extend((n as u64).to_be_bytes());
            }
        }
        // ponytail: an all-zero mask, so the data goes as it is; masking is against proxies, and here are none
        out.extend([0; 4]);
        out.extend_from_slice(data);
        self.0.lock().unwrap().write_all(&out)
    }
}

impl Receiver {
    /// The next whole message, answering pings on the way; `Err` says why there are no more.
    pub fn next(&mut self) -> Result<Vec<u8>, String> {
        let mut message = Vec::new();
        loop {
            let (fin, op, data) = self.frame().map_err(|e| e.to_string())?;
            match op {
                PING => drop(self.pong.frame(PONG, &data)),
                CLOSE => {
                    let reason = String::from_utf8_lossy(data.get(2..).unwrap_or_default()).into_owned();
                    return Err(if reason.is_empty() { "Rex closed the connection".into() } else { reason });
                }
                TEXT | BINARY | CONTINUATION => {
                    message.extend(data);
                    if fin {
                        return Ok(message);
                    }
                }
                _ => {} // PONG, and whatever comes after this version of the protocol
            }
        }
    }

    fn frame(&mut self) -> io::Result<(bool, u8, Vec<u8>)> {
        let mut head = [0; 2];
        self.read.read_exact(&mut head)?;
        let length = match head[1] & 0x7f {
            126 => {
                let mut n = [0; 2];
                self.read.read_exact(&mut n)?;
                u64::from(u16::from_be_bytes(n))
            }
            127 => {
                let mut n = [0; 8];
                self.read.read_exact(&mut n)?;
                u64::from_be_bytes(n)
            }
            n => u64::from(n),
        };
        if length > MAX_MESSAGE {
            return Err(io::Error::other(format!("a Rex message of {length} bytes")));
        }
        let mut data = vec![0; length as usize];
        self.read.read_exact(&mut data)?;
        Ok((head[0] & 0x80 != 0, head[0] & 0x0f, data))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;

    fn pair() -> (Sender, Receiver, UnixStream) {
        let (ours, server) = UnixStream::pair().unwrap();
        let sender = Sender(Arc::new(Mutex::new(ours.try_clone().unwrap())));
        (sender.clone(), Receiver { read: BufReader::new(ours), pong: sender }, server)
    }

    fn unmasked(fin: bool, op: u8, data: &[u8]) -> Vec<u8> {
        let mut frame = vec![u8::from(fin) << 7 | op];
        if data.len() < 126 {
            frame.push(data.len() as u8);
        } else {
            frame.push(126);
            frame.extend((data.len() as u16).to_be_bytes());
        }
        frame.extend_from_slice(data);
        frame
    }

    #[test]
    fn a_message_comes_whole_around_a_ping() {
        let (_sender, mut receiver, mut server) = pair();
        let long = vec![7; 300];
        server.write_all(&unmasked(false, TEXT, b"one ")).unwrap();
        server.write_all(&unmasked(true, PING, b"hi")).unwrap();
        server.write_all(&unmasked(true, CONTINUATION, &long)).unwrap();
        let message = receiver.next().unwrap();
        assert_eq!((&message[..4], message.len()), (&b"one "[..], 304));
        let mut pong = [0; 8];
        server.read_exact(&mut pong).unwrap();
        assert_eq!(pong, [0x80 | PONG, 0x82, 0, 0, 0, 0, b'h', b'i'], "masked, as a client must");
    }

    #[test]
    fn a_close_says_why() {
        let (_sender, mut receiver, mut server) = pair();
        server.write_all(&unmasked(true, CLOSE, b"\x0f\xa0block closed")).unwrap();
        assert_eq!(receiver.next().unwrap_err(), "block closed");
    }

    #[test]
    fn long_frames_carry_their_length() {
        let (sender, _receiver, mut server) = pair();
        sender.binary(&[1; 200]).unwrap();
        let mut head = [0; 8];
        server.read_exact(&mut head).unwrap();
        assert_eq!(head, [0x80 | BINARY, 0x80 | 126, 0, 200, 0, 0, 0, 0]);
    }
}
