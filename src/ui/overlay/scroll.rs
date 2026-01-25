use crate::ui::overlay::overlays::{ConfirmCommandState, DescribeOverlayState};

pub trait ScrollableOverlay {
    fn scroll_up(&mut self);
    fn scroll_down(&mut self);
}

impl ScrollableOverlay for DescribeOverlayState {
    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
}

impl ScrollableOverlay for ConfirmCommandState {
    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
}
