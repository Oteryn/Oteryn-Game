//! Exact donor Oppressor areas and cadence, composed through canonical actor conditions.
use super::*;
use oteryn_gameplay_domain::domain::condition::{ConditionGate, ConditionValues, StatusKind};
use oteryn_gameplay_domain::domain::soulpit::OppressorSpell;
use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use sha2::{Digest, Sha256};

// Crystal 201189f data/scripts/lib/register_spells.lua:428-451. The asymmetric zero
// cells in ROOT are source data; replacing this matrix by a radius would change targets.
const ROOT: [[u8; 15]; 15] = [
    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0],
    [0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
    [0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0],
    [0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0],
    [0, 0, 0, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
];
const FEAR: [[u8; 5]; 5] = [
    [0, 1, 1, 1, 0],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [0, 1, 1, 1, 0],
];

#[derive(Debug, Clone, Copy)]
pub struct SoulpitOppressorCast {
    pub source: ExactActorRef,
    pub current_target: Option<ExactActorRef>,
    pub now_micros: u64,
    pub draws: [u8; 3],
    pub decision_root: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct SoulpitOppressorTarget {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub conditions: Vec<ConditionValues>,
}

fn area<const N: usize>(
    mask: &[[u8; N]; N],
    source: MovementLocalPosition,
    target: MovementLocalPosition,
) -> bool {
    if source.floor != target.floor {
        return false;
    }
    let center = (N / 2) as i64;
    let x = i64::from(target.x) - i64::from(source.x) + center;
    let y = i64::from(target.y) - i64::from(source.y) + center;
    usize::try_from(y)
        .ok()
        .and_then(|y| mask.get(y))
        .and_then(|row| usize::try_from(x).ok().and_then(|x| row.get(x)))
        .is_some_and(|value| *value != 0)
}

fn definition(value: ConditionValues) -> Result<ConditionDefinition, CarrierError> {
    let key = match value {
        ConditionValues::TimedStatus {
            kind: StatusKind::Rooted,
            ..
        } => "oteryn:condition.soulpit.rooted",
        ConditionValues::TimedStatus {
            kind: StatusKind::Feared,
            ..
        } => "oteryn:condition.soulpit.feared",
        ConditionValues::Gate {
            gate: ConditionGate::Powerless,
            ..
        } => "oteryn:condition.soulpit.powerless",
        ConditionValues::IntenseHex { .. } => "oteryn:condition.soulpit.intense_hex",
        _ => return Err(CarrierError::InvalidCommitBinding),
    };
    ConditionDefinition::new(key, 201189, value).ok_or(CarrierError::InvalidCommitBinding)
}

impl ChannelActorCarrier {
    pub fn next_soulpit_oppressor_due(
        &self,
        continuity: &NamespaceContinuityGuard,
    ) -> Result<Option<u64>, CarrierError> {
        self.require_soulpit_instance(continuity)?;
        Ok(self
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::CreatureOccupied {
                    health: 1..,
                    spell_combat,
                    ..
                } => spell_combat
                    .soulpit
                    .and_then(|tag| tag.boss)
                    .filter(|boss| boss.ability == BossAbility::Oppressor)
                    .map(|boss| boss.next_oppressor_at_micros),
                _ => None,
            })
            .min())
    }

    /// The shared combat owner supplies its current target and current wall/PvE legality.
    /// All canonical condition successors and the caster's cadence commit in one carrier
    /// work item. A repeated call at the old deadline cannot apply or extend them again.
    pub fn drive_soulpit_oppressor(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        cast: SoulpitOppressorCast,
        mut legal_target: impl FnMut(
            ExactActorRef,
            GameSessionId,
            MovementLocalPosition,
            MovementLocalPosition,
        ) -> bool,
        mut attribute_damage_delta: impl FnMut(
            ExactActorRef,
            GameSessionId,
        ) -> Result<i32, CarrierError>,
    ) -> Result<Vec<SoulpitOppressorTarget>, CarrierError> {
        self.require_soulpit_instance(continuity)?;
        let source_index = self.validate_summon_actor(continuity, cast.source)?;
        self.assert_slot_spell_unreserved(source_index)?;
        let Slot::CreatureOccupied {
            health: 1..,
            spell_combat,
            position: Some(position),
            ..
        } = &self.slots[source_index]
        else {
            return Err(CarrierError::CreatureNotActionable);
        };
        let tag = spell_combat
            .soulpit
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        let boss = tag
            .boss
            .filter(|boss| boss.ability == BossAbility::Oppressor)
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if cast.now_micros < boss.next_oppressor_at_micros || cast.current_target.is_none() {
            return Ok(Vec::new());
        }
        self.lookup(
            continuity,
            cast.current_target
                .ok_or(CarrierError::InvalidCreatureTarget)?
                .0,
        )?;
        let source_position = MovementLocalPosition {
            x: position.position.x,
            y: position.position.y,
            floor: position.position.floor,
        };
        let effects = [
            OppressorSpell::RootAndFear,
            OppressorSpell::Powerless,
            OppressorSpell::IntenseHex,
        ];
        let mut passed = [false; 3];
        for index in 0..3 {
            passed[index] = effects[index]
                .triggers(cast.draws[index])
                .map_err(|_| CarrierError::InvalidDamage)?;
        }
        let next_due = cast
            .now_micros
            .checked_add(OPPRESSOR_INTERVAL_MICROS)
            .ok_or(CarrierError::DamageOverflow)?;
        let next_ordinal = boss
            .ordinal
            .checked_add(1)
            .ok_or(CarrierError::DamageOverflow)?;
        let decision_root = GameplayDecisionRoot::from_bytes(cast.decision_root);
        let mut changes = Vec::new();
        let mut outcomes = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            let Slot::Occupied {
                generation,
                committed: true,
                game_session_id: Some(session),
                position: Some(position),
                ..
            } = slot
            else {
                continue;
            };
            let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
                self.runtime_scope,
                self.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(*generation),
            ));
            let target_position = MovementLocalPosition {
                x: position.position.x,
                y: position.position.y,
                floor: position.position.floor,
            };
            if actor == cast.source
                || !legal_target(actor, *session, source_position, target_position)
            {
                continue;
            }
            let mut values = Vec::new();
            if passed[0] {
                if area(&ROOT, source_position, target_position) {
                    values.push(ConditionValues::TimedStatus {
                        kind: StatusKind::Rooted,
                        duration_ms: 3000,
                    });
                }
                if area(&FEAR, source_position, target_position) {
                    values.push(ConditionValues::TimedStatus {
                        kind: StatusKind::Feared,
                        duration_ms: 3000,
                    });
                }
            }
            if cast.current_target == Some(actor) {
                if passed[1] {
                    values.push(ConditionValues::Gate {
                        gate: ConditionGate::Powerless,
                        duration_ms: 3000,
                    });
                }
                if passed[2] {
                    values.push(ConditionValues::IntenseHex {
                        duration_ms: 3000,
                        damage_dealt_percent: 50,
                        healing_received_percent: 50,
                    });
                }
            }
            if values.is_empty() {
                continue;
            }
            let external_damage_delta = if values
                .iter()
                .any(|value| matches!(value, ConditionValues::IntenseHex { .. }))
            {
                attribute_damage_delta(actor, *session)?
            } else {
                0
            };
            let definitions = values
                .iter()
                .copied()
                .map(definition)
                .collect::<Result<Vec<_>, _>>()?;
            let bytes = Sha256::new()
                .chain_update(b"oteryn.soulpit.oppressor.v1")
                .chain_update(tag.handoff_id)
                .chain_update(cast.source.placement_identity())
                .chain_update(next_ordinal.to_be_bytes())
                .chain_update(actor.placement_identity())
                .finalize();
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&bytes[..16]);
            changes.push(self.prepare_soulpit_conditions(
                continuity,
                super::super::runtime_actor_conditions::SoulpitConditionApplication {
                    source: cast.source,
                    target: actor,
                    session: *session,
                    definitions: &definitions,
                    root: &decision_root,
                    occurrence: DecisionOccurrenceId::from_bytes(occurrence),
                    now: cast.now_micros,
                    external_damage_delta,
                },
            )?);
            outcomes.push(SoulpitOppressorTarget {
                actor,
                session: *session,
                conditions: values,
            });
        }
        self.publish_soulpit_conditions(changes);
        if let Slot::CreatureOccupied { spell_combat, .. } = &mut self.slots[source_index]
            && let Some(tag) = &mut spell_combat.soulpit
            && let Some(boss) = &mut tag.boss
        {
            boss.next_oppressor_at_micros = next_due;
            boss.ordinal = next_ordinal;
        }
        Ok(outcomes)
    }
}
