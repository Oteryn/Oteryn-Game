use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// DEATH-2 (first player death decision §4.3-§4.5): settle one runtime death and respawn the
    /// actor, in the order of [`respawn_in_order`]. With the session's progression binding the
    /// death is committed (or replayed) off the owner lane, with no lock held across the database;
    /// the dead actor is then placed at its respawn position in one owner step while the session's
    /// current durable fence is held, and its pending respawn consumed after that placement in the
    /// same transaction ([`Self::place_and_consume_settled_respawn`]); only then is it refilled in
    /// another owner step, which ends the death. `None` (a durable attempt without an outcome, a
    /// session without a durable death, a refused fence, a stale actor, a recorded cell no
    /// admission reproduces or a consumption without an outcome) keeps the player dead, and the
    /// next cadence tick retries the same occurrence.
    pub(in crate::gameplay_transport) async fn respawn_after_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        if matches!(
            actor.runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) {
            let (settled, _) = self.settle_death(actor, session, death, false).await;
            if settled != SessionDeath::Durable {
                return None;
            }
            let admitted = self.activity_runtimes.connection(session)?;
            if admitted.runtime_actor != Some(actor) {
                return None;
            }
            match self
                .return_due_activity(
                    admitted,
                    crate::durability::activity_handoff::ActivityReturnReason::Death {
                        occurrence: *death.occurrence.as_bytes(),
                    },
                )
                .await
            {
                Ok(super::super::native_activity::NativeActivityOutcome::Committed(commit)) => {
                    self.publish_activity_commit(&commit).ok()?;
                }
                Ok(super::super::native_activity::NativeActivityOutcome::Refused(_)) | Err(()) => {}
            }
            // The old actor/session never publishes a respawn result. The genuine
            // Channel successor crosses the activity barrier with full snapshots.
            return None;
        }
        // DEATH-ML-LOSS-1 (CP D1433, #2193 Codex P1 4304911165): the committed death's
        // post-loss build, carried from the settle to the respawn step that rebases the live
        // actor onto it, so the next training save submits the lowered durable build.
        let lowered = std::sync::OnceLock::new();
        respawn_in_order(
            async {
                let (settled, after) = self.settle_death(actor, session, death, false).await;
                if let Some(after) = after {
                    let _ = lowered.set(after);
                }
                settled
            },
            self.place_and_consume_settled_respawn(actor, session, death.occurrence),
            async {
                let now = self.owner_now();
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if let Some(after) = lowered.get() {
                    // A replayed death rebases again onto the same receipt; an actor this step
                    // cannot reach stays dead for the next cadence.
                    states
                        .get_mut(&runtime, actor, session)?
                        .rebase_after_death(after);
                }
                states.respawn(&runtime, actor, session, death.occurrence, now)
            },
        )
        .await
    }

    /// DEATH-2 §4.3-§4.4: with the session's progression binding, commit (or replay) the durable
    /// death of `death`, with no lock held across the database. Its pending respawn stays
    /// recorded for the placement.
    ///
    /// DEATH-ML-LOSS-1 (A13 §4.6, CP D1433): before a new death the actor's pending training is
    /// force-saved ([`Self::flush_training_before_death`]), and a committed death that lowered the
    /// build returns its post-loss build for the respawn's rebase. `releasing` marks the actor-end
    /// release, which settles a session whose training writes are fenced out once the fence left
    /// no training in memory (Codex P1 4307011952).
    pub(in crate::gameplay_transport) async fn settle_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
        releasing: bool,
    ) -> (
        SessionDeath,
        Option<crate::durability::character_build::DurableBuildState>,
    ) {
        if self.states.player_trades.transitioning(session) {
            return (SessionDeath::Retry, None);
        }
        self.states.player_trades.cancel(session);
        self.states.newhaven_travel.death(session);
        let mut lowered = None;
        let lowered_slot = &mut lowered;
        let settled =
            settle_session_death(&self.states.progression_sessions, session, |progression| {
                Box::pin(async move {
                    if !self
                        .flush_training_before_death(actor, session, death, releasing)
                        .await
                    {
                        return DeathCommit::Unavailable;
                    }
                    let (commit, after) = self.commit_death(session, &progression, death).await;
                    *lowered_slot = after;
                    commit
                })
            })
            .await;
        // Instance equipment is refreshed by complete_activity_death_return
        // after the genuine Channel handoff, before the actor is respawned.
        // The existing equipment owner cannot read an Instance as a Channel.
        if matches!(settled, SessionDeath::Durable)
            && !releasing
            && matches!(
                actor.runtime_scope(),
                crate::foundation::RuntimeScopeRefV1::Channel { .. }
            )
        {
            let Ok(Some(committed)) = self
                .root
                .reconcile_character_death(self.character, death.occurrence)
                .await
            else {
                return (SessionDeath::Retry, lowered);
            };
            let equipment_changed = committed
                .items
                .as_ref()
                .is_some_and(|items| items.amulet.is_some() || !items.lost.is_empty());
            if equipment_changed {
                // Drops and the AoL burn changed durable equipment. A respawn must
                // not retain protections or a swing weapon from the pre-death slots.
                let Some(_permit) = self.spell_lane_permit().await else {
                    return (SessionDeath::Retry, lowered);
                };
                let Ok(effects) = self.read_admission_effects(session).await else {
                    return (SessionDeath::Retry, lowered);
                };
                self.states
                    .attack
                    .lock()
                    .await
                    .forget_weapon(actor, session);
                let runtime = self.runtime.lock().await;
                if self
                    .install_admission_suppressions(&runtime, actor, session, effects)
                    .await
                    .is_none()
                {
                    return (SessionDeath::Retry, lowered);
                }
            }
        }
        (settled, lowered)
    }

    /// DEATH-RESTART-SAFETY-1 (#2004 Codex P1 4248558737): resolve the session's current fences
    /// and, in the Character's revision slot, place the dead actor while PostgreSQL holds them
    /// ([`DurabilityRoot::place_and_consume_pending_respawn_placed`]): the placement runs inside
    /// the fenced transaction, so a session that turns terminal or is superseded, a moved lease, a
    /// moved scope assignment or a revoked node incarnation commits either before the fence is
    /// asserted, and nothing is placed, or after the placement. A settle that replayed a retained
    /// receipt checked none of them. The owner step itself revalidates the owner's binding
    /// ([`place_authorized_respawn`]) and decides from the pending row read under the fence whether
    /// it may be consumed ([`settled_respawn_disposition`]); the row is deleted only after the
    /// placement, in the same transaction, which also upserts the placed respawn cell as the
    /// Character's last position (CHAR-POSITION-0 §3.3 step 1), so a crash right after the
    /// placement never puts the Character back at its death cell. No I/O runs while the runtime
    /// lock is held. `false` (no current fence, a failed cursor read, a refused fence or
    /// placement, a recorded cell no admission reproduces or a failed write) keeps the actor dead;
    /// the row stays for the next attempt or, after a restart, for the next admission.
    ///
    /// [`DurabilityRoot::place_and_consume_pending_respawn_placed`]: crate::durability::DurabilityRoot::place_and_consume_pending_respawn_placed
    pub(in crate::gameplay_transport) async fn place_and_consume_settled_respawn(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
    ) -> bool {
        self.place_and_consume_settled_respawn_for(actor, session, occurrence, None)
            .await
    }

    async fn place_and_consume_settled_respawn_for(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
        expected: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
    ) -> bool {
        let Some(respawn) = self.home_temple(session).await else {
            return false;
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        if !matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) || expected.is_some_and(|expected| expected != fence.item_fence())
        {
            return false;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(expected_character_revision) = slot.cursor(self.root, self.character).await else {
            return false;
        };
        let authorized = AuthorizedRespawn {
            character_id: fence.character_id,
            lease_generation: fence.character_lease_generation,
            scope_generation: fence.scope_ownership_generation,
        };
        // CHAR-POSITION-0 §3.2-§3.3: the respawn cell the placement puts the actor at, upserted as
        // the last position in the transaction that consumes the respawn. Its sequence is taken
        // after the death, and a dead actor is never captured, so no capture of the living actor
        // orders after it.
        let respawned = {
            let runtime = self.runtime.lock().await;
            self.placed_record(&runtime, respawn)
        };
        let runtime = &self.runtime;
        let placed = self
            .root
            .place_and_consume_pending_respawn_placed(
                self.character,
                self.holder,
                crate::durability::character_progression::CurrentCharacterGameplayFence {
                    expected_character_revision,
                    ..fence
                },
                occurrence,
                respawned,
                move |pending| async move {
                    let mut runtime = runtime.lock().await;
                    let admission_cell = respawn;
                    if !last_position::respawn_cell_recorded(admission_cell, &respawned) {
                        operator_event("settled_respawn_placement_refused reason=cell_changed");
                        return false;
                    }
                    match settled_respawn_disposition(occurrence, admission_cell, pending.as_ref())
                    {
                        SettledRespawn::Kept => {
                            operator_event(
                                "settled_respawn_kept reason=not_reproduced_by_admission",
                            );
                            false
                        }
                        SettledRespawn::Gone | SettledRespawn::Reproduced => {
                            place_authorized_respawn_at(
                                &mut runtime,
                                actor,
                                session,
                                &authorized,
                                respawn,
                            )
                        }
                    }
                },
            )
            .await;
        drop(slot);
        match placed {
            Ok(placed) => {
                if placed {
                    self.states
                        .last_positions
                        .respawned(session, respawned.record.cell);
                    // NPC-TALK-2: a respawned customer away from its NPC ends its conversation.
                    self.states
                        .chat
                        .close_out_of_range(&*self.runtime.lock().await, session);
                }
                placed
            }
            Err(error) => {
                operator_event(&format!("settled_respawn_placement_failed reason={error}"));
                false
            }
        }
    }

    /// Called only by the retained native Death return attempt, after actual
    /// actor/state transfer and before any connection barrier. The attempt records
    /// true synchronously, so no retry mistakes an arbitrary living actor for this death.
    pub(in crate::gameplay_transport) async fn complete_activity_death_return(
        &self,
        admitted: super::super::connection::AdmittedSession,
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
    ) -> bool {
        let (Some(actor), Some(expected)) = (admitted.runtime_actor, admitted.item_fence) else {
            return false;
        };
        if !matches!(
            expected.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) || actor.runtime_scope() != expected.runtime_scope
            || actor.scope_generation() != expected.scope_ownership_generation
        {
            return false;
        }
        let Ok(Some(committed)) = self
            .root
            .reconcile_character_death(self.character, occurrence)
            .await
        else {
            return false;
        };
        if committed.character_id != expected.character_id
            || committed.death_cell.runtime_scope.world_id() != expected.runtime_scope.world_id()
            || !matches!(
                committed.death_cell.runtime_scope,
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            )
        {
            return false;
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states
                .player_death(&runtime, actor, admitted.game_session_id)
                .is_none_or(|death| death.occurrence != occurrence)
            {
                return false;
            }
        }
        if committed
            .items
            .as_ref()
            .is_some_and(|items| items.amulet.is_some() || !items.lost.is_empty())
        {
            // A returned offline actor is still Reconnectable. Read its exact
            // current Channel equipment through the lifecycle fence, without
            // broadening the active-session command or observation readers.
            let Some(_permit) = self.spell_lane_permit().await else {
                return false;
            };
            let Ok(effects) = self.read_activity_death_effects(expected).await else {
                return false;
            };
            self.states
                .attack
                .lock()
                .await
                .forget_weapon(actor, admitted.game_session_id);
            let runtime = self.runtime.lock().await;
            if self
                .install_admission_suppressions(&runtime, actor, admitted.game_session_id, effects)
                .await
                .is_none()
            {
                return false;
            }
        }
        if !self
            .place_and_consume_settled_respawn_for(
                actor,
                admitted.game_session_id,
                occurrence,
                Some(expected),
            )
            .await
        {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if states
            .player_death(&runtime, actor, admitted.game_session_id)
            .is_none_or(|death| death.occurrence != occurrence)
        {
            return false;
        }
        if let Some(build) = committed.build_loss {
            let Some(state) = states.get_mut(&runtime, actor, admitted.game_session_id) else {
                return false;
            };
            state.rebase_after_death(&build.after);
        }
        // No await follows the atomic native respawn. The caller immediately
        // retains completion while still holding the unique return-attempt mutex.
        states
            .respawn(
                &runtime,
                actor,
                admitted.game_session_id,
                occurrence,
                self.owner_now(),
            )
            .is_some()
    }

    /// DEATH-2 for a closed client: an actor that died after its connection ended has no cadence
    /// left to settle the death, so its release settles it before the actor leaves, and the next
    /// admission consumes the respawn. `settled` is the death this release already committed, so a
    /// retry after the durable release does not write it again. `false` keeps the actor: the
    /// durable attempt had no outcome.
    pub(in crate::gameplay_transport) async fn settle_released_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        settled: &mut Option<actor_spell::PlayerDeath>,
    ) -> bool {
        let death = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            states.player_death(&runtime, actor, session)
        };
        match death {
            // A durable death leaves its pending respawn for the next admission (DEATH-0 §3.4); a
            // session without a durable death releases the actor with nothing written.
            Some(death) if *settled != Some(death) => {
                let (outcome, lowered) = self.settle_death(actor, session, death, true).await;
                match outcome {
                    SessionDeath::Durable | SessionDeath::NoDurableDeath => {
                        // DEATH-ML-LOSS-1 (A13 §4.6, CP D1439): the released actor continues
                        // from the death's post-loss build, as a respawn would rebase it, so the
                        // release's own training save and a resumed epoch never submit the
                        // pre-death build.
                        if let Some(after) = lowered {
                            let runtime = self.runtime.lock().await;
                            let mut states = self.states.spell_states.lock().await;
                            if let Some(state) = states.get_mut(&runtime, actor, session) {
                                state.rebase_after_death(&after);
                            }
                        }
                        *settled = Some(death);
                        true
                    }
                    SessionDeath::Retry => false,
                }
            }
            _ => true,
        }
    }

    /// DEATH-0 §3.4 and DEATH-2: a fresh admission with full vitals is the respawn of a death
    /// committed before the previous actor ended. In the Character's revision slot and under the
    /// admitted session's fences, before the session's other Character writes, its pending
    /// respawn is read, the new actor is placed at the recorded respawn position, and only then
    /// is the obligation consumed. The first-entry owner step already placed the actor for the
    /// respawn it read (`decided`, CHAR-POSITION-0 §3.3); only a respawn it did not see is placed
    /// here. A recorded cell that is not valid or not free uses the CHAR-POSITION-0 §3.3
    /// fallback, so a placement is always defined. A placement that cannot complete or a failed
    /// read or write leaves the row for the next admission; the Character writers refuse with
    /// `RespawnPending` until then. `true` only when exactly the placed respawn was consumed, or
    /// no respawn is pending and the first entry was placed from a successful last-position read
    /// (CHAR-POSITION-0 §3.3 steps 3 and 4): a failed read or placement leaves this admission not
    /// playable, as an unplaced respawn does.
    pub(in crate::gameplay_transport) async fn consume_admitted_respawn(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        session: GameSessionId,
        actor: ExactActorRef,
        decided: Option<&last_position::AdmissionPosition>,
    ) -> bool {
        // Death settlement also holds the spell lane before its revision slot.
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        // NODEBOOT-1: the reads before the placement are retried on a busy holder.
        let Ok(Some(fence)) = self.current_quest_fence_within(budget, session).await else {
            return false;
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(expected_character_revision) = budget
            .retry_unavailable(async || {
                slot.cursor(self.root, self.character)
                    .await
                    .map_err(|_| AdmissionRefusal::Unavailable)
            })
            .await
        else {
            return false;
        };
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            expected_character_revision,
            ..fence
        };
        let pending = match budget
            .retry_unavailable(async || {
                self.root
                    .pending_respawn(self.character, self.holder, fence)
                    .await
                    .map_err(|error| {
                        operator_event(&format!("pending_respawn_read_failed reason={error}"));
                        AdmissionRefusal::Unavailable
                    })
            })
            .await
        {
            Ok(Some(pending)) => pending,
            Ok(None) => {
                // CHAR-POSITION-0 §3.3 steps 3 and 4: no pending respawn, so the last position
                // the first-entry step was placed by. An entry without a successful read, or one
                // placed for a respawn that is gone, keeps the admission not playable.
                drop(slot);
                return decided
                    .is_some_and(|decided| self.track_restored(session, actor, decided, fence));
            }
            Err(_) => return false,
        };
        // A reconnect can arrive after the receipt committed but before its item operations.
        if self
            .resume_player_death_items(&permit, fence, pending.occurrence)
            .await
            .is_err()
        {
            return false;
        }
        let placed_at_entry = matches!(
            decided,
            Some(last_position::AdmissionPosition::Respawn(placed))
                if placed.occurrence == pending.occurrence
        );
        if !placed_at_entry
            && !self
                .place_admitted_respawn(session, actor, &pending.respawn_position)
                .await
        {
            operator_event("pending_respawn_placement_failed");
            return false;
        }
        // CHAR-POSITION-0 §3.3: the consumption upserts the placed cell as the last position, in
        // the same transaction, under the new lease's key.
        // A placed actor whose cell cannot be read back (stale or gone) consumes nothing: the
        // obligation stays for the next admission rather than be consumed without its cell.
        let Some(placed) = self.respawn_last_position(actor).await else {
            operator_event("pending_respawn_placed_cell_unavailable");
            return false;
        };
        let consumed = self
            .root
            .consume_pending_respawn_placed(
                self.character,
                self.holder,
                fence,
                Some(pending.occurrence),
                placed,
            )
            .await;
        if matches!(&consumed, Ok(Some(consumed)) if consumed.occurrence == pending.occurrence) {
            self.track_respawned(session, actor, &placed, fence);
        }
        if let Err(error) = &consumed {
            operator_event(&format!(
                "pending_respawn_consumption_failed reason={error}"
            ));
        }
        admitted_respawn_consumed(pending.occurrence, consumed)
    }

    /// DEATH-2b: place the admitted actor for its pending respawn, in one Channel owner turn: at
    /// the recorded cell if it is an admissible free cell, else by the CHAR-POSITION-0 §3.3
    /// fallback (`ChannelRuntimeV1::place_admitted_respawn`). `false` only when the actor cannot
    /// be placed at all, and then nothing moves.
    pub(in crate::gameplay_transport) async fn place_admitted_respawn(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        recorded: &[u8],
    ) -> bool {
        let mut runtime = self.runtime.lock().await;
        let pinned_digest = runtime.content_pin().server_artifact_digest();
        let door = self.door.lock().await;
        runtime
            .place_admitted_respawn(actor, session, respawn_cell(recorded), |cell| {
                respawn_cell_admissible(
                    self.movement_cells.index(),
                    self.movement_cells.scope(),
                    self.world_id,
                    pinned_digest,
                    door.blocking_cells(),
                    cell,
                )
            })
            .is_ok()
    }
}
