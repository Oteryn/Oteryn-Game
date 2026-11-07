//! Public read-model metadata only; the checked map determines active physical houses.
use super::*;
use crate::durability::public_projections::house_catalogue::{
    PublicHouseCatalogue, PublicHouseMetadata,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SHARDS: [&str; 2] = [
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/houses/houses-00000-00499.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/houses/houses-00500-00994.json"
    )),
];
#[derive(Deserialize)]
struct Shard {
    houses: Vec<House>,
}
#[derive(Deserialize)]
struct Identity {
    key: String,
}
#[derive(Deserialize)]
struct Ref {
    key: String,
}
#[derive(Deserialize)]
struct Provenance {
    engine_house: EngineHouse,
}
#[derive(Deserialize)]
struct EngineHouse {
    engine: String,
    house_id: u32,
    revision: String,
}
#[derive(Deserialize)]
struct House {
    identity: Identity,
    name: String,
    kind: String,
    town: Ref,
    size_sqm: u32,
    beds: u16,
    rent_gold: u64,
    provenance: Provenance,
    tiles: Vec<[u16; 3]>,
    #[serde(default)]
    retired: bool,
}
fn metadata(map: &crate::map::boot::BundleWorld) -> Result<Vec<PublicHouseMetadata>, ()> {
    // House catalogue owner contract §3: names/town/size/beds/rent are committed
    // client facts; engine house ids are a source join only, never public identity.
    let mut expected = map.facts().house_ids().collect::<BTreeSet<_>>();
    let mut keys = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut records = Vec::new();
    let mut digest = Sha256::new();
    digest.update(b"oteryn:public-house-catalogue:v2\0");
    digest.update(map.map_revision().as_bytes());
    for shard in SHARDS {
        digest.update(shard.as_bytes());
    }
    let digest: String = digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    for raw in SHARDS {
        let shard: Shard = serde_json::from_str(raw).map_err(|_| ())?;
        for h in shard.houses {
            let source = &h.provenance.engine_house;
            if source.engine != "crystalserver"
                || source.revision != "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
                || source.house_id == 0
                || !source_ids.insert(source.house_id)
                || !keys.insert(h.identity.key.clone())
            {
                return Err(());
            }
            if !expected.remove(&source.house_id) {
                continue;
            }
            // A donor numeric id alone cannot select metadata for another physical address.
            if !h.tiles.iter().any(|[x, y, z]| {
                i8::try_from(*z).ok().filter(|z| *z <= 15).is_some_and(|z| {
                    map.facts().house_id(crate::map::overlay::TilePos {
                        x: *x,
                        y: *y,
                        floor: -z,
                    }) == Some(source.house_id)
                })
            }) {
                return Err(());
            }
            if h.retired {
                continue;
            }
            if !crate::native_admission_source::public_projections::projection_name_valid(&h.name)
                || h.size_sqm == 0
                || h.rent_gold == 0
                || h.rent_gold > i64::MAX as u64
            {
                return Err(());
            }
            records.push(PublicHouseMetadata {
                house_key: h.identity.key,
                name: h.name,
                kind: h.kind,
                town: h.town.key,
                size_sqm: h.size_sqm,
                beds: h.beds,
                rent_gold: h.rent_gold,
                content_digest: digest.clone(),
            });
        }
    }
    if !expected.is_empty() {
        return Err(());
    }
    records.sort_unstable_by(|a, b| a.house_key.cmp(&b.house_key));
    Ok(records)
}

/// Source failure holds the house watermark; publication never gates gameplay.
pub(super) async fn capture(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    material: &Material,
    assignment: &RuntimeScopeAssignment,
    reset_epoch: u64,
    bundle: Option<&crate::map::boot::BundleWorld>,
) {
    if !material
        .config
        .platform
        .public_projections
        .as_ref()
        .is_some_and(|p| p.enabled)
    {
        return;
    }
    let update = |houses| PublicHouseCatalogue {
        world: material.world,
        channel: material.channel,
        generation: assignment.ownership_generation,
        reset_epoch,
        map_revision: bundle.map_or_else(String::new, |b| b.map_revision().to_owned()),
        houses,
    };
    if root
        .sync_public_house_catalogue(authority, node, update(None))
        .await
        .is_err()
    {
        event("event=public_projections family=house source=catalogue result=unavailable");
        return;
    }
    let records = match bundle.map(metadata).transpose() {
        Ok(Some(rows)) => rows,
        Ok(None) => Vec::new(),
        Err(()) => {
            event("event=public_projections family=house source=catalogue result=unavailable");
            return;
        }
    };
    if root
        .sync_public_house_catalogue(authority, node, update(Some(records)))
        .await
        .is_err()
    {
        event("event=public_projections family=house source=catalogue result=unavailable");
    }
}
