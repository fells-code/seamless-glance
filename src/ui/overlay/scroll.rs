use crate::ui::overlay::overlays::{ConfirmCommandState, DescribeOverlayState};

pub trait ScrollableOverlay {
    fn scroll_up(&mut self);
    fn scroll_down(&mut self);
    fn page_up(&mut self, lines: u16);
    fn page_down(&mut self, lines: u16);
    fn scroll_to_top(&mut self);
    fn scroll_to_bottom(&mut self);
}

impl ScrollableOverlay for DescribeOverlayState {
    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    fn page_up(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    fn page_down(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_add(lines);
    }

    fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll = u16::MAX;
    }
}

impl ScrollableOverlay for ConfirmCommandState {
    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    fn page_up(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    fn page_down(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_add(lines);
    }

    fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll = u16::MAX;
    }
}
