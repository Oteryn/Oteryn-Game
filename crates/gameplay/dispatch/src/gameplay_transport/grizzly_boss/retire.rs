//! Q10=B: clear a proven empty room immediately through its existing native owner.
//! The exact ACTIVE visit is re-read before physical cleanup, so an old timer or
//! retry can never remove the creature belonging to a subsequent room visit.
use super::*;
use crate::monster_owner_cycle::respawn_job::RespawnExit;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The entry's own committed-but-not-yet-installed receipt is not an early exit.
    /// Every pending source attempt stays protected until physical realization or
    /// until its original deadline expires; restart is covered by owner generation.
    pub(super) fn grizzly_visit_is_pending(&self, entry: &GrizzlyBossEntry) -> bool {
        self.states
            .grizzly_boss
            .pending
            .lock()
            .map_or(true, |pending| {
                pending
                    .values()
                    .any(|pending| pending.cause == QuestCause::Step(entry.cause))
            })
    }

    pub(super) async fn grizzly_room_occupied(
        &self,
        entry: &GrizzlyBossEntry,
    ) -> Result<bool, RespawnExit> {
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
        let Some(room) = room(entry.boss_id) else {
            return Ok(true);
        };
        Ok(runtime.positioned_melee_census().map_or(true, |census| {
            census.iter().any(|(_, position, session)| {
                session.is_some() && room.contains(position.position())
            })
        }))
    }

    pub(super) async fn reclaim_grizzly_entry_rooms(
        &self,
        permit: &SpellLanePermit,
        pending: &Pending,
        character: crate::domain::CharacterId,
    ) -> bool {
        let binding = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return false;
            }
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
            Err(_) => return false,
        };
        for entry in entries.into_iter().filter(|entry| {
            entry.character_id == character || entry.boss_id == pending.context.boss_id
        }) {
            if pending.cause == QuestCause::Step(entry.cause) {
                continue;
            }
            let timed = entry.expired || entry.scope_generation != binding.scope_generation();
            // An occupied destination is a normal portal refusal, decided below by
            // its physical census. Unknown retirement keeps the original Step pending.
            if !timed
                && (self.grizzly_visit_is_pending(&entry)
                    || !matches!(self.grizzly_room_occupied(&entry).await, Ok(false)))
            {
                continue;
            }
            if !matches!(
                self.retire_grizzly_visit(permit, &entry, timed).await,
                Ok(true)
            ) {
                return false;
            }
        }
        let old = match self
            .root
            .pending_grizzly_boss_entry(self.character, character)
            .await
        {
            Ok(old) => old,
            Err(_) => return false,
        };
        if let Some(entry) = old
            && entry.state == GrizzlyBossEntryState::ReturnPending
            && !self
                .return_grizzly_visit(permit, pending.actor, pending.session, &entry)
                .await
        {
            return false;
        }
        true
    }

    /// Returns true when the exact visit is retired/already absent, false when it
    /// remains occupied or awaits recovery. Only the current native lane can call it.
    /// `deadline_only` rechecks the database deadline/current owner generation;
    /// otherwise the complete native census must prove there are no room players.
    pub(in crate::gameplay_transport) async fn retire_grizzly_visit(
        &self,
        permit: &SpellLanePermit,
        original: &GrizzlyBossEntry,
        deadline_only: bool,
    ) -> Result<bool, RespawnExit> {
        let Some(room) = room(original.boss_id) else {
            return Ok(false);
        };
        let mut runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
        let binding = runtime.binding();
        if binding.world_id() != original.world_id || binding.channel_id() != original.channel_id {
            return Ok(false);
        }
        let census = match runtime.positioned_melee_census() {
            Ok(census) => census,
            Err(_) => return Ok(false),
        };
        if !deadline_only
            && (self.grizzly_visit_is_pending(original)
                || census.iter().any(|(_, position, session)| {
                    session.is_some() && room.contains(position.position())
                }))
        {
            return Ok(false);
        }
        // Re-read after obtaining the same physical lane/census held through cleanup.
        // A stale old-key retry is a no-op even if this room now holds a newer boss.
        let mut entry = match self
            .root
            .current_active_grizzly_boss_entry(
                self.character,
                self.holder,
                binding.world_id(),
                binding.channel_id(),
                binding.scope_generation(),
                original,
            )
            .await
        {
            Ok(Some(entry)) => entry,
            Ok(None) => return Ok(true),
            Err(CharacterProgressionError::AuthorityRejected) => {
                return Err(RespawnExit::StaleOwner);
            }
            Err(_) => return Ok(false),
        };
        if deadline_only && !entry.expired && entry.scope_generation == binding.scope_generation() {
            return Ok(false);
        }
        let monsters: Vec<_> = census
            .iter()
            .filter(|(_, position, session)| {
                session.is_none() && room.contains(position.position())
            })
            .map(|(actor, _, _)| *actor)
            .collect();
        if !monsters.iter().all(|actor| {
            runtime
                .encounter_actor_index(*actor)
                .is_ok_and(|index| runtime.assert_encounter_slot_unreserved(index).is_ok())
        }) {
            return Ok(false);
        }
        for monster in monsters {
            if runtime.contains_live_creature(monster)
                && runtime.remove_encounter_actor(monster).is_err()
            {
                return Ok(false);
            }
        }
        let players: Vec<_> = census
            .iter()
            .filter_map(|(actor, _, session)| {
                let session = (*session)?;
                runtime
                    .bound_attacker_lease(*actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == entry.character_id.as_bytes()
                    })
                    .then_some((*actor, session))
            })
            .collect();
        for (actor, session) in &players {
            self.remember_grizzly_return(*actor, *session, &entry);
        }
        let retired = if deadline_only {
            self.root
                .expire_grizzly_boss_entry(
                    self.character,
                    self.holder,
                    binding.world_id(),
                    binding.channel_id(),
                    binding.scope_generation(),
                    &entry,
                )
                .await
        } else {
            self.root
                .retire_vacated_grizzly_boss_entry(
                    self.character,
                    self.holder,
                    permit,
                    binding.world_id(),
                    binding.channel_id(),
                    binding.scope_generation(),
                    &entry,
                )
                .await
        };
        match retired {
            Ok(true) => {}
            Err(CharacterProgressionError::AuthorityRejected) => {
                return Err(RespawnExit::StaleOwner);
            }
            _ => return Ok(false),
        }
        entry.state = GrizzlyBossEntryState::ReturnPending;
        drop(runtime);
        let mut complete = true;
        for (actor, session) in players {
            if !self
                .return_grizzly_visit(permit, actor, session, &entry)
                .await
            {
                complete = false;
            }
        }
        Ok(complete)
    }
}
