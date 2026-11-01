//! Default-constructed Channel session state owned by the gameplay transport.

use super::{
    FenceHolders, PremiumSessions, ProgressionSessions, QuestSession, actor_spell,
    appearance_change, attack, chat_intent, connection::AdmittedSession, input_gate, last_position,
    level_up_live, loop_move, map_door, newhaven_travel, npc_command, tutorial_session,
};
use crate::foundation::GameSessionId;
use tokio::sync::Mutex;

/// Channel-local session state; keep fields sorted by name.
#[derive(Default)]
pub struct ChannelSessionStates {
    /// APP-3: the appearance-change command state of each session, forgotten with the session.
    pub appearance_commands: appearance_change::AppearanceCommands,
    /// The Channel owner's attack targets, fight modes and in-fight deadlines (ATTACK-1b).
    /// Always locked after `runtime` and `spell_states`, never before.
    pub attack: Mutex<attack::ChannelAttackStates>,
    /// Shared activity owner installed by the boss instance composition root.
    pub boss_activity_handoff: super::BossActivityHandoffSlot,
    /// Shared weak browse containers live only while a connection holds a view.
    pub(super) browse_fields: super::container_view::browse_cache::BrowseFields,
    /// CHAT-WIRE-1: speaker name, spam limiter and queued lines of each chatting session.
    pub chat: chat_intent::ChatRuntime,
    /// P22d: settled kills of tracked creatures awaiting a domain 98 refresh. A leaf lock.
    pub(super) cyclopedia_tracker_refresh: super::cyclopedia_tracker::TrackerRefresh,
    pub(super) daily_rewards: super::daily_reward::DailyRewardSessions,
    /// S16c: sessions whose Dawnport trial build commit awaits live installation; their last
    /// position capture is held like a retained NPC vocation choice. A leaf lock.
    pub(super) dawnport_trials: std::sync::Mutex<std::collections::HashSet<GameSessionId>>,
    /// SESSION-INGRESS-1: the ordered command stream and latest terminal result per session.
    pub command_ingress: super::session_ingress::SessionCommands,
    /// The releases holding each write fence of the slot (CHARM-DESC-FENCE-LEASE step c).
    pub fence_holders: FenceHolders,
    /// IDLEKICK: operator thresholds, shared by every connection served by this owner.
    pub gameplay_idle_policy: super::connection::GameplayIdlePolicy,
    pub(super) grizzly_boss: super::grizzly_boss::GrizzlyBossSessions,
    /// OPS-SAVE-2: closed by the shutdown flush; admissions and commands run under its permits.
    pub input_gate: std::sync::Arc<input_gate::InputGate>,
    pub(super) inspection_permissions: super::inspection::Permissions,
    /// CHAR-POSITION-1: the sessions whose last position this owner writes, and the write
    /// sequence generator (CHAR-POSITION-0 §3.2).
    pub last_positions: last_position::LastPositions,
    /// LEVELUP-LIVE-1: Characters whose kill settlement committed a level increase that the
    /// live actor has not installed yet. A leaf lock, never held across an await.
    pub(super) level_refresh: level_up_live::LevelRefresh,
    /// ITEM-LOOP-1: the Channel's loop Ground items as a map overlay, rebuilt after each drop or
    /// pickup. A leaf lock, never held across an await.
    pub loop_ground: loop_move::LoopGroundCache,
    /// Ended admitted sessions whose loss is durably recorded and whose grace has not ended:
    /// the only sessions a `ClientResume` can name. At most one entry per admitted session.
    pub lost: std::sync::Mutex<std::collections::HashMap<GameSessionId, AdmittedSession>>,
    pub(super) newhaven_travel: newhaven_travel::NewhavenTravel,
    /// NPC-CMD-1a: the NPC conversation and talk limit of each session with capability 3.
    pub npc_conversations: npc_command::NpcConversations,
    pub(super) player_trades: super::player_trade::TradeBook,
    /// PROF-WIRE-1: durable tracks installed for each admitted session. The leaf lock is never
    /// held across a database await.
    pub(super) proficiencies: std::sync::Mutex<
        std::collections::HashMap<
            GameSessionId,
            super::weapon_proficiency::WeaponProficiencyDispatch,
        >,
    >,
    /// PREM-1b (D476): the sessions holding each account's Premium registration, so a late
    /// release of one session never cancels another's pulls.
    pub premium_sessions: std::sync::Mutex<PremiumSessions>,
    /// The progression binding of each bound admitted session (ARCH-PROGRESSION-SOURCE-0 §1.4),
    /// inserted at fresh admission and removed in `retire`, so a `ClientResume` of a lost
    /// session finds the same binding, or when the durable death writer refuses its context
    /// (`settle_session_death`), which leaves the session unbound. An unbound session has no
    /// entry. Never held across an await.
    pub progression_sessions: ProgressionSessions,
    /// The quest copy of each admitted session (QUEST-STATE-0 §7), loaded at fresh admission
    /// and resume. Never held across an await.
    pub quest_sessions: std::sync::Mutex<std::collections::HashMap<GameSessionId, QuestSession>>,
    pub(super) quest_callbacks: super::quest_callbacks::PendingCallbacks,
    pub(super) resting_areas: super::resting_area::RestingAreaSessions,
    /// PARITY-EVENT-CALLBACKS-1: the currently due server-save countdown minute.
    pub(super) server_save: super::server_save_countdown::ServerSaveCountdownHub,
    /// VITALS-SOUL-1: the soul hook of each settled kill, applied on the killer's cadence.
    pub(super) soul_kills: super::soul_kill_live::SoulKills,
    /// The Channel owner's player vitals and cooldowns (spell cast §4). Always locked after
    /// `runtime`, never before, like `door`.
    pub spell_states: Mutex<actor_spell::ChannelSpellStates>,
    pub(super) timed_items: super::timed_item::TimedRuntime,
    /// PAR-P12a: Characters whose level, tracked kill or boss kill may have changed a title
    /// predicate; refreshed on the actor's cadence like `level_refresh`. A leaf lock.
    pub(super) title_refresh: level_up_live::LevelRefresh,
    pub(super) training: super::loop_training::TrainingRuntime,
    pub(super) tutorial_sessions: tutorial_session::TutorialSessions,
    /// Committed Atelier gems and grades, with the current Wheel vessel overlay.
    pub atelier_sessions: std::sync::Mutex<
        std::collections::HashMap<
            GameSessionId,
            Option<oteryn_gameplay_progression::atelier::AtelierState>,
        >,
    >,
    /// The Wheel allocation of each admitted session (WHEEL-0 §4 "Load"), after the admission
    /// Wheel reset; `None` when the load failed. The allocation only, never an eligibility
    /// result. Never held across an await.
    pub wheel_sessions: std::sync::Mutex<
        std::collections::HashMap<
            GameSessionId,
            Option<crate::durability::character_wheel::WheelAllocation>,
        >,
    >,
    /// The world-interaction admission gate (`WORLD_INTERACTION0` §3.4): a leaf lock, never held
    /// across another.
    pub world_gate: std::sync::Mutex<map_door::WorldActionGate>,
}

impl ChannelSessionStates {
    /// Forget exactly the state retired after the runtime removes a terminal session.
    pub(super) fn forget_session(&self, session: GameSessionId) {
        self.fence_holders.forget(session);
        self.appearance_commands.forget(session);
        self.chat.forget(session);
        self.command_ingress.forget(session);
        self.player_trades.forget(session);
        self.inspection_permissions.forget(session);
        self.npc_conversations.forget(session);
        self.grizzly_boss.forget(session);
        if let Ok(mut sessions) = self.quest_sessions.lock() {
            sessions.remove(&session);
        }
        if let Ok(mut sessions) = self.atelier_sessions.lock() {
            sessions.remove(&session);
        }
        if let Ok(mut sessions) = self.wheel_sessions.lock() {
            sessions.remove(&session);
        }
        if let Ok(mut sessions) = self.progression_sessions.lock() {
            sessions.remove(&session);
        }
        if let Ok(mut sessions) = self.proficiencies.lock() {
            sessions.remove(&session);
        }
        self.last_positions.forget(session);
        self.tutorial_sessions.forget(session);
        self.newhaven_travel.forget(session);
    }
}
