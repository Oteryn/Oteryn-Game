use super::*;
use crate::monster_owner_cycle::respawn_job::RespawnExit;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The already running boss cadence sweeps at most the thirty qualified room visits.
    /// The database clock decides expiry; owner replacement also returns abandoned visits.
    pub(in crate::gameplay_transport) async fn grizzly_boss_pass(
        &self,
    ) -> Result<bool, RespawnExit> {
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(false);
        };
        let retries = self
            .states
            .grizzly_boss
            .returns
            .lock()
            .map(|entries| {
                entries
                    .iter()
                    .map(|(session, (actor, entry))| (*session, *actor, entry.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut complete = true;
        for (session, actor, entry) in retries {
            // A prior expiry COMMIT may have succeeded without its response. The
            // remembered predecessor is only a retry obligation, never return authority.
            let reconciled = match self
                .root
                .pending_grizzly_boss_entry(self.character, entry.character_id)
                .await
            {
                Ok(Some(current)) if current.cause == entry.cause => {
                    current.state == GrizzlyBossEntryState::ReturnPending
                        && self
                            .return_grizzly_visit(&permit, actor, session, &current)
                            .await
                }
                Ok(_) => true,
                Err(_) => false,
            };
            if reconciled && let Ok(mut entries) = self.states.grizzly_boss.returns.lock() {
                entries.remove(&session);
            } else if !reconciled {
                complete = false;
            }
        }
        let binding = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
            runtime.binding()
        };
        let entries = match self
            .root
            .active_grizzly_boss_entries(
                self.character,
                self.holder,
                binding.world_id(),
                binding.channel_id(),
                binding.scope_generation(),
            )
            .await
        {
            Ok(entries) => entries,
            Err(CharacterProgressionError::AuthorityRejected) => {
                return Err(RespawnExit::StaleOwner);
            }
            Err(_) => return Ok(false),
        };
        for entry in entries {
            let timed = entry.expired || entry.scope_generation != binding.scope_generation();
            if !timed
                && (self.grizzly_visit_is_pending(&entry)
                    || self.grizzly_room_occupied(&entry).await?)
            {
                continue;
            }
            if !self.retire_grizzly_visit(&permit, &entry, timed).await? {
                complete = false;
            }
        }
        Ok(complete)
    }

    /// Checked at fresh admission/resume and before input, through the existing callback
    /// reconciliation hook. A remembered negative read is only a session-local optimization.
    pub(in crate::gameplay_transport) async fn reconcile_grizzly_boss(
        &self,
        session: GameSessionId,
    ) -> bool {
        let pending = match self.states.grizzly_boss.pending.lock() {
            Ok(pending) => pending.get(&session).cloned(),
            Err(_) => return false,
        };
        if let Some(pending) = pending {
            let Some(permit) = self.spell_lane_permit().await else {
                return false;
            };
            if !self.execute_grizzly_boss(&permit, &pending).await {
                return false;
            }
        }
        if self
            .states
            .grizzly_boss
            .checked
            .lock()
            .is_ok_and(|checked| checked.contains(&session))
        {
            return true;
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let entry = match self
            .root
            .pending_grizzly_boss_entry(self.character, fence.character_id)
            .await
        {
            Ok(entry) => entry,
            Err(_) => return false,
        };
        if let Some(mut entry) = entry {
            let (actor, binding) = {
                let runtime = self.runtime.lock().await;
                let Ok(census) = runtime.positioned_actor_census() else {
                    return false;
                };
                let Some((actor, _, _)) = census
                    .into_iter()
                    .find(|(_, _, current)| *current == Some(session))
                else {
                    return false;
                };
                (actor, runtime.binding())
            };
            if entry.state == GrizzlyBossEntryState::Active
                && (entry.expired || entry.scope_generation != binding.scope_generation())
                && entry.world_id == binding.world_id()
                && entry.channel_id == binding.channel_id()
            {
                if !matches!(self.grizzly_boss_pass().await, Ok(true)) {
                    return false;
                }
                let current = match self
                    .root
                    .pending_grizzly_boss_entry(self.character, fence.character_id)
                    .await
                {
                    Ok(Some(current)) => current,
                    Ok(None) => return self.remember_grizzly_session(session),
                    Err(_) => return false,
                };
                entry = current;
            }
            if entry.state == GrizzlyBossEntryState::ReturnPending {
                let Some(permit) = self.spell_lane_permit().await else {
                    return false;
                };
                if !self
                    .return_grizzly_visit(&permit, actor, session, &entry)
                    .await
                {
                    return false;
                }
            }
        }
        self.remember_grizzly_session(session)
    }

    fn remember_grizzly_session(&self, session: GameSessionId) -> bool {
        self.states
            .grizzly_boss
            .checked
            .lock()
            .is_ok_and(|mut checked| {
                checked.insert(session);
                true
            })
    }

    /// Retain before asking the database to expire a visit: an unknown COMMIT must
    /// remain recoverable even when the ACTIVE-only timer query no longer returns it.
    pub(in crate::gameplay_transport) fn remember_grizzly_return(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entry: &GrizzlyBossEntry,
    ) {
        if let Ok(mut entries) = self.states.grizzly_boss.returns.lock() {
            entries.insert(session, (actor, entry.clone()));
        }
        if let Ok(mut checked) = self.states.grizzly_boss.checked.lock() {
            checked.remove(&session);
        }
    }

    pub(super) async fn return_grizzly_visit(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entry: &GrizzlyBossEntry,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        if fence.character_id != entry.character_id {
            return false;
        }
        let Some(room) = room(entry.boss_id) else {
            return false;
        };
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
            || runtime.owner_fence().is_err()
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
        {
            return false;
        }
        let Ok(position) = runtime.read_actor_position(actor) else {
            return false;
        };
        if runtime.binding().world_id() == entry.world_id
            && room.contains(position.position())
            && !self.grizzly_return(
                &mut runtime,
                &mut states,
                map,
                actor,
                session,
                entry.exit,
                optional_pvp,
            )
        {
            return false;
        }
        drop(states);
        drop(runtime);
        // Persist through the existing LastPosition owner in the same transaction that
        // closes this return obligation, so an immediate restart cannot restore the room.
        let Some(position) = self.final_last_position(session, actor).await else {
            return false;
        };
        let closed = self
            .root
            .close_grizzly_boss_entry(self.character, self.holder, fence, entry.cause, position)
            .await;
        let acknowledged = match closed {
            Ok(true) => true,
            Ok(false) => self
                .root
                .pending_grizzly_boss_entry(self.character, fence.character_id)
                .await
                .is_ok_and(|pending| pending.is_none()),
            Err(_) => false,
        };
        if acknowledged {
            if let Ok(mut entries) = self.states.grizzly_boss.returns.lock() {
                entries.remove(&session);
            }
            self.states.chat.push_quest_feedback(
                session,
                "Your boss room time has ended. Report the encounter to Grizzly Adams.",
            );
            true
        } else {
            false
        }
    }
}
