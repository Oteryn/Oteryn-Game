use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Drop the lost entry of `session` only if it is still the one that ended at
    /// `generation`; a later resumed and lost again connection keeps its own entry.
    /// Keep this conditional removal separate from terminal cleanup: resume uses it too.
    pub(in crate::gameplay_transport) fn forget_lost(
        &self,
        session: GameSessionId,
        generation: u64,
    ) {
        if let Ok(mut lost) = self.states.lost.lock()
            && lost
                .get(&session)
                .is_some_and(|entry| entry.continuity.connection_generation == generation)
        {
            lost.remove(&session);
        }
    }

    /// CHARM-DESC-FENCE-LEASE step (a): fence the slot's damage writes for `session` under the
    /// runtime lock, with no I/O while it is held. Another unsettled transition of the session
    /// is waited for, never replaced.
    pub(in crate::gameplay_transport) async fn fence_transition(
        &self,
        actor: ExactActorRef,
        hold: &mut FenceHold,
    ) -> FenceStep {
        let mut runtime = self.runtime.lock().await;
        match self.states.fence_holders.fence(&mut runtime, actor, hold) {
            Ok(()) => FenceStep::Fenced,
            Err(CarrierError::WriteFenceBusy) => FenceStep::Wait,
            Err(error) => {
                self.states.fence_holders.release(hold);
                FenceStep::Refused(error)
            }
        }
    }

    /// Step (c) for a durable outcome that did not end the hold in this attempt (`NotApplicable`,
    /// `NotExpired`): lift the fence with its exact token only if a durable read shows the session
    /// still holding the lease. TERMINAL is absorbing, so a non-terminal read after the attempt
    /// proves the session held it at the attempt too. A terminal row settles as terminal, with the
    /// session kept fenced; an unreadable row keeps the fence. A fence another release of the
    /// epoch still holds is left to it (`FenceHolders`); a resume of the epoch may already have
    /// lifted it.
    pub(in crate::gameplay_transport) async fn settle_unended(
        &self,
        store: &FreshAdmissionStore,
        actor: ExactActorRef,
        hold: &mut FenceHold,
    ) -> UnendedSettle {
        let Ok(current) = store.current_session(hold.session).await else {
            return UnendedSettle::Unknown;
        };
        if current.session_state() == GameSessionState::Terminal {
            return UnendedSettle::Terminal;
        }
        match self
            .states
            .fence_holders
            .lift(&mut *self.runtime.lock().await, actor, hold)
        {
            Ok(()) => UnendedSettle::Lifted,
            Err(_) => UnendedSettle::Unknown,
        }
    }

    /// Step (c) for a TERMINAL session: the durable fact that allows removing the exact actor.
    /// Its fence is never lifted; the slot goes with it.
    pub(in crate::gameplay_transport) async fn retire(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> GraceExpiryResult {
        match self
            .runtime
            .lock()
            .await
            .remove_terminal_session(session, actor)
        {
            Ok(()) => {
                self.states.forget_session(session);
                GraceExpiryResult::Released
            }
            Err(_) => GraceExpiryResult::Unknown,
        }
    }

    /// PREM-1b (D476, #1722 Codex P2 4176492358): a release whose durable attempt did not end
    /// the session, but whose reconciliation found it TERMINAL, retires it and stops its Premium
    /// pulls like a proven release does.
    pub(in crate::gameplay_transport) async fn retire_reconciled(
        &self,
        account_id: [u8; 16],
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> GraceExpiryResult {
        self.release_premium(account_id, session);
        self.retire(session, actor).await
    }

    /// The fence, monk save, durable commit and settle steps of a terminal release that a
    /// connection decides (abandoned resume, capability mismatch), under one transition token
    /// minted for it. The actor leaves the Channel only after the durable TERMINAL fact.
    /// ATTACK0-RL-03 (ATTACK-0 §4): while `actor` has a running in-fight deadline, wait one hold
    /// step, run the Channel's coalesced monster melee pass and return `true`, so the caller
    /// checks again (a hit taken meanwhile extends the deadline); `false` once no deadline runs.
    /// The hold drives the pass itself because the held actor may be the Channel's last
    /// connection, and only connection cadences run it otherwise.
    pub(in crate::gameplay_transport) async fn hold_while_in_fight(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.hold_step_within(actor, session, Duration::MAX).await
    }

    /// One hold step of at most `limit`; `false`, without waiting, when nothing holds the actor.
    pub(in crate::gameplay_transport) async fn hold_step_within(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        limit: Duration,
    ) -> bool {
        let now = self.owner_now();
        let mut step = {
            let states = self.states.spell_states.lock().await;
            let attack = self.states.attack.lock().await;
            held_step(&states, &attack, actor, session, now, limit)
        };
        let recheck = {
            let states = self.states.spell_states.lock().await;
            !states.is_dead(actor) && pvp_live::tracked(&states, actor, session)
        };
        if step.is_none() && recheck {
            // A victim's settled death can extend this contributor's durable kill lock.
            // Recheck before release even when its cached ordinary hit lock has elapsed;
            // a session with no cached lock entry contributed to no PvP harm here.
            if !self.refresh_pvp_locks(actor, session).await {
                tokio::time::sleep(Duration::from_millis(250).min(limit)).await;
                return true;
            }
            let states = self.states.spell_states.lock().await;
            let attack = self.states.attack.lock().await;
            step = held_step(&states, &attack, actor, session, self.owner_now(), limit);
        }
        let Some(step) = step else {
            return false;
        };
        tokio::time::sleep(step).await;
        if self.ensure_source_map_initialized().await {
            self.drain_monster_melee().await;
        }
        true
    }

    /// The grace wait of a lost client (§3 item 6). While the actor is held in fight, the wait
    /// runs in hold steps that drive the melee pass, so a monster can still hit it and extend the
    /// hold when its connection was the Channel's last.
    pub(in crate::gameplay_transport) async fn wait_grace(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        wait: Duration,
    ) {
        let end = tokio::time::Instant::now() + wait;
        loop {
            let left = end.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return;
            }
            if !self.hold_step_within(actor, session, left).await {
                tokio::time::sleep(left).await;
                return;
            }
        }
    }
}
