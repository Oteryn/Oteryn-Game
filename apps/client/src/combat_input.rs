//! Compatibility exports and the shell's chronological command vocabulary.
use crate::input::StepDir;
pub use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};

/// What the shell asks of the session task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayCommand {
    HudStamina,
    BrowseFieldMove(oteryn_session::ItemMoveIntent),
    Depot(oteryn_session::DepotIntent),
    DepotContainer(oteryn_session::ItemHandle),
    AccountAchievements(u32),
    Animus(Vec<u8>),
    /// Capability-16 query encoded by its feature controls.
    QuestLog(Vec<u8>),
    Exiva(Vec<u8>),
    Prey(oteryn_client_runtime::prey::prey::Request),
    LoyaltySkills(Vec<u8>),
    Wheel(oteryn_client_runtime::wheel_allocation::WheelCommand),
    Feature(oteryn_client_runtime::raw_feature::FeatureCommand),
    SupportDelivery(
        oteryn_client_runtime::raw_feature::FeatureCommand,
        std::path::PathBuf,
    ),
    ServerMechanic(oteryn_client_runtime::server_mechanic::ServerMechanicCommand),
    Step(StepDir),
    TargetWalk(oteryn_session::ActorPosition),
    MoveContainerContents(Vec<oteryn_client_runtime::inventory::MoveRequest>),
    MoveItem(oteryn_client_runtime::inventory::MoveRequest),
    MoveContents(Vec<oteryn_client_runtime::inventory::MoveRequest>),
    ContainerView(oteryn_client_runtime::container_tree::ContainerViewIntent),
    WorldContext(oteryn_client_runtime::world_interaction::Command),
    Combat(CombatCommand),
    /// Combat command originating from the battle panel.
    Battle(CombatCommand),
    /// Leave the game (capability 21 `LOGOUT_INTENT`).
    Logout,
    TaskBoard(oteryn_client_runtime::task_board::TaskBoardCommand),
    Stash(oteryn_client_runtime::stash::StashCommand),
    NpcTalk(oteryn_session::npc_service::NpcTalkIntent),
    NpcTrade(oteryn_session::npc_service::NpcTradeIntent),
    NpcShop(oteryn_client_runtime::npc_shop::NpcShopCommand),
    PlayerTyping(oteryn_client_runtime::player_typing::PlayerTypingCommand),
    DailyReward(oteryn_client_runtime::daily_reward::DailyRewardCommand),
    BossRewards(oteryn_client_runtime::boss_rewards::BossRewardsCommand),
    Imbue(oteryn_client_runtime::imbue::ImbueCommand),
    /// Capability-60 direct player-trade intent.
    PlayerTrade(oteryn_client_runtime::player_trade::PlayerTradeCommand),
    Vip(oteryn_client_runtime::vip::VipCommand),
    Guild(oteryn_client_runtime::guild::GuildCommand),
    Party(oteryn_client_runtime::party::PartyCommand),
    Hireling(oteryn_client_runtime::hireling::HirelingCommand),
    Market(oteryn_client_runtime::market::MarketCommand),
    /// Dismiss the oldest retained line at the local Session owner, without a wire command.
    ChatClearOldest,
    ChatPublic(oteryn_session::ChatIntent),
    MovementSpeech(oteryn_session::ChatIntent),
}
