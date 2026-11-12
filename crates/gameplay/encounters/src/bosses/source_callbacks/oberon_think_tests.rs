use super::*;
use std::cell::Cell;

#[test]
fn oberon_asks_heals_and_summons_at_twenty_percent() {
    let mut owner = OberonThinkOwner::default();
    let draws = Cell::new(0);
    assert_eq!(
        owner.think(201, 1000, |_| {
            draws.set(draws.get() + 1);
            0
        }),
        Ok(vec![])
    );
    assert_eq!(draws.get(), 0);

    assert_eq!(
        owner.think(200, 1000, |n| if n == 9 { 8 } else { 1 }),
        Ok(vec![
            OberonEffect::RegisterImmunityEvent,
            OberonEffect::Say {
                text: "ULTAH SALID'AR, ESDO LO!",
            },
            OberonEffect::HealToFull,
            OberonEffect::Summon {
                creature_key: "oteryn:creature.falcon_paladin",
            },
        ])
    );
    assert_eq!(owner.asking(), Some(9));
    assert_eq!(owner.lives_used(), 1);
}

#[test]
fn oberon_has_exactly_three_lives() {
    let mut owner = OberonThinkOwner::default();
    let expected = Ok(vec![
        OberonEffect::RegisterImmunityEvent,
        OberonEffect::Say {
            text: "You appear like a worm among men!",
        },
        OberonEffect::HealToFull,
        OberonEffect::Summon {
            creature_key: "oteryn:creature.falcon_knight",
        },
    ]);

    for _ in 0..OBERON_LIVES {
        assert_eq!(owner.think(100, 1000, |_| 0), expected);
    }
    assert_eq!(owner.think(100, 1000, |_| 0), Ok(vec![]));
    assert_eq!(owner.lives_used(), OBERON_LIVES);
}

#[test]
fn out_of_range_draws_refuse_without_changing_oberon() {
    let mut owner = OberonThinkOwner::default();
    assert_eq!(
        owner.think(100, 1000, |n| n),
        Err(OberonRefusal::InvalidDraw)
    );
    assert_eq!(owner, OberonThinkOwner::default());

    assert_eq!(
        owner.think(100, 1000, |n| if n == 9 { 0 } else { 2 }),
        Err(OberonRefusal::InvalidDraw)
    );
    assert_eq!(owner, OberonThinkOwner::default());
}

#[test]
fn oberon_tables_match_the_donor() {
    assert_eq!(OBERON_ASKING.len(), 9);
    assert_eq!(
        OBERON_ASKING[1],
        "The world will suffer for its iddle laziness!"
    );
    assert_eq!(
        OBERON_ASKING[7],
        "I lead the most honourable and formidable following of knights!"
    );

    let mut owner = OberonThinkOwner::default();
    assert_eq!(owner.think(0, 0, |_| 0), Ok(vec![]));
}
