//! One source callback successor over the existing spell staging data clone.
use super::super::runtime_actor_companion::native_birth::PreparedNativeCreatureBirth;
use super::super::soul_war_mirror_runtime::PreparedMirrorReaction;
use super::*;

#[derive(Debug, Clone, Default)]
pub(super) struct MirrorBatchDelta {
    reactions: Vec<PreparedMirrorReaction>,
    spares: Vec<PreparedNativeCreatureBirth>,
    born: Vec<ExactActorRef>,
    vacated: Vec<usize>,
}
impl MirrorBatchDelta {
    pub(super) fn changed(&self) -> bool {
        !self.reactions.is_empty()
    }
    pub(super) fn require_reserved(&self, owner: &ChannelRuntimeV1) -> Result<(), Error> {
        for spare in &self.spares {
            let actor = spare.actor();
            let index = actor.actor_local_id() as usize - 1;
            if !matches!(&owner.carrier.slots[index],Slot::CreatureReserved{generation,..} if *generation==actor.0.actor_local_generation.0)
            {
                return Err(Error::SnapshotChanged);
            }
        }
        Ok(())
    }
    pub(super) fn retired(&self, actor: ExactActorRef) -> bool {
        self.reactions
            .iter()
            .any(|reaction| reaction.target == actor)
    }
    pub(super) fn try_stage(
        &mut self,
        owner: &ChannelRuntimeV1,
        batch: &OwnerCombatBatch,
        effect: &OwnerCombatEffect,
        next: &mut ChannelActorCarrier,
        binding: &[u8],
    ) -> Result<Option<OwnerDamageResult>, Error> {
        let index = next.validate_ref(&owner.continuity, effect.target.0)?;
        if !matches!(&next.slots[index], Slot::CreatureOccupied { spell_combat, .. }
            if spell_combat.soul_war_health.as_deref().is_some_and(|p| p.kind() == oteryn_gameplay_content::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::MirrorImage))
        {
            return Ok(None);
        }
        let (atom, valid) = match &effect.change {
            OwnerCombatChange::Damage {
                target_atom,
                magnitude,
            }
            | OwnerCombatChange::Heal {
                target_atom,
                magnitude,
            } => (target_atom.as_bytes(), *magnitude > 0),
            OwnerCombatChange::DamageWithHealing {
                target_atom,
                damage,
                healing,
            } => {
                // Absorption heals through combatChangeHealth(nullptr, ...).
                // Only a positive residual retains the original Player event.
                if *damage == 0 && *healing > 0 {
                    return Ok(None);
                }
                (target_atom.as_bytes(), *damage > 0 && *healing >= 0)
            }
            _ => return Ok(None),
        };
        if !valid {
            return Err(Error::InvalidMagnitude);
        }
        let source = owner.carrier.soul_war_health_context(
            &owner.continuity,
            Some(batch.caster),
            Some(batch.command),
            Some(batch.now_ms),
            super::super::soul_war_player_source::SoulWarPlayerSourceOrigin::Combat,
            &owner.content,
        )?;
        let Some(reaction) = next.prepare_source_mirror_reaction(
            &owner.continuity,
            owner.pinned_position_context(),
            effect.target,
            batch.caster,
            batch.command.game_session_id(),
            Some(&source),
            binding,
        )?
        else {
            return Ok(None);
        };
        let index = next.validate_ref(&owner.continuity, effect.target.0)?;
        if !matches!(&next.slots[index],Slot::CreatureOccupied{target_identity,..} if target_identity.as_ref()==atom)
        {
            return Err(Error::InvalidBatch);
        }
        self.reactions
            .try_reserve(1)
            .map_err(|_| Error::AllocationFailed)?;
        if let Some(birth) = &reaction.birth {
            self.born
                .try_reserve(1)
                .map_err(|_| Error::AllocationFailed)?;
            let at = birth.actor().actor_local_id() as usize - 1;
            if matches!(&owner.carrier.slots[at], Slot::VacantReusable { .. }) {
                self.spares
                    .try_reserve(1)
                    .map_err(|_| Error::AllocationFailed)?;
                self.spares.push(birth.clone());
            } else if !self.retired(ExactActorRef(ActorRef::from_scoped_owner_parts(
                owner.binding.runtime_scope,
                owner.binding.scope_generation,
                ActorLocalId(u32::try_from(at + 1).map_err(|_| Error::InvalidBatch)?),
                ActorLocalGeneration(
                    birth
                        .actor()
                        .0
                        .actor_local_generation
                        .0
                        .checked_sub(1)
                        .ok_or(Error::InvalidBatch)?,
                ),
            ))) {
                return Err(Error::InvalidBatch);
            }
            self.born.push(birth.actor());
        }
        let result = next.apply_source_mirror_reaction(&owner.continuity, reaction.clone())?;
        self.reactions.push(reaction);
        Ok(Some(result))
    }
    pub(super) fn finish(&mut self, next: &ChannelActorCarrier) -> Result<(), Error> {
        self.vacated
            .try_reserve(self.reactions.len())
            .map_err(|_| Error::AllocationFailed)?;
        for reaction in &self.reactions {
            let index = reaction.target.actor_local_id() as usize - 1;
            if matches!(next.slots[index], Slot::VacantReusable { .. })
                && !self.vacated.contains(&index)
            {
                self.vacated.push(index);
            }
        }
        Ok(())
    }
    pub(super) fn add_spare_replacements(
        &self,
        next: &ChannelActorCarrier,
        replacements: &mut Vec<(usize, Slot)>,
    ) -> Result<(), Error> {
        replacements
            .try_reserve(self.spares.len())
            .map_err(|_| Error::AllocationFailed)?;
        for spare in &self.spares {
            let index = spare.actor().actor_local_id() as usize - 1;
            if replacements.iter().any(|(at, _)| *at == index) {
                return Err(Error::InvalidBatch);
            }
            replacements.push((index, next.slots[index].clone()));
        }
        Ok(())
    }
    pub(super) fn validate(
        &self,
        owner: &ChannelRuntimeV1,
        released: &[usize],
        relocation: bool,
    ) -> Result<(), Error> {
        for spare in &self.spares {
            owner.validate_native_creature_birth(spare)?;
        }
        for reaction in &self.reactions {
            owner
                .carrier
                .check_remove(&owner.continuity, reaction.target.0, released)?;
            if owner.carrier.native_summons.has_children(reaction.target)
                || owner
                    .carrier
                    .native_summons
                    .child(reaction.target)
                    .is_some()
            {
                return Err(Error::InvalidBatch);
            }
        }
        let entries = u64::try_from(self.born.len())
            .map_err(|_| Error::InvalidBatch)?
            .checked_add(u64::from(relocation))
            .ok_or(Error::InvalidBatch)?;
        owner
            .carrier
            .tile_entry_counter
            .checked_add(entries)
            .ok_or(Error::Owner(CarrierError::PositionRevisionExhausted))?;
        Ok(())
    }
    #[allow(
        clippy::expect_used,
        reason = "held exact preflight makes all physical reservations deterministic before SQL"
    )]
    pub(super) fn reserve(&mut self, owner: &mut ChannelRuntimeV1) {
        for spare in &mut self.spares {
            owner
                .reserve_native_creature_birth(spare)
                .expect("prevalidated source spare reservation");
        }
    }
    #[allow(
        clippy::expect_used,
        reason = "only definite rollback after exact held reservation comparison may free a source spare"
    )]
    pub(super) fn release(&self, owner: &mut ChannelRuntimeV1) {
        for spare in &self.spares {
            owner
                .rollback_native_creature_birth(spare)
                .expect("prevalidated source spare rollback");
        }
    }
    #[allow(
        clippy::expect_used,
        reason = "all native slot indices were independently validated before the atomic publication"
    )]
    pub(super) fn install(&self, carrier: &mut ChannelActorCarrier) {
        for index in &self.vacated {
            let Slot::VacantReusable { next_free, .. } = &mut carrier.slots[*index] else {
                unreachable!("source delta final vacant slot");
            };
            *next_free = carrier.free_head;
            carrier.free_head =
                Some(u32::try_from(*index).expect("already allocated fixed slot index"));
            carrier.unindex_slot(*index);
        }
        for born in &self.born {
            carrier.install_next_tile_entry_order(born.actor_local_id() as usize - 1);
        }
    }
}
