//! The OS window: its background, traffic lights, saved frame, idle time, screen lock and Secure Keyboard Entry.
use super::*;

/// The window's background for the Translucent window setting: blurred where the system or compositor blurs.
pub(crate) fn window_background(translucent: bool) -> WindowBackgroundAppearance {
    if translucent { WindowBackgroundAppearance::Blurred } else { WindowBackgroundAppearance::Opaque }
}

/// AppKit lays the traffic lights out again while the window is inactive (a screen recorder takes focus), and gpui
/// moves them back only when it becomes key; setting the title (hidden) re-applies `traffic_light_position`.
pub(super) fn reapply_traffic_lights(window: &mut Window) {
    window.set_window_title("Shepherd");
}

/// A tick of the trackpad as a dragged tab, place or row takes another slot (a Force Touch trackpad's; nothing
/// elsewhere).
pub(super) fn haptic() {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSHapticFeedbackManager, NSHapticFeedbackPattern, NSHapticFeedbackPerformanceTime, NSHapticFeedbackPerformer};
        NSHapticFeedbackManager::defaultPerformer().performFeedbackPattern_performanceTime(NSHapticFeedbackPattern::Alignment, NSHapticFeedbackPerformanceTime::Now);
    }
}

/// How long the Mac may sit without a key or a pointer move before Shepherd gives its panes' size away.
// ponytail: a guess at "gone to another device"; a phone attaching sooner keeps Shepherd's size until then
pub(super) const IDLE_YIELD: Duration = Duration::from_secs(60);

/// Since the last key, click or pointer move anywhere on the Mac, not only in Shepherd.
#[cfg(target_os = "macos")]
pub(super) fn mac_idle() -> Duration {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
    }
    // SAFETY: plain values: kCGEventSourceStateHIDSystemState, kCGAnyInputEventType
    let secs = unsafe { CGEventSourceSecondsSinceLastEventType(1, u32::MAX) };
    Duration::try_from_secs_f64(secs).unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
pub(super) fn mac_idle() -> Duration {
    Duration::ZERO
}

/// The Mac's screen is locked (Ctrl+Cmd+Q, the lid closed): gone at once, no idle minute to wait out.
#[cfg(target_os = "macos")]
pub(super) fn screen_locked() -> bool {
    use std::ffi::c_void;
    type Ref = *const c_void;
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGSessionCopyCurrentDictionary() -> Ref;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFDictionaryGetValue(dict: Ref, key: Ref) -> Ref;
        fn CFStringCreateWithCString(alloc: Ref, s: *const std::ffi::c_char, encoding: u32) -> Ref;
        fn CFBooleanGetValue(b: Ref) -> bool;
        fn CFRelease(r: Ref);
    }
    const UTF8: u32 = 0x0800_0100;
    // SAFETY: CoreFoundation calls on objects created here and released here; the value is borrowed from the dict
    unsafe {
        let session = CGSessionCopyCurrentDictionary();
        if session.is_null() {
            return false;
        }
        let key = CFStringCreateWithCString(std::ptr::null(), c"CGSSessionScreenIsLocked".as_ptr(), UTF8);
        let value = CFDictionaryGetValue(session, key);
        let locked = !value.is_null() && CFBooleanGetValue(value);
        CFRelease(key);
        CFRelease(session);
        locked
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn screen_locked() -> bool {
    false
}

/// Turns macOS Secure Keyboard Entry on or off. The system counts enables, so it is called only on a change.
#[cfg(target_os = "macos")]
pub(super) fn secure_input(on: bool) {
    use std::sync::atomic::{AtomicBool, Ordering};
    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn EnableSecureEventInput() -> i32;
        fn DisableSecureEventInput() -> i32;
    }
    static ON: AtomicBool = AtomicBool::new(false);
    if ON.swap(on, Ordering::Relaxed) != on {
        // SAFETY: no arguments; balanced by `ON`
        unsafe {
            if on { EnableSecureEventInput() } else { DisableSecureEventInput() };
        }
    }
}

/// No other system has it.
#[cfg(not(target_os = "macos"))]
pub(super) fn secure_input(_: bool) {}

/// The window's last frame and its display, while that display is plugged in and the frame on it.
pub(super) fn saved_window(cx: &App) -> Option<(gpui::DisplayId, gpui::WindowBounds)> {
    let frame = SettingsGlobal::of(cx).settings.window.clone()?;
    let display = cx.displays().into_iter().find(|d| d.uuid().is_ok_and(|u| u.to_string() == frame.display))?;
    let bounds = frame.bounds_on(display.bounds().size)?;
    let bounds = match (frame.fullscreen, frame.maximized) {
        (true, _) => gpui::WindowBounds::Fullscreen(bounds),
        (_, true) => gpui::WindowBounds::Maximized(bounds),
        _ => gpui::WindowBounds::Windowed(bounds),
    };
    Some((display.id(), bounds))
}

/// The window's frame to save: in full screen, the one it leaves full screen to; zoomed, the one before (`last`).
pub(super) fn window_frame(window: &Window, last: Option<&WindowFrame>, cx: &App) -> Option<WindowFrame> {
    let display = window.display(cx)?.uuid().ok()?.to_string();
    let (bounds, fullscreen) = match window.window_bounds() {
        gpui::WindowBounds::Fullscreen(b) => (b, true),
        b => (b.get_bounds(), false),
    };
    let (x, y, width, height) = (bounds.origin.x.into(), bounds.origin.y.into(), bounds.size.width.into(), bounds.size.height.into());
    let frame = WindowFrame { display, x, y, width, height, fullscreen, maximized: false };
    Some(if !fullscreen && window.is_maximized() { frame.zoomed(last) } else { frame })
}
