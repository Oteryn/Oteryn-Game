use oteryn_gameplay_entities::carrier::Slot;
use oteryn_gameplay_entities::*;
pub trait RumBarrelCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn commit_rum_barrel_death(
        &mut self,
        ledger: &mut RumBarrelDeathLedger,
        barrel: ExactActorRef,
        server_draw: i64,
    ) -> Result<RumBarrelDeathResult, CarrierError>;
}
// Append in native physical runtime_actor_carrier. Conservative unambiguous topCreature subset.
#[derive(Debug, Default)]
pub struct RumBarrelDeathLedger {
    entries: Vec<RumBarrelDeathEntry>,
}
#[derive(Debug)]
struct RumBarrelDeathEntry {
    death: CreatureDeathOccurrenceKey,
    draw: i64,
    targets: Vec<(ExactActorRef, Option<OwnerDamageResult>)>,
}
#[derive(Debug, Clone)]
pub struct RumBarrelDeathResult {
    pub newly_committed: bool,
    pub targets: Vec<(ExactActorRef, OwnerDamageResult)>,
}
impl RumBarrelCarrierExt for ChannelRuntimeV1 {
    /// Source Lua draws ONCE per death, shared across all 3x3 targets, 66666..88888 lifedrain.
    /// `server_draw` must be supplied by the current server RNG owner, never network input.
    /// Same-tile multi-actor stack authority is unavailable: ambiguous tiles are skipped and flagged.
    fn commit_rum_barrel_death(
        &mut self,
        ledger: &mut RumBarrelDeathLedger,
        barrel: ExactActorRef,
        server_draw: i64,
    ) -> Result<RumBarrelDeathResult, CarrierError> {
        if !(66666..=88888).contains(&server_draw) {
            return Err(CarrierError::InvalidDamage);
        }
        let owner = self.borrow_combat_death();
        if owner.creature_target_identity(barrel)? != b"oteryn:creature.rum_barrel" {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let (death, center) = owner.projected_death(barrel)?;
        ledger
            .entries
            .retain(|e| self.encounter_death_is_current(e.death));
        let existed = ledger.entries.iter().any(|e| e.death == death);
        let entry = if let Some(index) = ledger.entries.iter().position(|e| e.death == death) {
            if ledger.entries[index].draw != server_draw {
                return Err(CarrierError::PlanConflict);
            }
            index
        } else {
            let context = self.encounter_position_context();
            let mut targets = Vec::new();
            targets
                .try_reserve(9)
                .map_err(|_| CarrierError::AllocationFailed)?;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let mut count = 0;
                    let mut target = None;
                    for (i, slot) in self.encounter_slots().iter().enumerate() {
                        let position = match slot {
                            Slot::Occupied {
                                position: Some(p), ..
                            }
                            | Slot::CreatureOccupied {
                                position: Some(p),
                                health: 1..,
                                ..
                            } => p,
                            _ => continue,
                        };
                        if position.context != context
                            || position.position.floor != center.floor
                            || i64::from(position.position.x) != i64::from(center.x) + dx
                            || i64::from(position.position.y) != i64::from(center.y) + dy
                        {
                            continue;
                        }
                        count += 1;
                        if let Slot::CreatureOccupied {
                            generation,
                            target_identity,
                            ..
                        } = slot
                            && target_identity.as_ref() == b"oteryn:creature.weak_spot"
                        {
                            target = Some(ExactActorRef(ActorRef::from_owner_parts(
                                self.binding().world_id(),
                                self.binding().channel_id(),
                                self.binding().scope_generation(),
                                ActorLocalId(
                                    u32::try_from(i + 1)
                                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                                ),
                                ActorLocalGeneration(*generation),
                            )));
                        }
                    }
                    if count == 1
                        && let Some(target) = target
                    {
                        targets.push((target, None));
                    }
                }
            }
            ledger
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            ledger.entries.push(RumBarrelDeathEntry {
                death,
                draw: server_draw,
                targets,
            });
            ledger.entries.len() - 1
        };
        let newly_committed = !existed
            || ledger.entries[entry]
                .targets
                .iter()
                .any(|(_, result)| result.is_none());
        // Once-per-death producer fixes the target census; retries cannot damage replacement actors.
        // Native per-target receipt remains authoritative if a later allocation fails mid callback.
        let mut completed = Vec::new();
        completed
            .try_reserve_exact(ledger.entries[entry].targets.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (target, result) in &mut ledger.entries[entry].targets {
            let committed = if let Some(committed) = *result {
                committed
            } else {
                let occurrence = format!(
                    "rum:{}:{}:{}",
                    death.actor_local_id(),
                    death.actor_local_generation(),
                    target.0.actor_local_id.0
                );
                let binding = format!("{occurrence}\0crystal-rum-lifedrain:{server_draw}");
                let committed = self.borrow_exact_actor_commit().commit_damage(
                    *target,
                    OwnerDamageCommand {
                        target: b"oteryn:creature.weak_spot",
                        occurrence: occurrence.as_bytes(),
                        binding: binding.as_bytes(),
                        damage: server_draw,
                    },
                )?;
                *result = Some(committed);
                committed
            };
            // Result storage was reserved before the first target HP write.
            completed.push((*target, committed));
        }
        Ok(RumBarrelDeathResult {
            newly_committed,
            targets: completed,
        })
    }
}

#[cfg(test)]
#[path = "native_death_tests.rs"]
mod tests;
