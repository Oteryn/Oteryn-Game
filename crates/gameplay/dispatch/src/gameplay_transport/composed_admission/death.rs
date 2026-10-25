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
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
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
            self.placed_record(&runtime, runtime.respawn_position())
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
                    let admission_cell = runtime.respawn_position();
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
                            place_authorized_respawn(&mut runtime, actor, session, &authorized)
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
        session: GameSessionId,
        actor: ExactActorRef,
        decided: Option<&last_position::AdmissionPosition>,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(expected_character_revision) = slot.cursor(self.root, self.character).await else {
            return false;
        };
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            expected_character_revision,
            ..fence
        };
        let pending = match self
            .root
            .pending_respawn(self.character, self.holder, fence)
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
            Err(error) => {
                operator_event(&format!("pending_respawn_read_failed reason={error}"));
                return false;
            }
        };
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
