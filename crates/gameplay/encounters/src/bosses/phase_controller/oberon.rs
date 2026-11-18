//! Crystal 201189f Oberon onThink/onSay. Live HP, children and immunity share the carrier turn.
use super::gates::DamageGates;
use crate::bosses::source_callbacks::oberon_think::{OBERON_KEY, OberonEffect, OberonThinkOwner};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition,
};

const MAX_FIGHTS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Fight {
    actor: ExactActorRef,
    rules: OberonThinkOwner,
    last_phase_sequence: u64,
}
impl Fight {
    pub(super) fn blocks(&self, actor: ExactActorRef) -> bool {
        self.actor == actor && self.rules.immune()
    }
}

pub struct Transition {
    pub speech: &'static str,
    pub summon: ExactActorRef,
}

/// The caller supplies the existing scheduler occurrence and its deterministic draws.
pub fn think(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    sequence: u64,
    draw: impl FnMut(u32) -> u32,
) -> Result<Option<Transition>, CarrierError> {
    if !runtime.matches_live_creature_identity(actor, OBERON_KEY.as_bytes()) {
        return Ok(None);
    }
    runtime.with_encounter_hooks::<DamageGates, _, CarrierError>(|runtime, phases| {
        phases
            .oberon
            .retain(|entry| runtime.contains_live_creature(entry.actor));
        let index = if let Some(index) = phases.oberon.iter().position(|entry| entry.actor == actor)
        {
            index
        } else {
            if phases.oberon.len() >= MAX_FIGHTS {
                return Err(CarrierError::CapacityExceeded);
            }
            phases
                .oberon
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            phases.oberon.push(Fight {
                actor,
                rules: OberonThinkOwner::default(),
                last_phase_sequence: 0,
            });
            phases.oberon.len() - 1
        };
        let entry = &mut phases.oberon[index];
        if sequence <= entry.last_phase_sequence || entry.rules.immune() {
            return Ok(None);
        }
        let current = runtime.companion_snapshot(actor)?;
        if current.state.master.is_some() {
            return Ok(None);
        }
        let health =
            u64::try_from(current.health).map_err(|_| CarrierError::InvalidCreatureHealth)?;
        let maximum = u64::try_from(current.maximum_health)
            .map_err(|_| CarrierError::InvalidCreatureHealth)?;
        let mut next = entry.rules;
        let effects = next
            .think(health, maximum, draw)
            .map_err(|_| CarrierError::PlanConflict)?;
        let [
            OberonEffect::RegisterImmunityEvent,
            OberonEffect::Say { text },
            OberonEffect::HealToFull,
            OberonEffect::Summon { creature_key },
        ] = effects.as_slice()
        else {
            return Ok(None);
        };
        let at = runtime.read_actor_position(actor)?.position();
        let health = runtime.companion_policy(creature_key)?.maximum_health;
        let content = runtime.content_pin().server_artifact_digest();
        let before = runtime.encounter_parts().0.clone();
        let transition = (|| {
            let summon = runtime.commit_quest_creature_spawn(&[(creature_key, health, at)])?[0];
            runtime.commit_source_creature_heal_batch(
                content,
                &[(actor, OBERON_KEY.into(), maximum, maximum)],
            )?;
            Ok(Transition {
                speech: text,
                summon,
            })
        })();
        match transition {
            Ok(transition) => {
                entry.rules = next;
                entry.last_phase_sequence = sequence;
                Ok(Some(transition))
            }
            Err(error) => {
                *runtime.encounter_parts_mut().0 = before;
                Err(error)
            }
        }
    })
}

/// Visit only exact live challenge actors that could hear this local say.
pub fn say(
    runtime: &mut ChannelRuntimeV1,
    message: &str,
    now_seconds: u64,
    hears: impl Fn(MovementLocalPosition) -> bool,
) -> Vec<(ExactActorRef, &'static str)> {
    if runtime
        .encounter_hooks()
        .and_then(|hooks| hooks.as_any().downcast_ref::<DamageGates>())
        .is_none_or(|phases| phases.oberon.is_empty())
    {
        return Vec::new();
    }
    runtime
        .with_encounter_hooks::<DamageGates, _, CarrierError>(|runtime, phases| {
            Ok(phases
                .oberon
                .iter_mut()
                .filter_map(|entry| {
                    let at = runtime.read_actor_position(entry.actor).ok()?.position();
                    if !hears(at) || !runtime.contains_live_creature(entry.actor) {
                        return None;
                    }
                    let accepted = entry.rules.answer(message, now_seconds)?;
                    Some((
                        entry.actor,
                        if accepted {
                            "Oberon roars in frustration!"
                        } else {
                            "Oberon laughs."
                        },
                    ))
                })
                .collect())
        })
        .unwrap_or_default()
}
