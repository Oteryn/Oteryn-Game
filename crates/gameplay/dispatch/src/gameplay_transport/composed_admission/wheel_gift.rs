use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn load_wheel_gift(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let Some(state) = states.get(&runtime, actor, session) else {
                return false;
            };
            if state.wheel_gift_loaded() {
                return true;
            }
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let Ok(remaining) = self
            .root
            .read_character_wheel_gift(self.character, fence.character_id)
            .await
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return false;
        };
        if state.wheel_gift_loaded() {
            return true;
        }
        if !state.restore_wheel_gift(remaining, self.owner_now().get()) {
            return false;
        }
        state.install_wheel_gift_spellbook(self.spells);
        true
    }

    /// A checkpoint and actor-end save use the same current fence and immutable retry identity.
    /// False holds lease release; a stale generation is terminal and cannot write.
    pub(in crate::gameplay_transport) async fn save_wheel_gift(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> bool {
        use crate::durability::character_progression::CharacterProgressionError as Error;
        // No DB work for an actor that did not activate the perk since its last checkpoint.
        let pending = {
            let runtime = self.runtime.lock().await;
            self.states
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .and_then(|state| state.wheel_gift_save(self.owner_now().get(), force))
        };
        if pending.is_none() {
            return true;
        }
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => return true,
            Err(_) => return false,
        };
        let _slot = self.revision_sequencer.acquire(fence.character_id).await;
        let snapshot = {
            let runtime = self.runtime.lock().await;
            self.states
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .and_then(|state| state.wheel_gift_save(self.owner_now().get(), force))
        };
        let Some((generation, remaining)) = snapshot else {
            return true;
        };
        let Some(occurrence) = SecureIdentifiers::random::<16>().and_then(|mut bytes| {
            bytes[6] = 0x70 | (bytes[6] & 0x0f);
            bytes[8] = 0x80 | (bytes[8] & 0x3f);
            crate::durability::character_wheel::WheelOccurrence::from_bytes(bytes).ok()
        }) else {
            return false;
        };
        for attempt in 0..RECONCILE_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            }
            match self
                .root
                .save_character_wheel_gift(
                    self.character,
                    self.holder,
                    fence.item_fence(),
                    occurrence,
                    remaining,
                )
                .await
            {
                Ok(()) => {
                    let runtime = self.runtime.lock().await;
                    if let Some(state) = self
                        .states
                        .spell_states
                        .lock()
                        .await
                        .get_mut(&runtime, actor, session)
                    {
                        state.wheel_gift_saved(generation);
                    }
                    return true;
                }
                Err(Error::AuthorityRejected) => return true,
                Err(Error::Unavailable(_)) => {}
                Err(_) => return false,
            }
        }
        false
    }

    pub(in crate::gameplay_transport) async fn flush_wheel_gifts(
        &self,
        settled: &mut std::collections::HashSet<GameSessionId>,
    ) -> usize {
        let mut failed = 0;
        for (session, actor) in self.states.last_positions.tracked_actors() {
            if settled.contains(&session) {
                continue;
            }
            if self.save_wheel_gift(actor, session, true).await {
                settled.insert(session);
            } else {
                failed += 1;
            }
        }
        failed
    }
}
