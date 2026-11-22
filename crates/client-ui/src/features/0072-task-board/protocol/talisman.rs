//! P05 purchase/upgrade through the existing full Task Board owner.
use super::{FullIntent, TaskBoardPanel};
use oteryn_client_runtime::raw_feature::FeatureCommand;

/// The highest level a Talisman path reaches; the server refuses an upgrade past it.
pub const MAX_LEVEL: u16 = 355;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Buy,
    /// Damage, life leech, loot, Bestiary (zero-based, task_board_v1.md).
    Upgrade(u8),
}

pub fn request(panel: &mut TaskBoardPanel, intent: Intent) -> Option<FeatureCommand> {
    super::full_request(
        panel,
        match intent {
            Intent::Buy => FullIntent::BuyTalisman,
            Intent::Upgrade(path) => FullIntent::UpgradeTalisman(path),
        },
    )
}
