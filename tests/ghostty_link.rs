use shepherd::ghostty::ffi;

#[test]
fn libghostty_links_and_initializes() {
    let rc = unsafe { ffi::ghostty_init(0, std::ptr::null_mut()) };
    assert_eq!(rc, 0, "ghostty_init failed");
    let info = unsafe { ffi::ghostty_info() };
    assert!(!info.version.is_null() && info.version_len > 0);
}
