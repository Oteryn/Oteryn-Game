//! Bounded source-owned world deadlines in the actual Channel owner cycle.
use super::ComposedFreshAdmission;

use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn run_source_owner_cycles(
        &self,
        shutdown: &oteryn_foundation::CancellationToken,
    ) {
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_none()
        {
            return;
        }
        let wake = self.states.spell_states.lock().await.owner_wake.clone();
        let mut npc_voices = {
            let runtime = self.runtime.lock().await;
            self.states
                .chat
                .npc_talk()
                .map(|book| super::npc_voice::NpcVoiceCycle::new(&runtime, book))
        };
        loop {
            let mut notified = pin!(wake.notified());
            notified.as_mut().enable();
            let delay_us = {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                if runtime.owner_fence().is_err() {
                    return;
                }
                let now = self.owner_now().get();
                states
                    .spell_timers
                    .as_ref()
                    .and_then(|timer| timer.next_due_time())
                    .map(|due| due.get())
                    .into_iter()
                    .chain(states.primal_callbacks.next_due_us())
                    .chain(states.spite_steps.next_due_us())
                    .chain(states.rotten_blood_callbacks.next_due_time())
                    .chain(
                        self.bundle_map()
                            .and_then(|map| map.facts.next_movement_decay_time()),
                    )
                    .chain(self.states.quest_callbacks.next_encounter_due_us())
                    .chain(
                        npc_voices
                            .as_ref()
                            .and_then(super::npc_voice::NpcVoiceCycle::next_due_us),
                    )
                    .min()
                    .map_or(1_000_000, |due| {
                        // An unavailable due producer cannot cause a busy loop.
                        // A newly committed schedule wakes this pass immediately.
                        let remaining = due.saturating_sub(now);
                        if remaining == 0 {
                            50_000
                        } else {
                            remaining.min(1_000_000)
                        }
                    })
            };
            let mut stopping = pin!(shutdown.cancelled());
            let mut delay = pin!(tokio::time::sleep(std::time::Duration::from_micros(
                delay_us
            )));
            let stopped = poll_fn(|context| {
                if stopping.as_mut().poll(context).is_ready() {
                    return Poll::Ready(true);
                }
                if notified.as_mut().poll(context).is_ready()
                    || delay.as_mut().poll(context).is_ready()
                {
                    Poll::Ready(false)
                } else {
                    Poll::Pending
                }
            })
            .await;
            if stopped {
                // Cancellation does not prove that player/other autonomous
                // owners have stopped. Serving performs the terminal source
                // drain only after that stronger shared quiescence boundary.
                return;
            }
            // Keep the population sweep outside any operation. Multi-pass
            // units such as equipment checkpoints scope their own grants.
            {
                let runtime = self.runtime.lock().await;
                if runtime.owner_fence().is_err() {
                    return;
                }
                if let Some(map) = self.bundle_map() {
                    map.facts.expire_movement_items(self.owner_now().get());
                }
                if let Ok(mut gate) = self.states.world_gate.lock() {
                    gate.begin_turn();
                }
            }
            // Rotate independent units, including empty discoveries. The two
            // Rotten slots retain cyclic pre/AI/post order across resumed sweeps;
            // no whole-population grant is introduced.
            let start = self.background_start("source-cycle", 14);
            for index in (start..14).chain(0..start) {
                self.background_turn("source-cycle", index + 1, async {
                    match index {
                        0 => {
                            let _ = self
                                .root
                                .with_background_operation(self.flush_daily_kill_statistics())
                                .await;
                        }
                        1 => {
                            self.tick_timed_items().await;
                        }
                        2 => {
                            let _ = self.ensure_source_map_initialized().await;
                        }
                        3 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(self.drain_source_item_deadlines())
                                    .await;
                                self.drain_spite_recovery_notices().await;
                            }
                        }
                        4 => {
                            if self.ensure_source_map_initialized().await {
                                self.drain_quest_encounter_deadlines().await;
                            }
                        }
                        5 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(self.drain_elemental_altar_bosses())
                                    .await;
                            }
                        }
                        6 => {
                            if self.ensure_source_map_initialized().await {
                                self.drain_source_party_deadlines_bounded().await;
                            }
                        }
                        7 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(
                                        self.drain_rotten_movement_callbacks(),
                                    )
                                    .await;
                            }
                        }
                        8 => {
                            if self.ensure_source_map_initialized().await {
                                let _ = self.drain_native_ai_timers().await;
                            }
                        }
                        9 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(
                                        self.drain_rotten_movement_callbacks(),
                                    )
                                    .await;
                            }
                        }
                        10 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(self.drain_bak_first_think_claims())
                                    .await;
                            }
                        }
                        11 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(self.drain_primal_pod_callbacks())
                                    .await;
                            }
                        }
                        12 => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(self.drain_primal_pod_expiries())
                                    .await;
                            }
                        }
                        _ => {
                            if self.ensure_source_map_initialized().await {
                                self.root
                                    .with_background_operation(self.drain_rotten_portal_callbacks())
                                    .await;
                            }
                        }
                    }
                })
                .await;
            }
            if let Some(cycle) = npc_voices.as_mut() {
                self.emit_npc_voices(cycle, self.owner_now().get()).await;
            }
        }
    }

    pub(super) async fn drain_source_party_deadlines_bounded(&self) {
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_none()
        {
            return;
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let now = self.owner_now().get();
            if runtime.owner_fence().is_err() || now < states.next_party_deadline_pass_us {
                return;
            }
            states.next_party_deadline_pass_us = now.saturating_add(1_000_000);
        }
        let start = self.background_start("party-deadlines", 4);
        for index in (start..4).chain(0..start) {
            self.background_turn("party-deadlines", index + 1, async {
                match index {
                    0 => {
                        let _ = self.drain_source_house_dispositions().await;
                    }
                    1 => {
                        let _ = self
                            .root
                            .with_background_operation(self.drain_source_pvp_expiry())
                            .await;
                    }
                    2 => {
                        let _ = self
                            .root
                            .with_background_operation(self.drain_source_party_expiry())
                            .await;
                    }
                    _ => {
                        let _ = self
                            .root
                            .with_background_operation(self.drain_source_party_offline_presence())
                            .await;
                    }
                }
            })
            .await;
        }
    }

    pub(super) async fn ensure_source_map_initialized(&self) -> bool {
        self.root
            .with_background_operation(self.ensure_source_map_initialized_inner())
            .await
    }

    async fn ensure_source_map_initialized_inner(&self) -> bool {
        #[cfg(test)]
        let stage = |name: &str| {
            if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
                eprintln!("SEAM_EVIDENCE source_map_initialize stage={name}");
            }
        };
        #[cfg(test)]
        stage("entered");
        let Some(room) = self
            .rooms()
            .gate_source_map_initialization()
            .filter(|room| room.source_world().is_some())
        else {
            return true;
        };
        // BOOSTED-1 is best effort here: movement must not depend on the daily choice,
        // and world_respawns retries it before spawns read it.
        let _ = self.initialize_daily_boost().await;
        let Some(source) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return false;
        };
        // Lock order (§1.2): the lane before any Channel guard.
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err()
            || source.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return false;
        }
        if matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) {
            return crate::spell::source_map_initialization::CurrentNativeMapInitialization::qualify_static_instance(
                &runtime, room, source,
            )
            .is_ok();
        }
        let binding = match crate::spell::source_map_initialization::CurrentNativeMapInitialization::qualify_current_binding(
            &runtime, room, source,
        ) {
            Ok(binding) => binding,
            Err(_reason) => {
                #[cfg(test)]
                stage(&format!("binding_refused:{_reason}"));
                return false;
            }
        };
        if states.source_map_initialized.as_ref() == Some(&binding) {
            return true;
        }
        let now = self.owner_now().get();
        if now < states.next_map_initialization_pass_us {
            return false;
        }
        states.next_map_initialization_pass_us = now.saturating_add(1_000_000);
        let proof = match crate::spell::source_map_initialization::CurrentNativeMapInitialization::from_current_owner(
            &runtime, room, source,
        ) {
            Ok(proof) => proof,
            Err(_reason) => {
                #[cfg(test)]
                stage(&format!("placements_refused:{_reason}"));
                return false;
            }
        };
        #[cfg(test)]
        stage("owner_sql_begin");
        if let Err(_error) = self
            .root
            .initialize_native_map_current_owner(&permit, self.character, self.holder, &proof)
            .await
        {
            #[cfg(test)]
            match _error {
                crate::durability::spell_item_transaction::SpellItemError::Database(
                    sqlx::Error::Database(error),
                ) => stage(&format!(
                    "owner_sql_refused:sqlstate={}",
                    error.code().as_deref().unwrap_or("unknown")
                )),
                crate::durability::spell_item_transaction::SpellItemError::Rejected(reason) => {
                    stage(&format!("owner_sql_refused:rejected={reason}"))
                }
                _ => stage("owner_sql_refused:other_error"),
            }
            return false;
        }
        #[cfg(test)]
        stage("owner_sql_committed");
        // Only a real SQL COMMIT sets this physical owner's readiness. Tile
        // reads still inspect current custody; this never restores moved items.
        states.source_map_initialized = Some(binding);
        true
    }

    pub(super) async fn drain_source_item_deadlines(&self) {
        let Some(source) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        // Lock order (§1.2): the lane before any Channel guard.
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let now = self.owner_now().get();
        if now < states.next_item_deadline_pass_us
            || runtime.owner_fence().is_err()
            || source.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return;
        }
        // Failed or uncertain commits also retry on the next bounded owner
        // pass. Immutable SQL deadlines reconcile them without another cause.
        states.next_item_deadline_pass_us = now.saturating_add(1_000_000);
        let scope = runtime.binding().runtime_scope();
        let mut ground_change = super::loop_move::GroundChange(Some(&self.states.loop_ground));
        let result = self
            .root
            .drain_spell_item_deadlines(
                &permit,
                self.character,
                self.holder,
                scope,
                runtime.binding().scope_generation().get(),
            )
            .await;
        if matches!(result, Ok(0)) {
            ground_change.0 = None;
        }
    }
}
