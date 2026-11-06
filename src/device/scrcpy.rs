//! Android phones on USB through scrcpy, as t3code's serve-emu streams them: scrcpy's server (assets/scrcpy-server-v4.0,
//! Apache-2.0) is pushed to the phone and run by `app_process`; its H.264 is decoded by VideoToolbox straight into
//! IOSurfaces, and touches and keys go back over its control socket. macOS only, for the decoder.
use super::surface::{self, Surface};
use super::{Button, Feed, Frame, Touch, Wake};
use core_foundation::base::TCFType;
use core_video::pixel_buffer::{CVPixelBuffer, CVPixelBufferRef, kCVPixelBufferLock_ReadOnly};
use objc2_core_foundation::CFRetained;
use objc2_core_video::CVImageBuffer;
use objc2_core_media::{CMBlockBuffer, CMFormatDescription, CMSampleBuffer, CMTime, CMVideoFormatDescriptionCreateFromH264ParameterSets, kCMBlockBufferAssureMemoryNowFlag};
use objc2_video_toolbox::{VTDecodeFrameFlags, VTDecodeInfoFlags, VTDecompressionSession};
use std::hash::{BuildHasher, Hasher};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::ptr::{NonNull, null, null_mut};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Bumping it means checking the framing below against serve-emu's `scrcpy.js`: it drifts between majors.
const VERSION: &str = "4.0";
const SERVER: &[u8] = include_bytes!("../../assets/scrcpy-server-v4.0");
const H264: u32 = 0x6832_3634;
/// The longest edge it encodes, as serve-emu asks: plenty for a panel.
const MAX_SIZE: u32 = 1280;

pub struct Session {
    control: Mutex<TcpStream>,
    video: TcpStream,
    /// The video's size now, which touches are in: scrcpy drops one made for another (the phone turned since).
    size: Arc<Mutex<(u32, u32)>>,
    server: std::process::Child,
    adb: PathBuf,
    serial: String,
    port: u16,
}

impl Session {
    /// Starts scrcpy's server on phone `serial` and streams it into `feed`; blocks until the video's up.
    pub(super) fn open(adb: PathBuf, serial: &str, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Session> {
        let run = |args: &[&str]| crate::process::answer(crate::process::command(&adb).arg("-s").arg(serial).args(args).output()?);
        // a 31-bit id: the server reads it as a signed int
        let scid = format!("{:08x}", std::collections::hash_map::RandomState::new().build_hasher().finish() & 0x7fff_ffff);
        // the server deletes its jar as it stops (`cleanup`), so each session pushes its own
        let jar = format!("/data/local/tmp/shepherd-scrcpy-{scid}.jar");
        let local = std::env::temp_dir().join(format!("shepherd-scrcpy-{scid}.jar"));
        std::fs::write(&local, SERVER)?;
        let pushed = run(&["push", &local.to_string_lossy(), &jar]);
        let _ = std::fs::remove_file(&local);
        pushed?;
        let port: u16 = run(&["forward", "tcp:0", &format!("localabstract:scrcpy_{scid}")])?.trim().parse()?;
        let mut server = crate::process::command(&adb)
            .args(["-s", serial, "shell", &format!("CLASSPATH={jar}"), "app_process", "/", "com.genymobile.scrcpy.Server", VERSION])
            .args([&format!("scid={scid}"), "log_level=info", "audio=false", "tunnel_forward=true", &format!("max_size={MAX_SIZE}")])
            .args(["max_fps=60", "clipboard_autosync=false"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        // its complaints, for when it quits; drained so it never blocks on a full pipe
        let stderr = server.stderr.take();
        let said = std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut s) = stderr {
                let _ = s.read_to_string(&mut text);
            }
            text
        });
        let (video, control, size) = match connect(port, &mut server) {
            Ok(up) => up,
            Err(e) => {
                let _ = server.kill();
                let _ = run(&["forward", "--remove", &format!("tcp:{port}")]);
                let text = said.join().unwrap_or_default();
                let last = text.lines().rev().find(|l| !l.trim().is_empty()).map(|l| format!(": {}", l.trim())).unwrap_or_default();
                return Err(e.context(format!("scrcpy's server did not start{last}")));
            }
        };
        let size = Arc::new(Mutex::new(size));
        // what the phone says back (its clipboard), unread
        let mut back = control.try_clone()?;
        std::thread::spawn(move || std::io::copy(&mut back, &mut std::io::sink()));
        let mut reader = video.try_clone()?;
        let shared = size.clone();
        std::thread::spawn(move || {
            if let Err(e) = stream(&mut reader, &shared, &feed, &wake) {
                feed.fail(format!("The phone's video stopped: {e:#}"));
                wake();
            }
        });
        Ok(Session { control: Mutex::new(control), video, size, server, adb, serial: serial.into(), port })
    }

    fn send(&self, message: &[u8]) {
        if let Ok(mut c) = self.control.lock() {
            let _ = c.write_all(message);
        }
    }

    pub(super) fn touch(&self, phase: Touch, x: f64, y: f64) {
        let size = self.size.lock().map_or((0, 0), |s| *s);
        let action = match phase {
            Touch::Down => 0,
            Touch::Up => 1,
            Touch::Move => 2,
        };
        self.send(&touch_message(action, x, y, size));
    }

    fn press(&self, keycode: u32) {
        self.send(&[key_message(0, keycode), key_message(1, keycode)].concat());
    }

    pub(super) fn button(&self, button: Button) {
        match button {
            // BACK_OR_SCREEN_ON: back, or the screen woken when it's off
            Button::Back => self.send(&[4, 0, 4, 1]),
            Button::Home => self.press(3),
            Button::Recents => self.press(187),
        }
    }

    pub(super) fn key(&self, name: &str) -> bool {
        keycode(name).map(|k| self.press(k)).is_some()
    }

    pub(super) fn text(&self, text: &str) {
        text_messages(text).iter().for_each(|m| self.send(m));
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.video.shutdown(Shutdown::Both);
        if let Ok(c) = self.control.lock() {
            let _ = c.shutdown(Shutdown::Both);
        }
        let _ = self.server.kill();
        let _ = self.server.wait();
        let (adb, serial, port) = (self.adb.clone(), self.serial.clone(), self.port);
        std::thread::spawn(move || crate::process::command(adb).args(["-s", &serial, "forward", "--remove", &format!("tcp:{port}")]).output());
    }
}

/// The server's two sockets, video first, and the video's size. Through `adb forward` a connection opens before the
/// server listens, and is closed at once: the video socket's first byte says it's really up.
fn connect(port: u16, server: &mut std::process::Child) -> anyhow::Result<(TcpStream, TcpStream, (u32, u32))> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut video = loop {
        if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
            s.set_read_timeout(Some(Duration::from_secs(2)))?;
            if s.read_exact(&mut [0]).is_ok() {
                break s;
            }
        }
        anyhow::ensure!(server.try_wait()?.is_none(), "it quit");
        anyhow::ensure!(Instant::now() < deadline, "it did not answer in 10 seconds");
        std::thread::sleep(Duration::from_millis(100));
    };
    let control = TcpStream::connect(("127.0.0.1", port))?;
    control.set_nodelay(true)?;
    // the phone's name, then the stream's: codec, flags, width, height
    let mut meta = [0; 80];
    video.read_exact(&mut meta)?;
    video.set_read_timeout(None)?;
    let word = |at: usize| u32::from_be_bytes([meta[at], meta[at + 1], meta[at + 2], meta[at + 3]]);
    anyhow::ensure!(word(64) == H264, "the phone's video is not H.264");
    Ok((video, control, (word(72), word(76))))
}

/// The video's packets until the socket closes, each frame decoded into `feed`.
fn stream(video: &mut TcpStream, size: &Mutex<(u32, u32)>, feed: &Arc<Feed>, wake: &Wake) -> anyhow::Result<()> {
    let mut decoder: Option<(Vec<Vec<u8>>, Decoder)> = None;
    let mut header = [0; 12];
    let mut packet = Vec::new();
    loop {
        video.read_exact(&mut header)?;
        let word = |at: usize| u32::from_be_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]]);
        // a session packet: the video's new size, as the phone turned
        if header[0] & 0x80 != 0 {
            if let Ok(mut s) = size.lock() {
                *s = (word(4), word(8));
            }
            continue;
        }
        let len = word(8) as usize;
        anyhow::ensure!(len > 0 && len <= 16 << 20, "a packet of {len} bytes");
        packet.resize(len, 0);
        video.read_exact(&mut packet)?;
        let (sets, avcc) = split(&packet);
        // a config packet's SPS and PPS; a change (a turn) makes the decoder anew
        if sets.len() >= 2 && decoder.as_ref().is_none_or(|(had, _)| *had != sets) {
            let made = Decoder::new(&sets, feed.clone(), wake.clone())?;
            decoder = Some((sets, made));
        }
        if let Some((_, d)) = &decoder
            && !avcc.is_empty()
            && let Err(e) = d.decode(&avcc)
        {
            feed.fail(format!("A frame of the phone's video would not decode: {e:#}"));
            wake();
        }
    }
}

/// A packet's NAL units, from its Annex B start codes: its parameter sets apart, and the rest each behind a 4-byte
/// length, as VideoToolbox takes them.
fn split(packet: &[u8]) -> (Vec<Vec<u8>>, Vec<u8>) {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= packet.len() {
        if packet[i..i + 3] == [0, 0, 1] {
            starts.push(i + 3);
            i += 3;
        } else {
            i += 1;
        }
    }
    let (mut sets, mut avcc) = (Vec::new(), Vec::with_capacity(packet.len() + 16));
    for (n, &start) in starts.iter().enumerate() {
        let mut end = starts.get(n + 1).map_or(packet.len(), |next| next - 3);
        // a 4-byte start code's leading zero belongs to neither
        while n + 1 < starts.len() && end > start && packet[end - 1] == 0 {
            end -= 1;
        }
        let nal = &packet[start..end];
        match nal.first().map(|b| b & 0x1f) {
            None => {}
            Some(7 | 8) => sets.push(nal.to_vec()),
            Some(_) => {
                avcc.extend((nal.len() as u32).to_be_bytes());
                avcc.extend(nal);
            }
        }
    }
    (sets, avcc)
}

/// A VideoToolbox session for one SPS and PPS, which hands each frame to the feed as a BGRA IOSurface.
struct Decoder {
    session: CFRetained<VTDecompressionSession>,
    format: CFRetained<CMFormatDescription>,
    output: block2::RcBlock<dyn Fn(i32, VTDecodeInfoFlags, *mut CVImageBuffer, CMTime, CMTime)>,
}

impl Decoder {
    fn new(sets: &[Vec<u8>], feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Decoder> {
        let mut pointers: Vec<NonNull<u8>> = sets.iter().filter_map(|s| NonNull::new(s.as_ptr().cast_mut())).collect();
        let mut sizes: Vec<usize> = sets.iter().map(Vec::len).collect();
        let mut format: *const CMFormatDescription = null();
        // the pointers and sizes are the sets', alive for the call; it copies what it keeps
        let status = unsafe {
            CMVideoFormatDescriptionCreateFromH264ParameterSets(None, sets.len(), NonNull::from(&mut pointers[..]).cast(), NonNull::from(&mut sizes[..]).cast(), 4, NonNull::from(&mut format))
        };
        anyhow::ensure!(status == 0, "its SPS and PPS were refused ({status})");
        let format = unsafe { CFRetained::from_raw(NonNull::new(format.cast_mut()).ok_or_else(|| anyhow::anyhow!("no format description"))?) };
        let attributes = surface::attributes(None);
        // the same dictionary, as objc2's type
        let attributes = unsafe { &*(attributes.as_concrete_TypeRef() as *const objc2_core_foundation::CFDictionary) };
        let mut session: *mut VTDecompressionSession = null_mut();
        let status = unsafe { VTDecompressionSession::create(None, &format, None, Some(attributes), null(), NonNull::from(&mut session)) };
        anyhow::ensure!(status == 0, "VideoToolbox would not decode it ({status})");
        let session = unsafe { CFRetained::from_raw(NonNull::new(session).ok_or_else(|| anyhow::anyhow!("no decompression session"))?) };
        let output = block2::RcBlock::new(move |status: i32, _: VTDecodeInfoFlags, image: *mut CVImageBuffer, _: CMTime, _: CMTime| {
            if status != 0 || image.is_null() {
                return;
            }
            // VideoToolbox's buffer, retained for as long as the frame's shown
            let buffer = unsafe { CVPixelBuffer::wrap_under_get_rule(image as CVPixelBufferRef) };
            feed.put(frame(buffer));
            wake();
        });
        Ok(Decoder { session, format, output })
    }

    /// Decodes one frame's NAL units; the frame lands in the feed before this returns.
    fn decode(&self, avcc: &[u8]) -> anyhow::Result<()> {
        let mut block: *mut CMBlockBuffer = null_mut();
        let status = unsafe { CMBlockBuffer::create_with_memory_block(None, null_mut(), avcc.len(), None, null(), 0, avcc.len(), kCMBlockBufferAssureMemoryNowFlag, NonNull::from(&mut block)) };
        anyhow::ensure!(status == 0, "no block buffer ({status})");
        let block = unsafe { CFRetained::from_raw(NonNull::new(block).ok_or_else(|| anyhow::anyhow!("no block buffer"))?) };
        let status = unsafe { CMBlockBuffer::replace_data_bytes(NonNull::from(avcc).cast(), &block, 0, avcc.len()) };
        anyhow::ensure!(status == 0, "the block buffer was not filled ({status})");
        let mut sample: *mut CMSampleBuffer = null_mut();
        let len = avcc.len();
        let status = unsafe { CMSampleBuffer::create_ready(None, Some(&block), Some(&self.format), 1, 0, null(), 1, &len, NonNull::from(&mut sample)) };
        anyhow::ensure!(status == 0, "no sample buffer ({status})");
        let sample = unsafe { CFRetained::from_raw(NonNull::new(sample).ok_or_else(|| anyhow::anyhow!("no sample buffer"))?) };
        // no asynchronous flag: the output block runs before this returns
        let status = unsafe { self.session.decode_frame_with_output_handler(&sample, VTDecodeFrameFlags(0), null_mut(), block2::RcBlock::as_ptr(&self.output)) };
        anyhow::ensure!(status == 0, "VideoToolbox said {status}");
        Ok(())
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { self.session.invalidate() };
    }
}

/// A decoded buffer as a frame: drawn from the buffer itself, its pixels copied for the 3D view's JPEGs.
// ponytail: a copy per frame the 3D view mostly doesn't use; read the surface in `Frame::jpeg` if it ever shows
fn frame(buffer: CVPixelBuffer) -> Frame {
    let (w, h, stride) = (buffer.get_width(), buffer.get_height(), buffer.get_bytes_per_row());
    let mut pixels = Vec::with_capacity(w * h * 4);
    if buffer.lock_base_address(kCVPixelBufferLock_ReadOnly) == 0 {
        let base = unsafe { buffer.get_base_address() } as *const u8;
        for y in 0..h {
            // the row is readable while the buffer's locked, and `stride` fits its width
            pixels.extend_from_slice(unsafe { std::slice::from_raw_parts(base.add(y * stride), w * 4) });
        }
        buffer.unlock_base_address(kCVPixelBufferLock_ReadOnly);
    }
    Frame::decoded(w as u32, h as u32, pixels, Surface::of(buffer))
}

/// INJECT_TOUCH: a finger at `x`, `y` (0 to 1) on a video of `size`, by serve-emu's layout.
fn touch_message(action: u8, x: f64, y: f64, (w, h): (u32, u32)) -> [u8; 32] {
    let at = |v: f64, n: u32| ((v.clamp(0., 1.) * n as f64) as i32).min(n as i32 - 1).max(0);
    let mut m = [0; 32];
    m[0] = 2;
    m[1] = action;
    // pointer 0, 8 bytes, left zero
    m[10..14].copy_from_slice(&at(x, w).to_be_bytes());
    m[14..18].copy_from_slice(&at(y, h).to_be_bytes());
    m[18..20].copy_from_slice(&(w as u16).to_be_bytes());
    m[20..22].copy_from_slice(&(h as u16).to_be_bytes());
    let down = action != 1;
    m[22..24].copy_from_slice(&(if down { 0xffff_u16 } else { 0 }).to_be_bytes());
    // the action's button, then the buttons held: the primary
    m[24..28].copy_from_slice(&1_u32.to_be_bytes());
    m[28..32].copy_from_slice(&u32::from(down).to_be_bytes());
    m
}

/// INJECT_KEYCODE: no repeat, no meta keys.
fn key_message(action: u8, keycode: u32) -> [u8; 14] {
    let mut m = [0; 14];
    m[0] = 0;
    m[1] = action;
    m[2..6].copy_from_slice(&keycode.to_be_bytes());
    m
}

/// INJECT_TEXT, in pieces of at most 300 bytes, as the server reads them.
fn text_messages(text: &str) -> Vec<Vec<u8>> {
    let mut messages = Vec::new();
    let mut piece = String::new();
    for c in text.chars() {
        if piece.len() + c.len_utf8() > 300 {
            messages.push(std::mem::take(&mut piece));
        }
        piece.push(c);
    }
    messages.extend((!piece.is_empty()).then_some(piece));
    messages.into_iter().map(|p| [&[1], &(p.len() as u32).to_be_bytes()[..], p.as_bytes()].concat()).collect()
}

/// GPUI's key names as Android's keycodes.
fn keycode(name: &str) -> Option<u32> {
    Some(match name {
        "enter" => 66,
        "backspace" => 67,
        "delete" => 112,
        "tab" => 61,
        "escape" => 111,
        "left" => 21,
        "right" => 22,
        "up" => 19,
        "down" => 20,
        "home" => 122,
        "end" => 123,
        "pageup" => 92,
        "pagedown" => 93,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_b_splits_into_sets_and_avcc() {
        let packet = [0, 0, 0, 1, 0x67, 1, 2, 0, 0, 1, 0x68, 3, 0, 0, 0, 1, 0x65, 4, 5, 0];
        let (sets, avcc) = split(&packet);
        assert_eq!(sets, [vec![0x67, 1, 2], vec![0x68, 3]]);
        // the last unit keeps its trailing zero: it's data, not a start code's
        assert_eq!(avcc, [0, 0, 0, 4, 0x65, 4, 5, 0]);
    }

    #[test]
    fn control_messages_lay_out_as_scrcpy_reads_them() {
        let m = touch_message(0, 0.5, 1., (100, 200));
        assert_eq!(m[..2], [2, 0]);
        assert_eq!(m[10..22], [0, 0, 0, 50, 0, 0, 0, 199, 0, 100, 0, 200]);
        assert_eq!(m[22..], [0xff, 0xff, 0, 0, 0, 1, 0, 0, 0, 1]);
        assert_eq!(touch_message(1, 0., 0., (100, 200))[22..], [0, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(key_message(1, 66), [0, 1, 0, 0, 0, 66, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(text_messages("hi"), [vec![1, 0, 0, 0, 2, b'h', b'i']]);
        let long = text_messages(&"é".repeat(151));
        assert_eq!(long.iter().map(|m| m.len() - 5).collect::<Vec<_>>(), [300, 2]);
    }

    /// A live phone's (or an emulator's: scrcpy takes either) first frame, by `SHEPHERD_SCRCPY_SERIAL`; its JPEG goes
    /// to `SHEPHERD_SCRCPY_SHOT` if set.
    #[test]
    #[ignore]
    fn a_live_phone_streams() {
        let serial = std::env::var("SHEPHERD_SCRCPY_SERIAL").expect("SHEPHERD_SCRCPY_SERIAL");
        let feed = Arc::new(Feed::default());
        let session = Session::open(super::super::android::adb_path().unwrap(), &serial, feed.clone(), Arc::new(|| {})).unwrap();
        let (_, frame) = feed.wait_newer(0, Duration::from_secs(10)).expect("a frame");
        assert_eq!(frame.pixels.len(), (frame.width * frame.height * 4) as usize);
        assert!(frame.surface.is_some() && frame.pixels.iter().any(|&p| p != 0));
        if let Ok(to) = std::env::var("SHEPHERD_SCRCPY_SHOT") {
            std::fs::write(to, frame.jpeg().unwrap()).unwrap();
        }
        drop(session);
    }
}
