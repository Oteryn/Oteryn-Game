//! Extra donor callbacks use the ordinary corpse loot plan and its existing mint receipts.
use crate::combat::{LootDefinitionRef, LootTableDefinition, LootTableEntry};
use crate::content::creature_reward::CreatureRewardTable;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_domain::domain::soulpit::{
    DropFacts, SOUL_CORE_SOURCE_IDS, SOUL_PRISM_SOURCE_ID, SourceMonsterFacts, plan_drop,
    soul_core_name, source_bestiary_candidates, source_monster,
};

/// The donor looks up the literal lower-case monster name; variation bindings must not
/// silently turn an absent core into a different species' core.
fn source_core(table: &CreatureRewardTable, monster_name: &str) -> Option<u32> {
    let name = format!("{} soul core", monster_name.to_lowercase());
    SOUL_CORE_SOURCE_IDS.iter().copied().find(|id| {
        soul_core_name(*id) == Some(name.as_str()) && table.admitted_source_item(*id).is_some()
    })
}

fn slain_source_name(registration_name: &str) -> &str {
    // register_monster_type.name overrides Monster:getName() without changing the
    // registry key (for example Old Wolf -> Wolf). This is exact source metadata,
    // not the separate Soulpit variation/entry alias table.
    source_monster(registration_name).map_or(registration_name, |monster| monster.name)
}

fn category_candidates(registration_name: &str) -> Vec<&'static SourceMonsterFacts> {
    let Some(monster) = source_monster(registration_name) else {
        return Vec::new();
    };
    // monsters.cpp:394-404 enumerates the complete, first-wins race-ID map and
    // compares numeric Bestiary.race. Class text and native-profile availability
    // do not constrain this draw; some real candidates have no Soul Core at all.
    source_bestiary_candidates()
        .filter(|candidate| candidate.race == monster.race)
        .collect()
}

pub(in crate::gameplay_transport) fn soul_core_drops(
    runtime: &ChannelRuntimeV1,
    table: &CreatureRewardTable,
    creature: &str,
    occurrence: [u8; 16],
    fiendish: bool,
    recipient_can_receive_loot: bool,
) -> Option<Vec<LootDefinitionRef>> {
    if !fiendish || !recipient_can_receive_loot {
        return Some(Vec::new());
    }
    let facts = table.encounter_facts(creature)?;
    let percent = |purpose| {
        super::draw(runtime, occurrence, purpose, 1, 100).and_then(|draw| u8::try_from(draw).ok())
    };
    let same_species_roll = percent("SOUL_CORE_SAME_SPECIES")?;
    let core_roll = percent("SOUL_CORE_DROP")?;
    let prism_roll = percent("SOUL_PRISM_DROP")?;
    // Do not filter out missing cores before the draw: the donor can select one and stop.
    let candidates = category_candidates(&facts.display_name);
    let category_core = if candidates.is_empty() {
        None
    } else {
        let high = u32::try_from(candidates.len()).ok()?.checked_sub(1)?;
        let selected = usize::try_from(super::draw(
            runtime,
            occurrence,
            "SOUL_CORE_CATEGORY",
            0,
            high,
        )?)
        .ok()?;
        source_core(table, candidates.get(selected)?.name)
    };
    let plan = plan_drop(DropFacts {
        fiendish,
        recipient_can_receive_loot,
        same_species_roll,
        core_roll,
        prism_roll,
        same_species_core: source_core(table, slain_source_name(&facts.display_name)),
        category_core,
        prism_available: table.admitted_source_item(SOUL_PRISM_SOURCE_ID).is_some(),
    })
    .ok()?;
    let mut items = Vec::new();
    if let Some(id) = plan.soul_core {
        items.push(table.admitted_source_item(id)?.clone());
    }
    if plan.soul_prism {
        items.push(table.admitted_source_item(SOUL_PRISM_SOURCE_ID)?.clone());
    }
    Some(items)
}

pub(in crate::gameplay_transport) fn warden_scroll(
    runtime: &ChannelRuntimeV1,
    table: &CreatureRewardTable,
    occurrence: [u8; 16],
) -> Option<LootDefinitionRef> {
    let id = super::draw(runtime, occurrence, "ECHO_WARDEN_SCROLL", 53751, 53774)?;
    table.admitted_source_item(id).cloned()
}

/// Append only after stamina and ordinary loot modifiers. The callbacks already made their
/// source draws; each selected item is one unit, independent of Gut/Prey/event multipliers.
pub(in crate::gameplay_transport) fn append_callback_loot(
    table: &mut LootTableDefinition,
    items: &[LootDefinitionRef],
) {
    table
        .entries
        .extend(items.iter().map(|item| LootTableEntry {
            item: item.clone(),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(1_000_000),
            details: Default::default(),
        }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_drop_uses_source_race_and_retains_candidates_without_native_profiles_or_cores() {
        let constructs = category_candidates("Cake Golem");
        assert!(
            constructs
                .iter()
                .any(|candidate| candidate.race_id == Some(680))
        );
        let none_race = category_candidates("Terrified Elephant");
        assert!(
            none_race
                .iter()
                .any(|candidate| candidate.race_id == Some(0))
        );
        assert!(
            none_race
                .iter()
                .any(|candidate| candidate.registration_name == "The Unwelcome")
        );
        assert!(
            !none_race
                .iter()
                .any(|candidate| candidate.registration_name == "Elephant")
        );
        assert!(
            category_candidates("Old Wolf")
                .iter()
                .all(|candidate| candidate.race == 0)
        );
        assert!(
            category_candidates("Wolf")
                .iter()
                .all(|candidate| candidate.race != 0)
        );
        assert!(category_candidates("oteryn:creature.wolf").is_empty());
        assert_eq!(slain_source_name("Old Wolf"), "Wolf");
        assert_eq!(slain_source_name("Nomad Blue"), "Nomad");
        assert_eq!(
            slain_source_name("fixture:unknown-source"),
            "fixture:unknown-source"
        );
    }
}
