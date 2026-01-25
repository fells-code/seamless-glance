use crate::{resources::ssh::SshContext, ui::overlay::scroll::ScrollableOverlay};

pub struct DescribeOverlayState {
    pub title: String,
    pub content: String,
    pub scroll: u16,
}

pub struct ConfirmCommandState {
    pub title: String,
    pub command: String,
    pub scroll: u16,
}

pub struct SelectSshKeyState {
    pub title: String,
    pub context: SshContext,
    pub selected: usize,
}

pub enum OverlayState {
    Describe(DescribeOverlayState),
    ConfirmCommand(ConfirmCommandState),
    SelectSshKey(SelectSshKeyState),
}

impl OverlayState {
    pub fn scroll_up(&mut self) {
        match self {
            OverlayState::Describe(o) => o.scroll_up(),
            OverlayState::ConfirmCommand(o) => o.scroll_up(),
            OverlayState::SelectSshKey(_) => todo!(),
        }
    }

    pub fn scroll_down(&mut self) {
        match self {
            OverlayState::Describe(o) => o.scroll_down(),
            OverlayState::ConfirmCommand(o) => o.scroll_down(),
            OverlayState::SelectSshKey(_) => todo!(),
        }
    }
}
