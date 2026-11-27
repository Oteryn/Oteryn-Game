use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpseExceptions {
    // Exact profile keys whose served source declares no body.
    no_body: BTreeMap<String, String>,
    // Exact body references whose ground-container proof is still incomplete.
    held_body: BTreeMap<String, String>,
}

#[test]
fn every_creature_corpse_admits_or_has_an_explicit_source_exception()
-> Result<(), Box<dyn std::error::Error>> {
    let (creatures, section) = protected_loot_tests::documents()?;
    let items = serde_json::from_slice(include_bytes!(
        "../../../../../content/items/definitions/spell-native-profiles.json"
    ))?;
    let mut exceptions: CorpseExceptions = serde_json::from_slice(include_bytes!(
        "../../../../../tools/content-schema/native-gameplay/verified-loot-corpse-exceptions.json"
    ))?;
    let rewards =
        CreatureRewardTable::from_pin_with_native_corpses(&creatures, Some(&section), Some(&items));
    let mut failures = Vec::new();
    for record in &creatures.records {
        let key = &record.profile.target.key;
        let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
            failures.push(format!("{key}: expected a Creature profile"));
            continue;
        };
        let declared = profile
            .details
            .as_ref()
            .and_then(|details| details.corpse_item.as_ref());
        let result = rewards.row(key);
        match (declared, result) {
            (None, Ok(row)) if row.corpse_item.is_none() => {
                // Runtime-authorized corpse-free encounter child.
                if !exceptions
                    .no_body
                    .remove(key)
                    .is_some_and(|reason| !reason.is_empty())
                {
                    failures.push(format!(
                        "{key}: corpse-free encounter child has no exception"
                    ));
                }
            }
            (None, Err(NoSettlementReason::CorpseItemMissing)) => {
                if !exceptions
                    .no_body
                    .remove(key)
                    .is_some_and(|reason| !reason.is_empty())
                {
                    failures.push(format!("{key}: absent corpse has no source exception"));
                }
            }
            (Some(corpse), Err(NoSettlementReason::CorpseItemInadmissible)) => {
                if !exceptions.held_body.remove(key).is_some_and(|entry| {
                    entry
                        .split_once(": ")
                        .is_some_and(|(body, reason)| body == corpse.key && !reason.is_empty())
                }) {
                    failures.push(format!("{key}: {} refused corpse admission", corpse.key));
                }
            }
            (Some(corpse), Ok(row))
                if row.corpse_item.as_ref().is_some_and(|item| {
                    item.production_key == corpse.key && item.revision_ref == corpse.revision
                }) => {}
            // These outcomes are decided after the corpse admission check. The
            // loot tests cover their separate table and yield qualifications.
            (
                Some(_),
                Err(
                    NoSettlementReason::LootItemInadmissible | NoSettlementReason::LootTableMissing,
                ),
            ) => {}
            (_, result) => failures.push(format!("{key}: unexpected reward result {result:?}")),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(
        exceptions.no_body.is_empty(),
        "stale no-body exceptions: {:?}",
        exceptions.no_body
    );
    assert!(
        exceptions.held_body.is_empty(),
        "stale held-body exceptions: {:?}",
        exceptions.held_body
    );
    Ok(())
}

#[test]
fn braindeath_and_bug_have_lootable_ground_corpses() -> Result<(), Box<dyn std::error::Error>> {
    let (creatures, section) = protected_loot_tests::documents()?;
    let items: crate::content::native_gameplay::ItemProfilesDocument = serde_json::from_slice(
        include_bytes!("../../../../../content/items/definitions/spell-native-profiles.json"),
    )?;
    let rewards =
        CreatureRewardTable::from_pin_with_native_corpses(&creatures, Some(&section), Some(&items));
    for (creature, key) in [
        ("oteryn:creature.braindeath", "oteryn:item.tibia.i7256"),
        ("oteryn:creature.bug", "oteryn:item.tibia.i5990"),
    ] {
        let row = rewards.row(creature).map_err(NoSettlementReason::as_str)?;
        assert_eq!(
            row.corpse_item
                .as_ref()
                .map(|body| body.production_key.as_str()),
            Some(key)
        );
        assert!(!row.loot_table.entries.is_empty(), "{creature}");
        let fact = section
            .items
            .iter()
            .find(|fact| fact.item.key == key)
            .ok_or("corpse facts")?;
        assert!(fact.materializable);
        assert_eq!(fact.container_capacity, Some(10));
        let policy = items
            .records
            .iter()
            .find(|profile| profile.production_definition.production_key == key)
            .ok_or("native corpse policy")?;
        assert!(policy.admission.materializable);
        assert_eq!(
            policy.admission.legal_destinations,
            [crate::content::native_gameplay::NativeItemDestination::Ground]
        );
    }
    Ok(())
}

#[test]
fn dragon_lord_capacity_requires_matching_ground_policy() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::content::native_gameplay::{ItemProfilesDocument, NativeItemDestination};

    let (creatures, section) = protected_loot_tests::documents()?;
    let mut items: ItemProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../content/items/definitions/spell-native-profiles.json"
    ))?;
    let corpse = "oteryn:item.tibia.i5984";
    let dragon = "oteryn:creature.dragon_lord";
    let rewards =
        CreatureRewardTable::from_pin_with_native_corpses(&creatures, Some(&section), Some(&items));
    assert_eq!(
        verified_source_corpses(Some(&items))
            .large_containers
            .get(corpse),
        Some(&56)
    );
    assert_ne!(
        rewards.row(dragon),
        Err(NoSettlementReason::CorpseItemInadmissible)
    );
    let profile = items
        .records
        .iter_mut()
        .find(|profile| profile.production_definition.production_key == corpse)
        .ok_or("Dragon Lord native corpse profile")?;
    profile.admission.legal_destinations = vec![NativeItemDestination::CharacterInventory];
    let rewards =
        CreatureRewardTable::from_pin_with_native_corpses(&creatures, Some(&section), Some(&items));
    assert_eq!(
        rewards.row(dragon),
        Err(NoSettlementReason::CorpseItemInadmissible)
    );
    Ok(())
}
