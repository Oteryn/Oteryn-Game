//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6): the `LOGOUT_INTENT` handler, its terminal
//! release, and the reconciliation of a logout whose commit was not acknowledged.

use oteryn_protocol_oteryn::logout::{LogoutResult, MAX_LOGOUT_RETRY_AFTER_MS};

use super::connection::{AdmittedSession, ControllerBinding, GraceExpiryResult};
use super::{
    AdmissionRefusal, AdmissionRetryBudget, ComposedFreshAdmission, FenceHold, ReleaseAttempt,
    TerminalRelease, UnendedSettle, operator_event,
};
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

/// A release checkpoint that could not settle before the lease transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReleaseRetryStage {
    Drome,
    Death,
    TimedItems,
    ItemEffects,
    Fence,
    WheelGift,
    Monk,
    SpellTraining,
    Gourmet,
    Conditions,
    ConditionsHolder,
    KillRewards,
    Familiar,
    OfflineTraining,
}

impl ReleaseRetryStage {
    fn label(self) -> &'static str {
        match self {
            Self::Drome => "drome_sync",
            Self::Death => "death_save",
            Self::TimedItems => "timed_items",
            Self::ItemEffects => "item_effects",
            Self::Fence => "fence_wait",
            Self::WheelGift => "wheel_gift",
            Self::Monk => "monk_save",
            Self::SpellTraining => "training_save",
            Self::Gourmet => "gourmet_checkpoint",
            Self::Conditions | Self::ConditionsHolder => "condition_save",
            Self::KillRewards => "kill_rewards",
            Self::Familiar => "familiar_save",
            Self::OfflineTraining => "offline_training",
        }
    }
}

/// Retained across release attempts so a checkpoint retry uses the same fence and budget.
#[derive(Default)]
pub(super) struct ReleaseProgress<'r> {
    pub(super) settled_death: Option<super::actor_spell::PlayerDeath>,
    pub(super) kills: Option<super::KillReleaseMark>,
    retry: Option<LogoutReleaseRetry<'r>>,
}

struct LogoutReleaseRetry<'r> {
    root: &'r crate::durability::DurabilityRoot,
    budget: AdmissionRetryBudget<'r>,
    command_id: Option<u64>,
    started: tokio::time::Instant,
    attempts: u32,
}

impl<'r> ReleaseProgress<'r> {
    fn for_logout(root: &'r crate::durability::DurabilityRoot, command_id: Option<u64>) -> Self {
        let started = tokio::time::Instant::now();
        Self {
            retry: Some(LogoutReleaseRetry {
                root,
                budget: AdmissionRetryBudget::for_holder_window(
                    root,
                    AdmissionRetryBudget::holder_wait_window(),
                ),
                command_id,
                started,
                attempts: 0,
            }),
            ..Self::default()
        }
    }

    pub(super) async fn retry(&mut self, stage: ReleaseRetryStage) -> bool {
        let Some(retry) = &mut self.retry else {
            return false;
        };
        #[cfg(test)]
        let _ = LOGOUT_RELEASE_PROBE.try_with(|probe| {
            probe
                .retry_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if probe.first_retry.set(stage).is_ok() {
                probe.retry_observed.notify_one();
            }
        });
        retry.attempts += 1;
        let (holder, held_ms) = retry.root.checked_out_semantic_pass().map_or_else(
            || ("none".to_owned(), 0),
            |(caller, held)| (caller.to_string(), held.as_millis()),
        );
        operator_event(&format!(
            "event=logout_release_retry phase={} command_id={} attempt={} elapsed_ms={} holder={holder} held_ms={held_ms}",
            stage.label(),
            retry.command_id.unwrap_or_default(),
            retry.attempts,
            retry.started.elapsed().as_millis(),
        ));
        // In-flight checkpoints settle normally. A new attempt must start in the
        // original window, even when an earlier phase already used part of it.
        retry
            .budget
            .retries_unavailable(AdmissionRefusal::Unavailable)
            .await
            && retry.budget.check_deadline().is_ok()
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct LogoutReleaseProbe {
    pub(super) before_conditions: tokio::sync::Notify,
    pub(super) resume_conditions: tokio::sync::Notify,
    pub(super) retry_observed: tokio::sync::Notify,
    pub(super) retry_exhausted: tokio::sync::Notify,
    pub(super) logout_finished: tokio::sync::Notify,
    pub(super) first_retry: std::sync::OnceLock<ReleaseRetryStage>,
    pub(super) retry_count: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
tokio::task_local! {
    pub(super) static LOGOUT_RELEASE_PROBE: std::sync::Arc<LogoutReleaseProbe>;
}

pub(super) struct LogoutZones {
    pub(super) no_logout: bool,
    pub(super) protection_zone: Option<bool>,
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
    /// Retained NOLOGOUT bit 3 at the current runtime position. Entry-room fixtures have
    /// no authored NOLOGOUT tiles; production bundle facts are read without spell gating.
    pub(super) fn no_logout_tile(
        &self,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<bool> {
        if let Some((_, house)) = runtime.house_observer(actor).ok()? {
            return self
                .logout_zones_for_cell(
                    runtime,
                    crate::content::LogicalCell {
                        x: house.position.x,
                        y: house.position.y,
                        z: i32::from(house.position.floor),
                    },
                )
                .map(|zones| zones.no_logout);
        }
        match self.movement_cells().index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_) => {
                let at = super::bundle_map_position(runtime, self.movement_cells(), actor)?;
                let flags = at.map.zone_flags(crate::map::overlay::TilePos {
                    x: u16::try_from(at.at.x).ok()?,
                    y: u16::try_from(at.at.y).ok()?,
                    floor: i8::try_from(at.at.floor).ok()?,
                })?;
                Some(flags.0)
            }
            _ => Some(false),
        }
    }

    /// §1.6: one `LOGOUT_INTENT` of the admitted session. A pending durable spell commit is
    /// `BUSY`, a running in-fight deadline is `IN_FIGHT`; otherwise the session ends at once in a
    /// `TerminalRelease::Logout`, with no grace period. Neither refusal changes the session or
    /// the actor. The in-fight read and the fence are separate steps: a hit landing between
    /// them is settled by the release's own saves, as for a closed client whose deadline just
    /// ended.
    pub(super) async fn logout_session(&self, admitted: AdmittedSession) -> LogoutReply {
        self.logout_session_for_command(admitted, None).await
    }

    pub(super) async fn logout_session_for_command(
        &self,
        admitted: AdmittedSession,
        command_id: Option<u64>,
    ) -> LogoutReply {
        self.logout_session_inner(admitted, false, command_id)
            .await
            .unwrap_or(LogoutReply::Result(LogoutResult::Busy))
    }

    /// `channel_switch`: the release is the safe exit of a voluntary Channel switch (chsw) and
    /// keeps the Party membership for the fresh admission into the target Channel.
    pub(super) async fn logout_session_as(
        &self,
        admitted: AdmittedSession,
        channel_switch: bool,
    ) -> Result<LogoutReply, oteryn_protocol_oteryn::channel_switch::SwitchLock> {
        self.logout_session_inner(admitted, channel_switch, None)
            .await
    }

    async fn logout_session_inner(
        &self,
        admitted: AdmittedSession,
        channel_switch: bool,
        command_id: Option<u64>,
    ) -> Result<LogoutReply, oteryn_protocol_oteryn::channel_switch::SwitchLock> {
        let _trade_hold = channel_switch
            .then(|| {
                self.states
                    .player_trades
                    .hold_for_switch(admitted.game_session_id)
            })
            .transpose()?;
        let Ok(_foreground) = self.root.foreground_demand() else {
            operator_event("event=logout_busy stage=foreground_demand");
            return Ok(LogoutReply::Result(LogoutResult::Busy));
        };
        let (actor, controller) = match self.logout_preflight(admitted).await {
            Ok(proven) => proven,
            Err(reply) => return Ok(reply),
        };
        // Every activity participant must return through its durable owner
        // before the ordinary terminal release. No caller may orphan a room.
        if matches!(
            actor.runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) {
            operator_event("event=logout_busy stage=activity_scope");
            return Ok(LogoutReply::Result(LogoutResult::Busy));
        }
        self.release_logout(admitted, actor, controller, channel_switch, command_id)
            .await
    }

    pub(super) async fn logout_preflight(
        &self,
        admitted: AdmittedSession,
    ) -> Result<(ExactActorRef, ControllerBinding), LogoutReply> {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            operator_event("event=logout_busy stage=admitted_binding");
            return Err(LogoutReply::Result(LogoutResult::Busy));
        };
        let session = admitted.game_session_id;
        let Some(zones) = self.logout_zones(actor).await else {
            operator_event("event=logout_busy stage=zones_before_reconcile");
            return Err(LogoutReply::Result(LogoutResult::Busy));
        };
        if zones.no_logout {
            return Err(LogoutReply::NoLogout);
        }
        if self.states.newhaven_travel.held(session) {
            operator_event("event=logout_busy stage=newhaven_travel");
            return Err(LogoutReply::Result(LogoutResult::Busy));
        }
        if !self
            .reconcile_spells_before_control_loss(actor, session)
            .await
        {
            operator_event("event=logout_busy stage=spell_reconcile");
            return Err(LogoutReply::Result(LogoutResult::Busy));
        }
        if !self.refresh_pvp_locks(actor, session).await {
            // An unreadable fence or PvP row proves no fight deadline. Close through the
            // ordinary unknown/control-loss path without authorizing a terminal release.
            operator_event("event=logout_unknown stage=pvp_refresh kind=unknown");
            return Err(LogoutReply::Unknown);
        }
        // Reconciliation can finish a retained spell relocation. Use its current cell
        // for both zone checks instead of carrying a pre-reconciliation PZ exception.
        let Some(zones) = self.logout_zones(actor).await else {
            operator_event("event=logout_busy stage=zones_after_reconcile");
            return Err(LogoutReply::Result(LogoutResult::Busy));
        };
        if zones.no_logout {
            return Err(LogoutReply::NoLogout);
        }
        if let Some(retry_after_ms) = self
            .logout_in_fight(actor, session, zones.protection_zone)
            .await
        {
            return Err(LogoutReply::Result(LogoutResult::InFight {
                retry_after_ms,
            }));
        }
        Ok((actor, controller))
    }

    pub(super) async fn logout_zones(&self, actor: ExactActorRef) -> Option<LogoutZones> {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        let mut runtime = self.runtime.lock().await;
        // This format has no OTBM zones; retain its existing logout behavior, including
        // engineering runtimes whose synthetic content pin does not bind a map projection.
        if matches!(
            self.movement_cells().index(),
            NativeMovementCollisionIndex::Entry(_)
        ) {
            return Some(LogoutZones {
                no_logout: false,
                protection_zone: super::actor_spell::qualified_protection_zone(
                    &mut runtime,
                    self.movement_cells(),
                    actor,
                ),
            });
        }
        let position = if let Some((_, house)) = runtime.house_observer(actor).ok()? {
            crate::foundation::MovementLocalPosition {
                x: house.position.x,
                y: house.position.y,
                floor: house.position.floor,
            }
        } else {
            let position = runtime.borrow_movement_position().read(actor).ok()?;
            if position.context() != runtime.pinned_movement_context() {
                return None;
            }
            position.position()
        };
        let cell = crate::content::LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        };
        self.logout_zones_for_cell(&runtime, cell)
    }

    pub(super) fn logout_zones_for_cell(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        cell: crate::content::LogicalCell,
    ) -> Option<LogoutZones> {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        let scope = self.movement_cells().scope();
        if scope.world_id != runtime.binding().world_id()
            || scope.world_id != runtime.content_pin().world_id()
        {
            return None;
        }
        match self.movement_cells().index() {
            NativeMovementCollisionIndex::Bundle(index) => {
                use crate::map::boot::BundleCollisionIndexExt;
                use sha2::{Digest, Sha256};
                // Bundle cells inherit the entry scope's gameplay generation. The zone
                // evidence belongs to the independently pinned map, not that artifact.
                let map_revision: [u8; 32] =
                    Sha256::digest(scope.map_revision.as_str().as_bytes()).into();
                if map_revision != runtime.content_pin().map_revision_digest() {
                    return None;
                }
                // Check the bundle scope even on a tile whose mutable stack is unknown to spells.
                index.lookup(scope, cell).ok()?;
                let pos = crate::content::bundle_spell_tiles::tile_pos(cell)?;
                let (no_logout, protection_zone) = index.map().zone_flags(pos)?;
                Some(LogoutZones {
                    no_logout,
                    protection_zone: Some(protection_zone),
                })
            }
            NativeMovementCollisionIndex::Source(world) => {
                if scope.generation_digest != runtime.content_pin().server_artifact_digest() {
                    return None;
                }
                let tile = world.immutable_tile(scope, cell).ok()?;
                Some(LogoutZones {
                    no_logout: tile.source_step()?.no_logout,
                    protection_zone: Some(tile.flags().protection_zone),
                })
            }
            // The engineering fixture format has no OTBM zones.
            NativeMovementCollisionIndex::Entry(_) => Some(LogoutZones {
                no_logout: false,
                protection_zone: None,
            }),
        }
    }
    /// The `retry_after_ms` of a running in-fight deadline (ATTACK-0 §4), or `None`. A dead
    /// actor is not held, as at a closed client's release (`held_step`). The PvE deadline
    /// and the PvP block do not hold an actor standing in a protection zone
    /// (`in_fight_logout_retry`).
    pub(super) async fn logout_in_fight(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        in_protection_zone: Option<bool>,
    ) -> Option<u32> {
        let fact = self.combat_lock_fact(actor, session).await;
        in_fight_logout_retry(
            fact.pvp == super::combat_lock_fact::PvpLockSource::LiveBlocked,
            fact.pve_in_fight_until.map(|until| until.get()),
            fact.sampled_at().get(),
            in_protection_zone,
        )
    }
    /// Retry a deferred actor-end checkpoint within the command holder window. All attempts
    /// share the transition fence and the original transport. An unknown lease transaction
    /// is handed to control-loss reconciliation without replay.
    async fn release_logout(
        &self,
        admitted: AdmittedSession,
        actor: ExactActorRef,
        controller: ControllerBinding,
        channel_switch: bool,
        command_id: Option<u64>,
    ) -> Result<LogoutReply, oteryn_protocol_oteryn::channel_switch::SwitchLock> {
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let store = if channel_switch {
            store.with_channel_switch()
        } else {
            store
        };
        let release = TerminalRelease::Logout(controller.transport);
        // Nothing is fenced yet when no transition id can be minted.
        let token = {
            let mut runtime = self.runtime.lock().await;
            if channel_switch
                && let Some(lock) = self.switch_encounter_lock(&runtime, actor, session)
            {
                return Err(lock);
            }
            let Ok(token) = release.fence(&mut runtime) else {
                operator_event("event=logout_busy stage=release_fence");
                return Ok(LogoutReply::Result(LogoutResult::Busy));
            };
            token
        };
        let mut hold = FenceHold::new(session, token);
        let step = {
            let mut progress = if channel_switch {
                ReleaseProgress::default()
            } else {
                ReleaseProgress::for_logout(self.root, command_id)
            };
            let step = loop {
                let step = self
                    .release_attempt(&admitted, &store, release, &mut hold, &mut progress)
                    .await;
                let Err(stage) = step else {
                    break step;
                };
                // Conditions already used the remaining window at their save point.
                if matches!(
                    stage,
                    ReleaseRetryStage::Conditions | ReleaseRetryStage::ConditionsHolder
                ) || !progress.retry(stage).await
                {
                    break step;
                }
            };
            if matches!(&step, Ok(ReleaseAttempt::Done(GraceExpiryResult::Released)))
                && let Some(retry) = &progress.retry
                && retry.attempts > 0
            {
                operator_event(&format!(
                    "event=logout_release_completed command_id={} attempt={} elapsed_ms={}",
                    retry.command_id.unwrap_or_default(),
                    retry.attempts + 1,
                    retry.started.elapsed().as_millis(),
                ));
            }
            step
        };
        if let Err(stage) = step
            && !channel_switch
        {
            #[cfg(test)]
            let _ = LOGOUT_RELEASE_PROBE.try_with(|probe| probe.retry_exhausted.notify_one());
            operator_event(&format!(
                "event=logout_busy stage=release_retry_exhausted phase={}",
                stage.label()
            ));
        }
        Ok(match step {
            Ok(ReleaseAttempt::Done(GraceExpiryResult::Released)) => {
                LogoutReply::Result(LogoutResult::Accepted)
            }
            // The session is not this logout's to end, and its fence is lifted.
            Ok(ReleaseAttempt::Done(GraceExpiryResult::NotApplicable)) => {
                operator_event("event=logout_busy stage=release_not_applicable");
                LogoutReply::Result(LogoutResult::Busy)
            }
            // A refused fence, or a committed release whose actor could not be removed: the
            // lifecycle reconciles from the row.
            Ok(ReleaseAttempt::Done(GraceExpiryResult::Unknown)) => {
                operator_event("event=logout_unknown stage=release_attempt kind=unknown");
                LogoutReply::Unknown
            }
            Err(ReleaseRetryStage::ConditionsHolder) if !channel_switch => {
                // Every attempt that returned a retry stage stopped before the terminal
                // release. Its exact local token can be lifted without another checkout.
                let lifted = self.states.fence_holders.lift(
                    &mut *self.runtime.lock().await,
                    actor,
                    &mut hold,
                );
                if lifted.is_ok() {
                    LogoutReply::Result(LogoutResult::Busy)
                } else {
                    self.states.fence_holders.keep(hold);
                    LogoutReply::Unknown
                }
            }
            Err(stage) => match self.settle_unended(&store, actor, &mut hold).await {
                UnendedSettle::Lifted => {
                    operator_event(&format!(
                        "event=logout_busy stage=release_retry_lifted phase={}",
                        stage.label()
                    ));
                    LogoutReply::Result(LogoutResult::Busy)
                }
                UnendedSettle::Terminal => {
                    match self
                        .retire_reconciled(controller.account_id, session, actor)
                        .await
                    {
                        GraceExpiryResult::Released => LogoutReply::Result(LogoutResult::Accepted),
                        _ => {
                            operator_event("event=logout_unknown stage=retire kind=unknown");
                            LogoutReply::Unknown
                        }
                    }
                }
                UnendedSettle::Unknown => {
                    self.states.fence_holders.keep(hold);
                    operator_event(&format!(
                        "event=logout_unknown stage=logout_settle kind=unknown phase={}",
                        stage.label()
                    ));
                    LogoutReply::Unknown
                }
            },
            Ok(ReleaseAttempt::Unknown | ReleaseAttempt::Retry) => {
                self.states.fence_holders.keep(hold);
                operator_event("event=logout_unknown stage=release_attempt kind=unknown");
                LogoutReply::Unknown
            }
        })
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
        let Ok(budget) = AdmissionRetryBudget::for_foreground_command(self.root) else {
            return GraceExpiryResult::Unknown;
        };
        let Ok(current) = budget
            .retry_durability("terminal_session", async || {
                store.current_session(session).await
            })
            .await
        else {
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
        let gone = {
            let runtime = self.runtime.lock().await;
            runtime.house_actor_state(actor, session).is_err()
                && runtime.player_control_facts(actor, session).is_err()
        };
        if gone {
            self.release_premium(controller.account_id, session);
            // This already-gone path only forgot fences; keep broader retirement cleanup separate.
            self.states.fence_holders.forget(session);
            if AdmissionRetryBudget::for_command(self.root)
                .retry_durability("account_closure", async || {
                    let absent = {
                        let runtime = self.runtime.lock().await;
                        runtime
                            .prove_player_session_absent_in(
                                session,
                                current.current_runtime_scope(),
                            )
                            .ok()
                            .flatten()
                    }
                    .ok_or(crate::durability::DurabilityError::CommitRejected)?;
                    self.root
                        .acknowledge_account_closure_offline(self.character, self.holder, absent)
                        .await
                })
                .await
                .is_err()
            {
                return GraceExpiryResult::Unknown;
            }
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

impl AdmissionRetryBudget<'_> {
    /// Retry only a holder acquisition failure. Storage and ambiguous commit failures
    /// retain their original outcome; neither is evidence that replay is safe.
    /// Each attempt returns its own future, without borrowing mutable callback
    /// state, so the retry remains usable from boxed Send activity futures.
    pub(super) async fn retry_durability<T, F, Fut>(
        &self,
        stage: &'static str,
        mut step: F,
    ) -> Result<T, crate::durability::DurabilityError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, crate::durability::DurabilityError>>,
    {
        use crate::durability::DurabilityError;
        loop {
            match step().await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    if matches!(error, DurabilityError::RootUnavailable)
                        && self
                            .retries_unavailable(AdmissionRefusal::Unavailable)
                            .await
                    {
                        continue;
                    }
                    let kind = match &error {
                        DurabilityError::RootUnavailable => "holder_busy",
                        DurabilityError::Database(_)
                        | DurabilityError::Migration(_)
                        | DurabilityError::SchemaIncompatible(_) => "storage",
                        DurabilityError::RootPassDeadlineExceeded
                        | DurabilityError::CommitTransactionTimeout => "deadline",
                        DurabilityError::CommitOutcomeUnknown => "commit_unknown",
                        DurabilityError::SessionUseAuthorization(_)
                        | DurabilityError::CommitRejected => "authority",
                        DurabilityError::InvalidStoredState
                        | DurabilityError::InvalidConfiguration
                        | DurabilityError::AdmissionGameSessionLedgerExhausted => "invalid_state",
                        DurabilityError::Unavailable | DurabilityError::RootTaskFailed => {
                            "unavailable"
                        }
                    };
                    operator_event(&format!(
                        "event=session_durability_failed stage={stage} kind={kind}"
                    ));
                    return Err(error);
                }
            }
        }
    }
}
