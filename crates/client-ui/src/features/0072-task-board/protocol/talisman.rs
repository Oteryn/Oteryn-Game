//! P05 purchase/upgrade through the existing full Task Board owner.
use super::{TaskBoardPanel, task_board};
use oteryn_client_runtime::raw_feature::FeatureCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Buy,
    /// Damage, life leech, loot, Bestiary (zero-based, task_board_v1.md).
    Upgrade(u8),
}

pub fn request(panel: &mut TaskBoardPanel, intent: Intent) -> Option<FeatureCommand> {
    let (action, selection) = match intent {
        Intent::Buy => (17, 0),
        Intent::Upgrade(path) => (18, u16::from(path)),
    };
    super::protocol::command(
        panel,
        task_board::full::Request {
            action,
            selection,
            race: String::new(),
        },
    )
}
