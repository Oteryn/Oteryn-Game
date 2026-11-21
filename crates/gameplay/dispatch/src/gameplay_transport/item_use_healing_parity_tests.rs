//! Healing parity matrix, potion adapter: drives the item-use path's own reads for every row of
//! `docs/reference/parity/healing.json` whose case key starts with `potion.`. The spell rows run
//! in the spell crate (`healing_parity_matrix_tests.rs`); both share one loader file.
//!
//! The values come from the activated content the item-use gate resolves for a held item
//! (`oteryn:item.tibia.i<id>`), and the level and vocation restriction is `requirements_met`, the
//! exact function `use` calls. Nothing is re-implemented here.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::content::ReferenceRestoreResource;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;

#[path = "../../../spell/src/spell/parity_rows.rs"]
mod parity_rows;
use parity_rows::{Observed, Row};

const HEALING: &str = include_str!("../../../../../docs/reference/parity/healing.json");

fn vocation(name: &str) -> Vocation {
    match name {
        "elite_knight" => Vocation::EliteKnight,
        "royal_paladin" => Vocation::RoyalPaladin,
        "master_sorcerer" => Vocation::MasterSorcerer,
        "elder_druid" => Vocation::ElderDruid,
        "exalted_monk" => Vocation::ExaltedMonk,
        other => panic!("unknown vocation {other}"),
    }
}

fn observe(state: &NativeGameplayState, row: &Row) -> Observed {
    let id = u32::try_from(row.int("item")).expect("item id");
    let key = format!("oteryn:item.tibia.i{id}");
    let mut seen = Observed::new();
    // An id the activated content does not admit as a potion is recorded, not guessed.
    let Some(policy) = state
        .item_index()
        .and_then(|index| index.revision(&key))
        .and_then(|revision| state.item_policy(&key, revision))
    else {
        seen.insert("admitted", 0);
        return seen;
    };
    let semantics = &policy.record().semantics;
    let ReferenceItemField::Known(ReferenceItemConsumption::Potion(potion)) =
        &semantics.consumption
    else {
        seen.insert("admitted", 0);
        return seen;
    };
    seen.insert("admitted", 1);
    for restore in &potion.restores {
        let (low, high) = match restore.resource {
            ReferenceRestoreResource::Health => ("health_min", "health_max"),
            ReferenceRestoreResource::Mana => ("mana_min", "mana_max"),
        };
        seen.insert(low, i64::from(restore.min));
        seen.insert(high, i64::from(restore.max));
    }
    let facts = CharacterCastFacts {
        vocation: vocation(row.str("vocation")),
        level: u32::try_from(row.int("level")).expect("level"),
        magic_level: 0,
        max_health: 20_000,
        max_mana: 50_000,
        max_soul: 100,
    };
    seen.insert(
        "usable",
        i64::from(requirements_met(&semantics.use_requirements, facts)),
    );
    seen
}

#[test]
fn potion_rows_follow_their_status_on_the_item_use_owner() {
    let world =
        crate::foundation::WorldId::decode(&[1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1])
            .expect("world");
    let controller = activated_with_loot_tables(world, None).0;
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    let rows = parity_rows::load(HEALING);
    let mut ran = 0;
    for row in rows.iter().filter(|row| row.case.starts_with("potion.")) {
        assert_eq!(row.case, "potion.use", "{}: unknown potion case", row.id);
        let observed = observe(state, row);
        if observed.get("admitted") == Some(&0) {
            assert_eq!(
                row.status,
                parity_rows::Status::Pending,
                "{}: not admitted",
                row.id
            );
            ran += 1;
            continue;
        }
        for resource in ["health", "mana"] {
            if let (Some(low), Some(high)) = (
                observed.get(format!("{resource}_min").as_str()),
                observed.get(format!("{resource}_max").as_str()),
            ) {
                assert!(
                    0 < *low && low <= high,
                    "{}: {resource} {low}..{high}",
                    row.id
                );
            }
        }
        row.judge(&observed);
        ran += 1;
    }
    assert!(ran > 0);
}
