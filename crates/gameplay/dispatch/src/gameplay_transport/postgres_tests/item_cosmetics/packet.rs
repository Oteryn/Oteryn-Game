//! Explicit synthetic authoring used only by W19d PostgreSQL tests.
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet;
use oteryn_gameplay_items::item_cosmetics::{self as rules, GrantKind};
use serde_json::{Value, json};

pub(super) const COMPETING_GRANT: &str = "oteryn:appearance-grant/npc/competing-steed-proposed";
pub(super) const COMPETING_PROVENANCE: &str = "synthetic-npc-grant-v1";
pub(super) const FUNGUS: &str = "oteryn:outfit.item_use_fungus_proposed";
pub(super) const AFFLICTED: &str = "oteryn:outfit.item_use_afflicted_proposed";
pub(super) const MOUNT: &str = "oteryn:mount.item_use_steed_proposed";

pub(super) fn source_look(packet: &Value, look: u32) -> TestResult<Value> {
    for shard in packet["source_shards"].as_array().ok_or("shards")? {
        let source: Value = serde_json::from_str(shard["json"].as_str().ok_or("source JSON")?)?;
        for record in source["records"].as_array().ok_or("records")? {
            if record["source_id"] == look {
                return Ok(
                    json!({"id":look,"source_record_sha256":record["source_record_sha256"]}),
                );
            }
        }
    }
    Err(format!("the pinned client has no test look {look}").into())
}

pub(super) fn cosmetics_packet() -> TestResult<Vec<u8>> {
    let catalogue = rules::catalogue().ok_or("source cosmetic rules")?;
    let steed = catalogue
        .grants
        .iter()
        .find(|g| g.key == "oteryn:item-grant.mount.184")
        .ok_or("source steed")?;
    let mut packet = cosmetics_test_packet();
    let looks = [430, 431, 514, 516, steed.look_type.ok_or("mount look")?]
        .into_iter()
        .map(|look| Ok((look, source_look(&packet, look)?)))
        .collect::<TestResult<std::collections::BTreeMap<_, _>>>()?;
    let author = &mut packet["authoring"];
    author["document"]["content_revision"] = "content-1".into();
    for (key, female, male) in [(FUNGUS, 514, 516), (AFFLICTED, 430, 431)] {
        let mut outfit = author["document"]["outfits"][0].clone();
        outfit["key"] = key.into();
        outfit["female"] = looks[&female].clone();
        outfit["male"] = looks[&male].clone();
        outfit["premium"] = false.into();
        outfit["starter_male"] = false.into();
        outfit["starter_female"] = false.into();
        outfit["starter_addons_male"] = 0.into();
        outfit["starter_addons_female"] = 0.into();
        author["document"]["outfits"]
            .as_array_mut()
            .ok_or("outfits")?
            .push(outfit);
    }
    author["document"]["mounts"][0]["key"] = MOUNT.into();
    author["document"]["mounts"][0]["premium"] = false.into();
    author["document"]["mounts"][0]["look"] = looks[&steed.look_type.ok_or("mount look")?].clone();
    let mut mappings: Vec<_> = catalogue.grants.iter().filter_map(|grant| {
        let target = match grant.kind {
            GrantKind::Mount if grant.mount_id == Some(184) => json!({"kind":"Mount","mount_key":MOUNT}),
            GrantKind::Outfit | GrantKind::Addon => {
                let key = match grant.look_types.as_slice() {
                    [514, 516] => FUNGUS,
                    [430, 431] => AFFLICTED,
                    _ => return None,
                };
                if grant.kind == GrantKind::Outfit {
                    json!({"kind":"Outfit","outfit_key":key})
                } else {
                    json!({"kind":"Addon","outfit_key":key,"addon":grant.addon})
                }
            }
            _ => return None,
        };
        Some(json!({"grant_key":grant.key,"target":target,"compatible_provenance":[rules::PROVENANCE]}))
    }).collect();
    mappings.push(json!({"grant_key":COMPETING_GRANT,"target":{"kind":"Mount","mount_key":MOUNT},"compatible_provenance":[COMPETING_PROVENANCE]}));
    author["document"]["grant_mappings"] = mappings.into();
    let originals = author["promotions"].as_array().ok_or("promotions")?.clone();
    let mut promotions = originals.clone();
    for index in 1..3 {
        for original in &originals {
            if let Some(field) = original["path"]
                .as_str()
                .ok_or("path")?
                .strip_prefix("/outfits/0/")
            {
                let mut promotion = original.clone();
                promotion["path"] = format!("/outfits/{index}/{field}").into();
                promotions.push(promotion);
            }
        }
    }
    let mut mapping = originals.first().ok_or("promotion template")?.clone();
    mapping["path"] = "/grant_mappings".into();
    promotions.push(mapping);
    for promotion in &mut promotions {
        let path = promotion["path"].as_str().ok_or("path")?;
        promotion["value"] = author["document"]
            .pointer(path)
            .ok_or("promoted value")?
            .clone();
    }
    author["promotions"] = promotions.into();
    Ok(serde_json::to_vec(&packet)?)
}
