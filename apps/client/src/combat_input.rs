//! Compatibility exports and the shell's chronological command vocabulary.
use crate::input::StepDir;
pub use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};

/// What the shell asks of the session task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayCommand {
    Animus(Vec<u8>),
    /// Capability-16 query encoded by its feature controls.
    QuestLog(Vec<u8>),
    Exiva(Vec<u8>),
    LoyaltySkills(Vec<u8>),
    Wheel(oteryn_client_runtime::wheel_allocation::WheelCommand),
    Step(StepDir),
    Combat(CombatCommand),
    /// Leave the game (capability 21 `LOGOUT_INTENT`).
    Logout,
    TaskBoard(oteryn_client_runtime::task_board::TaskBoardCommand),
    Stash(oteryn_client_runtime::stash::StashCommand),
    NpcShop(oteryn_client_runtime::npc_shop::NpcShopCommand),
    Prey(oteryn_client_runtime::prey::PreyCommand),
    Imbue(oteryn_client_runtime::imbue::ImbueCommand),
    /// Capability-60 direct player-trade intent.
    PlayerTrade(oteryn_client_runtime::player_trade::PlayerTradeCommand),
    Vip(oteryn_client_runtime::vip::VipCommand),
    Guild(oteryn_client_runtime::guild::GuildCommand),
    Party(oteryn_client_runtime::party::PartyCommand),
    Hireling(oteryn_client_runtime::hireling::HirelingCommand),
    Market(oteryn_client_runtime::market::MarketCommand),
}
