//! Source-only lowering into the existing combat owner's closed CharmEffect shapes.
//! Caller must bound raw input bytes before calling (the canonical Content adapter owns
//! EvidenceLimits). This does not qualify a generation or advertise effect availability.

use super::ContentError;
use super::charm_source_json::{CharmSourceMembers, ExactPercent, ensure_empty, take};
use crate::combat::charm_effects::{
    CharmDamageKind, CharmEffect, CharmElement, CharmPercent, CharmResource,
};
use serde_json::value::RawValue;

fn invalid() -> ContentError {
    ContentError::InvalidArtifact("invalid Charm source effect")
}
fn percent(value: u32) -> Result<CharmPercent, ContentError> {
    CharmPercent::from_hundredths(value).ok_or_else(invalid)
}

fn take_percent(members: &mut CharmSourceMembers, key: &str) -> Result<CharmPercent, ContentError> {
    percent(take::<ExactPercent>(members, key)?.0)
}

pub(crate) fn lower_effect(raw: &RawValue) -> Result<CharmEffect, ContentError> {
    let mut members =
        serde_json::from_str::<CharmSourceMembers>(raw.get()).map_err(|_| invalid())?;
    let kind: String = take(&mut members, "type")?;
    let effect = match kind.as_str() {
        "attack_proc_damage" => CharmEffect::AttackProcDamage {
            damage: damage(&mut members)?,
            percent_of_creature_max_health: take_percent(
                &mut members,
                "percent_of_creature_max_health",
            )?,
            damage_cap_level_multiplier: take(&mut members, "damage_cap_level_multiplier")?,
        },
        "attack_proc_resource_damage" => CharmEffect::AttackProcResourceDamage {
            damage: damage(&mut members)?,
            resource: match take::<String>(&mut members, "resource")?.as_str() {
                "health" => CharmResource::Health,
                "mana" => CharmResource::Mana,
                _ => return Err(invalid()),
            },
            percent_of_own_maximum: take_percent(&mut members, "percent_of_own_maximum")?,
            damage_cap_percent_of_creature_max_health: take_percent(
                &mut members,
                "damage_cap_percent_of_creature_max_health",
            )?,
        },
        "kill_area_damage" => CharmEffect::KillAreaDamage {
            damage: damage(&mut members)?,
            percent_of_creature_max_health: take_percent(
                &mut members,
                "percent_of_creature_max_health",
            )?,
            damage_cap_level_multiplier: take(&mut members, "damage_cap_level_multiplier")?,
        },
        "reflect_damage_taken" => CharmEffect::ReflectDamageTaken {
            damage: damage(&mut members)?,
        },
        "paralyse_creature_on_attack" => CharmEffect::ParalyseCreatureOnAttack {
            duration_ms: take(&mut members, "duration_ms")?,
        },
        "paralyse_creature_after_its_attack" => CharmEffect::ParalyseCreatureAfterItsAttack {
            duration_ms: take(&mut members, "duration_ms")?,
        },
        "haste_after_hit" => CharmEffect::HasteAfterHit {
            duration_ms: take(&mut members, "duration_ms")?,
        },
        "prevent_creature_flee" => CharmEffect::PreventCreatureFlee {
            duration_ms: take(&mut members, "duration_ms")?,
        },
        "dodge_attack" => CharmEffect::DodgeAttack,
        "cleanse_after_hit" => CharmEffect::CleanseAfterHit,
        "death_loss_reduction" => CharmEffect::DeathLossReduction,
        "skinning_chance_bonus" => CharmEffect::SkinningChanceBonus,
        "creature_product_bonus" => CharmEffect::CreatureProductBonus,
        "critical_hit_chance" => CharmEffect::CriticalHitChance,
        "critical_extra_damage" => CharmEffect::CriticalExtraDamage,
        "life_leech" => CharmEffect::LifeLeech,
        "mana_leech" => CharmEffect::ManaLeech,
        "mana_drain_inversion" => CharmEffect::ManaDrainInversion,
        _ => return Err(invalid()),
    };
    ensure_empty(&members)?;
    Ok(effect)
}

fn damage(members: &mut CharmSourceMembers) -> Result<CharmDamageKind, ContentError> {
    let element = match take::<String>(members, "element")?.as_str() {
        "physical" => CharmElement::Physical,
        "fire" => CharmElement::Fire,
        "earth" => CharmElement::Earth,
        "energy" => CharmElement::Energy,
        "ice" => CharmElement::Ice,
        "death" => CharmElement::Death,
        "holy" => CharmElement::Holy,
        _ => return Err(invalid()),
    };
    Ok(CharmDamageKind {
        element,
        ignores_resistances: take(members, "ignores_resistances")?,
        reduced_by_armor: take(members, "reduced_by_armor")?,
    })
}

#[cfg(test)]
#[path = "charm_source_effect_tests.rs"]
mod tests;
