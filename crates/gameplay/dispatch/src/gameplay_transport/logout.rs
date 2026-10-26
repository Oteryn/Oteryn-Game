//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6): the `LOGOUT_INTENT` handler, its terminal
//! release, and the reconciliation of a logout whose commit was not acknowledged.

use oteryn_protocol_oteryn::logout::{LogoutResult, MAX_LOGOUT_RETRY_AFTER_MS};

use super::connection::{AdmittedSession, ControllerBinding, GraceExpiryResult};
use super::{ComposedFreshAdmission, FenceHold, ReleaseAttempt, TerminalRelease, UnendedSettle};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState};
/// What the connection does after one logout command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutReply {
    /// NOLOGOUT refuses without releasing the session; capability 92 carries its text.
    NoLogout,
    /// Write this result. `Accepted` only once the terminal release has committed and the actor
    /// has left the Channel; the connection then stops reading and closes.
    Result(LogoutResult),
    /// The combat state or release outcome is unknown: no result is written and the transport
    /// closes. The control-loss lifecycle reconciles from the durable row, keeping any release
    /// fence until the outcome is known (`ControlLossResult::Terminal`).
    Unknown,
}
/// `retry_after_ms` of an in-fight refusal: the time left until `until`, rounded up to whole
/// milliseconds and within `1..=ATTACK0-RL-03`.
pub(super) fn in_fight_retry_after_ms(until_micros: u64, now_micros: u64) -> u32 {
    let left = until_micros.saturating_sub(now_micros).div_ceil(1_000);
    u32::try_from(left)
        .unwrap_or(u32::MAX)
        .clamp(1, MAX_LOGOUT_RETRY_AFTER_MS)
}
/// The `retry_after_ms` of a logout refused for a fight, or `None`. An actor whose cell is a
/// qualified protection zone is not refused, PvP block or not, as Canary's
/// `ProtocolGame::logout` (`protocolgame.cpp:1158`) checks `CONDITION_INFIGHT` (which carries
/// the pz lock) only outside `TILESTATE_PROTECTIONZONE`. Elsewhere, and on an unknown cell, a
/// live PvP block refuses, then the PvE in-fight deadline.
pub(super) fn in_fight_logout_retry(
    pvp_blocked: bool,
    pve_in_fight_until_micros: Option<u64>,
    now_micros: u64,
    in_protection_zone: Option<bool>,
) -> Option<u32> {
    if in_protection_zone == Some(true) {
        return None;
    }
    if pvp_blocked {
        return Some(1000);
    }
    pve_in_fight_until_micros.map(|until| in_fight_retry_after_ms(until, now_micros))
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// §1.6: one `LOGOUT_INTENT` of the admitted session. A pending durable spell commit is
    /// `BUSY`, a running in-fight deadline is `IN_FIGHT`; otherwise the session ends at once in a
    /// `TerminalRelease::Logout`, with no grace period. Neither refusal changes the session or
    /// the actor. The in-fight read and the fence are separate steps: a hit landing between
    /// them is settled by the release's own saves, as for a closed client whose deadline just
    /// ended.
    pub(super) async fn logout_session(&self, admitted: AdmittedSession) -> LogoutReply {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return LogoutReply::Result(LogoutResult::Busy);
        };
        let session = admitted.game_session_id;
        match self.no_logout_zone(actor).await {
            Some(true) => return LogoutReply::NoLogout,
            None => return LogoutReply::Result(LogoutResult::Busy),
            Some(false) => {}
        }
        if self.states.newhaven_travel.held(session) {
            return LogoutReply::Result(LogoutResult::Busy);
        }
        if !self
            .reconcile_spells_before_control_loss(actor, session)
            .await
        {
            return LogoutReply::Result(LogoutResult::Busy);
        }
        if !self.refresh_pvp_locks(actor, session).await {
            // An unreadable fence or PvP row proves no fight deadline. Close through the
            // ordinary unknown/control-loss path without authorizing a terminal release.
            return LogoutReply::Unknown;
        }
        if let Some(retry_after_ms) = self.logout_in_fight(actor, session).await {
            return LogoutReply::Result(LogoutResult::InFight { retry_after_ms });
        }
        self.release_logout(admitted, actor, controller).await
    }

    async fn no_logout_zone(&self, actor: ExactActorRef) -> Option<bool> {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        // This format has no OTBM zones; retain its existing logout behavior, including
        // engineering runtimes whose synthetic content pin does not bind a map projection.
        if matches!(
            self.movement_cells.index(),
            NativeMovementCollisionIndex::Entry(_)
        ) {
            return Some(false);
        }
        let mut runtime = self.runtime.lock().await;
        let scope = self.movement_cells.scope();
        if scope.world_id != runtime.binding().world_id()
            || scope.world_id != runtime.content_pin().world_id()
            || scope.generation_digest != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        let position = runtime.borrow_movement_position().read(actor).ok()?;
        if position.context() != runtime.pinned_movement_context() {
            return None;
        }
        let position = position.position();
        let cell = crate::content::LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        };
        match self.movement_cells.index() {
            NativeMovementCollisionIndex::Bundle(index) => {
                use crate::map::boot::BundleCollisionIndexExt;
                // Check the bundle scope even on a tile whose mutable stack is unknown to spells.
                index.lookup(scope, cell).ok()?;
                let pos = crate::content::bundle_spell_tiles::tile_pos(cell)?;
                let tile = index.map().overlay.base().tile(pos.x, pos.y, pos.floor)?;
                Some(tile.no_logout())
            }
            NativeMovementCollisionIndex::Source(world) => Some(
                world
                    .immutable_tile(scope, cell)
                    .ok()?
                    .source_step()?
                    .no_logout,
            ),
            // The engineering fixture format has no OTBM zones.
            NativeMovementCollisionIndex::Entry(_) => Some(false),
        }
    }
    /// The `retry_after_ms` of a running in-fight deadline (ATTACK-0 §4), or `None`. A dead
    /// actor is not held, as at a closed client's release (`held_step`). The PvE deadline
    /// and the PvP block do not hold an actor standing in a protection zone
    /// (`in_fight_logout_retry`).
    async fn logout_in_fight(&self, actor: ExactActorRef, session: GameSessionId) -> Option<u32> {
        let in_protection_zone = super::actor_spell::qualified_protection_zone(
            &mut *self.runtime.lock().await,
            self.movement_cells,
            actor,
        );
        let fact = self.combat_lock_fact(actor, session).await;
        in_fight_logout_retry(
            fact.pvp == super::combat_lock_fact::PvpLockSource::LiveBlocked,
            fact.pve_in_fight_until.map(|until| until.get()),
            fact.sampled_at().get(),
            in_protection_zone,
        )
    }
    /// One attempt of the release on the `Abandoned` path, with no retry: the client retries a
    /// `BUSY`. A failure before the commit reads the durable row and settles the fence
    /// (`settle_unended`) before `BUSY`, so no character write stays fenced.
    async fn release_logout(
        &self,
        admitted: AdmittedSession,
        actor: ExactActorRef,
        controller: ControllerBinding,
    ) -> LogoutReply {
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let release = TerminalRelease::Logout(controller.transport);
        // Nothing is fenced yet when no transition id can be minted.
        let Ok(token) = release.fence(&mut *self.runtime.lock().await) else {
            return LogoutReply::Result(LogoutResult::Busy);
        };
        let mut hold = FenceHold::new(session, token);
        let mut settled_death = None;
        let mut kills = None;
        let step = self
            .release_attempt(
                &admitted,
                &store,
                release,
                &mut hold,
                &mut settled_death,
                &mut kills,
            )
            .await;
        match step {
            ReleaseAttempt::Done(GraceExpiryResult::Released) => {
                LogoutReply::Result(LogoutResult::Accepted)
            }
            // The session is not this logout's to end, and its fence is lifted.
            ReleaseAttempt::Done(GraceExpiryResult::NotApplicable) => {
                LogoutReply::Result(LogoutResult::Busy)
            }
            // A refused fence, or a committed release whose actor could not be removed: the
            // lifecycle reconciles from the row.
            ReleaseAttempt::Done(GraceExpiryResult::Unknown) => LogoutReply::Unknown,
            ReleaseAttempt::Retry => match self.settle_unended(&store, actor, &mut hold).await {
                UnendedSettle::Lifted => LogoutReply::Result(LogoutResult::Busy),
                UnendedSettle::Terminal => {
                    match self
                        .retire_reconciled(controller.account_id, session, actor)
                        .await
                    {
                        GraceExpiryResult::Released => LogoutReply::Result(LogoutResult::Accepted),
                        _ => LogoutReply::Unknown,
                    }
                }
                UnendedSettle::Unknown => {
                    self.states.fence_holders.keep(hold);
                    LogoutReply::Unknown
                }
            },
            ReleaseAttempt::Unknown => {
                self.states.fence_holders.keep(hold);
                LogoutReply::Unknown
            }
        }
    }
    /// §1.6: a session whose current row is TERMINAL when its transport loss is committed ended
    /// in a release whose acknowledgement was lost. Its actor is retired: Premium release, the
    /// exact actor slot removed, its fences forgotten. An unreadable row is `Unknown`, and the
    /// lifecycle retries behind its backoff with the fence kept.
    pub(super) async fn reconcile_terminal_session(
        &self,
        admitted: AdmittedSession,
    ) -> GraceExpiryResult {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return GraceExpiryResult::NotApplicable;
        };
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok(current) = store.current_session(session).await else {
            return GraceExpiryResult::Unknown;
        };
        if current.session_state() != GameSessionState::Terminal {
            return GraceExpiryResult::NotApplicable;
        }
        // KILL-REWARD-COMP-1 §1.3: the ended session's marks and parked kills go, as at a seal
        // of an already terminal session.
        self.states
            .attack
            .lock()
            .await
            .kills()
            .discard_gone(session, None);
        // Another release of the session already removed the slot.
        let gone = self
            .runtime
            .lock()
            .await
            .player_control_facts(actor, session)
            .is_err();
        if gone {
            self.release_premium(controller.account_id, session);
            // This already-gone path only forgot fences; keep broader retirement cleanup separate.
            self.states.fence_holders.forget(session);
            return GraceExpiryResult::Released;
        }
        self.retire_reconciled(controller.account_id, session, actor)
            .await
    }
    /// §1.6: the transport of an ACTIVE session was lost after an unknown logout. The fence that
    /// logout kept is lifted under the runtime lock and its kill marks are released, so the
    /// recorded loss and its grace expiry fence and settle as for any loss.
    pub(super) async fn lift_kept_logout(&self, session: GameSessionId, actor: ExactActorRef) {
        let holds = self.states.fence_holders.take_transitions(session);
        if holds.is_empty() {
            return;
        }
        {
            let mut runtime = self.runtime.lock().await;
            for mut hold in holds {
                if self
                    .states
                    .fence_holders
                    .lift(&mut runtime, actor, &mut hold)
                    .is_err()
                {
                    self.states.fence_holders.release(&mut hold);
                }
            }
        }
        self.release_orphaned_kills(session).await;
    }
}
