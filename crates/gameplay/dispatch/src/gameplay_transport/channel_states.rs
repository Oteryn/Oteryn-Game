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
    pub(super) admission_queue: std::sync::Mutex<super::admission_queue::AdmissionWaitQueue>,
    pub(super) analyser: std::sync::Arc<super::analyser::AnalyserStreams>,
    /// APP-3: the appearance-change command state of each session, forgotten with the session.
    pub appearance_commands: appearance_change::AppearanceCommands,
    pub(super) arena_matches: super::arena_match::Matches,
    /// The Channel owner's attack targets, fight modes and in-fight deadlines (ATTACK-1b).
    /// Always locked after `runtime` and `spell_states`, never before.
    pub attack: Mutex<attack::ChannelAttackStates>,
    /// Shared activity owner installed by the boss instance composition root.
    pub boss_activity_handoff: super::BossActivityHandoffSlot,
    /// Shared weak browse containers live only while a connection holds a view.
    pub(super) browse_fields: super::container_view::browse_cache::BrowseFields,
    /// Optional local informational publication; never an admission authority.
    pub(super) bulletin_source: Option<std::sync::Arc<super::bulletin::BulletinSource>>,
    /// Immutable Cake source geometry derived from the active bundle, never custody.
    pub(super) cake_sources: super::cake_event::SourceCache,
    pub casts: std::sync::OnceLock<super::cast::CastNode>,
    /// CHAT-WIRE-1: speaker name, spam limiter and queued lines of each chatting session.
    pub chat: chat_intent::ChatRuntime,
    /// P22d: settled kills of tracked creatures awaiting a domain 98 refresh. A leaf lock.
    pub(super) cyclopedia_tracker_refresh: super::cyclopedia_tracker::TrackerRefresh,
    pub(super) daily_rewards: super::daily_reward::DailyRewardSessions,
    pub(super) dawnport_departures: super::dawnport_departure::Departures,
    /// S16c: sessions whose Dawnport trial build commit awaits live installation; their last
    /// position capture is held like a retained NPC vocation choice. A leaf lock.
    pub(super) dawnport_trials: std::sync::Mutex<std::collections::HashSet<GameSessionId>>,
    /// SESSION-INGRESS-1: the ordered command stream and latest terminal result per session.
    pub command_ingress: super::session_ingress::SessionCommands,
    /// The releases holding each write fence of the slot (CHARM-DESC-FENCE-LEASE step c).
    pub fence_holders: FenceHolders,
    /// IDLEKICK: operator thresholds, shared by every connection served by this owner.
    pub gameplay_idle_policy: super::connection::GameplayIdlePolicy,
    /// RAIDS-GLOBAL: copies of the idle owner's facts, removed with the session.
    pub(super) gameplay_idle_presence: super::gameplay_idle_presence::GameplayIdlePresence,
    pub(super) grizzly_boss: super::grizzly_boss::GrizzlyBossSessions,
    /// OPS-SAVE-2: closed by the shutdown flush; admissions and commands run under its permits.
    pub(super) kill_source_owner: super::public_kill_statistics::ActivityKillSourceOwner,
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
    /// Wakes a resume that observed the durable loss before `lost` publication finished.
    pub(super) lost_changed: tokio::sync::Notify,
    /// Set by the production listener from its explicit fresh-entry opt-in.
    /// Internal composed owners have no node rollout configuration.
    pub(super) native_pre_admission_disabled: bool,
    pub(super) newhaven_travel: newhaven_travel::NewhavenTravel,
    /// NPC-CMD-1a: the NPC conversation and talk limit of each session with capability 3.
    pub npc_conversations: npc_command::NpcConversations,
    pub(super) player_trades: super::player_trade::TradeBook,
    pub(super) primal_menace: super::primal_menace::PrimalMenace,
    pub(super) primal_pods: super::primal_pods::State,
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
    /// RAIDS-GLOBAL: the Channel and its Activity scopes share current recipients.
    pub(super) server_messages: std::sync::Arc<super::raid_notices::ServerMessageHub>,
    pub(super) server_modals: std::sync::Arc<super::server_modal::ModalHub>,
    /// PARITY-EVENT-CALLBACKS-1: the currently due server-save countdown minute.
    pub(super) server_save: super::server_save_countdown::ServerSaveCountdownHub,
    /// VITALS-SOUL-1: the soul hook of each settled kill, applied on the killer's cadence.
    pub(super) soul_kills: super::soul_kill_live::SoulKills,
    pub(super) source_volatile_items: super::source_volatile_items::SourceVolatileItems,
    /// Single-use Soulseal entry quotes, bound to the exact session and content pin.
    pub(super) soulseal_quotes: super::task_board::soulseals::Quotes,
    /// The Channel owner's player vitals and cooldowns (spell cast §4). Always locked after
    /// `runtime`, never before, like `door`.
    pub spell_states: Mutex<actor_spell::ChannelSpellStates>,
    /// NX-1: current-session staff flags and uncertain world-effect receipts.
    pub(super) staff_world: super::staff_tools::WorldTools,
    /// Per-owner simulation clock control for native timer scenarios, never SQL time.
    #[cfg(test)]
    pub(super) test_owner_time: std::sync::atomic::AtomicU64,
    pub(super) tibiadrome: super::tibiadrome::Drome,
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
    pub(super) fn insert_lost(&self, session: AdmittedSession) -> Result<(), ()> {
        self.lost
            .lock()
            .map_err(|_| ())?
            .insert(session.game_session_id, session);
        self.lost_changed.notify_waiters();
        Ok(())
    }

    pub(super) fn remove_lost_if(
        &self,
        session: GameSessionId,
        predicate: impl FnOnce(&AdmittedSession) -> bool,
    ) {
        let removed = self
            .lost
            .lock()
            .ok()
            .and_then(|mut lost| predicate(lost.get(&session)?).then(|| lost.remove(&session)))
            .flatten()
            .is_some();
        if removed {
            self.lost_changed.notify_waiters();
        }
    }

    pub(super) fn transfer_lost_to(
        &self,
        destination: &Self,
        old: GameSessionId,
        next: AdmittedSession,
    ) -> Result<(), ()> {
        let mut from = self.lost.lock().map_err(|_| ())?;
        let mut to = destination.lost.lock().map_err(|_| ())?;
        to.try_reserve(1).map_err(|_| ())?;
        from.remove(&old);
        to.insert(next.game_session_id, next);
        drop(to);
        drop(from);
        self.lost_changed.notify_waiters();
        destination.lost_changed.notify_waiters();
        Ok(())
    }

    /// Activity owners retain independent gameplay state, the origin Channel's
    /// connection recipients, and its existing node shutdown/input gate.
    pub(super) fn for_activity(&self) -> Self {
        Self {
            casts: self.casts.clone(),
            analyser: std::sync::Arc::clone(&self.analyser),
            gameplay_idle_policy: self.gameplay_idle_policy,
            input_gate: std::sync::Arc::clone(&self.input_gate),
            server_messages: std::sync::Arc::clone(&self.server_messages),
            server_modals: std::sync::Arc::clone(&self.server_modals),
            ..Default::default()
        }
    }

    /// Forget exactly the state retired after the runtime removes a terminal session.
    pub(super) fn forget_session(&self, session: GameSessionId) {
        self.staff_world.forget(session);
        self.dawnport_departures.forget(session);
        self.analyser.forget(session);
        self.fence_holders.forget(session);
        self.appearance_commands.forget(session);
        self.chat.forget(session);
        self.command_ingress.forget(session);
        self.gameplay_idle_presence.forget(session);
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
        self.server_messages.forget(session);
        self.newhaven_travel.forget(session);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, WorldId};

    #[tokio::test]
    async fn transferring_lost_session_wakes_both_registered_waiters() {
        fn uuid_v7(tag: u8) -> [u8; 16] {
            let mut bytes = [tag; 16];
            bytes[6] = 0x70 | (tag & 0x0f);
            bytes[8] = 0x80 | (tag & 0x3f);
            bytes
        }
        let source = ChannelSessionStates::default();
        let destination = ChannelSessionStates::default();
        let world = WorldId::decode(&uuid_v7(1)).expect("world");
        let channel = ChannelId::decode(&uuid_v7(2)).expect("channel");
        let old = GameSessionId::decode(&uuid_v7(3)).expect("old session");
        let session = |game_session_id| AdmittedSession {
            game_session_id,
            world_id: world,
            channel_id: channel,
            runtime_actor: None,
            first_entry: super::super::connection::FirstEntryOutcome::Positioned,
            controller: None,
            continuity: super::super::connection::SessionContinuity::FRESH,
            item_fence: None,
        };
        source
            .insert_lost(session(old))
            .expect("insert lost session");
        let mut source_notified = std::pin::pin!(source.lost_changed.notified());
        source_notified.as_mut().enable();
        let mut destination_notified = std::pin::pin!(destination.lost_changed.notified());
        destination_notified.as_mut().enable();
        destination
            .lost
            .lock()
            .expect("destination lost map")
            .reserve(1);
        source
            .transfer_lost_to(
                &destination,
                old,
                session(GameSessionId::decode(&uuid_v7(4)).expect("new session")),
            )
            .expect("transfer lost session");
        for notified in [source_notified, destination_notified] {
            tokio::time::timeout(std::time::Duration::from_millis(50), notified)
                .await
                .expect("lost transfer notification");
        }
    }
}
