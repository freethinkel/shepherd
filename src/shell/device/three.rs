//! The 3D device view: t3code's viewer, bundled with three.js into `assets/device3d/viewer.js` from
//! `web/device3d`, as a page of the panel's like a browser tab's (`shell/web.rs` makes, places and shows it). Its
//! page asks the `shepherd-device` scheme for the session's newest frame (a JPEG, each request waiting for one newer
//! than its last) and for Apple's model of the device (`crate::device::models`); touches and keys come back over IPC.
use super::*;
use std::borrow::Cow;
use std::time::Duration;
use wry::http::{Request, Response};

const SCHEME: &str = "shepherd-device";
const PAGE: &str = include_str!("../../../assets/device3d/index.html");
const VIEWER: &[u8] = include_bytes!("../../../assets/device3d/viewer.js");
/// How long a frame request waits for a new frame before the page asks again.
const FRAME_WAIT: Duration = Duration::from_secs(2);

/// The page's address: which device, for its shape and model, and the `backdrop` it paints under it.
pub(super) fn url(device: &Device, backdrop: &str) -> String {
    let name = percent_encoding::utf8_percent_encode(&device.name, percent_encoding::NON_ALPHANUMERIC);
    let platform = if device.platform == Platform::Android { "android" } else { "ios" };
    format!("{SCHEME}://localhost/index.html?platform={platform}&name={name}{backdrop}")
}

/// The page's own background, as its address's query: none on macOS, where the page is see-through over the panel.
/// Elsewhere a child web view is opaque (WebView2's over GPUI's window, WebKitGTK's in an X11 child window), so it
/// paints the device tab's stage: the window's fill under the panel's wash and the stage's.
pub(super) fn backdrop(theme: &Theme) -> String {
    if cfg!(target_os = "macos") {
        return String::new();
    }
    let gpui::Rgba { r, g, b, .. } = theme.surface.blend(theme.wash(0.05)).blend(theme.wash(0.03)).into();
    let byte = |c: f32| (c.clamp(0., 1.) * 255.).round() as u8;
    format!("&bg=%23{:02x}{:02x}{:02x}", byte(r), byte(g), byte(b))
}

/// The page's scheme and IPC on the web view `builder` makes.
/// `ready` is told once the page has drawn the device.
pub(in crate::shell) fn attach<'a>(
    builder: wry::WebViewBuilder<'a>,
    session: Rc<Session>,
    feed: Arc<Feed>,
    rt: tokio::runtime::Handle,
    ready: impl Fn() + 'static,
) -> wry::WebViewBuilder<'a> {
    builder
        .with_transparent(cfg!(target_os = "macos"))
        .with_asynchronous_custom_protocol(SCHEME.into(), move |_, request, responder| serve(request, &feed, &rt, responder))
        .with_ipc_handler(move |request| match request.body().as_str() {
            r#"{"ready":true}"# => ready(),
            message => input(&session, message),
        })
}

fn respond(status: u16, mime: &str, body: impl Into<Cow<'static, [u8]>>) -> Response<Cow<'static, [u8]>> {
    Response::builder().status(status).header("Content-Type", mime).body(body.into()).expect("a response")
}

/// What the page asked for, answered off the main thread where it waits or downloads.
fn serve(request: Request<Vec<u8>>, feed: &Arc<Feed>, rt: &tokio::runtime::Handle, responder: wry::RequestAsyncResponder) {
    let path = request.uri().path().to_string();
    match path.as_str() {
        "/" | "/index.html" => responder.respond(respond(200, "text/html", PAGE.as_bytes())),
        "/viewer.js" => responder.respond(respond(200, "text/javascript", VIEWER)),
        "/frame" => {
            let after = request.uri().query().and_then(|q| q.strip_prefix("after=")).and_then(|n| n.parse().ok()).unwrap_or(0);
            let feed = feed.clone();
            rt.spawn_blocking(move || match feed.wait_newer(after, FRAME_WAIT).and_then(|(n, f)| Some((n, f.screen, f.jpeg().ok()?))) {
                Some((n, screen, jpeg)) => {
                    let mut response = Response::builder().status(200).header("Content-Type", "image/jpeg").header("x-seq", n.to_string());
                    // a foldable's: which of an iPhone Duo's screens this is, and the hinge
                    if let Some(screen) = screen {
                        response = response.header("x-screen", screen.to_string());
                    }
                    if let Some(fold) = feed.fold() {
                        response = response.header("x-fold", fold.json());
                    }
                    responder.respond(response.body(Cow::Owned(jpeg)).expect("a response"));
                }
                None => responder.respond(respond(204, "text/plain", &[][..])),
            });
        }
        _ => match path.strip_prefix("/model/") {
            Some(id) => {
                let id = id.to_string();
                rt.spawn_blocking(move || match crate::device::models::usdz(&id) {
                    Ok(bytes) => responder.respond(respond(200, "model/vnd.usdz+zip", bytes)),
                    Err(e) => responder.respond(respond(404, "text/plain", e.to_string().into_bytes())),
                });
            }
            None => responder.respond(respond(404, "text/plain", &[][..])),
        },
    }
}

/// A touch, key, text or hinge move from the page, for the device.
fn input(session: &Session, message: &str) {
    let Ok(m) = serde_json::from_str::<serde_json::Value>(message) else { return };
    // a pinch over a foldable, and a Duo turned in the view to face another way
    if let Some(angle) = m["hinge"].as_f64() {
        return session.hinge(Hinge::Angle(angle));
    }
    if let Some(held) = m["hold"].as_str() {
        let held = ["portrait", "landscape_left", "landscape_right", "portrait_upside_down"].into_iter().find(|o| *o == held);
        return session.hinge(Hinge::Hold(held.unwrap_or("portrait")));
    }
    let (x, y) = (m["x"].as_f64().unwrap_or(0.), m["y"].as_f64().unwrap_or(0.));
    match (m["touch"].as_str(), m["key"].as_str(), m["text"].as_str(), m["note"].as_str()) {
        (Some("begin"), ..) => session.touch(Touch::Down, x, y),
        (Some("move"), ..) => session.touch(Touch::Move, x, y),
        (Some("end"), ..) => session.touch(Touch::Up, x, y),
        (_, Some(key), ..) => drop(session.key(key)),
        (_, _, Some(text), _) => session.text(text),
        // the page's own trouble (no WebGL, a model that wouldn't load): it falls back on its own, this is for a log
        (_, _, _, Some(note)) => eprintln!("shepherd: device 3D view: {note}"),
        _ => {}
    }
}
