//! Private per-mechanic command bodies. The connection owns routing order and input gates.

pub(super) mod achievements;
pub(super) mod appearance;
pub(super) mod bed;
pub(super) mod chat;
pub(super) mod combat_intent;
pub(super) mod container_view;
pub(super) mod cyclopedia;
pub(super) mod daily_reward;
pub(super) mod depot;
pub(super) mod guild;
pub(super) mod highscores;
pub(super) mod house;
pub(super) mod house_move_out;
pub(super) mod item_look;
pub(super) mod item_move;
pub(super) mod npc;
pub(super) mod npc_trade;
pub(super) mod party;
pub(super) mod quest_log;
pub(super) mod spell;
pub(super) mod staff;
pub(super) mod timed_item;
pub(super) mod use_item;
pub(super) mod vip;
pub(super) mod world_event;

use super::{
    AccountAchievementsReply, AttackIntentDisposition, ChatIntentResult, ContainerViewOutcome,
    HighscoresReply, ItemLookResult, ItemLookSelection, ItemMoveOutcome, ItemViewDelta,
    LogoutResult, NpcIntentDisposition, NpcUpdate, OpenDecision, QuestLogDomain, SpellCastOutcome,
    StepOutcome, UseOutcome,
};

pub(super) enum Dispatch {
    Vip(Option<Vec<u8>>),
    /// STEP-PACE-WIRE-1: the step and its `step_wait_ms` pacing receipt (0: none).
    Step(StepOutcome, u32),
    Use(UseOutcome),
    Spell(SpellCastOutcome),
    Achievements(AccountAchievementsReply),
    /// RANK-1 (HS-WIRE-1): command 14, a read of the session's World snapshots.
    Highscores(HighscoresReply),
    Guild(Vec<u8>),
    Party(Vec<u8>),
    HouseMoveOut(Vec<u8>),
    Timed(Vec<u8>, Vec<ItemViewDelta>),
    Appearance(Option<oteryn_protocol_oteryn::appearance::ChangeResult>),
    DailyReward(Option<Vec<u8>>),
    WorldEvent(Result<Vec<u8>, ()>),
    Training(oteryn_protocol_oteryn::loop_training::TrainingResult),
    /// ITEM-VIEW-1b: `USE` with an item target; the domain 11 delta follows the result.
    UseItem(OpenDecision, Option<ItemViewDelta>),
    /// BAGS-WIRE-1: command 21; the domain 14 delta follows the result.
    ContainerView(ContainerViewOutcome, Option<ItemViewDelta>),
    /// ITEM-MOVE-1: command 9; the domain 11 and domain 9 deltas follow the result.
    ItemMove(ItemMoveOutcome, Vec<ItemViewDelta>),
    Depot(oteryn_protocol_oteryn::depot::DepotReply),
    /// QUEST-LOG-WIRE-1: an accepted command 22; the domain 16 delta follows the result.
    QuestLog(QuestLogDomain),
    Staff(Option<Vec<u8>>),
    Look(ItemLookResult, ItemLookSelection),
    /// ATTACK-1b: commands 11 and 12; the domain 10 delta follows the result.
    Attack(AttackIntentDisposition),
    /// CHAT-WIRE-1: command 13; lines reach the hearers' domain 12 on their own ticks.
    Chat(ChatIntentResult),
    /// NPC-CMD-1a: commands 7 and 8; the domain 7 payload follows the result.
    Npc(NpcIntentDisposition, NpcUpdate, bool),
    /// NPC-TRADE-2: the result, the domain 9 deltas of a committed trade, then the domain
    /// 7 and 8 payloads.
    NpcTrade(NpcIntentDisposition, NpcUpdate, Vec<ItemViewDelta>),
    /// LOGOUT-WIRE-1: command 23; `ACCEPTED` ends the connection after the result.
    Logout(LogoutResult),
    House(Option<Vec<u8>>),
    Unregistered,
}
