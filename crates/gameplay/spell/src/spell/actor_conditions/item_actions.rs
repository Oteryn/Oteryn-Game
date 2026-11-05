//! Item actions mutate the existing condition/vitals successor and lifecycle.
use super::*;

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

/// Crystal fireworks_rocket.lua:11 calls addHealth directly. It does not run armor,
/// mana-shield or incoming combat reductions; the shared HP/death floor still applies.
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
    stage_hit(state, amount, now_us, false, false, None)
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
