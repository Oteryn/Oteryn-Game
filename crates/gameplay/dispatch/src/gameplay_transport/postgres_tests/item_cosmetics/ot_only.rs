//! Complete synthetic random pools: prove exclusion even when every donor grant resolves.
//! Derived in memory; never writes or promotes NX-5's production cosmetic files.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet;
use oteryn_gameplay_items::item_cosmetics::{self as rules, GrantKind};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn packet() -> TestResult<Vec<u8>> {
    let catalogue = rules::catalogue().ok_or("cosmetic source rules")?;
    let mut packet = cosmetics_test_packet();
    let original = packet["authoring"].clone();
    let mut outfits = original["document"]["outfits"]
        .as_array()
        .ok_or("outfits")?
        .clone();
    let mut mounts = original["document"]["mounts"]
        .as_array()
        .ok_or("mounts")?
        .clone();
    let mut outfit_keys = BTreeMap::from([(
        vec![128, 136],
        outfits[0]["key"].as_str().ok_or("starter key")?.to_owned(),
    )]);
    let mut mount_keys = BTreeMap::from([(
        139,
        mounts[0]["key"].as_str().ok_or("mount key")?.to_owned(),
    )]);
    let required: BTreeSet<&str> = catalogue
        .rules
        .iter()
        .filter(|rule| [6579, 37746].contains(&rule.item_id))
        .flat_map(|rule| &rule.choices)
        .flat_map(|choice| {
            choice
                .grants
                .iter()
                .chain(&choice.required)
                .chain(&choice.complete_grants)
                .chain(std::iter::once(&choice.owned))
        })
        .map(String::as_str)
        .collect();
    let mut mappings = Vec::new();
    for grant in catalogue
        .grants
        .iter()
        .filter(|grant| required.contains(grant.key.as_str()))
    {
        let target = match grant.kind {
            GrantKind::Outfit | GrantKind::Addon => {
                let key = if let Some(key) = outfit_keys.get(&grant.look_types) {
                    key.clone()
                } else {
                    let [female, male] = grant.look_types.as_slice() else {
                        return Err("outfit pair".into());
                    };
                    let key = format!("oteryn:outfit.srx4_proposed_{female}_{male}");
                    let mut outfit = outfits[0].clone();
                    outfit["key"] = key.clone().into();
                    outfit["female"] = super::packet::source_look(&packet, *female)?;
                    outfit["male"] = super::packet::source_look(&packet, *male)?;
                    outfit["premium"] = false.into();
                    outfit["starter_male"] = false.into();
                    outfit["starter_female"] = false.into();
                    outfit["starter_addons_male"] = 0.into();
                    outfit["starter_addons_female"] = 0.into();
                    outfits.push(outfit);
                    outfit_keys.insert(grant.look_types.clone(), key.clone());
                    key
                };
                if grant.kind == GrantKind::Outfit {
                    json!({"kind":"Outfit","outfit_key":key})
                } else {
                    json!({"kind":"Addon","outfit_key":key,"addon":grant.addon})
                }
            }
            GrantKind::Mount => {
                let look = grant.look_type.ok_or("mount look")?;
                let key = if let Some(key) = mount_keys.get(&look) {
                    key.clone()
                } else {
                    let key = format!("oteryn:mount.srx4_proposed_{look}");
                    let mut mount = mounts[0].clone();
                    mount["key"] = key.clone().into();
                    mount["look"] = super::packet::source_look(&packet, look)?;
                    mount["premium"] = false.into();
                    mounts.push(mount);
                    mount_keys.insert(look, key.clone());
                    key
                };
                json!({"kind":"Mount","mount_key":key})
            }
        };
        mappings.push(json!({"grant_key":grant.key,"target":target,"compatible_provenance":[rules::PROVENANCE]}));
    }
    outfits.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
    mounts.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
    let author = &mut packet["authoring"];
    author["document"]["outfits"] = outfits.into();
    author["document"]["mounts"] = mounts.into();
    author["document"]["grant_mappings"] = mappings.into();
    let originals = original["promotions"].as_array().ok_or("promotions")?;
    let mut promotions = Vec::new();
    for promotion in originals {
        let path = promotion["path"].as_str().ok_or("promotion path")?;
        let copies = if path.starts_with("/outfits/0/") {
            author["document"]["outfits"]
                .as_array()
                .ok_or("outfits")?
                .len()
        } else if path.starts_with("/mounts/0/") {
            author["document"]["mounts"]
                .as_array()
                .ok_or("mounts")?
                .len()
        } else {
            1
        };
        for index in 0..copies {
            let path = path.replacen("/0/", &format!("/{index}/"), 1);
            let mut copy = promotion.clone();
            copy["path"] = path.clone().into();
            copy["value"] = author["document"]
                .pointer(&path)
                .ok_or("promoted value")?
                .clone();
            promotions.push(copy);
        }
    }
    let mut mapping = originals.first().ok_or("promotion template")?.clone();
    mapping["path"] = "/grant_mappings".into();
    mapping["value"] = author["document"]["grant_mappings"].clone();
    promotions.push(mapping);
    author["promotions"] = promotions.into();
    validate_candidate(&packet)?;
    Ok(serde_json::to_vec(&packet)?)
}

pub(super) fn qualify_complete_pool(native: &NativeGameplayState) -> TestResult {
    let content = native.cosmetics().ok_or("synthetic cosmetic catalogue")?;
    qualify_view(content)
}

fn qualify_view(content: &crate::content::native_gameplay::NativeCosmeticsView) -> TestResult {
    let catalogue = rules::catalogue().ok_or("cosmetic source rules")?;
    for id in [6579, 37746] {
        let rule = rules::rule(&format!("oteryn:item.tibia.i{id}")).ok_or("OT rule")?;
        assert!(
            rules::qualify_rule(catalogue, rule, content).is_ok(),
            "every random grant must resolve before owner exclusion: {id}"
        );
    }
    Ok(())
}

// Retain the underlying compiler diagnostic if fixture assembly ever drifts.
fn validate_candidate(packet: &serde_json::Value) -> TestResult {
    use crate::content::native_gameplay::cosmetics_loader::{
        ProposedCosmeticsBinding, compile_candidate,
    };
    use sha2::{Digest, Sha256};
    let mut document = packet["authoring"]["document"].clone();
    document["schema"] = "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1".into();
    let appearances = document["source_appearances_sha256"]
        .as_str()
        .ok_or("appearances digest")?;
    let mut digest = [0; 32];
    for (i, out) in digest.iter_mut().enumerate() {
        *out = u8::from_str_radix(&appearances[i * 2..i * 2 + 2], 16)?;
    }
    let bytes = serde_json::to_vec(&document)?;
    let manifest = packet["source_manifest"].as_str().ok_or("manifest")?;
    let shards = packet["source_shards"]
        .as_array()
        .ok_or("shards")?
        .iter()
        .map(|s| {
            Ok((
                s["path"].as_str().ok_or("shard path")?,
                s["json"].as_str().ok_or("shard JSON")?.as_bytes(),
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    compile_candidate(
        &bytes,
        ProposedCosmeticsBinding {
            document_sha256: Sha256::digest(&bytes).into(),
            content_revision: document["content_revision"]
                .as_str()
                .ok_or("content revision")?,
            manifest_sha256: Sha256::digest(manifest.as_bytes()).into(),
            appearances_sha256: digest,
            client_version: "15.30",
        },
        manifest.as_bytes(),
        &shards,
    )
    .map_err(debug)?;
    Ok(())
}

#[test]
fn srx4_complete_synthetic_random_pools_pass_the_real_cosmetic_decoder() -> TestResult {
    let bytes = packet()?;
    let content =
        crate::content::native_gameplay::test_support::cosmetics_fixture::decode_cosmetics(&bytes)
            .map_err(debug)?;
    qualify_view(&content)
}
