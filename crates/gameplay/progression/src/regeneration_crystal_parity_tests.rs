//! PAR-C10: online regeneration compared with Crystal `201189f8` `data/XML/vocations.xml`
//! (`gainhpticks`, `gainhpamount`, `gainmanaticks`, `gainmanaamount`) and
//! `ConditionRegeneration::executeCondition`. The Crystal values are reference data, not code.
use crate::regeneration::{
    FED_HEALTH_PERIOD_MS, FED_MANA_PERIOD_MS, RegenerationState, UNFED_HEALTH_PERIOD_MS,
    UNFED_MANA_PERIOD_MS, credit_second, rate,
};
use oteryn_gameplay_content::content::spell_data::Vocation;

/// Crystal `vocations.xml`: (vocation, health ticks ms, mana ticks ms). Amounts are 1 HP, 2 MP.
const CRYSTAL_TICKS: [(Vocation, u32, u32); 9] = [
    (Vocation::None, 12_000, 6_000),
    (Vocation::Sorcerer, 12_000, 3_000),
    (Vocation::Druid, 12_000, 3_000),
    (Vocation::MasterSorcerer, 12_000, 2_000),
    (Vocation::ElderDruid, 12_000, 2_000),
    (Vocation::Paladin, 8_000, 4_000),
    (Vocation::RoyalPaladin, 6_000, 3_000),
    (Vocation::Knight, 6_000, 6_000),
    (Vocation::EliteKnight, 4_000, 6_000),
];
/// Crystal Monk and Exalted Monk ticks, equal to TibiaWiki `Calculator/Hit Points Per Second`
/// and `Calculator/Mana Per Second`. The owner replaced D5b for Monk on 2026-10-29 with
/// Paladin/Royal Paladin periods. The live clocks use the news periods below.
const CRYSTAL_MONK_TICKS: [(Vocation, u32, u32); 2] = [
    (Vocation::Monk, 7_000, 4_000),
    (Vocation::ExaltedMonk, 5_000, 3_000),
];

#[test]
fn c10_vocation_amounts_and_ticks_equal_crystal_for_every_non_monk_vocation() {
    for (vocation, health_ticks, mana_ticks) in CRYSTAL_TICKS {
        let got = rate(vocation);
        assert_eq!(
            (got.health_amount, got.health_period_ms),
            (1, health_ticks),
            "{vocation:?}"
        );
        assert_eq!(
            (got.mana_amount, got.mana_period_ms),
            (2, mana_ticks),
            "{vocation:?}"
        );
    }
}

#[test]
fn c10_monk_ticks_are_the_owner_decided_paladin_divergence_from_crystal() {
    for ((vocation, health_ticks, mana_ticks), owner_ticks) in CRYSTAL_MONK_TICKS
        .into_iter()
        .zip([(8_000, 4_000), (6_000, 3_000)])
    {
        let got = rate(vocation);
        assert_eq!((got.health_amount, got.mana_amount), (1, 2), "{vocation:?}");
        assert_eq!(
            (got.health_period_ms, got.mana_period_ms),
            owner_ticks,
            "{vocation:?}: owner decision 2026-10-29 follows Paladin/Royal Paladin"
        );
        assert_ne!(
            (got.health_period_ms, got.mana_period_ms),
            (health_ticks, mana_ticks),
            "{vocation:?}: owner-decided Paladin/Royal Paladin ticks diverge from Crystal"
        );
    }
}

#[test]
fn c10_production_clocks_are_the_recorded_news_8541_8543_divergence_from_crystal() {
    // Crystal: a fed player gains 1 HP per `gainhpticks` and 2 MP per `gainmanaticks`; an
    // unfed one gains nothing. Oteryn (NEWS-HIST-REGEN-1): always on, one vocation amount per
    // fixed 3 s/5 s (fed) or 4 s/6 s (hungry) for every vocation.
    assert_eq!((FED_HEALTH_PERIOD_MS, FED_MANA_PERIOD_MS), (3_000, 5_000));
    assert_eq!(
        (UNFED_HEALTH_PERIOD_MS, UNFED_MANA_PERIOD_MS),
        (4_000, 6_000)
    );
    for (vocation, health_ticks, mana_ticks) in CRYSTAL_TICKS {
        let minute = 60_000;
        let crystal_fed = (minute / health_ticks, 2 * (minute / mana_ticks));
        let mut fed = RegenerationState {
            health: 1,
            mana: 0,
            regen_health_ms: 0,
            regen_mana_ms: 0,
        };
        let mut hungry = fed;
        for _ in 0..60 {
            credit_second(&mut fed, rate(vocation), u32::MAX, u32::MAX, true);
            credit_second(&mut hungry, rate(vocation), u32::MAX, u32::MAX, false);
        }
        assert_eq!((fed.health - 1, fed.mana), (20, 24), "{vocation:?}");
        assert_eq!((hungry.health - 1, hungry.mana), (15, 20), "{vocation:?}");
        assert_ne!((fed.health - 1, fed.mana), crystal_fed, "{vocation:?}");
    }
}
