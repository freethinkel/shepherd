use super::{App, ffi, view::SurfaceView};
use gpui::{Bounds, Pixels, Window};
use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::CString;

pub struct Surface {
    view: Retained<SurfaceView>,
    parent: Retained<NSView>,
    raw: ffi::ghostty_surface_t,
}

/// GPUI's content NSView; native views go below its overlay so GPUI popovers stay on top.
fn parent_view(window: &Window) -> anyhow::Result<Retained<NSView>> {
    let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(window).map_err(|e| anyhow::anyhow!("{e:?}"))?.as_raw() else {
        anyhow::bail!("not an AppKit window");
    };
    unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }.ok_or_else(|| anyhow::anyhow!("no NSView"))
}

impl Surface {
    pub fn new(app: App, window: &mut Window, pane_id: &str) -> anyhow::Result<Self> {
        let mtm = MainThreadMarker::new().ok_or_else(|| anyhow::anyhow!("not on the main thread"))?;
        window.enable_scene_overlay().map_err(|e| anyhow::anyhow!("{e}"))?;
        let parent = parent_view(window)?;
        let view = SurfaceView::new(mtm);
        parent.addSubview(&view);
        for sibling in parent.subviews() {
            if sibling.class().name() == c"GPUIOverlayView" {
                parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, Some(&sibling));
                break;
            }
        }
        let command = CString::new(crate::attach::command_line(&std::env::current_exe()?, pane_id))?;
        let raw = unsafe {
            let mut config = ffi::ghostty_surface_config_new();
            config.platform_tag = ffi::GHOSTTY_PLATFORM_MACOS;
            config.platform.macos.nsview = Retained::as_ptr(&view) as *mut _;
            config.scale_factor = f64::from(window.scale_factor());
            config.command = command.as_ptr();
            ffi::ghostty_surface_new(app.0, &config)
        };
        anyhow::ensure!(!raw.is_null(), "ghostty_surface_new failed for {pane_id}");
        view.ivars().surface.set(raw);
        Ok(Self { view, parent, raw })
    }

    pub fn set_bounds(&self, bounds: Bounds<Pixels>, window: &Window) {
        let (x, y) = (f64::from(f32::from(bounds.origin.x)), f64::from(f32::from(bounds.origin.y)));
        let (w, h) = (f64::from(f32::from(bounds.size.width)).max(1.0), f64::from(f32::from(bounds.size.height)).max(1.0));
        let y = if self.parent.isFlipped() { y } else { self.parent.bounds().size.height - y - h };
        let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(w, h));
        if self.view.frame() != frame {
            self.view.setFrame(frame);
            let scale = f64::from(window.scale_factor());
            unsafe {
                ffi::ghostty_surface_set_content_scale(self.raw, scale, scale);
                ffi::ghostty_surface_set_size(self.raw, (w * scale) as u32, (h * scale) as u32);
            }
        }
    }

    /// The shim ended (pane closed, herdr gone, attach refused); Ghostty shows "process exited".
    pub fn exited(&self) -> bool {
        unsafe { ffi::ghostty_surface_process_exited(self.raw) }
    }

    pub fn focus(&self) {
        if let Some(window) = self.view.window() {
            window.makeFirstResponder(Some(&self.view));
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.view.ivars().surface.set(std::ptr::null_mut());
        unsafe { ffi::ghostty_surface_free(self.raw) };
        self.view.removeFromSuperview();
    }
}
