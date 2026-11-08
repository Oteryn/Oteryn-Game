//! Oteryn-authored player help, available without a network request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Basics,
    Recommendations,
    Updates,
    Features,
    Help,
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
            Page::Updates => "Updates\r\n\r\nRelease news and event descriptions are published by your operator in the client bulletin. Open News or browse Calendar at sign-in for the bundled feed. Updates never ask for payment or provider credentials in this guide.",
            Page::Features => "Features\r\n\r\nCharacter: inspect skills, equipment, appearance and conditions.\r\nExploration: use the map, minimap and markers to plan a route.\r\nProgression: browse quests and track unfinished missions; inspect achievements and creature progression.\r\nEconomy: use shops, containers, depot, stash and market panels. Review transfers before confirming.\r\nCommunity: use chat, party, guild and VIP panels.\r\nClient settings: adjust input, audio and presentation from Preferences.",
            Page::Help => "Help and support\r\n\r\nBug report: Ctrl+Shift+F12 opens categories for map, typo, client behavior and other issues. Include reproduction steps and review the exact report. Build details are optional.\r\nViolation: Ctrl+Alt+R opens player reports. Select name, statement or bot; choose the operator's reason code and quote evidence. An optional translation is marked unverified. Review before sending.\r\nCrash report: after a crash, the next launch offers a preview and asks before saving a local report. No automatic upload occurs.\r\nConnection problems: use Retry sign-in to start a fresh attempt. For installation problems, use the launcher installation check and follow the repair instructions.\r\nAbout and optional survey are available in the sign-in window.",
            Page::Basics => "Player Guide\r\n\r\nMovement: use arrow keys or left-click a reachable map tile.\r\nInteraction: right-click a creature or object to inspect its available actions.\r\nInventory: select an item before using, moving or equipping it. The server confirms changes.\r\nCombat: select a target and check your fight and chase settings. Stop attacking before approaching other players.\r\nTutorial overlay: Ctrl+Alt+Shift+N toggles contextual hints, target markers and the mouse trail.\r\nNotices: Ctrl+Shift+N reopens this guide and the latest tutorial/server-save messages.\r\nLeaving: use the logout action and wait for server confirmation; a refusal leaves the session active.",
            Page::Recommendations => "Recommendations\r\n\r\nBefore exploring: check equipment, carrying capacity and healing supplies.\r\nBefore a hunt: choose a suitable target, check your route back and keep reserve supplies.\r\nDuring a hunt: monitor health, mana and conditions; stop and recover before continuing if resources are low.\r\nBefore server-save: read the latest announcement in Notices and find a safe place to log out.\r\nAfter connection loss: return to sign-in and start a fresh browser admission. Do not reuse an old ticket.\r\nIf something looks wrong: record reproduction steps and review a bug report before sending it.",
        }.to_owned()
    }
}
