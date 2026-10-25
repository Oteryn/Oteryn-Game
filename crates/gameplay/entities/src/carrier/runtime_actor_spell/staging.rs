//! Private phases for staging one atomic spell successor on cloned slots.

use super::*;

struct EffectStaging {
    next: ChannelActorCarrier,
    touched: Vec<usize>,
    player_effects: Vec<OwnerCombatEffect>,
    effect_receipts: Vec<EffectReceipt>,
}

impl ChannelRuntimeV1 {
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    pub(super) fn stage_spell_successor(
        &self,
        batch: &OwnerCombatBatch,
        caster_index: usize,
        binding: &[u8],
        phase: u16,
        deferred: bool,
        caster_state: &ActorCombatState,
    ) -> Result<StagedSpellBatch, Error> {
        // Only data is cloned. The original independently current continuity is borrowed by
        // every carrier operation below; no namespace grant/capability is cloned or reissued.
        let mut next = self.carrier.clone();
        state_mut(&mut next.slots[caster_index])?.pending_owner = None;
        let touched = vec![caster_index];
        let mut player_effects = Vec::new();
        let mut effect_receipts = Vec::new();
        player_effects
            .try_reserve(batch.effects.len())
            .map_err(|_| Error::AllocationFailed)?;
        effect_receipts
            .try_reserve(batch.effects.len())
            .map_err(|_| Error::AllocationFailed)?;
        let mut staging = EffectStaging {
            next,
            touched,
            player_effects,
            effect_receipts,
        };
        self.stage_spell_effects(batch, binding, deferred, &mut staging)?;
        let EffectStaging {
            mut next,
            mut touched,
            player_effects,
            effect_receipts,
        } = staging;
        let receipt = CombatBatchReceipt {
            applied: true,
            caster: batch.caster,
            command: batch.command,
            effects: effect_receipts,
        };
        let updated_caster = state_mut(&mut next.slots[caster_index])?;
        updated_caster
            .records
            .try_reserve(1)
            .map_err(|_| Error::AllocationFailed)?;
        if updated_caster.records.len() >= MAX_COMMAND_RECEIPTS {
            updated_caster.records.remove(0);
        }
        updated_caster.records.push(BatchRecord {
            character: batch.attacker,
            lease: batch.current_lease_generation,
            command: batch.command,
            phase,
            original_batch: batch.clone(),
            receipt: receipt.clone(),
        });
        let last = batch.effects.last().map_or(0, |e| e.sub_ordinal);
        let mark = (
            batch.attacker,
            batch.current_lease_generation,
            batch.command.game_session_id(),
            batch.command.command_id().get(),
            last,
        );
        if updated_caster
            .high_water
            .is_none_or(|old| (mark.1, mark.3, mark.4) > (old.1, old.3, old.4))
        {
            updated_caster.high_water = Some(mark);
        }
        if let Some(authority) = &batch.deferred {
            updated_caster.deferred_high_water = authority.admission_sequence();
        }
        touched.sort_unstable();
        let originals = touched
            .iter()
            .map(|i| (*i, self.carrier.slots[*i].clone()))
            .collect();
        let replacements = touched
            .iter()
            .map(|i| {
                let mut slot = next.slots[*i].clone();
                state_mut(&mut slot)
                    .expect("qualified touched actor")
                    .pending_owner = None;
                (*i, slot)
            })
            .collect();
        Ok(StagedSpellBatch {
            reservation: caster_state.pending_owner.clone(),
            batch: batch.clone(),
            originals,
            replacements,
            companion_touch_indices: vec![],
            relocation: None,
            player_preflight_bound: player_effects.is_empty() && batch.anchor.is_none(),
            player_effects,
            receipt,
        })
    }

    fn stage_spell_effects(
        &self,
        batch: &OwnerCombatBatch,
        binding: &[u8],
        deferred: bool,
        staging: &mut EffectStaging,
    ) -> Result<(), Error> {
        let EffectStaging {
            next,
            touched,
            player_effects,
            effect_receipts,
        } = staging;
        let digest: [u8; 32] = Sha256::digest(binding).into();
        for effect in &batch.effects {
            let index = self
                .carrier
                .validate_ref(&self.continuity, effect.target.0)?;
            if !matches!(&self.carrier.slots[index],Slot::Occupied{generation,..}|Slot::CreatureOccupied{generation,..}
                if *generation==effect.target.0.actor_local_generation.0)
            {
                return Err(Error::Owner(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::StaleActorGeneration,
                ));
            }
            if state(&self.carrier.slots[index])?.pending_source.is_some() {
                return Err(Error::SnapshotChanged);
            }
            if state(&self.carrier.slots[index])?
                .pending_owner
                .as_ref()
                .is_some_and(|r| !r.matches(batch, binding))
            {
                return Err(Error::SnapshotChanged);
            }
            // Only the data clone loses its matched own reservation. The actual
            // original remains sealed while raw commit validates this proposed successor.
            state_mut(&mut next.slots[index])?.pending_owner = None;
            if !touched.contains(&index) {
                touched.push(index);
            }
            let mut health_receipt = None;
            if matches!(next.slots[index], Slot::Occupied { .. }) {
                let Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    ..
                } = &next.slots[index]
                else {
                    return Err(Error::InvalidBatch);
                };
                if matches!(effect.change, OwnerCombatChange::MonsterAi(_)) {
                    return Err(Error::InvalidCondition);
                }
                player_effects.push(effect.clone());
            } else {
                health_receipt =
                    self.stage_creature_effect(batch, effect, index, next, &digest, deferred)?;
            }
            effect_receipts.push(EffectReceipt {
                target: effect.target,
                sub_ordinal: effect.sub_ordinal,
                health: health_receipt,
            });
        }
        Ok(())
    }

    fn stage_creature_effect(
        &self,
        batch: &OwnerCombatBatch,
        effect: &OwnerCombatEffect,
        index: usize,
        next: &mut ChannelActorCarrier,
        digest: &[u8; 32],
        deferred: bool,
    ) -> Result<Option<OwnerDamageResult>, Error> {
        let mut health_receipt = None;
        match &effect.change {
            OwnerCombatChange::CompanionMaster(assignment) => {
                if assignment.actor() != effect.target
                    || assignment.owner() != batch.caster
                    || assignment.session() != batch.command.game_session_id()
                    || batch.deferred.is_some()
                {
                    return Err(Error::InvalidCondition);
                }
                self.validate_companion_assignment(assignment)?;
                let Slot::CreatureOccupied {
                    companion: Some(companion),
                    ..
                } = &mut next.slots[index]
                else {
                    return Err(Error::InvalidCondition);
                };
                companion.master = Some(super::runtime_actor_companion::CompanionMaster {
                    actor: assignment.owner(),
                    session: assignment.session(),
                });
            }
            OwnerCombatChange::CompanionConditions(update) => {
                if update.expected.actor != effect.target {
                    return Err(Error::InvalidCondition);
                }
                self.validate_companion_snapshot(&update.expected)?;
                let before = &update.expected.state;
                let after = &update.next;
                if before.policy != after.policy
                    || before.master != after.master
                    || before.creation_speed_delta != after.creation_speed_delta
                    || before.outfit_look_type != after.outfit_look_type
                    || before.object_look_type != after.object_look_type
                    || before.expires_at_unix != after.expires_at_unix
                    || before.lifecycle_epoch != after.lifecycle_epoch
                    || before.party_protection != after.party_protection
                    || before.created_at_us != after.created_at_us
                    || before.familiar_defense != after.familiar_defense
                {
                    return Err(Error::InvalidCondition);
                }
                let Slot::CreatureOccupied {
                    companion: Some(companion),
                    ..
                } = &mut next.slots[index]
                else {
                    return Err(Error::InvalidCondition);
                };
                companion.conditions = after.conditions.clone();
            }
            OwnerCombatChange::Damage {
                target_atom,
                magnitude,
            } => {
                let mut hit_binding = digest.to_vec();
                hit_binding.extend_from_slice(&effect.sub_ordinal.to_be_bytes());
                health_receipt = Some(next.commit_creature_damage_inner_bounded(
                    &self.continuity,
                    effect.target.0,
                    OwnerDamageCommand {
                        target: target_atom.as_bytes(),
                        occurrence: &[],
                        binding: &hit_binding,
                        damage: *magnitude,
                    },
                    Some(AttackerCommand::new(
                        batch.attacker,
                        batch.current_lease_generation,
                        batch.command,
                        effect.sub_ordinal,
                    )),
                    false,
                    MAX_EFFECTS as u16,
                    deferred,
                    Some(batch.now_ms),
                )?);
            }
            OwnerCombatChange::DamageWithHealing {
                target_atom,
                damage,
                healing,
            } => {
                if *damage < 0 || *healing <= 0 {
                    return Err(Error::InvalidMagnitude);
                }
                let before_health;
                {
                    let Slot::CreatureOccupied {
                        target_identity,
                        health,
                        spell_combat,
                        ..
                    } = &mut next.slots[index]
                    else {
                        return Err(Error::InvalidBatch);
                    };
                    if target_identity.as_ref() != target_atom.as_bytes() {
                        return Err(Error::Owner(oteryn_gameplay_domain::domain::carrier_values::CarrierError::CreatureTargetMismatch));
                    }
                    if *health <= 0 || spell_combat.maximum_health <= 0 {
                        return Err(Error::Owner(oteryn_gameplay_domain::domain::carrier_values::CarrierError::CreatureNotActionable));
                    }
                    before_health = *health;
                    *health = health
                        .saturating_add(*healing)
                        .min(spell_combat.maximum_health);
                }
                if *damage > 0 {
                    let mut hit_binding = digest.to_vec();
                    hit_binding.extend_from_slice(&effect.sub_ordinal.to_be_bytes());
                    next.commit_creature_damage_inner_bounded(
                        &self.continuity,
                        effect.target.0,
                        OwnerDamageCommand {
                            target: target_atom.as_bytes(),
                            occurrence: &[],
                            binding: &hit_binding,
                            damage: *damage,
                        },
                        Some(AttackerCommand::new(
                            batch.attacker,
                            batch.current_lease_generation,
                            batch.command,
                            effect.sub_ordinal,
                        )),
                        false,
                        MAX_EFFECTS as u16,
                        deferred,
                        Some(batch.now_ms),
                    )?;
                }
                let Slot::CreatureOccupied { health, .. } = &next.slots[index] else {
                    return Err(Error::InvalidBatch);
                };
                health_receipt = Some(OwnerDamageResult {
                    applied: true,
                    health_before: before_health,
                    health_after: *health,
                });
            }
            OwnerCombatChange::Heal {
                target_atom,
                magnitude,
            } => {
                let Slot::CreatureOccupied {
                    target_identity,
                    health,
                    spell_combat,
                    ..
                } = &mut next.slots[index]
                else {
                    return Err(Error::InvalidBatch);
                };
                if target_identity.as_ref() != target_atom.as_bytes() {
                    return Err(Error::Owner(oteryn_gameplay_domain::domain::carrier_values::CarrierError::CreatureTargetMismatch));
                }
                if *magnitude <= 0 {
                    return Err(Error::InvalidMagnitude);
                }
                if *health <= 0 || spell_combat.maximum_health <= 0 {
                    return Err(Error::Owner(oteryn_gameplay_domain::domain::carrier_values::CarrierError::CreatureNotActionable));
                }
                let before = *health;
                *health = before
                    .saturating_add(*magnitude)
                    .min(spell_combat.maximum_health);
                health_receipt = Some(OwnerDamageResult {
                    applied: true,
                    health_before: before,
                    health_after: *health,
                });
            }
            OwnerCombatChange::MonsterAi(ai) => {
                if !matches!(
                    next.slots[index],
                    Slot::CreatureOccupied { health: 1.., .. }
                ) {
                    return Err(Error::Owner(oteryn_gameplay_domain::domain::carrier_values::CarrierError::CreatureNotActionable));
                }
                if ai.forced_distance.is_none() && ai.challenged_to.is_none() {
                    return Err(Error::InvalidCondition);
                }
                if ai
                    .forced_distance
                    .is_some_and(|(_, expires)| expires <= batch.now_ms)
                {
                    return Err(Error::InvalidCondition);
                }
                if let Some((challenged, expires)) = ai.challenged_to {
                    self.carrier.validate_ref(&self.continuity, challenged.0)?;
                    if expires <= batch.now_ms {
                        return Err(Error::InvalidCondition);
                    }
                }
                let current = state_mut(&mut next.slots[index])?;
                let mut merged = current.monster_ai.clone().unwrap_or(MonsterAiState {
                    forced_distance: None,
                    challenged_to: None,
                });
                if ai.forced_distance.is_some() {
                    merged.forced_distance = ai.forced_distance;
                }
                if ai.challenged_to.is_some() {
                    merged.challenged_to = ai.challenged_to;
                }
                current.monster_ai = Some(merged);
            }
            OwnerCombatChange::DispelParalysis => {
                state_mut(&mut next.slots[index])?.paralysed = false;
            }
            _ => return Err(Error::InvalidCondition),
        }
        Ok(health_receipt)
    }
}
