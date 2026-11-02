//! Crystal 201189f gourmet food uses the real condition and vitals owner.
//! Conflict keys replace attributes, expire haste and clear effects on death.
use super::*;
use oteryn_gameplay_domain::domain::condition::attributes::{
    AttributeModifier, AttributeModifiers,
};
use oteryn_gameplay_domain::domain::condition::{ConflictKey, SpeedRange};
use oteryn_gameplay_domain::domain::hireling::food::{
    GOURMET_BUFF_SECONDS, GourmetBuff, GourmetConditions, GourmetEffect, gourmet_effect,
};

fn apply_buff(
    state: &mut PlayerSpellState,
    item_id: u32,
    duration_ms: u32,
    now_ms: u64,
) -> Result<(), SpellCastDisposition> {
    if duration_ms == 0 || duration_ms > GOURMET_BUFF_SECONDS * 1000 {
        return rejected();
    }
    let mut modifiers = AttributeModifiers::default();
    let values = match gourmet_effect(item_id).ok_or(SpellCastDisposition::Rejected)? {
        GourmetEffect::Shielding(n) => {
            modifiers.shielding = Some(AttributeModifier::Add(i32::from(n)));
            None
        }
        GourmetEffect::Distance(n) => {
            modifiers.distance = Some(AttributeModifier::Add(i32::from(n)));
            None
        }
        GourmetEffect::Magic(n) => {
            modifiers.magic_points = Some(AttributeModifier::Add(i32::from(n)));
            None
        }
        GourmetEffect::Melee(n) => {
            modifiers.melee = Some(AttributeModifier::Add(i32::from(n)));
            None
        }
        GourmetEffect::Fishing(n) => {
            modifiers.fishing = Some(AttributeModifier::Add(i32::from(n)));
            None
        }
        GourmetEffect::Speed(n) => {
            // Native SpeedRange describes target speed; this source sets an additive delta.
            let target = i32::try_from(
                state
                    .base_speed
                    .checked_add(u32::from(n))
                    .ok_or(SpellCastDisposition::Rejected)?,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            Some(ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: 0,
                    b_min: target,
                    a_max: 0,
                    b_max: target,
                },
                duration_ms,
            })
        }
        _ => return rejected(),
    }
    .unwrap_or(ConditionValues::SourceAttributes {
        modifiers,
        duration_ms,
    });
    let key = format!("item.hireling.gourmet.i{item_id}");
    apply_definition(state, definition(&key, values)?, now_ms, 0)
}

/// PREPARE a direct successor; only a known native USE commit publishes it.
pub fn stage_gourmet_food(
    state: &PlayerSpellState,
    item_id: u32,
    now_ms: u64,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    if state.health == 0 {
        return rejected();
    }
    let mut next = match gourmet_effect(item_id).ok_or(SpellCastDisposition::Rejected)? {
        GourmetEffect::RestoreHealthPercent(percent) => {
            let amount = u64::from(state.facts.max_health) * u64::from(percent) / 100;
            state
                .after_health_gain(amount)
                .ok_or(SpellCastDisposition::Rejected)?
                .0
        }
        GourmetEffect::RestoreManaPercent(percent) => {
            let amount = u64::from(state.facts.max_mana) * u64::from(percent) / 100;
            state
                .after_mana_gain(amount)
                .ok_or(SpellCastDisposition::Rejected)?
                .0
        }
        GourmetEffect::None => state.clone(),
        _ => {
            let mut next = state.clone();
            apply_buff(&mut next, item_id, GOURMET_BUFF_SECONDS * 1000, now_ms)?;
            next
        }
    };
    // Full pools and burnt dishes still consume a source occurrence. One revision
    // contains the complete successor; a replay never applies a second heal.
    next.revision = state
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    Ok(next)
}

/// Remaining ticks of only this source's live native conditions. Replaced,
/// dispelled, expired or death-cleared conditions naturally produce no row.
pub fn gourmet_conditions(state: &PlayerSpellState, now_us: u64) -> GourmetConditions {
    let read = |conflict| {
        let condition = state.conditions.get(conflict)?;
        let item_id = condition
            .definition()
            .key()
            .strip_prefix("item.hireling.gourmet.i")?
            .parse::<u32>()
            .ok()?;
        let remaining_ms = u32::try_from(condition.remaining_duration_ms(now_us)?).ok()?;
        let buff = GourmetBuff {
            item_id,
            remaining_ms,
        };
        buff.valid().then_some(buff)
    };
    GourmetConditions {
        attributes: read(ConflictKey::Attributes),
        haste: read(ConflictKey::Speed),
    }
}

/// Newly admitted actors resume persistent default conditions with remaining ticks
/// (condition.cpp175,356; iologindata_save_player.cpp264-276). A same-actor reconnect
/// retains its current native owner and does not call this function again.
pub fn restore_gourmet_food(
    state: &mut PlayerSpellState,
    saved: GourmetConditions,
    now_ms: u64,
) -> Result<(), SpellCastDisposition> {
    if !saved.valid() {
        return rejected();
    }
    let mut next = state.clone();
    for buff in [saved.attributes, saved.haste].into_iter().flatten() {
        apply_buff(&mut next, buff.item_id, buff.remaining_ms, now_ms)?;
    }
    *state = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn actor() -> PlayerSpellState {
        PlayerSpellState::new(
            crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level: 100,
                magic_level: 50,
                max_health: 1001,
                max_mana: 501,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap()
    }
    #[test]
    fn actual_skill_consumers_use_exact_source_bonuses_and_keep_base() {
        for (item, index, delta) in [(29408, 5, 7), (29409, 4, 7), (29411, 1, 3), (29413, 6, 30)] {
            let original = actor();
            let next = stage_gourmet_food(&original, item, 1000).unwrap();
            assert_eq!(
                next.owned_skill_adjustments(1_000_000)
                    .unwrap()
                    .adjust_skill(50, index),
                Some(50 + delta)
            );
            assert_eq!(
                next.owned_skill_adjustments(3_601_000_000)
                    .unwrap()
                    .adjust_skill(50, index),
                Some(50)
            );
            assert_eq!(next.character_facts(), original.character_facts());
        }
        let next = stage_gourmet_food(&actor(), 29410, 0).unwrap();
        assert_eq!(next.owned_effective_magic_level(0), Ok(53));
        assert_eq!(next.owned_effective_magic_level(3_600_000_000), Ok(50));
    }
    #[test]
    fn attributes_replace_haste_coexists_and_relog_preserves_remaining_ticks() {
        let state = stage_gourmet_food(&actor(), 29408, 0).unwrap();
        let state = stage_gourmet_food(&state, 29409, 600_000).unwrap();
        let state = stage_gourmet_food(&state, 29412, 1_200_000).unwrap();
        let skills = state.owned_skill_adjustments(1_200_000_000).unwrap();
        assert_eq!(skills.adjust_shielding(50), Some(50));
        assert_eq!(skills.adjust_skill(50, 4), Some(57));
        // effective_speed returns (application base, target), not the delta.
        // The real movement consumer reads this fixed source delta (+80).
        assert_eq!(state.conditions.speed_delta_at(1_200_000_000), 80);
        let saved = gourmet_conditions(&state, 1_300_000_000);
        assert_eq!(saved.attributes.unwrap().remaining_ms, 2_900_000);
        assert_eq!(saved.haste.unwrap().remaining_ms, 3_500_000);
        let mut relog = actor();
        restore_gourmet_food(&mut relog, saved, 9_000_000).unwrap();
        assert_eq!(gourmet_conditions(&relog, 9_000_000_000), saved);
    }
    #[test]
    fn recovery_uses_current_maxima_clamps_and_creates_no_food_regeneration() {
        let mut state = actor();
        state.health = 10;
        state.mana = 10;
        assert_eq!(stage_gourmet_food(&state, 29414, 0).unwrap().health, 310);
        assert_eq!(stage_gourmet_food(&state, 29415, 0).unwrap().mana, 160);
        assert_eq!(stage_gourmet_food(&actor(), 29414, 0).unwrap().health, 1001);
        assert!(stage_gourmet_food(&actor(), 35173, 0).is_err());
        assert!(
            stage_gourmet_food(&actor(), 29416, 0)
                .unwrap()
                .conditions
                .instances()
                .is_empty()
        );
    }
}
