//! The production charm catalogue against the values Global publishes (Tibia news #8140,
//! https://www.tibia.com/news/?subtopic=newsarchive&id=8140, re-read by Content on 2026-11-15).
//! Percent values are hundredths of a percent (10% is 1000).
// Test code: a missing charm or stage fails the test with its name.
#![allow(clippy::panic)]

use super::{CharmCatalogueRead, CharmEffect};
use oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue;
use oteryn_gameplay_domain::domain::charm::{CharmCategory, CharmKey, MAJOR_STAGE_ECHOES};

const MAJOR_COSTS_HIGH: [u32; 3] = [600, 900, 3000];
const MINOR_ECHO_COSTS: [u32; 3] = [100, 150, 225];

/// `(charm, stage values)` for every charm whose stage values Global lists.
const GLOBAL_STAGE_VALUES: [(&str, [u32; 3]); 14] = [
    ("adrenaline_burst", [600, 900, 1200]),
    ("bless", [600, 900, 1200]),
    ("carnage", [1000, 2000, 2200]),
    ("cleanse", [600, 900, 1200]),
    ("cripple", [600, 900, 1200]),
    ("fatal_hold", [3000, 4500, 6000]),
    ("gut", [600, 900, 1200]),
    ("numb", [600, 900, 1200]),
    ("overflux", [500, 1000, 1100]),
    ("overpower", [500, 1000, 1100]),
    ("savage_blow", [2000, 4000, 4400]),
    ("scavenge", [6000, 9000, 12000]),
    ("vampiric_embrace", [160, 240, 320]),
    ("void_inversion", [2000, 3000, 4000]),
];

/// Minor charms with Global's 6/9/12% trigger and their echo prices.
const MINOR_CHARMS: [&str; 10] = [
    "adrenaline_burst",
    "bless",
    "cleanse",
    "cripple",
    "fatal_hold",
    "gut",
    "numb",
    "scavenge",
    "vampiric_embrace",
    "void_inversion",
];

const ELEMENTAL_CHARMS: [&str; 7] = [
    "curse",
    "divine_wrath",
    "enflame",
    "freeze",
    "poison",
    "wound",
    "zap",
];

fn key(name: &str) -> String {
    format!("oteryn:charm.{name}")
}

fn stage_values(catalogue: &CanonicalCharmCatalogue, name: &str) -> [u32; 3] {
    let definition = catalogue
        .charm(&key(name))
        .unwrap_or_else(|| panic!("{name} is not in the production catalogue"));
    [1, 2, 3].map(|stage| {
        definition
            .stage(stage)
            .unwrap_or_else(|| panic!("{name} stage {stage}"))
            .hundredths()
    })
}

fn rule(catalogue: &CanonicalCharmCatalogue, name: &str) -> (CharmCategory, [u32; 3]) {
    let rule = catalogue
        .catalogue()
        .get(&CharmKey::new(key(name)).unwrap_or_else(|_| panic!("{name} key")))
        .unwrap_or_else(|_| panic!("{name} is not in the production catalogue"));
    (rule.category, rule.stage_costs)
}

fn production() -> CanonicalCharmCatalogue {
    CanonicalCharmCatalogue::embedded().unwrap_or_else(|_| panic!("embedded charm catalogue"))
}

#[test]
fn charm_stage_values_match_global() {
    let catalogue = production();
    for (name, values) in GLOBAL_STAGE_VALUES {
        assert_eq!(stage_values(&catalogue, name), values, "{name}");
    }
    // Void's Call: 0.8/1.2/1.6%.
    assert_eq!(stage_values(&catalogue, "voids_call"), [80, 120, 160]);
}

#[test]
fn charm_damage_caps_match_global() {
    let catalogue = production();
    let effect = |name: &str| {
        catalogue
            .charm(&key(name))
            .unwrap_or_else(|| panic!("{name} is not in the production catalogue"))
            .effect()
    };
    match effect("carnage") {
        CharmEffect::KillAreaDamage {
            damage_cap_level_multiplier,
            ..
        } => assert_eq!(damage_cap_level_multiplier, 6, "carnage cap is 6 x level"),
        other => panic!("carnage effect {other:?}"),
    }
    for name in ["overpower", "overflux"] {
        match effect(name) {
            CharmEffect::AttackProcResourceDamage {
                damage_cap_percent_of_creature_max_health,
                ..
            } => assert_eq!(
                damage_cap_percent_of_creature_max_health.hundredths(),
                800,
                "{name} cap is 8% of the target's health"
            ),
            other => panic!("{name} effect {other:?}"),
        }
    }
    for name in ELEMENTAL_CHARMS {
        match effect(name) {
            CharmEffect::AttackProcDamage {
                damage_cap_level_multiplier,
                ..
            } => assert_eq!(damage_cap_level_multiplier, 2, "{name} cap is 2 x level"),
            other => panic!("{name} effect {other:?}"),
        }
    }
}

#[test]
fn charm_costs_and_echoes_match_global() {
    let catalogue = production();
    for name in ["carnage", "overflux", "overpower"] {
        assert_eq!(
            rule(&catalogue, name),
            (CharmCategory::Major, MAJOR_COSTS_HIGH),
            "{name} costs 600/900/3000 charm points"
        );
    }
    for name in MINOR_CHARMS.into_iter().chain(["voids_call"]) {
        assert_eq!(
            rule(&catalogue, name),
            (CharmCategory::Minor, MINOR_ECHO_COSTS),
            "{name} costs 100/150/225 minor charm echoes"
        );
    }
    assert_eq!(MAJOR_STAGE_ECHOES, [50, 100, 200]);
}
