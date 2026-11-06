//! The native Hydra Egg nest has no source UID. Its exact position and
//! appearance are cross-checked in samples/native-rewards/hydra-egg.json.
use super::{BoundCell, BoundPlacement, RewardClaimBase, RewardClaimTile, ServedClaim};
use crate::content::{DefinitionFamily, DefinitionRevisionRef, PlacementKey, ProductionKey,
    RewardClaimItem, RewardClaimPlacement, TypedDefinitionRef};

/// Bind this one source nest, failing closed on missing or ambiguous roots.
/// `matches` reads the immutable server map facts, never client-supplied IDs.
pub fn bind_hydra_egg(
    base: &impl RewardClaimBase,
    matches: impl Fn(u64, u16) -> bool,
) -> Option<ServedClaim> {
    let tile = base.tile(32954, 32695, -8)?;
    let mut selected = None;
    for (ordinal, _) in tile.depths().iter().filter(|&&depth| depth == 0).enumerate() {
        let key = oteryn_world_bundle::bundle::placement_key(-8, 32954, 32695,
            u8::try_from(ordinal).ok()?)?;
        if matches(key, 5676) && selected.replace(key).is_some() {
            return None;
        }
    }
    let bundle_placement_key = selected?;
    Some(ServedClaim {
        key: ProductionKey::new("oteryn:reward-claim.quest.hydra_egg.nest").ok()?,
        revision: DefinitionRevisionRef::new("hydra-egg-native-r1").ok()?,
        placements: vec![BoundPlacement {
            placement: RewardClaimPlacement {
                placement: PlacementKey::new("oteryn:placement/reward-claim.quest.hydra_egg.nest/0").ok()?,
                items: vec![RewardClaimItem {
                    item: TypedDefinitionRef::new(DefinitionFamily::Item,
                        ProductionKey::new("oteryn:item.tibia.i4839").ok()?,
                        DefinitionRevisionRef::new("definition-r1").ok()?),
                    count: 1,
                }],
                achievement: None, quest_transition: None, key_number: None,
            },
            cell: BoundCell { x: 32954, y: 32695, floor: -8 },
            bundle_placement_key,
            contents: Vec::new(), root_text: None, root_fluid_subtype: None,
            root_charges_remaining: None,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Base;
    struct Tile;
    impl RewardClaimBase for Base {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            ((x,y,floor) == (32954,32695,-8)).then_some(Tile)
        }
    }
    impl RewardClaimTile for Tile {
        fn depths(&self) -> &[u8] { &[0, 0] }
        fn unique(&self, _: u8) -> Option<super::super::UniqueEntry> { None }
    }

    #[test]
    fn hydra_nest_requires_one_exact_native_appearance_and_no_fabricated_uid() -> Result<(), &'static str> {
        let nest = oteryn_world_bundle::bundle::placement_key(-8,32954,32695,1).ok_or("key")?;
        let bound = bind_hydra_egg(&Base, |key, appearance| key == nest && appearance == 5676)
            .ok_or("native nest")?;
        assert_eq!(bound.placements[0].bundle_placement_key, nest);
        assert_eq!(bound.placements[0].placement.items[0].item.key().as_str(), "oteryn:item.tibia.i4839");
        assert!(bind_hydra_egg(&Base, |_, _| false).is_none());
        assert!(bind_hydra_egg(&Base, |_, _| true).is_none(), "ambiguous roots refuse");
        Ok(())
    }
}
