//! Reconcile selectable definitions with source admissions and the World evidence census.
use super::*;
use crate::content::native_gameplay::{NativeGameplayInput, mount_taming, native_cosmetics};
use crate::content::store_cosmetics;

#[derive(Deserialize)]
struct ItemSources {
    provenance: String,
    rules: Vec<ItemSource>,
}

#[derive(Deserialize)]
struct ItemSource {
    item_id: u32,
    donor: String,
    mode: String,
    choices: Vec<ItemChoice>,
}

#[derive(Deserialize)]
struct ItemChoice {
    grants: Vec<String>,
    required: Vec<String>,
}

#[test]
fn shipped_cosmetics_reconcile_source_admissions_and_authored_targets() -> TestResult {
    let root = repository();
    let input = NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
        .map_err(debug)?;
    let cosmetics = native_cosmetics::decode(&input.cosmetics.bytes)
        .map_err(debug)?
        .ok_or("production cosmetics")?;
    let item_bytes = std::fs::read(
        root.join("tools/content-schema/native-gameplay/item-cosmetic-actions-native.json"),
    )?;
    let items: ItemSources = serde_json::from_slice(&item_bytes)?;
    let digest = sha(&item_bytes);
    let mut active_mounts = BTreeSet::new();
    let mut item_grants = BTreeSet::new();
    let mut admitted_sources = BTreeSet::new();
    let mut excluded_sources = BTreeSet::new();
    for rule in &items.rules {
        let key = format!("oteryn:item.tibia.i{}", rule.item_id);
        let admitted = cosmetics.allows_item_acquisition(&key, &rule.donor, digest);
        let source_bound = match rule.mode.as_str() {
            "random_outfit" | "random_mount" => true,
            "direct" | "parts" | "pieces" => {
                rule.donor.starts_with("data-crystal/scripts/special/")
            }
            mode => return Err(format!("unclassified item source mode {mode}").into()),
        };
        if admitted {
            admitted_sources.insert(rule.item_id);
        } else if source_bound {
            excluded_sources.insert(rule.item_id);
        }
        for grant in rule.choices.iter().flat_map(|choice| &choice.required) {
            assert!(
                cosmetics.authored_grant_target(grant).is_some(),
                "{key} has an unknown prerequisite {grant}"
            );
            item_grants.insert(grant.as_str());
        }
        for grant in rule.choices.iter().flat_map(|choice| &choice.grants) {
            let target = cosmetics
                .authored_grant_target(grant)
                .ok_or_else(|| format!("{key} has no authored target for {grant}"))?;
            assert_eq!(
                cosmetics.earned_unlock(grant, &items.provenance, target),
                UnlockFact::Compatible,
                "{key}: {grant}"
            );
            item_grants.insert(grant.as_str());
            if (!source_bound || admitted)
                && let EarnedCosmetic::Mount(mount) = target
            {
                active_mounts.insert(mount);
            }
        }
    }
    // The S06 owner admission covers the addon doll. NX-5's exclusion still
    // applies to the random mount source, even with compatible target mappings.
    assert_eq!(admitted_sources, BTreeSet::from([6579]));
    assert_eq!(excluded_sources, BTreeSet::from([37746]));

    let mut taming_grants = BTreeSet::new();
    for rule in mount_taming::rules() {
        let target = EarnedCosmetic::Mount(&rule.mount_key);
        assert_eq!(
            cosmetics.authored_grant_target(&rule.grant_key),
            Some(target)
        );
        assert_eq!(
            cosmetics.earned_unlock(&rule.grant_key, mount_taming::PROVENANCE, target),
            UnlockFact::Compatible,
            "{}",
            rule.item_key
        );
        active_mounts.insert(rule.mount_key.as_str());
        taming_grants.insert(rule.grant_key.as_str());
    }
    assert!(!taming_grants.is_empty());

    let mut store_outfits = BTreeSet::new();
    for offer in store_cosmetics::offers().ok_or("production cosmetic Store catalogue")? {
        assert!(
            store_cosmetics::compatible(&cosmetics, offer),
            "{}",
            offer.key
        );
        if offer.preview.kind == 2 {
            store_outfits.insert(offer.unlock_key.as_str());
        } else {
            active_mounts.insert(offer.unlock_key.as_str());
        }
    }

    // A remaining mapping is a source-owner obligation. A quest or Task Board
    // namespace identifies the owner, but grants no credit for route reachability.
    let mut outfit_targets = BTreeSet::new();
    for mapping in &cosmetics.document.grant_mappings {
        let key = mapping.grant_key.as_str();
        let target = grant_mappings::primary_target(mapping);
        if key.starts_with("oteryn:taming.") {
            assert!(
                taming_grants.contains(key),
                "taming mapping with no rule: {key}"
            );
        } else if key.starts_with("oteryn:item-grant.") {
            // Include excluded draws and prerequisite-only identities in the
            // identity census without treating them as new acquisitions.
            assert!(
                item_grants.contains(key),
                "item mapping with no source: {key}"
            );
        } else {
            assert!(
                key.contains("/quest-grant/")
                    || key.ends_with("/quest-grant")
                    || key.starts_with("oteryn:appearance-grant/quest/")
                    || key.starts_with("oteryn:taskboard/shop/"),
                "mapping needs a source owner: {key}"
            );
        }
        match target {
            EarnedCosmetic::Outfit(outfit) => {
                outfit_targets.insert((outfit, 0));
            }
            EarnedCosmetic::Addon(outfit, addon) => {
                outfit_targets.insert((outfit, addon));
            }
            EarnedCosmetic::Mount(_) => {}
        }
    }

    for outfit in &cosmetics.document.outfits {
        for component in 0..=outfit.addon_count {
            let starter = [Sex::Female, Sex::Male].into_iter().all(|sex| {
                if component == 0 {
                    cosmetics.outfit_is_starter(&outfit.key, sex)
                } else {
                    cosmetics.addon_is_starter(&outfit.key, sex, component)
                }
            });
            // An exact look pair accounts for a retained definition in the
            // source census. Selection still needs the authored alias binding.
            let has_source = store_outfits.contains(outfit.key.as_str())
                || outfit_targets.iter().any(|(key, part)| {
                    *part == component
                        && cosmetics.look_type(key, Sex::Female) == Some(outfit.female.id)
                        && cosmetics.look_type(key, Sex::Male) == Some(outfit.male.id)
                });
            assert!(
                starter || has_source,
                "{} component {component} needs a source owner",
                outfit.key
            );
        }
    }
    reconcile_mounts(&root, &cosmetics, &active_mounts)?;
    Ok(())
}

fn reconcile_mounts(
    root: &std::path::Path,
    cosmetics: &crate::content::native_gameplay::NativeCosmeticsView,
    active: &BTreeSet<&str>,
) -> TestResult {
    const PREFIX: &str = "oteryn:content.mount.";
    let census = std::fs::read_to_string(
        root.join("docs/reference/content-prep/S06-mount-sources-2026-11-25.md"),
    )?;
    let mut world = BTreeMap::new();
    for line in census.lines().filter_map(|line| line.strip_prefix("| `")) {
        if let Some((key, rest)) = line.split_once("` | ")
            && let Some(slug) = key.strip_prefix(PREFIX)
            && let Some((class, _)) = rest.split_once(" | ")
        {
            world.insert(slug, class);
        }
    }

    // Equal suffixes reconcile the routine World/editor aliases. Reviewed
    // exceptions are named so a new unmatched identity needs an explicit decision.
    let mut owner_followup = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    for key in cosmetics.mount_keys() {
        if active.contains(key) {
            continue;
        }
        let slug = key.strip_prefix(PREFIX).ok_or("native mount key")?;
        let world_key = match slug {
            "ancient_demonosaur" => "alpha_demonosaur",
            "bright_percht_sleigh_final" => "finished_bright_percht_sleigh",
            "cold_percht_sleigh_final" => "finished_cold_percht_sleigh",
            "dark_percht_sleigh_final" => "finished_dark_percht_sleigh",
            "hellish_demonosaur" => "hell_demonosaur",
            "lady_bug" => "ladybug_mount",
            "noxious_ripptor" => "ripptor",
            "white_gloom_maw" => "gloom_maw_mount",
            "wind_up_loco" => "gloothomotive",
            _ => slug,
        };
        match world
            .get(world_key)
            .or_else(|| world.get(format!("{slug}_mount").as_str()))
            .copied()
        {
            Some("World" | "Store / event") => {
                owner_followup.insert(key);
            }
            Some("No donor source") => {
                unresolved.insert(key);
            }
            // Catalogue-only additions and rental variants have separate owners.
            None if matches!(
                slug,
                "jaracal"
                    | "radiant_nimbus"
                    | "rented_horse"
                    | "rented_horse_brown"
                    | "rented_horse_gray"
            ) =>
            {
                owner_followup.insert(key);
            }
            class => return Err(format!("unreconciled mount {key}: {class:?}").into()),
        }
    }
    assert!(owner_followup.contains("oteryn:content.mount.blue_rolling_barrel"));
    assert!(unresolved.contains("oteryn:content.mount.hellish_demonosaur"));
    Ok(())
}
