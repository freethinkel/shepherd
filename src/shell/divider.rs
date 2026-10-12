//! Dragging a split divider: the panes reflow at its ratio while it moves, and herdr gets the final one.
use super::*;

impl Shell {
    pub(super) fn start_divider(&mut self, tab_id: String, split: &SplitFrame, cx: &mut Context<Self>) {
        let Some(grid) = self.grid_bounds.get() else { return };
        let (origin, size) = if split.vertical {
            (f32::from(grid.origin.x) + f32::from(grid.size.width) * split.frame.x, f32::from(grid.size.width) * split.frame.w)
        } else {
            (f32::from(grid.origin.y) + f32::from(grid.size.height) * split.frame.y, f32::from(grid.size.height) * split.frame.h)
        };
        cx.set_global(GridHold(true));
        self.divider = Some(DividerDrag {
            tab_id,
            path: split.path.clone(),
            vertical: split.vertical,
            start_px: origin,
            len_px: size,
            ratio: split.ratio,
            throttle: Throttle::default(),
        });
        cx.notify();
    }

    pub(super) fn drag_divider(&mut self, position: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = &mut self.divider else { return };
        let pointer = f32::from(if drag.vertical { position.x } else { position.y });
        drag.ratio = drag_ratio(drag.start_px, drag.len_px, pointer);
        if drag.throttle.ready(Instant::now()) {
            let command = Command::SetSplitRatio { tab_id: drag.tab_id.clone(), path: drag.path.clone(), ratio: drag.ratio };
            self.send(command, cx, |_, _, _| {});
        }
        cx.notify();
    }

    pub(super) fn end_divider(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.divider.take() else { return };
        self.settling = Some((drag.path.clone(), drag.ratio));
        // The last position, unthrottled, so herdr ends where the pointer let go.
        self.send(Command::SetSplitRatio { tab_id: drag.tab_id, path: drag.path, ratio: drag.ratio }, cx, |_, _, _| {});
        // Panes held their grids during the drag: re-measure every one now (refresh bypasses view caching).
        cx.set_global(GridHold(false));
        cx.refresh_windows();
        // herdr normally answers within a few frames; never hold a stale preview longer than a second.
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let _ = this.update(cx, |shell, cx| {
                shell.settling = None;
                cx.notify();
            });
        })
        .detach();
    }
}
