//! Crystal201189f soul_war.lua:296-331 and soul_war_mechanics.lua:93-120.
//! Entry removes Weeping Soul corpses; a newly allocated Instance with the exact
//! empty source layout needs no removals. The native party-death transaction
//! applies SoulWarBossesDeath to the actual damage contributors.
use super::activity_template::BossLeverCandidate;
use crate::content::native_gameplay::NativeGameplayState;
use serde::Deserialize;

const SPITE_KEY: &str = "oteryn:creature.goshnar_s_spite";

/// Source setup capability consumed only by the fresh Instance spawn owner.
#[derive(Debug)]
pub struct QualifiedSpiteClosure {
    digest: [u8; 32],
}

impl QualifiedSpiteClosure {
    pub fn source_digest(&self) -> [u8; 32] {
        self.digest
    }

    pub(super) fn implements(&self, hook: &str) -> bool {
        hook == "SoulWarBossesDeath"
    }

    pub(super) fn implements_setup(&self, hook: &str) -> bool {
        hook == "config.onUseExtra"
    }
}

#[derive(Deserialize)]
struct SourceRegion {
    tiles: Vec<SourceTile>,
}

#[derive(Deserialize)]
struct SourceTile {
    source_position: [i32; 3],
    items: Vec<SourceItem>,
}

#[derive(Deserialize)]
struct SourceItem {
    server_item_id: u32,
}

pub(super) fn qualify(
    native: &NativeGameplayState,
    candidate: &BossLeverCandidate,
) -> Option<QualifiedSpiteClosure> {
    if candidate.creature_key.as_deref() != Some(SPITE_KEY)
        || candidate.path != "data-global/lib/quests/soul_war.lua"
        || candidate.sha256 != "89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75"
        || candidate.line != 296
        || candidate.required_callbacks != ["SoulWarBossesDeath"]
        || candidate.blockers != ["config.onUseExtra"]
        || !candidate.callbacks_known
    {
        return None;
    }
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|row| row.profile.target.key == SPITE_KEY)?;
    if !super::primal_closure::abilities_closed(record)
        || native
            .reward_table()
            .row(SPITE_KEY)
            .ok()?
            .corpse_item
            .as_ref()?
            .production_key
            != "oteryn:item.tibia.i33867"
    {
        return None;
    }
    for id in [33864, 33865, 33866, 33867, 33876] {
        native.item_policy(&format!("oteryn:item.tibia.i{id}"), "definition-r1")?;
    }
    let low = candidate.zone_from?;
    let high = candidate.zone_to?;
    let source: SourceRegion = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/crystal-endgame-regions.json"
    ))
    .ok()?;
    if source.tiles.len() != 3683
        || source.tiles.iter().any(|tile| {
            low.iter()
                .zip(tile.source_position)
                .zip(high)
                .all(|((a, p), b)| *a <= p && p <= b)
                && tile.items.iter().any(|item| item.server_item_id == 33876)
        })
    {
        return None;
    }
    Some(QualifiedSpiteClosure {
        digest: native.source_digest(),
    })
}
