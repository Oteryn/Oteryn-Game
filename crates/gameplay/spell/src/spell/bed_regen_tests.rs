use super::*;
use crate::spell::{Vocation, cast::CharacterCastFacts};

#[test]
fn bed_regen_rest_preserves_food_and_online_periods_and_clears_on_death() -> Result<(), &'static str>
{
    let facts = CharacterCastFacts {
        vocation: Vocation::Knight,
        level: 20,
        magic_level: 0,
        max_health: 365,
        max_mana: 150,
        max_soul: 100,
    };
    let initial = PlayerSpellState::new(facts, 0, 0).ok_or("actor")?;
    let mut fed = initial.after_food(31, 1000).ok_or("eat")?;
    fed.regen_health_ms = 1234;
    fed.regen_mana_ms = 567;
    let saved = fed.bed_rest_snapshot(1_567_000);
    assert_eq!(saved, [100, 30433, 1234, 567]);
    let mut fresh = PlayerSpellState::new(facts, 0, 0).ok_or("actor")?;
    fresh.restore_bed_rest(saved, 9_000_000).ok_or("restore")?;
    assert_eq!(fresh.bed_rest_snapshot(9_000_000), saved);
    crate::spell::actor_conditions::clear_on_lifecycle(&mut fresh);
    assert_eq!(fresh.bed_rest_snapshot(9_000_000)[1..], [0, 0, 0]);
    fresh
        .restore_bed_rest([5, 1_200_000, 0, 0], 9_000_000)
        .ok_or("cap restore")?;
    assert_eq!(fresh.bed_rest_snapshot(9_000_000), [5, 1_200_000, 0, 0]);
    Ok(())
}
