impl ComposedFreshAdmission<'_, '_, '_> {
    /// A committed native scope transfer keeps the same LastPosition sequence/cadence owner.
    pub(super) async fn track_transferred_position(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
        actor: ExactActorRef,
        record: &LastPositionRecord,
        character: crate::domain::CharacterId,
    ) -> Option<()> {
        if self
            .states
            .last_positions
            .transferred(source, destination, actor, record)
        {
            return Some(());
        }
        let revision = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?
            .revision;
        self.states
            .last_positions
            .track(destination, actor, Some(Saved::of(record)), revision);
        Some(())
    }

    /// Capture `track`'s actor's current cell and `session`'s vitals under the runtime lock,
    /// with its record and a fresh write sequence. A dead actor (DEATH-2 §4.2, until its
    /// respawn) is not captured. Vitals that cannot be read (no spell state, or a health of 0 not
    /// yet recorded as a death) keep the ones the track last wrote, so a capture never saves the
    /// maxima it did not see.
    #[cfg(test)]
    pub(super) async fn last_position_capture_is_held(&self, session: GameSessionId) -> bool {
        let Some(track) = self.states.last_positions.tracked(session) else {
            return false;
        };
        matches!(
            self.capture_last_position(&track, session).await,
            Captured::Held | Captured::Vocation(_)
        )
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "The tutorial vocation case is compiled in only one PG shard")]
    pub(super) fn last_position_checked_for_test(&self, session: GameSessionId) -> Option<Instant> {
        self.states.last_positions.tracked(session).map(|track| track.checked)
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "The tutorial vocation case is compiled in only one PG shard")]
    pub(super) fn last_position_cached_for_test(&self, session: GameSessionId) -> bool {
        self.states.last_positions.tracked(session).is_some_and(|track| track.written.is_some())
    }

    async fn capture_retained_tutorial_position(
        &self,
        track: &Track,
        session: GameSessionId,
        character: crate::domain::CharacterId,
        retained: RetainedPosition,
    ) -> Captured {
        #[cfg(test)]
        retained_capture_pause(RetainedCaptureBoundary::BeforeSlot).await;
        if character != retained.character {
            return Captured::Held;
        }
        let Some(_slot) = self.revision_sequencer.try_acquire(character) else {
            return Captured::Held;
        };
        self.capture_position(track, session, VocationCapture::Recheck(retained.occurrence)).await
    }

    async fn capture_last_position(&self, track: &Track, session: GameSessionId) -> Captured {
        self.capture_position(track, session, VocationCapture::Hold).await
    }

    async fn capture_position(
        &self,
        track: &Track,
        session: GameSessionId,
        vocation: VocationCapture,
    ) -> Captured {
        let actor = track.actor;
        let kept = track.written.and_then(|saved| saved.vitals);
        let runtime = self.runtime.lock().await;
        // A retained build may have committed its refill before live installation. Decide
        // which vitals are authoritative under the same guard that captures the live state.
        let pending_vocation_refill = match self.states.tutorial_sessions.vocation(session) {
            Ok(None) => self.states.last_positions.tracked(session)
                .filter(|current| current.actor == actor)
                .and_then(|current| current.trial_refill),
            Ok(Some(retained)) => match vocation {
                VocationCapture::Final => Some(retained.request.occurrence),
                VocationCapture::Hold => return Captured::Vocation(RetainedPosition {
                    character: retained.fence.character_id,
                    occurrence: retained.request.occurrence,
                }),
                VocationCapture::Recheck(occurrence) if retained.request.occurrence == occurrence => Some(occurrence),
                _ => return Captured::Held,
            },
            Err(()) => return Captured::Held,
        };
        if self.has_pending_paid_quest_callback(session)
            || self.states.dawnport_departures.held(session)
            || self
                .states
                .npc_conversations
                .dawnport_choice(session)
                .is_some_and(|choice| choice.retained.is_some())
            || self
                .states
                .dawnport_trials
                .lock()
                .map_or(true, |held| held.contains(&session))
        {
            return Captured::Held;
        }
        let house = runtime.house_actor_state(actor, session).ok();
        let (vitals, rest) = {
            let states = self.states.spell_states.lock().await;
            if states.enchanting_pending.contains(&(actor, session)) {
                return Captured::Held;
            }
            if states.is_dead(actor) {
                return Captured::Dead;
            }
            if states.has_pending_spell_commit(actor, session) {
                return Captured::Held;
            }
            // The native House owner above proves the actual actor/session/lease before
            // reading the same PlayerSpellState tuple. Generic Channel APIs remain closed.
            let state = if house.is_some() {
                states
                    .actors
                    .iter()
                    .find(|(held, bound, _)| *held == actor && *bound == session)
                    .map(|(_, _, state)| state)
            } else {
                states.get(&runtime, actor, session)
            };
            let vitals = match state.map(|state| state.saved_vitals()) {
                Some(Some((health, mana))) => LastVitals::new(health, mana).or(kept),
                Some(None) => None,
                None => kept,
            };
            let rest = state
                .map(|state| state.bed_rest_snapshot(self.owner_now().get()))
                .or_else(|| track.written.and_then(|saved| saved.rest));
            (vitals, rest)
        };
        let (cell, house) = if let Some(state) = house {
            let Some(house) =
                crate::durability::house_scope_handoff::HouseId::new(self.world_id, &state.house)
            else {
                return Captured::Gone;
            };
            (
                MovementLocalPosition {
                    x: state.position.x,
                    y: state.position.y,
                    floor: state.position.floor,
                },
                Some(HouseLastPosition {
                    house,
                    origin_channel: self.channel_id,
                    origin: LastPositionCell {
                        x: state.origin_position.x,
                        y: state.origin_position.y,
                        floor: state.origin_position.floor,
                    },
                }),
            )
        } else {
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                return Captured::Gone;
            };
            if snapshot.context() != runtime.pinned_movement_context() {
                return Captured::Gone;
            }
            (snapshot.position(), None)
        };
        let mut placed = self.placed_record(&runtime, cell);
        placed.record.vitals = vitals;
        placed.record.rest = rest;
        Captured::At(Box::new(Capture {
            record: placed.record,
            house,
            write_sequence: placed.write_sequence,
            at: Instant::now(),
            pending_vocation_refill,
        }))
    }

    pub(super) fn invalidate_first_vocation_position(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        at: Instant,
    ) {
        if let Ok(mut tracks) = self.states.last_positions.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
            && track.actor == actor
            && track.written_at <= at
        {
            // The initial projection and the subsequent refill both bypass the periodic cache.
            track.written = None;
            track.written_at = at;
        }
    }

    pub(super) fn set_trial_refill(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        occurrence: Option<BuildOccurrence>,
    ) {
        if let Ok(mut tracks) = self.states.last_positions.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
            && track.actor == actor
        {
            track.trial_refill = occurrence;
            track.written = None;
            track.written_at = Instant::now();
        }
    }

    /// The record of `cell` in the pinned context, with a fresh write sequence and no vitals (a
    /// placed respawn is at its maxima); taken under the runtime lock the caller holds.
    pub(super) fn placed_record(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        cell: MovementLocalPosition,
    ) -> FinalLastPosition {
        let pin = runtime.content_pin();
        FinalLastPosition {
            record: LastPositionRecord {
                rest: None,
                world_id: self.world_id,
                cell: cell_of(cell),
                coordinate_frame_digest: pin.frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: pin.server_artifact_digest(),
                vitals: None,
            },
            write_sequence: self.states.last_positions.next_sequence(),
        }
    }
    /// §3.2 "At logout": the final position of a tracked session's fenced actor, which the
    /// terminal release writes in its own transaction. `None` for an untracked session.
    pub(super) async fn final_last_position(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> Option<FinalLastPosition> {
        if !self.resume_dawnport_departure(actor, session).await {
            return None;
        }
        let track = self.states.last_positions.tracked(session)?;
        if track.actor != actor {
            return None;
        }
        match self.capture_last_position(&track, session).await {
            Captured::At(capture) if capture.pending_vocation_refill.is_none() => Some(FinalLastPosition {
                record: capture.record,
                write_sequence: capture.write_sequence,
            }),
            _ => None,
        }
    }

    /// The release store of `session`: `store` carrying the final position of a tracked
    /// session's fenced actor, or `store` itself.
    pub(super) async fn final_position_store(
        &self,
        store: &FreshAdmissionStore,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> FreshAdmissionStore {
        let store = if self
            .runtime
            .lock()
            .await
            .house_actor_state(actor, session)
            .is_ok()
        {
            store.with_house_release_owner(self.holder)
        } else {
            store.clone()
        };
        let Some(track) = self
            .states
            .last_positions
            .tracked(session)
            .filter(|track| track.actor == actor)
        else {
            return store;
        };
        let capture = self
            .capture_position(&track, session, VocationCapture::Final)
            .await;
        match capture {
            Captured::At(capture) => {
                let position = FinalLastPosition {
                    record: capture.record,
                    write_sequence: capture.write_sequence,
                };
                match (capture.house, capture.pending_vocation_refill) {
                    (Some(house), _) => store.with_final_house_position(position, house, self.holder),
                    (None, Some(occurrence)) => store.with_final_position_pending_refill(position, occurrence),
                    (None, None) => store.with_final_position(position),
                }
            }
            Captured::Dead | Captured::Held | Captured::Vocation(_) | Captured::Gone => store.clone(),
        }
    }

    /// DEATH-2b with CHAR-POSITION-0 §3.3 step 1: the record the respawn consumption upserts for
    /// the placed cell; the session is tracked once the consumption settles.
    pub(super) async fn respawn_last_position(
        &self,
        actor: ExactActorRef,
    ) -> Option<FinalLastPosition> {
        let runtime = self.runtime.lock().await;
        let snapshot = runtime.read_actor_position(actor).ok()?;
        Some(self.placed_record(&runtime, snapshot.position()))
    }

    /// Track `session` after its respawn consumption under `fence` wrote `placed`.
    pub(super) fn track_respawned(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        placed: &FinalLastPosition,
        fence: CurrentCharacterGameplayFence,
    ) {
        self.states.last_positions.track(
            session,
            actor,
            Some(Saved::of(&placed.record)),
            fence.expected_character_revision,
        );
    }
    /// §3.3: the durable position of a new runtime actor, read under `fence` before the
    /// first-entry owner step: a pending respawn (step 1), else the last position (steps 3 and
    /// 4). `Err` for a failed read (a storage or fence error): the actor is left unpositioned
    /// and the admission is not playable.
    pub(super) async fn read_admission_position(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<AdmissionPosition, ()> {
        if let Some(coordinator) = self.premium_coordinator {
            // First entry has not registered its session's Premium account yet.
            // Settle deployment delivery before borrowing the bed transaction's
            // holder; that transaction resolves the account under its own fence.
            coordinator
                .with_premium_batch_gate(&[], |_, _| Box::pin(async {}))
                .await
                .map_err(|_| ())?;
        }
        let premium: Option<&dyn BedPremiumGate> = self.premium_coordinator.map(|gate| gate as _);
        self.root
            .settle_bed_regen(self.character, self.holder, fence, premium)
            .await
            .map_err(|_| ())?;
        match self
            .root
            .pending_respawn(self.character, self.holder, fence)
            .await
        {
            Ok(Some(pending)) => return Ok(AdmissionPosition::Respawn(pending)),
            Ok(None) => {}
            Err(error) => {
                operator_event(&format!("pending_respawn_read_failed reason={error}"));
                return Err(());
            }
        }
        match self
            .root
            .read_character_last_position(self.character, self.holder, fence)
            .await
        {
            Ok(Some(stored)) => Ok(AdmissionPosition::Stored(stored)),
            Ok(None) => Ok(AdmissionPosition::FirstLogin),
            Err(error) => {
                operator_event(&format!("last_position_read_failed reason={error}"));
                Err(())
            }
        }
    }

    /// §3.3 in the first-entry owner step, under the Channel-owner lock the caller holds:
    /// initialize `actor` and place it at the cell `decided` selects
    /// ([`initialize_first_entry_at`]), so every owner that reads its position after this step
    /// sees the placed cell.
    pub(super) async fn initialize_admitted_position(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        decided: &AdmissionPosition,
    ) -> Result<FirstEntryPosition, CarrierError> {
        let (pinned_digest, frame_digest) = {
            let pin = runtime.content_pin();
            (pin.server_artifact_digest(), pin.frame_binding_digest())
        };
        let input = first_entry_input(decided, self.world_id, frame_digest, |cell| {
            house_cell(self.movement_cells(), cell)
        });
        // A respawn's fallback is DEATH-2b's; a stored cell's fallback never lands on a house
        // tile either.
        let stored = matches!(decided, AdmissionPosition::Stored(_));
        let door = self.door.lock().await;
        initialize_first_entry_at(runtime, actor, session, input, |cell| {
            respawn_cell_admissible(
                self.movement_cells().index(),
                self.movement_cells().scope(),
                self.world_id,
                pinned_digest,
                door.blocking_cells(),
                cell,
            ) && !(stored && house_cell(self.movement_cells(), cell))
        })
    }

    /// §3.3 steps 3 and 4, once no pending respawn is left to consume: track `session` with the
    /// stored cell and vitals its first entry was decided by. `false` when that entry was decided
    /// by a pending respawn that is no longer pending: the admission is not playable, nothing is
    /// written and the session stays untracked, so it never overwrites the stored row.
    pub(super) fn track_restored(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        decided: &AdmissionPosition,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        let written = match decided {
            AdmissionPosition::Stored(stored) => Some(Saved::of(&stored.record)),
            AdmissionPosition::FirstLogin | AdmissionPosition::Relocated(_, _, _) => None,
            AdmissionPosition::Respawn(_) | AdmissionPosition::Arrival(_, _, _) => {
                operator_event("admitted_respawn_no_longer_pending");
                return false;
            }
        };
        self.states.last_positions.track(
            session,
            actor,
            written,
            fence.expected_character_revision,
        );
        true
    }
}
