//! Source-qualified Summer callbacks composed with the shared spell and condition owners.
//! Entry, party gates, cooldowns and normal/final transitions remain source-unavailable.
use crate::content::ProjectV2AuthoringProfileData;
use crate::content::native_gameplay::CreatureProfileRecord;
use crate::creature_damage_spell::SpellSource;
use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
use oteryn_gameplay_foundation::foundation::condition::DamageSchedule;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;

/// Crystal 201189f, spells_phosphorus_curse.lua:8-13. The Lua accumulator is
/// multiplied before flooring each emitted tick; flooring the accumulator changes the rule.
pub const PHOSPHORUS_CURSE_TICKS: [u32; 21] = [
    300, 345, 396, 456, 524, 603, 693, 798, 917, 1055, 1213, 1395, 1605, 1845, 2122, 2441, 2807,
    3228, 3712, 4269, 4909,
];

/// Admit exactly the already-authored source body. Other Summer encounters retain the
/// typed missing closure and `W16_ADMISSION_AND_PHASE_SOURCE_MISSING` provenance marker.
pub fn is_phosphorus_curse(record: &CreatureProfileRecord, source: &SpellSource) -> bool {
    if record.profile.target.key != "oteryn:creature.phosphorus_final"
        || record.profile.target.revision != "definition-r1"
        || source.creature_key != record.profile.target.key
        || source.ability.key != "oteryn:ability.spell.phosphorus_curse"
        || source.ability.revision != "definition-r1"
        || source.entry_index != 1
        || source.list != ScheduleList::Attack
        || source.element.as_deref() != Some("death")
        || source.minimum != 0
        || source.maximum != 0
        || !source.needs_target
        || source.needs_direction
        || source.range != 0
        || source.area.is_some()
        || source.chain.is_some()
        || source.windup.is_some()
        || !source.variants.is_empty()
        || !source.appearances.is_empty()
        || source.appearance_cast
        || source.dispel_invisible
        || source.source_none_correction
        || !record
            .source_qualification
            .as_deref()
            .is_some_and(|note| note.contains("201189f871d23597b82caf88fd611454a2ddefb8"))
    {
        return false;
    }
    let Some(ProjectV2AuthoringProfileData::Behavior(behavior)) =
        record.behavior.as_ref().map(|profile| &profile.data)
    else {
        return false;
    };
    // Crystal phosphorus_final.lua:84: native AI scheduling, not a second timer.
    if !behavior.attacks.get(1).is_some_and(|entry| {
        entry.ability == source.ability
            && entry.interval_ms == 12_000
            && entry.chance_ppm == 1_000_000
            && entry
                .magnitude
                .as_ref()
                .is_some_and(|value| value.minimum == 0 && value.maximum == 0)
    }) {
        return false;
    }
    let [condition] = source.conditions.as_slice() else {
        return false;
    };
    if condition.key() != "oteryn:effect.spell.phosphorus_curse.effect-condition-1"
        || condition.revision() != 1
        || condition.values()
            != (ConditionValues::DamageSchedule {
                element: DotElement::Cursed,
            })
    {
        return false;
    }
    let Some(DamageSchedule::Fixed {
        delayed: true,
        segments,
    }) = condition.source_damage_schedule()
    else {
        return false;
    };
    segments.len() == PHOSPHORUS_CURSE_TICKS.len()
        && segments
            .iter()
            .zip(PHOSPHORUS_CURSE_TICKS)
            .all(|(segment, damage)| {
                segment.count == 1 && segment.interval_ms == 2_000 && segment.amount == damage
            })
}
