//! Ordinary source chain callbacks retain actual source, target and phase; damage resolves at due.
//! They share the real owner timer lane; they never manufacture a native spell profile.
use super::chain::{CHAIN_STEP_DELAY_MICROS, ChainShape, TilePosition};
use super::combat_batch::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, TimedCombatEffect,
};
use super::delayed_execution::{
    CastMetadata, Error, ScheduleRequest, SpellTimerOccurrence, TimerPayload,
};
use super::{Execution, SpellBook, SpellDefinition};
use crate::ability::AbilityOccurrence;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelRuntimeV1, CharacterId, CommandRef, ExactActorRef};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OrdinaryCastBinding {
    pub(crate) source: Arc<SpellDefinition>,
    pub(crate) content_digest: [u8; 32],
    pub(crate) caster: ExactActorRef,
    pub(crate) attacker: CharacterId,
    pub(crate) command: CommandRef,
    pub(crate) occurrence: AbilityOccurrence,
    pub(crate) parent_binding: Vec<u8>,
    pub(crate) cast_at: SemanticTimeMicros,
    pub(crate) cast_position: TilePosition,
}
impl OrdinaryCastBinding {
    pub(crate) fn metadata(&self) -> CastMetadata<'_> {
        CastMetadata {
            caster: self.caster,
            attacker: self.attacker,
            command: self.command,
            occurrence: &self.occurrence,
            parent_binding: &self.parent_binding,
            cast_at: self.cast_at,
            cast_position: self.cast_position,
            cast_snapshot: None,
        }
    }
    pub(crate) fn active(&self, book: &SpellBook, runtime: &ChannelRuntimeV1) -> bool {
        if self.content_digest != runtime.content_pin().server_artifact_digest()
            || self.source.authored.is_none()
            || !ordinary_chain(&self.source)
        {
            return false;
        }
        let mut index = 1u32;
        while let Some(number) = std::num::NonZeroU32::new(index) {
            let Some((spell, selected)) = book.source_indexed(number) else {
                return false;
            };
            if selected && spell == self.source.as_ref() {
                return true;
            }
            let Some(next) = index.checked_add(1) else {
                return false;
            };
            index = next;
        }
        false
    }
}
#[derive(Debug, Clone)]
pub(crate) struct ResolvedOrdinaryCombat {
    pub(crate) batch: OwnerCombatBatch,
    pub(crate) presentation: Arc<crate::gameplay_transport::PreparedSpellPresentation>,
    pub(crate) magnitude: Arc<super::magnitude_owner::PreparedMagnitudeOwner>,
    pub(crate) owned: super::owned_cast_facts::OwnedCastFacts,
}
#[derive(Debug, Clone)]
pub(crate) struct SavedOrdinaryCombat {
    pub(crate) binding: OrdinaryCastBinding,
    pub(crate) phase: u16,
    pub(crate) delay_ms: u64,
    pub(crate) target: ExactActorRef,
    pub(crate) step: u32,
    pub(crate) from: TilePosition,
    pub(crate) sub_ordinal: u16,
    /// Actual first due numerical result, retained across ambiguous read/commit outcomes.
    pub(crate) resolved: Option<ResolvedOrdinaryCombat>,
}
impl SavedOrdinaryCombat {
    pub(crate) fn validate(
        &self,
        occurrence: SpellTimerOccurrence,
        due: SemanticTimeMicros,
    ) -> Result<(), Error> {
        let chain = self
            .binding
            .source
            .chain
            .as_ref()
            .ok_or(Error::InvalidPayload)?;
        let delay = self.delay_ms.checked_mul(1000).ok_or(Error::Bounds)?;
        let step = delay / CHAIN_STEP_DELAY_MICROS;
        if self.binding.source.authored.is_none()
            || !ordinary_chain(&self.binding.source)
            || self.binding.content_digest == [0; 32]
            || occurrence.phase != self.phase
            || self.phase >= 256
            || delay % CHAIN_STEP_DELAY_MICROS != 0
            || step > u64::from(chain.max_targets)
            || step != u64::from(self.step)
            || (chain.shape == ChainShape::Fork && step > 1)
            || self.binding.cast_at.get().checked_add(delay) != Some(due.get())
            || usize::from(self.sub_ordinal) >= super::combat_batch::MAX_EFFECTS
        {
            return Err(Error::InvalidPayload);
        }
        // Current source ordinary chains have exactly one damage definition. Numeric
        // values are resolved at due; no cast-time damage or condition snapshot is retained.
        match &self.binding.source.execution {
            Execution::Effects(effects)
                if effects.len() == 1
                    && matches!(effects[0], super::SpellEffect::Damage { .. }) =>
            {
                Ok(())
            }
            _ => Err(Error::InvalidPayload),
        }
    }
    pub(crate) fn validates_effect(&self, effect: &OwnerCombatEffect) -> bool {
        effect.target == self.target
            && effect.sub_ordinal == self.sub_ordinal
            && match &effect.change {
                OwnerCombatChange::Damage {
                    target_atom,
                    magnitude,
                } => !target_atom.is_empty() && !target_atom.contains('\0') && *magnitude >= 0,
                OwnerCombatChange::DamageWithHealing {
                    target_atom,
                    damage,
                    healing,
                } => {
                    !target_atom.is_empty()
                        && !target_atom.contains('\0')
                        && *damage >= 0
                        && *healing > 0
                }
                _ => false,
            }
    }
    pub(crate) fn current_target(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.contains_live_creature(self.target)
            && runtime
                .player_control_facts(self.binding.caster, self.binding.command.game_session_id())
                .is_ok()
            && self.binding.content_digest == runtime.content_pin().server_artifact_digest()
    }
}
fn ordinary_chain(spell: &SpellDefinition) -> bool {
    spell.chain.is_some()
        && matches!(
            spell.execution,
            Execution::Effects(_) | Execution::AbilityVariants(_)
        )
}

/// Called while the physical/state owner turn is held, before payment and live draws.
/// The caller preflights source selections, then installs the reservation only
/// after the real combined cast receipt. Supplied placeholder damage is discarded.
pub(crate) fn ordinary_timer_requests(
    runtime: &ChannelRuntimeV1,
    spell: &SpellDefinition,
    original: &OwnerCombatBatch,
    occurrence: &AbilityOccurrence,
    cast_position: TilePosition,
    delayed: &[TimedCombatEffect],
) -> Result<Vec<ScheduleRequest>, Error> {
    if delayed.is_empty() {
        return Ok(Vec::new());
    }
    if !ordinary_chain(spell)
        || spell.authored.is_none()
        || delayed.len() > super::delayed_execution::PENDING_PER_KEY
        || original.deferred.is_some()
        || original.current_lease_generation == 0
        || original.occurrence != occurrence.clone().into()
        || original.binding.is_empty()
        || original.binding.len() > super::combat_batch::MAX_BINDING_BYTES
        || !runtime
            .player_control_facts(original.caster, original.command.game_session_id())
            .is_ok_and(|f| f.control_loss.is_none())
    {
        return Err(Error::InvalidPayload);
    }
    let binding = OrdinaryCastBinding {
        source: Arc::new(spell.clone()),
        content_digest: runtime.content_pin().server_artifact_digest(),
        caster: original.caster,
        attacker: original.attacker,
        command: original.command,
        occurrence: occurrence.clone(),
        parent_binding: original.binding.clone(),
        cast_at: SemanticTimeMicros::from_micros(
            original.now_ms.checked_mul(1000).ok_or(Error::Bounds)?,
        ),
        cast_position,
    };
    let capture: serde_json::Value =
        serde_json::from_slice(&original.binding).map_err(|_| Error::InvalidPayload)?;
    let origins = capture
        .get("source_chain_origins")
        .and_then(serde_json::Value::as_array)
        .ok_or(Error::InvalidPayload)?;
    if origins.len() != delayed.len() {
        return Err(Error::InvalidPayload);
    }
    let mut requests = Vec::new();
    requests
        .try_reserve(delayed.len())
        .map_err(|_| Error::Allocation)?;
    for (index, timed) in delayed.iter().enumerate() {
        if delayed[..index]
            .iter()
            .any(|old| old.effect.sub_ordinal == timed.effect.sub_ordinal)
            || original
                .effects
                .iter()
                .any(|old| old.sub_ordinal == timed.effect.sub_ordinal)
            || !runtime.contains_live_creature(timed.effect.target)
        {
            return Err(Error::InvalidPayload);
        }
        let phase = u16::try_from(index).map_err(|_| Error::Bounds)?;
        let occurrence = SpellTimerOccurrence {
            command: binding.command,
            phase,
        };
        let delay = timed.delay_ms.checked_mul(1000).ok_or(Error::Bounds)?;
        let due = SemanticTimeMicros::from_micros(
            binding
                .cast_at
                .get()
                .checked_add(delay)
                .ok_or(Error::Bounds)?,
        );
        if !matches!(timed.effect.change, OwnerCombatChange::Damage { .. }) {
            return Err(Error::InvalidPayload);
        }
        let step = u32::try_from(delay / CHAIN_STEP_DELAY_MICROS).map_err(|_| Error::Bounds)?;
        let mut matches = origins.iter().filter_map(|row| {
            let row = row.as_array()?;
            if row.len() != 5
                || row[0].as_u64()? != u64::from(timed.effect.target.actor_local_id())
                || row[1].as_u64()? != u64::from(step)
            {
                return None;
            }
            Some(TilePosition {
                x: i32::try_from(row[2].as_i64()?).ok()?,
                y: i32::try_from(row[3].as_i64()?).ok()?,
                floor: i16::try_from(row[4].as_i64()?).ok()?,
            })
        });
        let from = matches.next().ok_or(Error::InvalidPayload)?;
        if matches.next().is_some() || from.floor != cast_position.floor {
            return Err(Error::InvalidPayload);
        }
        let saved = SavedOrdinaryCombat {
            binding: binding.clone(),
            phase,
            delay_ms: timed.delay_ms,
            target: timed.effect.target,
            step,
            from,
            sub_ordinal: timed.effect.sub_ordinal,
            resolved: None,
        };
        saved.validate(occurrence, due)?;
        requests.push(ScheduleRequest {
            occurrence,
            due,
            payload: TimerPayload::OrdinaryCombat(saved),
        });
    }
    Ok(requests)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::foundation::owner_timer::OwnerClock;
    use crate::foundation::{
        ChannelContentPin, ChannelId, CommandId, GameSessionId, MovementLocalPosition, NodeId,
        RuntimeScopeRefV1, WorldId,
    };
    use crate::spell::delayed_execution::SpellTimerOwner;

    #[test]
    fn actual_source_chain_schedules_zero_phase_and_refuses_unqualified_due_damage() {
        let id = |n| [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n];
        let world = WorldId::decode(&id(60)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(61)).unwrap(),
            NodeId::decode(&id(62)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(91)).unwrap();
        let reservation = runtime.reserve_fresh_session(session).unwrap();
        let caster = runtime.commit_fresh_session(reservation).unwrap();
        let target = runtime
            .admit_test_creature(MovementLocalPosition {
                x: 1,
                y: 1,
                floor: 7,
            })
            .unwrap();
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))
        .unwrap();
        let row = catalog["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["bundle"]["spell"]["name"]
                    .as_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case("lightning"))
            })
            .unwrap();
        let source =
            crate::spell::authoring::spell_from_bundle(&row["bundle"], &row["dependencies"])
                .unwrap();
        assert!(ordinary_chain(&source));
        let book = SpellBook::new(vec![source.clone()]).unwrap();
        let occurrence = AbilityOccurrence::new(
            "ordinary-timer:actual-source",
            crate::ability::RevisionSet::new(
                "rules:1",
                "content:1",
                "world:1",
                "formula:1",
                "simulation:1",
            )
            .unwrap(),
        )
        .unwrap();
        let original = OwnerCombatBatch {
            caster, attacker:CharacterId::decode(&id(92)).unwrap(), current_lease_generation:1,
            command:CommandRef::new(session, CommandId::new(1).unwrap()),
            occurrence:occurrence.clone().into(), binding:serde_json::to_vec(&serde_json::json!({"source_chain_origins":[[u64::from(target.actor_local_id()),1,0,0,7]]})).unwrap(),
            anchor:None, now_ms:0, effects:vec![OwnerCombatEffect { target,sub_ordinal:0,
                change:OwnerCombatChange::Damage { target_atom:runtime.creature_spell_target_atom(target).unwrap(), magnitude:1 }}], deferred:None,
        };
        let delayed = vec![TimedCombatEffect {
            delay_ms: 50,
            effect: OwnerCombatEffect {
                target,
                sub_ordinal: 1,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).unwrap(),
                    magnitude: 7,
                },
            },
        }];
        let position = TilePosition {
            x: 0,
            y: 0,
            floor: 7,
        };
        let mut phase_zero_original = original.clone();
        phase_zero_original.effects.clear();
        phase_zero_original.binding = serde_json::to_vec(&serde_json::json!({
            "source_chain_origins":[[u64::from(target.actor_local_id()),0,0,0,7]]}))
        .unwrap();
        let mut zero = delayed[0].clone();
        zero.delay_ms = 0;
        zero.effect.sub_ordinal = 0;
        let zero_requests = ordinary_timer_requests(
            &runtime,
            &source,
            &phase_zero_original,
            &occurrence,
            position,
            &[zero],
        )
        .unwrap();
        assert_eq!(zero_requests[0].due.get(), 0);
        let TimerPayload::OrdinaryCombat(zero_saved) = &zero_requests[0].payload else {
            panic!("phase zero source");
        };
        assert_eq!(zero_saved.step, 0);
        assert!(zero_saved.resolved.is_none());
        let requests = ordinary_timer_requests(
            &runtime,
            &source,
            &original,
            &occurrence,
            position,
            &delayed,
        )
        .unwrap();
        let TimerPayload::OrdinaryCombat(saved) = &requests[0].payload else {
            panic!("ordinary source carrier");
        };
        assert!(saved.binding.active(&book, &runtime));
        assert!(saved.current_target(&runtime)); // It is a creature, not a caster-session player.
        let mut changed_source = saved.binding.clone();
        changed_source.source = Arc::new({
            let mut changed = source.clone();
            changed.cooldown_micros += 1;
            changed
        });
        assert!(!changed_source.active(&book, &runtime));
        let mut changed_pin = saved.binding.clone();
        changed_pin.content_digest[0] ^= 1;
        assert!(!changed_pin.active(&book, &runtime));
        assert!(
            saved
                .validate(
                    SpellTimerOccurrence {
                        command: original.command,
                        phase: 1
                    },
                    requests[0].due
                )
                .is_err()
        );
        assert!(
            saved
                .validate(
                    requests[0].occurrence,
                    SemanticTimeMicros::from_micros(51_000)
                )
                .is_err()
        );
        let mut owner = SpellTimerOwner::new(
            RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            ),
            runtime.binding().scope_generation(),
        )
        .unwrap();
        let stamp = runtime.issue_owner_work().unwrap();
        let reservation = owner
            .schedule_reservation(runtime.owner_fence().unwrap(), stamp, requests)
            .unwrap();
        let proof = owner
            .preflight_install(
                runtime.owner_fence().unwrap(),
                stamp,
                &original,
                reservation,
            )
            .unwrap();
        let staged = runtime.stage_spell_batch(&original).unwrap();
        assert!(runtime.commit_spell_batch(staged).unwrap().applied);
        owner.install_preflighted(proof);
        struct Clock(u64);
        impl OwnerClock for Clock {
            fn now(&self) -> SemanticTimeMicros {
                SemanticTimeMicros::from_micros(self.0)
            }
        }
        let mut callback = |live: &ChannelRuntimeV1,
                            payload: &TimerPayload,
                            _: SpellTimerOccurrence,
                            at: SemanticTimeMicros,
                            _: crate::foundation::RuntimeWorkStamp| {
            let TimerPayload::OrdinaryCombat(saved) = payload else {
                return Err(Error::InvalidPayload);
            };
            Ok(OwnerCombatBatch {
                effects: vec![OwnerCombatEffect {
                    target: saved.target,
                    sub_ordinal: saved.sub_ordinal,
                    change: OwnerCombatChange::Damage {
                        target_atom: live.creature_spell_target_atom(saved.target).unwrap(),
                        magnitude: 7,
                    },
                }],
                now_ms: at.get() / 1000,
                anchor: None,
                deferred: None,
                ..original.clone()
            })
        };
        let stamp = runtime.issue_owner_work().unwrap();
        assert!(
            owner
                .fire_due_current(&mut runtime, &Clock(49_999), stamp, &mut callback)
                .unwrap()
                .receipts
                .is_empty()
        );
        assert_eq!(
            runtime.creature_combat_facts(target, 49).unwrap().health,
            19
        );
        let stamp = runtime.issue_owner_work().unwrap();
        let report = owner
            .fire_due_current(&mut runtime, &Clock(50_000), stamp, &mut callback)
            .unwrap();
        assert!(matches!(report.blocked, Some((_, Error::SubstitutedBatch))));
        assert!(report.receipts.is_empty());
        assert_eq!(
            runtime.creature_combat_facts(target, 50).unwrap().health,
            19
        );
        let stamp = runtime.issue_owner_work().unwrap();
        assert!(
            owner
                .fire_due_current(&mut runtime, &Clock(50_001), stamp, &mut callback)
                .unwrap()
                .receipts
                .is_empty()
        );
        assert_eq!(
            runtime.creature_combat_facts(target, 50).unwrap().health,
            19
        );
        runtime.remove_test_actor(target).unwrap();
        assert!(!runtime.contains_live_creature(target));
    }
}
