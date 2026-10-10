use super::ffi;
use futures::StreamExt;
use futures::channel::mpsc;
use std::ffi::c_void;
use std::sync::OnceLock;

static WAKE: OnceLock<mpsc::UnboundedSender<()>> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct App(pub(crate) ffi::ghostty_app_t);

extern "C" fn wakeup(_: *mut c_void) {
    if let Some(tx) = WAKE.get() {
        let _ = tx.unbounded_send(());
    }
}

extern "C" fn action(_: ffi::ghostty_app_t, _: ffi::ghostty_target_s, action: ffi::ghostty_action_s) -> bool {
    // ponytail: plan 2 maps new_split / goto_split / close_surface / new_tab… to herdr calls here.
    eprintln!("ghostty action {}", action.tag);
    false
}

extern "C" fn read_clipboard(_: *mut c_void, _: ffi::ghostty_clipboard_e, _: *mut c_void) -> bool {
    false // paste: plan 2 (ghostty_surface_complete_clipboard_request with the pasteboard text)
}

extern "C" fn confirm_read_clipboard(
    _: *mut c_void,
    _: *const std::ffi::c_char,
    _: *mut c_void,
    _: ffi::ghostty_clipboard_request_e,
) {
}

extern "C" fn write_clipboard(
    _: *mut c_void,
    _: ffi::ghostty_clipboard_e,
    content: *const ffi::ghostty_clipboard_content_s,
    len: usize,
    _: bool,
) {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;
    if len == 0 || content.is_null() {
        return;
    }
    let data = unsafe { (*content).data };
    if data.is_null() {
        return;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(data) }.to_string_lossy();
    unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        pb.setString_forType(&NSString::from_str(&text), NSPasteboardTypeString);
    }
}

extern "C" fn close_surface(_: *mut c_void, _: bool) {}

impl App {
    pub fn init(cx: &mut gpui::App) -> Self {
        unsafe {
            assert_eq!(ffi::ghostty_init(0, std::ptr::null_mut()), 0, "ghostty_init");
            let config = ffi::ghostty_config_new();
            ffi::ghostty_config_load_default_files(config);
            ffi::ghostty_config_finalize(config);
            let runtime = ffi::ghostty_runtime_config_s {
                userdata: std::ptr::null_mut(),
                supports_selection_clipboard: false,
                wakeup_cb: Some(wakeup),
                action_cb: Some(action),
                read_clipboard_cb: Some(read_clipboard),
                confirm_read_clipboard_cb: Some(confirm_read_clipboard),
                write_clipboard_cb: Some(write_clipboard),
                close_surface_cb: Some(close_surface),
            };
            let raw = ffi::ghostty_app_new(&runtime, config);
            assert!(!raw.is_null(), "ghostty_app_new");
            let (tx, mut rx) = mpsc::unbounded();
            WAKE.set(tx).ok();
            let app = App(raw);
            cx.spawn(async move |_| {
                while rx.next().await.is_some() {
                    ffi::ghostty_app_tick(app.0);
                }
            })
            .detach();
            app
        }
    }
}
