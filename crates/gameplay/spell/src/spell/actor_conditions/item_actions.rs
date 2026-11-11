//! Item actions mutate the existing condition/vitals successor and lifecycle.
use super::*;

/// A Quest disguise is a projection of current fenced progress, with no independent timer.
/// Only this owner's condition is removed when its Quest range ends.
pub fn stage_quest_disguise(
    state: &PlayerSpellState,
    look_type: Option<u32>,
    now_ms: u64,
) -> Result<Option<PlayerSpellState>, SpellCastDisposition> {
    use oteryn_gameplay_domain::domain::condition::ConflictKey;
    const KEY: &str = "quest.disguise";
    if state.health == 0 {
        return Ok(None);
    }
    if look_type == Some(0) {
        return rejected();
    }
    let current = state.conditions.get(ConflictKey::Outfit);
    let ours = current.is_some_and(|value| value.definition().key() == KEY);
    if (look_type.is_none() && !ours)
        || (ours && look_type.is_some() && state.conditions.outfit_at(now_us(now_ms)?) == look_type)
    {
        return Ok(None);
    }
    let mut next = state.clone();
    next.conditions.remove_type(ConditionType::Outfit);
    if let Some(look_type) = look_type {
        let definition = ConditionDefinition::new_permanent_outfit(KEY, 1, look_type)
            .ok_or(SpellCastDisposition::Rejected)?;
        apply_definition(&mut next, definition, now_ms, 0)?;
    }
    next.advance_batch_revision()?;
    Ok(Some(next))
}

/// Crystal costume_bags.lua:16 uses a monster appearance for the sourced duration.
pub fn stage_item_costume(
    state: &PlayerSpellState,
    look_type: u32,
    duration_ms: u32,
    now_ms: u64,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    if state.health == 0 || look_type == 0 || duration_ms == 0 {
        return rejected();
    }
    let mut next = state.clone();
    apply_definition(
        &mut next,
        definition(
            "item.temporary.costume",
            ConditionValues::Outfit {
                look_type,
                duration_ms,
            },
        )?,
        now_ms,
        0,
    )?;
    next.advance_batch_revision()?;
    Ok(next)
}

/// Crystal 201189f fireworks_rocket.lua:11 and creature_functions.cpp:591-608:
/// Lua HP loss skips armor/origin reductions but retains mana shields and survival.
pub fn stage_item_health_loss(
    state: &PlayerSpellState,
    amount: u32,
    now_us: u64,
) -> Result<
    (
        PlayerSpellState,
        crate::ability::creature_bite::FlooredDamage,
    ),
    SpellCastDisposition,
> {
    stage_hit(
        state,
        amount,
        now_us,
        false,
        false,
        None,
        HitDelivery::DirectHealth,
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "fixed valid player state is a required test fixture"
    )]
    use super::*;
    fn player() -> PlayerSpellState {
        PlayerSpellState::new(
            super::super::super::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level: 100,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("player")
    }
    #[test]
    fn quest_disguise_has_no_timer_is_idempotent_and_clears_its_own_condition() {
        let original = player();
        let disguised = stage_quest_disguise(&original, Some(1160), 1)
            .expect("disguise")
            .expect("changed");
        assert_eq!(disguised.conditions.outfit_at(u64::MAX - 1), Some(1160));
        assert_eq!(disguised.revision(), original.revision() + 1);
        assert!(
            stage_quest_disguise(&disguised, Some(1160), 2)
                .expect("same")
                .is_none()
        );
        let clear = stage_quest_disguise(&disguised, None, 2)
            .expect("clear")
            .expect("changed");
        assert_eq!(clear.conditions.outfit_at(2000), None);
        assert_eq!(clear.revision(), disguised.revision() + 1);
        assert!(
            stage_quest_disguise(&clear, None, 3)
                .expect("same")
                .is_none()
        );
        assert!(stage_quest_disguise(&original, Some(0), 1).is_err());
    }

    #[test]
    fn inactive_quest_disguise_preserves_an_unrelated_costume() {
        let current = stage_item_costume(&player(), 924, 3_000_000, 1).expect("costume");
        assert!(
            stage_quest_disguise(&current, None, 2)
                .expect("unrelated")
                .is_none()
        );
        assert_eq!(current.conditions.outfit_at(2000), Some(924));
    }

    #[test]
    fn costume_expires_at_the_original_fifty_minute_deadline() {
        let state = player();
        let next = stage_item_costume(&state, 924, 3_000_000, 1).expect("costume");
        assert_eq!(next.conditions.outfit_at(1000), Some(924));
        assert_eq!(next.conditions.outfit_at(3_000_001_000), None);
        assert_eq!(next.revision(), state.revision() + 1);
        assert!(stage_item_costume(&state, 0, 1, 0).is_err());
        assert!(stage_item_costume(&state, 924, 0, 0).is_err());
    }
    #[test]
    fn rocket_health_loss_retains_lethal_floor_and_clears_lifecycle_conditions() {
        let mut state = player();
        state.health = 7;
        let (next, damage) = stage_item_health_loss(&state, 10, 0).expect("source health loss");
        assert_eq!(damage.health_after, 0);
        assert_eq!(next.health, 0);
        assert_eq!(next.revision(), state.revision() + 1);
        assert!(stage_item_health_loss(&next, 10, 0).is_err());
    }
}
