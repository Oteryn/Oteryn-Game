//! Oteryn-authored player help, available without a network request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Basics,
    Recommendations,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Guide {
    page: Page,
}
impl Guide {
    pub fn select(&mut self, page: Page) {
        self.page = page;
    }
    pub fn text(&self) -> String {
        match self.page {
            Page::Basics => "Player Guide\r\n\r\nMovement: use arrow keys or left-click a reachable map tile.\r\nInteraction: right-click a creature or object to inspect its available actions.\r\nInventory: select an item before using, moving or equipping it. The server confirms changes.\r\nCombat: select a target and check your fight and chase settings. Stop attacking before approaching other players.\r\nTutorial overlay: Ctrl+Alt+Shift+N toggles contextual hints, target markers and the mouse trail.\r\nNotices: Ctrl+Shift+N reopens this guide and the latest tutorial/server-save messages.\r\nLeaving: use the logout action and wait for server confirmation; a refusal leaves the session active.",
            Page::Recommendations => "Recommendations\r\n\r\nBefore exploring: check equipment, carrying capacity and healing supplies.\r\nBefore a hunt: choose a suitable target, check your route back and keep reserve supplies.\r\nDuring a hunt: monitor health, mana and conditions; stop and recover before continuing if resources are low.\r\nBefore server-save: read the latest announcement in Notices and find a safe place to log out.\r\nAfter connection loss: return to sign-in and start a fresh browser admission. Do not reuse an old ticket.\r\nIf something looks wrong: record reproduction steps and review a bug report before sending it.",
        }.to_owned()
    }
}
