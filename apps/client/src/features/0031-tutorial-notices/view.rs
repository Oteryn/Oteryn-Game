use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn notices_panel(&self) -> oteryn_client_ui::tutorial_notices::NoticePanel {
        lock(&self.mailbox).notices.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(test)]
#[path = "guide_tests.rs"]
mod guide_tests;

/// The native compendium presenter for live notices and the two player-guide tabs.
#[cfg(any(windows, test))]
pub(super) fn compendium_text(
    panel: &oteryn_client_ui::tutorial_notices::NoticePanel,
    tab: usize,
) -> Option<String> {
    if !panel.open {
        return None;
    }
    if tab == 0 {
        return Some(oteryn_client_ui::tutorial_notices::panel_text(panel));
    }
    let mut guide = oteryn_client_ui::guide::Guide::default();
    guide.select(match tab {
        1 => oteryn_client_ui::guide::Page::Basics,
        2 => oteryn_client_ui::guide::Page::Recommendations,
        _ => return None,
    });
    Some(guide.text())
}
