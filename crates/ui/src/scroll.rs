//! Keyboard paging for the application's scrollable panels.
use gpui_kit::{prelude::*, *};

actions!(panel_scroll, [PageUp, PageDown]);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("pageup", PageUp, Some("ScrollablePanel")),
        KeyBinding::new("pagedown", PageDown, Some("ScrollablePanel")),
    ]);
}

/// Adds page navigation to a panel using its retained scroll state.
pub trait PageScroll: StatefulInteractiveElement + Sized {
    fn page_scroll(self, scroll: &ScrollHandle) -> Self {
        let up = scroll.clone();
        let down = scroll.clone();
        self.track_scroll(scroll)
            .focusable()
            .key_context("ScrollablePanel")
            .on_action(move |_: &PageUp, window, cx| page(&up, false, window, cx))
            .on_action(move |_: &PageDown, window, cx| page(&down, true, window, cx))
    }

    /// Adds the same navigation to a panel backed by a virtualized list.
    fn page_scroll_list(self, list: &ListState) -> Self {
        let up = list.clone();
        let down = list.clone();
        self.focusable()
            .key_context("ScrollablePanel")
            .on_action(move |_: &PageUp, window, _| {
                up.scroll_by(-up.viewport_bounds().size.height * 0.9);
                window.refresh();
            })
            .on_action(move |_: &PageDown, window, _| {
                down.scroll_by(down.viewport_bounds().size.height * 0.9);
                window.refresh();
            })
    }
}

impl<T: StatefulInteractiveElement> PageScroll for T {}

fn page(scroll: &ScrollHandle, down: bool, window: &mut Window, cx: &mut App) {
    if scroll.max_offset().y <= px(0.) {
        cx.propagate();
        return;
    }
    let mut offset = scroll.offset();
    // Keep some overlapping content so the reader can follow the next page.
    let step = scroll.bounds().size.height * 0.9;
    offset.y = (offset.y + if down { -step } else { step }).clamp(-scroll.max_offset().y, px(0.));
    scroll.set_offset(offset);
    window.refresh();
}
