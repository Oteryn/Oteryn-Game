//! The actual familiar receives the blast; its current master only qualifies membership.
use super::*;
use sha2::{Digest, Sha256};
impl ChannelRuntimeV1 {
    pub fn commit_tibiadrome_familiar_death_blast(
        &mut self,
        actor: ExactActorRef,
        dying: ExactActorRef,
        ordinal: u64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.owner_fence()?;
        let familiar = self.companion_snapshot(actor)?;
        if !familiar.state.policy.is_familiar {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let master = familiar
            .state
            .master
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        self.player_control_facts(master.actor, master.session)?;
        let (run, permitted) = self
            .drome_actor_modifiers(master.actor)
            .map(|policy| {
                (
                    policy.run,
                    policy.selected.contains(&DromeModifier::GoingDownWithMe),
                )
            })
            .ok_or(CarrierError::InvalidDamage)?;
        if !permitted
            || !self.is_tibiadrome(master.actor)
            || self
                .drome_actor_modifiers(dying)
                .is_none_or(|policy| policy.run != run)
            || self.tibiadrome_last_hitter(dying) != Some(actor)
        {
            return Err(CarrierError::InvalidDamage);
        }
        self.borrow_combat_death().projected_death(dying)?;
        if self
            .carrier
            .encounter_hooks
            .as_ref()
            .is_some_and(|hooks| hooks.contains(actor))
        {
            return Err(CarrierError::PositionContextMismatch);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let key: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:drome:familiar-health:v1")
            .chain_update(run)
            .chain_update(actor.placement_identity())
            .chain_update(actor.actor_local_generation().to_be_bytes())
            .finalize()
            .into();
        let occurrence = format!("drome-familiar:{key:x?}:{ordinal}");
        let mut binding = occurrence.as_bytes().to_vec();
        binding.push(0);
        binding.extend_from_slice(&dying.placement_identity());
        binding.extend_from_slice(&dying.actor_local_generation().to_be_bytes());
        let Slot::CreatureOccupied { committed, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if committed
            .reflection_marks
            .get(&key)
            .is_some_and(|mark| ordinal <= *mark)
        {
            let record = committed
                .entries
                .iter()
                .find(|record| record.reflection == Some((key, ordinal)))
                .ok_or(CarrierError::StaleAttackerSequence)?;
            if record.binding.as_ref() != binding.as_slice() {
                return Err(CarrierError::PlanConflict);
            }
            return Ok(OwnerDamageResult {
                applied: false,
                ..record.result
            });
        }
        if !committed.reflection_marks.contains_key(&key)
            && committed.reflection_marks.len()
                >= crate::carrier::equipment_reflection::REFLECTION_STREAMS_MAX
        {
            return Err(CarrierError::DamageReceiptCapacityExceeded);
        }
        let damage = i64::try_from(i128::from(familiar.health) * 40 / 100)
            .map_err(|_| CarrierError::DamageOverflow)?;
        if damage == 0 {
            // A settled zero fraction must remain settled if a later heal raises HP.
            let Slot::CreatureOccupied { committed, .. } = &mut self.carrier.slots[index] else {
                return Err(CarrierError::NotCreature);
            };
            committed.reflection_marks.insert(key, ordinal);
            return Ok(OwnerDamageResult {
                applied: false,
                health_before: familiar.health,
                health_after: familiar.health,
            });
        }
        // Reuse canonical HP preflight/publication and its bounded source replay marks.
        // No player damage contribution is attributed for this environmental health loss.
        let (index, next, result) = self.carrier.prepare_creature_damage_inner_bounded(
            &self.continuity,
            actor.0,
            OwnerDamageCommand {
                target: familiar.state.policy.definition_key.as_bytes(),
                occurrence: occurrence.as_bytes(),
                binding: &binding,
                damage,
            },
            None,
            false,
            None,
            false,
            None,
        )?;
        let Some(mut next) = next else {
            return Ok(result);
        };
        let Slot::CreatureOccupied { committed, .. } = &mut next else {
            return Err(CarrierError::NotCreature);
        };
        let record = committed
            .entries
            .last_mut()
            .ok_or(CarrierError::PlanConflict)?;
        record.reflection = Some((key, ordinal));
        committed.reflection_marks.insert(key, ordinal);
        self.carrier.slots[index] = next;
        Ok(result)
    }
}
