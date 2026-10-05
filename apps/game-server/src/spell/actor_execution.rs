//! Focus effects at the existing player cast's PRIMARY COMMIT.
//!
//! This operates on the real `PlayerSpellState` successor prepared by `cast`.
//! The Channel owner still proves actor/session authority and compare-commits
//! that successor. Resource debit, cooldown successor reset/rearm and the one
//! vitals revision remain in the common cast anchor. No durable stance or
//! alternate actor state is introduced; the accepted solo/no-virtue interim
//! remains until those owning services are composed.

use std::sync::OnceLock;

use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::Value;

use super::Vocation;
use super::cast::PlayerSpellState;
use super::native_actor_states::{FocusFacts, FocusPlan, plan_focus};

const PROFILE_JSON: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
);

/// Check the complete closed parameters before reading state or drawing. The
/// outer reader separately qualifies the cast header and dependency closure.
fn qualified_parameters(parameters: &Value) -> bool {
    static FOCUS: OnceLock<Option<Vec<Value>>> = OnceLock::new();
    FOCUS
        .get_or_init(|| {
            let export: Value = serde_json::from_str(PROFILE_JSON).ok()?;
            if export.get("revision")?.as_str()? != "spell-p2-r20" {
                return None;
            }
            let values: Vec<Value> = export
                .get("profiles")?
                .as_array()?
                .iter()
                .filter_map(|row| {
                    let native = row.get("execution")?.get("native_behavior")?;
                    (native.get("key")?.as_str()? == "monk_focus")
                        .then(|| native.get("parameters").cloned())
                        .flatten()
                })
                .collect();
            (values.len() == 2).then_some(values)
        })
        .as_ref()
        .is_some_and(|values| values.iter().any(|value| value == parameters))
}

fn reject<E>(_: E) -> SpellCastDisposition {
    SpellCastDisposition::Rejected
}

/// Apply a Focus spell's primary actor effects atomically to `next`. `profile`
/// is the canonical `native_behavior.parameters` object. On every error even
/// this candidate successor is unchanged, and no resource/revision is paid here.
/// A gained-charge heal uses the existing cast occurrence's world draw and is
/// applied to the real caster under the accepted solo-party interim.
pub(crate) fn apply_focus(
    next: &mut PlayerSpellState,
    profile: &Value,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<FocusPlan, SpellCastDisposition> {
    if !qualified_parameters(profile)
        || !matches!(next.facts.vocation, Vocation::Monk | Vocation::ExaltedMonk)
        || next.health > next.facts.max_health
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut monk = next
        .monk
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    monk.accept_command().map_err(reject)?;
    let plan = plan_focus(
        profile,
        FocusFacts {
            harmony: monk.harmony(),
            serene: monk.serene(),
            forced_until: monk.serene_forced_until(),
            now,
            level: next.facts.level,
            sustain_active: next.stance
                == Some(super::native_actor_states::StandardStance::Sustain),
        },
    )
    .map_err(reject)?;
    let gained = monk.commit_fill().map_err(reject)?;
    if gained != plan.gained_charges || monk.harmony() != plan.harmony_after {
        return Err(SpellCastDisposition::Rejected);
    }
    if plan.arm_forced_serene {
        monk.commit_focus_serenity(now).map_err(reject)?;
    }
    if monk.serene() != plan.serene_after || monk.serene_forced_until() != plan.forced_until {
        return Err(SpellCastDisposition::Rejected);
    }
    let health = match &plan.healing {
        None => next.health,
        Some(heal) => {
            let roll = draw(heal.bounds.minimum, heal.bounds.maximum);
            let magnitude =
                u64::try_from(heal.finish_draw(roll).map_err(reject)?).map_err(reject)?;
            u32::try_from(
                u64::from(next.health)
                    .saturating_add(magnitude)
                    .min(u64::from(next.facts.max_health)),
            )
            .map_err(reject)?
        }
    };
    next.monk = Some(monk);
    next.health = health;
    Ok(plan)
}

#[cfg(test)]
#[path = "actor_execution_tests.rs"]
mod tests;
