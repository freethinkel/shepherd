//! Our NSView for a Ghostty surface: libghostty draws into its layer; we translate AppKit input.
use super::ffi;
use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect};
use std::cell::Cell;
use std::ffi::CString;

pub struct Ivars {
    pub surface: Cell<ffi::ghostty_surface_t>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "ShepherdSurfaceView"]
    #[ivars = Ivars]
    pub struct SurfaceView;

    impl SurfaceView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            self.with_surface(|s| unsafe { ffi::ghostty_surface_set_focus(s, true) });
            true
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            self.with_surface(|s| unsafe { ffi::ghostty_surface_set_focus(s, false) });
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let action = if event.isARepeat() { ffi::GHOSTTY_ACTION_REPEAT } else { ffi::GHOSTTY_ACTION_PRESS };
            self.key(event, action, text_of(event));
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) { self.key(event, ffi::GHOSTTY_ACTION_RELEASE, None); }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            // A modifier went down if its flag is now set; ghostty needs both edges.
            let flag = match event.keyCode() {
                0x38 | 0x3C => NSEventModifierFlags::Shift,
                0x3B | 0x3E => NSEventModifierFlags::Control,
                0x3A | 0x3D => NSEventModifierFlags::Option,
                0x37 | 0x36 => NSEventModifierFlags::Command,
                0x39 => NSEventModifierFlags::CapsLock,
                _ => return,
            };
            let down = event.modifierFlags().contains(flag);
            self.key(event, if down { ffi::GHOSTTY_ACTION_PRESS } else { ffi::GHOSTTY_ACTION_RELEASE }, None);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if let Some(window) = self.window() { window.makeFirstResponder(Some(self)); }
            self.button(event, ffi::GHOSTTY_MOUSE_PRESS, ffi::GHOSTTY_MOUSE_LEFT);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) { self.button(event, ffi::GHOSTTY_MOUSE_RELEASE, ffi::GHOSTTY_MOUSE_LEFT); }

        #[unsafe(method(rightMouseDown:))]
        fn right_down(&self, event: &NSEvent) { self.button(event, ffi::GHOSTTY_MOUSE_PRESS, ffi::GHOSTTY_MOUSE_RIGHT); }

        #[unsafe(method(rightMouseUp:))]
        fn right_up(&self, event: &NSEvent) { self.button(event, ffi::GHOSTTY_MOUSE_RELEASE, ffi::GHOSTTY_MOUSE_RIGHT); }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) { self.position(event); }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let precise = event.hasPreciseScrollingDeltas();
            let (mut dx, mut dy) = (event.scrollingDeltaX(), event.scrollingDeltaY());
            if precise { dx *= 2.0; dy *= 2.0; } // Ghostty's own AppKit view does the same
            self.with_surface(|s| unsafe { ffi::ghostty_surface_mouse_scroll(s, dx, dy, precise as i32) });
        }
    }
);

impl SurfaceView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(Ivars { surface: Cell::new(std::ptr::null_mut()) });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }

    fn with_surface(&self, f: impl FnOnce(ffi::ghostty_surface_t)) {
        let s = self.ivars().surface.get();
        if !s.is_null() {
            f(s);
        }
    }

    fn key(&self, event: &NSEvent, action: ffi::ghostty_input_action_e, text: Option<CString>) {
        let unshifted = event
            .charactersIgnoringModifiers()
            .and_then(|s| s.to_string().chars().next())
            .map_or(0, u32::from);
        let key = ffi::ghostty_input_key_s {
            action,
            mods: mods_of(event),
            consumed_mods: ffi::GHOSTTY_MODS_NONE,
            keycode: u32::from(event.keyCode()),
            text: text.as_ref().map_or(std::ptr::null(), |t| t.as_ptr()),
            unshifted_codepoint: unshifted,
            composing: false,
        };
        self.with_surface(|s| unsafe {
            ffi::ghostty_surface_key(s, key);
        });
    }

    fn button(&self, event: &NSEvent, state: ffi::ghostty_input_mouse_state_e, button: ffi::ghostty_input_mouse_button_e) {
        self.position(event);
        self.with_surface(|s| unsafe {
            ffi::ghostty_surface_mouse_button(s, state, button, mods_of(event));
        });
    }

    fn position(&self, event: &NSEvent) {
        let p: NSPoint = self.convertPoint_fromView(event.locationInWindow(), None);
        let height = self.frame().size.height;
        self.with_surface(|s| unsafe { ffi::ghostty_surface_mouse_pos(s, p.x, height - p.y, mods_of(event)) });
    }
}

// ponytail: no NSTextInputClient yet, so no IME composition (CJK, dead keys); plan 2 adds it.
fn text_of(event: &NSEvent) -> Option<CString> {
    let s = event.characters()?.to_string();
    let c = u32::from(s.chars().next()?);
    // Control characters and AppKit's function-key range: ghostty encodes these from keycode + mods.
    if c < 0x20 || (0xF700..=0xF8FF).contains(&c) {
        return None;
    }
    CString::new(s).ok()
}

fn mods_of(event: &NSEvent) -> ffi::ghostty_input_mods_e {
    let f = event.modifierFlags();
    let mut m = ffi::GHOSTTY_MODS_NONE;
    if f.contains(NSEventModifierFlags::Shift) { m |= ffi::GHOSTTY_MODS_SHIFT; }
    if f.contains(NSEventModifierFlags::Control) { m |= ffi::GHOSTTY_MODS_CTRL; }
    if f.contains(NSEventModifierFlags::Option) { m |= ffi::GHOSTTY_MODS_ALT; }
    if f.contains(NSEventModifierFlags::Command) { m |= ffi::GHOSTTY_MODS_SUPER; }
    if f.contains(NSEventModifierFlags::CapsLock) { m |= ffi::GHOSTTY_MODS_CAPS; }
    m
}
