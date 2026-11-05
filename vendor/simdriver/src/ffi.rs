//! C ABI over the public [`Driver`] (plan Task 10).
//!
//! Every `sd_` entry point catches Rust panics at the boundary (unwinding into C is UB)
//! and reports failures through [`sd_last_error`], which is thread-local: the pointer it
//! returns stays valid until the next `sd_` call on the same thread.
//!
//! Callbacks run on the driver's internal threads and may run concurrently (frame worker
//! and supervisor); `sd_frame.data` is borrowed and only valid inside the callback.
//! `Event::Note` has no C channel and is dropped.
#![allow(non_snake_case, non_camel_case_types)]
use crate::devices::{Appearance, DeviceSelector};
use crate::driver::{Button as DriverButton, Driver, Event, Modifiers, Options, State};
use crate::gesture::ScrollPhase;
use crate::grid::Grid;
use crate::protocol::Phase;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use std::ops::Deref;
use std::os::raw::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::Duration;

/// One delivered frame. `data` points at `len` tightly packed BGRA bytes and is valid
/// only inside the frame callback.
#[repr(C)]
pub struct sd_frame {
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub seq: u64,
    pub data: *const u8,
    pub len: usize,
}

pub type sd_frame_cb = Option<unsafe extern "C" fn(frame: *const sd_frame, user: *mut c_void)>;
pub type sd_state_cb = Option<unsafe extern "C" fn(state: i32, user: *mut c_void)>;

/// The opaque handle: a started Driver plus the C callbacks it delivers to.
pub struct sd_driver {
    driver: Driver,
}

/// What the on_event closure needs; raw pointers, so the C side must keep `user` alive
/// for the driver's lifetime and tolerate calls from any thread.
struct Cbs {
    frame: sd_frame_cb,
    state: sd_state_cb,
    user: *mut c_void,
}

// SAFETY: the pointers are only ever dereferenced as C function pointers / passed back
// to C verbatim; the C host contract (header comment) covers their lifetime.
unsafe impl Send for Cbs {}
unsafe impl Sync for Cbs {}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Deref<Target = str>) {
    LAST_ERROR.with(|slot| {
        let s = CString::new(&*msg)
            .unwrap_or_else(|_| CString::new("error message contained a NUL byte").unwrap());
        *slot.borrow_mut() = Some(s);
    });
}

/// Each `sd_` call starts with no error, so `sd_last_error` is empty after a success.
fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

fn panic_text(e: Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or_else(|| {
            e.downcast_ref::<&str>()
                .copied()
                .unwrap_or("panic with a non-string payload")
        })
        .to_string()
}

/// Run `f`, turning a panic into `Err(msg)`.
fn guarded(f: impl FnOnce() + std::panic::UnwindSafe) -> Result<(), String> {
    catch_unwind(f).map_err(panic_text)
}

/// Run a device call for `sd_<operation>`: an error or a panic becomes `false` plus the
/// last error, so no panic unwinds into C.
fn device_call(operation: &str, f: impl FnOnce() -> Result<(), crate::error::Error>) -> bool {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            set_error(format!("{operation}: {e}"));
            false
        }
        Err(p) => {
            set_error(format!("panic in sd_{operation}: {}", panic_text(p)));
            false
        }
    }
}

fn state_code(s: State) -> i32 {
    match s {
        State::Starting => 0,
        State::Live => 1,
        State::InputDisplaced => 2,
        State::Restarting => 3,
        State::Stopped => 4,
    }
}

fn phase(code: i32) -> Option<Phase> {
    match code {
        0 => Some(Phase::Down),
        1 => Some(Phase::Move),
        2 => Some(Phase::Up),
        _ => None,
    }
}

fn scroll_phase(code: i32) -> Option<ScrollPhase> {
    match code {
        0 => Some(ScrollPhase::Began),
        1 => Some(ScrollPhase::Changed),
        2 => Some(ScrollPhase::Ended),
        3 => Some(ScrollPhase::Momentum),
        _ => None,
    }
}

fn button(code: i32) -> Option<DriverButton> {
    match code {
        0 => Some(DriverButton::Home),
        1 => Some(DriverButton::Lock),
        2 => Some(DriverButton::SideButton),
        3 => Some(DriverButton::Siri),
        4 => Some(DriverButton::VolumeUp),
        5 => Some(DriverButton::VolumeDown),
        _ => None,
    }
}

fn mods(mask: u32) -> Modifiers {
    Modifiers {
        alt: mask & 1 != 0,
        shift: mask & 2 != 0,
        ctrl: mask & 4 != 0,
        meta: mask & 8 != 0,
    }
}

/// Borrowed C string → valid UTF-8, or an error message naming the argument.
fn utf8<'a>(ptr: *const c_char, what: &str) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err(format!("{what} is NULL"));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| format!("{what} is not valid UTF-8"))
}

fn deliver(cbs: &Cbs, ev: Event) {
    // A panic while translating or inside a Rust trampoline must not unwind into the
    // driver's threads' owners; the C callbacks themselves cannot legally unwind.
    let _ = guarded(AssertUnwindSafe(|| match ev {
        Event::Frame(f) => {
            if let Some(cb) = cbs.frame {
                let frame = sd_frame {
                    width: f.width,
                    height: f.height,
                    source_width: f.source_width,
                    source_height: f.source_height,
                    seq: f.seq,
                    data: f.data.as_ptr(),
                    len: f.data.len(),
                };
                unsafe { cb(&frame, cbs.user) };
            }
        }
        Event::State(s) => {
            if let Some(cb) = cbs.state {
                unsafe { cb(state_code(s), cbs.user) };
            }
        }
        Event::Note(_) => {}
    }));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_start(
    udid: *const c_char,
    fps: u32,
    scale: u32,
    helper_dir: *const c_char,
    frame_cb: sd_frame_cb,
    state_cb: sd_state_cb,
    user: *mut c_void,
) -> *mut sd_driver {
    clear_error();
    let mut run = || -> Result<*mut sd_driver, String> {
        let selector = if udid.is_null() {
            DeviceSelector::FirstBootedOrIphone
        } else {
            DeviceSelector::Udid(utf8(udid, "udid")?.to_string())
        };
        let helper_dir = if helper_dir.is_null() {
            None
        } else {
            Some(PathBuf::from(utf8(helper_dir, "helper_dir")?))
        };
        let opts = Options {
            device: selector,
            fps: NonZeroU32::new(fps).map_or(60, NonZeroU32::get),
            scale: NonZeroU32::new(scale).map_or(2, NonZeroU32::get),
            helper_dir,
            screen_size: None,
            touch_target: 0,
            rotate_cw: false,
        };
        let cbs = Cbs {
            frame: frame_cb,
            state: state_cb,
            user,
        };
        Driver::start(opts, move |ev| deliver(&cbs, ev))
            .map(|driver| Box::into_raw(Box::new(sd_driver { driver })))
            .map_err(|e| e.to_string())
    };
    match catch_unwind(AssertUnwindSafe(&mut run)) {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            set_error(e);
            std::ptr::null_mut()
        }
        Err(p) => {
            set_error(format!("panic in sd_start: {}", panic_text(p)));
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_pointer(d: *mut sd_driver, phase_code: i32, x: f64, y: f64, mask: u32) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| {
        if let Some(p) = phase(phase_code) {
            d.driver.pointer(p, x, y, mods(mask));
        }
    }));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_scroll(
    d: *mut sd_driver,
    phase_code: i32,
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| {
        if let Some(p) = scroll_phase(phase_code) {
            d.driver.scroll(p, x, y, dx, dy);
        }
    }));
}

/// Two fingers at (x1, y1) and (x2, y2), each 0..1 in the box; `phase_code` as for
/// `sd_pointer`. For rotation and off-centre pinches.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_touch2(
    d: *mut sd_driver,
    phase_code: i32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| {
        if let Some(p) = phase(phase_code) {
            d.driver.touch2(p, x1, y1, x2, y2);
        }
    }));
}

/// A tap at (x, y), each 0..1 in the box: a pointer down and up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_tap(d: *mut sd_driver, x: f64, y: f64) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| d.driver.tap(x, y)));
}

/// Tap twice at (x, y), each 0..1 in the box, with a short pause for iOS double-tap recognition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_double_tap(d: *mut sd_driver, x: f64, y: f64) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| d.driver.double_tap(x, y)));
}

/// Press at (x, y), hold for `hold_secs`, then lift — iOS's long press (context menus).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_long_press(d: *mut sd_driver, x: f64, y: f64, hold_secs: f64) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| {
        d.driver
            .long_press(x, y, Duration::from_secs_f64(hold_secs.max(0.0)));
    }));
}

/// Drag from (x1, y1) to (x2, y2), each 0..1 in the box: down, hold `hold_secs`, move
/// over `move_secs`, lift.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_drag(
    d: *mut sd_driver,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    hold_secs: f64,
    move_secs: f64,
) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| {
        d.driver.drag(
            x1,
            y1,
            x2,
            y2,
            Duration::from_secs_f64(hold_secs.max(0.0)),
            Duration::from_secs_f64(move_secs.max(0.0)),
        );
    }));
}

/// The frontmost app's accessibility tree as JSON (one node: `role`, `label`?, `value`?,
/// `id`?, `enabled`, `frame` [x,y,w,h] in screen points, `children`?). Blocks; the first
/// call takes 1–2 s. Returns NULL on failure (see `sd_last_error`); free the result
/// with `sd_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_accessibility_json(d: *mut sd_driver) -> *mut c_char {
    clear_error();
    if d.is_null() {
        set_error("d is NULL");
        return std::ptr::null_mut();
    }
    let d = unsafe { &*d };
    let run = || -> Result<CString, String> {
        let root = d.driver.accessibility().map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&root).map_err(|e| e.to_string())?;
        CString::new(json).map_err(|_| "snapshot JSON contained a NUL byte".to_string())
    };
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(s)) => s.into_raw(),
        Ok(Err(e)) => {
            set_error(e);
            std::ptr::null_mut()
        }
        Err(p) => {
            set_error(format!("panic in sd_accessibility_json: {}", panic_text(p)));
            std::ptr::null_mut()
        }
    }
}

/// Frees a string returned by simdriver. NULL-safe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_string_free(s: *mut c_char) {
    clear_error();
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

/// Rows of the grid with `cols` columns over a `screen_w`×`screen_h` point screen.
#[unsafe(no_mangle)]
pub extern "C" fn sd_grid_rows(screen_w: f64, screen_h: f64, cols: u32) -> u32 {
    clear_error();
    Grid::new((screen_w, screen_h), cols).rows
}

/// Centre of grid cell `name` (e.g. "C4": row C, column 4; rows skip E) in 0..1 of the screen,
/// written to `*x` and `*y`. False (nothing written) for a name outside the grid or a
/// NULL argument.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_grid_cell(
    screen_w: f64,
    screen_h: f64,
    cols: u32,
    name: *const c_char,
    x: *mut f64,
    y: *mut f64,
) -> bool {
    clear_error();
    if x.is_null() || y.is_null() {
        return false;
    }
    let name = match utf8(name, "name") {
        Ok(n) => n,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    match Grid::new((screen_w, screen_h), cols).center(name) {
        Some((cx, cy)) => {
            unsafe {
                *x = cx;
                *y = cy;
            }
            true
        }
        None => false,
    }
}

/// The device screen in points, written to `*w` and `*h`. Returns false (and writes
/// nothing) until input is connected, or if `d`, `w` or `h` is NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_screen_points(d: *mut sd_driver, w: *mut f64, h: *mut f64) -> bool {
    clear_error();
    if d.is_null() || w.is_null() || h.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    match catch_unwind(AssertUnwindSafe(|| d.driver.screen_points())) {
        Ok(Some((sw, sh))) => {
            unsafe {
                *w = sw;
                *h = sh;
            }
            true
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_key(d: *mut sd_driver, name: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    let name = match utf8(name, "name") {
        Ok(n) => n,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    match catch_unwind(AssertUnwindSafe(|| d.driver.key(name))) {
        Ok(ok) => ok,
        Err(p) => {
            set_error(format!("panic in sd_key: {}", panic_text(p)));
            false
        }
    }
}

/// Press a key combination such as `cmd+a`. Modifiers are cmd, shift, alt, or ctrl.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_key_combo(d: *mut sd_driver, combo: *const c_char) -> bool {
    clear_error();
    if d.is_null() || combo.is_null() {
        set_error("driver or combo is NULL");
        return false;
    }
    let d = unsafe { &*d };
    let Ok(combo) = (unsafe { CStr::from_ptr(combo) }).to_str() else {
        set_error("key combo is not valid UTF-8");
        return false;
    };
    match catch_unwind(AssertUnwindSafe(|| d.driver.key(combo))) {
        Ok(ok) => ok,
        Err(p) => {
            set_error(format!("panic in sd_key_combo: {}", panic_text(p)));
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_open_url(d: *mut sd_driver, url: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    match utf8(url, "url") {
        Ok(url) => device_call("open_url", || d.driver.open_url(url)),
        Err(e) => {
            set_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_launch(d: *mut sd_driver, bundle_id: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    match utf8(bundle_id, "bundle_id") {
        Ok(id) => device_call("launch", || d.driver.launch(id)),
        Err(e) => {
            set_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_terminate(d: *mut sd_driver, bundle_id: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    match utf8(bundle_id, "bundle_id") {
        Ok(id) => device_call("terminate", || d.driver.terminate(id)),
        Err(e) => {
            set_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_set_location(d: *mut sd_driver, lat: f64, lon: f64) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    device_call("set_location", || d.driver.set_location(lat, lon))
}

/// Appearance code: 0 = light, 1 = dark.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_set_appearance(d: *mut sd_driver, appearance: i32) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let mode = match appearance {
        0 => Appearance::Light,
        1 => Appearance::Dark,
        _ => {
            set_error("appearance must be 0 (light) or 1 (dark)");
            return false;
        }
    };
    let d = unsafe { &*d };
    device_call("set_appearance", || d.driver.set_appearance(mode))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_grant_permission(
    d: *mut sd_driver,
    service: *const c_char,
    bundle_id: *const c_char,
) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    let service = match utf8(service, "service") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    let bundle_id = match utf8(bundle_id, "bundle_id") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    device_call("grant_permission", || {
        d.driver.grant_permission(service, bundle_id)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_clipboard_write(d: *mut sd_driver, text: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    match utf8(text, "text") {
        Ok(text) => device_call("clipboard_write", || d.driver.clipboard_write(text)),
        Err(e) => {
            set_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_clipboard_read(d: *mut sd_driver) -> *mut c_char {
    clear_error();
    if d.is_null() {
        return std::ptr::null_mut();
    }
    let d = unsafe { &*d };
    match catch_unwind(AssertUnwindSafe(|| d.driver.clipboard_read())) {
        Ok(Ok(text)) => CString::new(text)
            .map(CString::into_raw)
            .unwrap_or_else(|_| {
                set_error("clipboard contained a NUL byte");
                std::ptr::null_mut()
            }),
        Ok(Err(e)) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
        Err(p) => {
            set_error(format!("panic in sd_clipboard_read: {}", panic_text(p)));
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_revoke_permission(
    d: *mut sd_driver,
    service: *const c_char,
    bundle_id: *const c_char,
) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    let service = match utf8(service, "service") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    let id = match utf8(bundle_id, "bundle_id") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    device_call("revoke_permission", || {
        d.driver.revoke_permission(service, id)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_reset_permission(
    d: *mut sd_driver,
    service: *const c_char,
    bundle_id: *const c_char,
) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    let service = match utf8(service, "service") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    let id = match utf8(bundle_id, "bundle_id") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    device_call("reset_permission", || {
        d.driver.reset_permission(service, id)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_set_content_size(d: *mut sd_driver, size: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    match utf8(size, "size") {
        Ok(s) => device_call("set_content_size", || d.driver.set_content_size(s)),
        Err(e) => {
            set_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_status_bar_override(d: *mut sd_driver) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    device_call("status_bar_override", || d.driver.status_bar_override())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_status_bar_clear(d: *mut sd_driver) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    device_call("status_bar_clear", || d.driver.status_bar_clear())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_push_notification(
    d: *mut sd_driver,
    bundle_id: *const c_char,
    payload_path: *const c_char,
) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    let id = match utf8(bundle_id, "bundle_id") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    let path = match utf8(payload_path, "payload_path") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    device_call("push_notification", || d.driver.push_notification(id, path))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_reset_keychain(d: *mut sd_driver) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    device_call("reset_keychain", || d.driver.reset_keychain())
}

/// Clear an app's saved data container. This explicitly deletes its data and terminates it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_clear_app_data(d: *mut sd_driver, bundle_id: *const c_char) -> bool {
    clear_error();
    if d.is_null() {
        return false;
    }
    let d = unsafe { &*d };
    let id = match utf8(bundle_id, "bundle_id") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    device_call("clear_app_data", || d.driver.clear_app_data(id))
}

/// Return installed simulator apps as JSON; free the result with `sd_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_apps(d: *mut sd_driver) -> *mut c_char {
    clear_error();
    if d.is_null() {
        return std::ptr::null_mut();
    }
    let d = unsafe { &*d };
    match catch_unwind(AssertUnwindSafe(|| d.driver.apps())) {
        Ok(Ok(apps)) => match serde_json::to_string(&apps) {
            Ok(json) => CString::new(json)
                .map(CString::into_raw)
                .unwrap_or_else(|_| {
                    set_error("app list contained a NUL byte");
                    std::ptr::null_mut()
                }),
            Err(e) => {
                set_error(format!("could not serialize app list: {e}"));
                std::ptr::null_mut()
            }
        },
        Ok(Err(e)) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
        Err(p) => {
            set_error(format!("panic in sd_apps: {}", panic_text(p)));
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_text(d: *mut sd_driver, s: *const c_char) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let s = match utf8(s, "text") {
        Ok(t) => t.to_string(),
        Err(e) => {
            set_error(e);
            return;
        }
    };
    let _ = guarded(AssertUnwindSafe(|| d.driver.text(&s)));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_button(
    d: *mut sd_driver,
    button_code: i32,
    presses: u32,
    hold_secs: f32,
) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { &*d };
    let _ = guarded(AssertUnwindSafe(|| {
        if let Some(b) = button(button_code) {
            d.driver.button(b, presses, hold_secs);
        }
    }));
}

/// Stops the driver and frees the handle. NULL-safe; the handle must not be used after.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sd_stop(d: *mut sd_driver) {
    clear_error();
    if d.is_null() {
        return;
    }
    let d = unsafe { Box::from_raw(d) };
    if let Err(e) = guarded(AssertUnwindSafe(|| d.driver.stop())) {
        set_error(format!("panic in sd_stop: {e}"));
    }
}

/// The last error on this thread: valid until the next `sd_` call on this thread.
/// Empty when the last call succeeded. Never NULL.
#[unsafe(no_mangle)]
pub extern "C" fn sd_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or_else(|| c"".as_ptr() as *const c_char, |s| s.as_ptr())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        assert_eq!(state_code(State::Live), 1);
        assert_eq!(state_code(State::Stopped), 4);
        assert_eq!(phase(0), Some(Phase::Down));
        assert_eq!(phase(3), None);
        assert_eq!(scroll_phase(3), Some(ScrollPhase::Momentum));
        assert_eq!(scroll_phase(4), None);
        assert_eq!(button(5), Some(DriverButton::VolumeDown));
        assert_eq!(button(6), None);
        assert!(mods(1 | 2).alt && mods(1 | 2).shift && !mods(3).ctrl);
    }
}
