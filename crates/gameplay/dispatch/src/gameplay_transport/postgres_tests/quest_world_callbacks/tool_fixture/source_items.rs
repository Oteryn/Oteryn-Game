//! Authoring inputs for the mounted fixture. Existing imported Item identities stay qualified.
use super::TestResult;
use crate::content::native_gameplay::{NativeGameplayInput, PinnedGameplayBytes};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn bind(native: &mut NativeGameplayInput) -> TestResult {
    let native_items = native
        .item_profiles
        .as_mut()
        .ok_or("native item profiles")?;
    let mut document: Value = serde_json::from_slice(&native_items.bytes)?;
    let profiles = document["records"]
        .as_array_mut()
        .ok_or("native profile records")?;
    let template = |id: u32| -> TestResult<Value> {
        let key = format!("oteryn:item.tibia.i{id}");
        profiles
            .iter()
            .find(|row| row["production_definition"]["production_key"] == key)
            .cloned()
            .ok_or_else(|| format!("Item template {id}").into())
    };
    let portable = template(123)?;
    let mut gas = profile(template(33876)?, 2127, "poison gas", 0);
    let mut corpse = profile(template(12526)?, 18174, "dead necromancer", 0);
    // Crystal 201189f8 data/items/items.xml:47-50,155-158,4356-4360 and 38647-38655.
    gas["semantics"]["temporal"]["value"]["duration"]["value"] = json!(2000);
    corpse["semantics"]["container"]["value"]["capacity"]["value"] = json!(20);
    corpse["semantics"]["temporal"]["value"]["duration"]["value"] = json!(300000);
    corpse["semantics"]["temporal"]["value"]["decay_target"]["value"]["key"] =
        json!("oteryn:item.tibia.i18175");
    extend_profile(profiles, gas);
    extend_profile(profiles, corpse);
    for (id, name, weight) in [
        (107, "special flask", 180),
        (113, "dwarven emperor's beard", 50),
        (130, "exploding cookie", 10),
        (135, "special flask", 180),
        (144, "mummy disguise", 900),
        (10320, "book of necromantic rituals", 358),
        (11475, "necromantic robe", 220),
        (18933, "incantation fragment", 10),
        (18934, "incantation scroll", 10),
        (19136, "nail case", 200),
        (19160, "Omrabas' talking skull", 2180),
    ] {
        extend_profile(profiles, profile(portable.clone(), id, name, weight));
    }
    replace_bytes(native_items, &document)?;
    let loot = native.loot_tables.as_mut().ok_or("native Loot table")?;
    let mut document: Value = serde_json::from_slice(&loot.bytes)?;
    for item in document["items"]
        .as_array_mut()
        .ok_or("native Loot item records")?
    {
        if item["item"]["key"] == "oteryn:item.tibia.i18174" {
            item["container_capacity"] = json!(20);
            item["materializable"] = json!(true);
        }
        if matches!(
            item["item"]["key"].as_str(),
            Some("oteryn:item.tibia.i10320" | "oteryn:item.tibia.i11475")
        ) {
            item["materializable"] = json!(true);
        }
        if item["item"]["key"] == "oteryn:item.tibia.i3448" {
            item["stackable"] = json!(true);
        }
    }
    let tables = document["tables"]
        .as_array_mut()
        .ok_or("native Loot tables")?;
    let key = "oteryn:loot.creature.necromancer_servant";
    if !tables.iter().any(|table| table["identity"]["key"] == key) {
        // Crystal 201189f8 data-global/monster/humans/necromancer_servant.lua:66-72.
        let entries = [(3031, 742300, 70), (3448, 742300, 5), (10320, 742300, 1),
            (18933, 1_000_000, 1), (11475, 542300, 1), (3079, 50000, 1)]
            .map(|(id, probability, max)| json!({"item": {"family": "Item", "key": format!("oteryn:item.tibia.i{id}"), "revision": "definition-r1"},
                "probability_ppm": probability, "min_count": 1, "max_count": max}));
        tables.push(
            json!({"identity": {"family": "Loot", "key": key, "revision": "definition-r1"},
            "algorithm": "IndependentBernoulliPpm", "entries": entries}),
        );
    }
    replace_bytes(loot, &document)
}

fn profile(mut base: Value, id: u32, name: &str, weight: u32) -> Value {
    let key = format!("oteryn:item.tibia.i{id}");
    base["production_definition"]["production_key"] = json!(key);
    base["production_binding"]["external_id"] = json!(id.to_string());
    base["production_binding"]["target"]["key"] = json!(key);
    base["authoring"]["item"]["key"] = json!(format!("candidate:item/{id}"));
    base["semantics"]["physical"]["value"]["weight"]["value"] = json!(weight);
    base["semantics"]["presentation"]["value"]["name"]["value"] = json!(name);
    base
}

fn extend_profile(profiles: &mut Vec<Value>, profile: Value) {
    if !profiles
        .iter()
        .any(|row| row["production_definition"] == profile["production_definition"])
    {
        profiles.push(profile);
    }
}

fn replace_bytes(pin: &mut PinnedGameplayBytes, document: &Value) -> TestResult {
    pin.bytes = serde_json::to_vec(document)?;
    pin.sha256 = Sha256::digest(&pin.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(())
}
