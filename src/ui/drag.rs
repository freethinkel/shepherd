//! Files dragged out of a row, Comma's `desktop/src/drag.rs`: on macOS the drag is handed to an AppKit session as it
//! starts, so its ghost goes on past the window's edge and the files drop on a pane (as Finder's do), Finder or
//! another app. Off macOS nothing takes it.
use gpui::{AnyElement, App, Div, Hsla, Pixels, Point, SharedString, Size, Window, div, img, prelude::*, svg};
use std::path::PathBuf;

/// How the dragged rows look, lifted.
#[derive(Clone)]
pub struct Ghost {
    pub paths: Vec<PathBuf>,
    pub title: SharedString,
    /// An asset: an SVG tinted, or an image as it is.
    pub icon: SharedString,
    pub tint: Option<Hsla>,
    pub size: Size<Pixels>,
    pub radius: Pixels,
    /// Before the icon, and after the title.
    pub pad: (Pixels, Pixels),
    pub icon_size: Pixels,
    /// Between the icon and the title.
    pub gap: Pixels,
    pub text_size: Pixels,
    pub bg: Hsla,
    pub border: Hsla,
    pub text: Hsla,
}

impl Ghost {
    pub fn element(&self) -> Div {
        let icon: AnyElement = match self.tint {
            Some(tint) => svg().path(self.icon.clone()).size(self.icon_size).flex_none().text_color(tint).into_any_element(),
            None => img(self.icon.clone()).size(self.icon_size).flex_none().into_any_element(),
        };
        div()
            .w(self.size.width)
            .h(self.size.height)
            .pl(self.pad.0)
            .pr(self.pad.1)
            .flex()
            .items_center()
            .gap(self.gap)
            .rounded(self.radius)
            .bg(self.bg)
            .border_1()
            .border_color(self.border)
            .shadow_lg()
            .text_size(self.text_size)
            .text_color(self.text)
            .child(icon)
            .child(div().flex_1().min_w_0().truncate().child(self.title.clone()))
    }
}

/// GPUI's drag of a `Ghost`, drawn until AppKit takes it.
#[derive(Clone)]
pub struct Dragged(pub Ghost);

impl Render for Dragged {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        self.0.element()
    }
}

/// For `on_drag`: GPUI's drag that has just started goes to AppKit, the ghost's top-left `grab` back from the pointer.
pub fn go_native(ghost: Ghost, grab: Point<Pixels>, window: &mut Window, cx: &mut App) {
    window.defer(cx, move |window, cx| {
        if hand_off(&ghost, window.mouse_position() - grab) {
            cx.stop_active_drag(window);
        }
    });
}

/// Hands the drag under way to AppKit, the ghost's top-left at `origin` in the window: its image goes on with the
/// pointer and the pasteboard carries the files. False where there is no native drag (off macOS, or no mouse drag
/// being handled).
#[cfg(not(target_os = "macos"))]
fn hand_off(_: &Ghost, _: Point<Pixels>) -> bool {
    false
}

#[cfg(target_os = "macos")]
use mac::hand_off;

#[cfg(target_os = "macos")]
#[allow(deprecated)] // NSFilenamesPboardType: what GPUI's windows read a drop's files from
mod mac {
    use super::Ghost;
    use gpui::{AssetSource, Hsla, Pixels, Point, Rgba};
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool, NSObject, NSObjectProtocol, ProtocolObject};
    use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, define_class, msg_send};
    use objc2_app_kit::{
        NSApplication, NSBezierPath, NSColor, NSCompositingOperation, NSDragOperation, NSDraggingContext, NSDraggingItem, NSDraggingSession, NSDraggingSource,
        NSEvent, NSEventType, NSFilenamesPboardType, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSGraphicsContext, NSImage, NSLineBreakMode,
        NSMutableParagraphStyle, NSParagraphStyleAttributeName, NSPasteboardItem, NSPasteboardTypeFileURL, NSShadow, NSStringDrawing,
    };
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSPoint, NSRect, NSSize, NSString, NSURL};
    use std::cell::RefCell;

    /// Room around the ghost for its shadow.
    const SHADOW: f64 = 12.;

    thread_local! {
        /// The session's source, kept alive until the next drag.
        static SOURCE: RefCell<Option<Retained<Source>>> = const { RefCell::new(None) };
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        struct Source;

        unsafe impl NSObjectProtocol for Source {}

        unsafe impl NSDraggingSource for Source {
            #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
            fn mask(&self, _: &NSDraggingSession, context: NSDraggingContext) -> NSDragOperation {
                // a copy: Finder copies the files; a pane pastes their paths, no "+" on the pointer over Shepherd
                match context {
                    NSDraggingContext::WithinApplication => NSDragOperation::Copy | NSDragOperation::Generic,
                    _ => NSDragOperation::Copy,
                }
            }
        }
    );

    pub fn hand_off(ghost: &Ghost, origin: Point<Pixels>) -> bool {
        let Some(mtm) = MainThreadMarker::new() else { return false };
        let Some(event) = NSApplication::sharedApplication(mtm).currentEvent() else { return false };
        if event.r#type() != NSEventType::LeftMouseDragged || ghost.paths.is_empty() {
            return false;
        }
        let Some(window) = event.window(mtm) else { return false };
        let Some(content) = window.contentView() else { return false };
        // GPUI's view takes the mouse: the session starts from it
        let view = content.subviews().iter().find(|v| v.class().name() == c"GPUIView").unwrap_or(content);

        let (w, h) = (f64::from(f32::from(ghost.size.width)), f64::from(f32::from(ghost.size.height)));
        let (x, top) = (f64::from(f32::from(origin.x)), f64::from(f32::from(origin.y)));
        let y = if view.isFlipped() { top } else { view.bounds().size.height - top - h };
        let frame = NSRect::new(NSPoint::new(x - SHADOW, y - SHADOW), NSSize::new(w + 2. * SHADOW, h + 2. * SHADOW));
        let image = image(ghost, frame.size);

        // one item a file, the ghost on the first: GPUI's windows read every item's NSFilenamesPboardType
        let mut items = Vec::new();
        for (i, path) in ghost.paths.iter().enumerate() {
            let item = NSPasteboardItem::new();
            let path = NSString::from_str(&path.to_string_lossy());
            let Some(href) = NSURL::fileURLWithPath(&path).absoluteString() else { return false };
            item.setString_forType(&href, unsafe { NSPasteboardTypeFileURL });
            let paths = NSArray::from_retained_slice(&[path]);
            unsafe { item.setPropertyList_forType(&paths, NSFilenamesPboardType) };
            let dragging = NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), &ProtocolObject::from_retained(item));
            unsafe { dragging.setDraggingFrame_contents(frame, (i == 0).then_some(&*image)) };
            items.push(dragging);
        }

        let source: Retained<Source> = unsafe { msg_send![Source::alloc(mtm), init] };
        let items = NSArray::from_retained_slice(&items);
        let session = view.beginDraggingSessionWithItems_event_source(&items, &event, ProtocolObject::from_ref(&*source));
        session.setAnimatesToStartingPositionsOnCancelOrFail(true);
        SOURCE.with(|s| *s.borrow_mut() = Some(source));

        // AppKit has the mouse now and GPUI won't see it let go: end GPUI's press, or its window keeps replaying the
        // last drag event (it does while a press lasts, to scroll a selection).
        let up = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            NSEventType::LeftMouseUp,
            event.locationInWindow(),
            event.modifierFlags(),
            event.timestamp(),
            event.windowNumber(),
            None,
            0,
            1,
            0.,
        );
        if let Some(up) = up {
            view.mouseUp(&up);
        }
        true
    }

    fn color(c: Hsla) -> Retained<NSColor> {
        let c = Rgba::from(c);
        NSColor::colorWithSRGBRed_green_blue_alpha(c.r.into(), c.g.into(), c.b.into(), c.a.into())
    }

    /// The ghost as `Ghost::element` draws it, in an image `size` big with the shadow's room around it.
    pub(super) fn image(ghost: &Ghost, size: NSSize) -> Retained<NSImage> {
        let ghost = ghost.clone();
        let icon = crate::assets::Assets.load(&ghost.icon).ok().flatten().and_then(|data| {
            let data = match ghost.tint {
                Some(tint) => {
                    let c = Rgba::from(tint);
                    let hex = format!("#{:02x}{:02x}{:02x}", (c.r * 255.) as u8, (c.g * 255.) as u8, (c.b * 255.) as u8);
                    String::from_utf8_lossy(&data).replace("currentColor", &hex).into_bytes()
                }
                None => data.into_owned(),
            };
            NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&data))
        });
        let draw = block2::RcBlock::new(move |_: NSRect| -> Bool {
            let px = |p: Pixels| f64::from(f32::from(p));
            let card = NSRect::new(NSPoint::new(SHADOW, SHADOW), NSSize::new(size.width - 2. * SHADOW, size.height - 2. * SHADOW));
            let radius = px(ghost.radius).min(card.size.height / 2.);
            let shape = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(card, radius, radius);
            NSGraphicsContext::saveGraphicsState_class();
            let shadow = NSShadow::new();
            shadow.setShadowColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(0., 0., 0., 0.18)));
            shadow.setShadowBlurRadius(10.);
            shadow.setShadowOffset(NSSize::new(0., 4.));
            shadow.set();
            color(ghost.bg).setFill();
            shape.fill();
            NSGraphicsContext::restoreGraphicsState_class();
            color(ghost.border).setStroke();
            shape.setLineWidth(1.);
            shape.stroke();

            let font = NSFont::systemFontOfSize(px(ghost.text_size));
            let line = font.ascender() - font.descender();
            let icon_size = px(ghost.icon_size);
            let icon_x = card.origin.x + px(ghost.pad.0);
            let middle = card.origin.y + card.size.height / 2.;
            if let Some(icon) = &icon {
                unsafe {
                    icon.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                        NSRect::new(NSPoint::new(icon_x, middle - icon_size / 2.), NSSize::new(icon_size, icon_size)),
                        NSRect::ZERO,
                        NSCompositingOperation::SourceOver,
                        1.,
                        true,
                        None,
                    )
                };
            }
            let title_x = icon_x + icon_size + px(ghost.gap);
            let title_w = card.origin.x + card.size.width - px(ghost.pad.1) - title_x;
            text(&ghost.title, &font, ghost.text, NSRect::new(NSPoint::new(title_x, middle - line / 2.), NSSize::new(title_w.max(0.), line)));
            Bool::YES
        });
        NSImage::imageWithSize_flipped_drawingHandler(size, true, &draw)
    }

    /// One line, cut short with an ellipsis.
    fn text(s: &str, font: &NSFont, tint: Hsla, rect: NSRect) {
        let style = NSMutableParagraphStyle::new();
        style.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        let tint = color(tint);
        let attributes = unsafe {
            NSDictionary::<NSString, AnyObject>::from_slices(
                &[NSFontAttributeName, NSForegroundColorAttributeName, NSParagraphStyleAttributeName],
                &[font.as_ref(), tint.as_ref(), style.as_ref()],
            )
        };
        unsafe { NSString::from_str(s).drawInRect_withAttributes(rect, Some(&attributes)) };
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use gpui::px;

    /// The native ghost draws, a file icon in it (set SHEPHERD_GHOST_TIFF to a path to look at it).
    #[test]
    fn native_ghost_draws() {
        let ghost = Ghost {
            paths: vec!["/tmp/main.rs".into()],
            title: "A rather long file name that gets cut short.rs".into(),
            icon: crate::ui::file_icons::asset_path("main.rs", crate::theme::Appearance::Dark),
            tint: None,
            size: gpui::size(px(240.), px(28.)),
            radius: px(7.),
            pad: (px(22.), px(8.)),
            icon_size: px(16.),
            gap: px(6.),
            text_size: px(13.),
            bg: gpui::white(),
            border: gpui::hsla(0., 0., 0., 0.1),
            text: gpui::black(),
        };
        let image = mac::image(&ghost, objc2_foundation::NSSize::new(264., 52.));
        let tiff = image.TIFFRepresentation().expect("drawn");
        if let Ok(path) = std::env::var("SHEPHERD_GHOST_TIFF") {
            std::fs::write(path, tiff.to_vec()).unwrap();
        }
        assert!(tiff.len() > 1000);
    }
}
