//! Source chests whose exact native objects have no legacy script UID.
//! Reward shapes, item admission and once identities remain in the existing catalogue.
use super::*;

/// Bind only the source-backed Beregar, Elvenbane and smithhammer objects.
/// `matches` reads immutable server facts for the actual top-level palette entry.
pub fn bind_standalone_chests(
    base: &impl RewardClaimBase,
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
    matches: impl Fn(u64, u32, u16) -> bool,
) -> Result<Vec<ServedClaim>, ClaimsError> {
    let sources = [
        (
            "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.gold_coin",
            6079,
            32778,
            32253,
            -8,
            2472,
        ),
        (
            "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer",
            6078,
            32776,
            32253,
            -8,
            2472,
        ),
        (
            "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.talon",
            6077,
            32774,
            32253,
            -8,
            2472,
        ),
        (
            "oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.bp1",
            6255,
            32588,
            31645,
            -3,
            2434,
        ),
        (
            "oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.bp2",
            6256,
            32588,
            31644,
            -3,
            2434,
        ),
        (
            "oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.dwarven_shield",
            6253,
            32591,
            31647,
            -3,
            2472,
        ),
        (
            "oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.morning_star",
            6254,
            32590,
            31647,
            -3,
            2472,
        ),
        (
            "oteryn:reward-claim.quest.u8_4.the_hidden_city_of_beregar.bpthe_undeath_stare",
            6283,
            32546,
            31522,
            -11,
            1983,
        ),
    ];
    let mut served = Vec::new();
    for shard in REWARD_CLAIM_SHARDS {
        let document: Value =
            serde_json::from_str(shard).map_err(|_| ClaimsError("not a JSON document"))?;
        let records = document["records"]
            .as_array()
            .ok_or(ClaimsError("no records"))?;
        for row in records {
            let definition = row["definition"]
                .as_object()
                .ok_or(ClaimsError("no definition"))?;
            let Some(source) = sources
                .iter()
                .find(|source| row["definition"]["identity"]["key"] == source.0)
            else {
                continue;
            };
            let Ok(candidate) = candidate(definition, admitted)? else {
                continue;
            };
            let [placement] = candidate.placements.as_slice() else {
                continue;
            };
            let cell = BoundCell {
                x: source.2,
                y: source.3,
                floor: source.4,
            };
            if placement.cell != Some(cell)
                || placement.crystal_unique_id != Some(source.1)
                || placement.appearance != u64::from(source.5)
            {
                continue;
            }
            let Some(tile) = base.tile(cell.x, cell.y, cell.floor) else {
                continue;
            };
            let mut found = None;
            let mut ambiguous = false;
            for ordinal in 0..tile.depths().iter().filter(|&&depth| depth == 0).count() {
                let Ok(ordinal) = u8::try_from(ordinal) else {
                    ambiguous = true;
                    break;
                };
                let Some(id) = tile.palette_id(ordinal) else {
                    ambiguous = true;
                    break;
                };
                let Some(key) = bundle::placement_key(cell.floor, cell.x, cell.y, ordinal) else {
                    ambiguous = true;
                    break;
                };
                if matches(key, id, source.5) && found.replace(key).is_some() {
                    ambiguous = true;
                    break;
                }
            }
            let Some(bundle_placement_key) = found.filter(|_| !ambiguous) else {
                continue;
            };
            let local = source
                .0
                .strip_prefix("oteryn:")
                .ok_or(ClaimsError("claim namespace"))?;
            served.push(ServedClaim {
                key: ProductionKey::new(source.0)?,
                revision: DefinitionRevisionRef::new(&candidate.revision)?,
                placements: vec![BoundPlacement {
                    placement: RewardClaimPlacement {
                        placement: PlacementKey::new(&format!("oteryn:placement/{local}/0"))?,
                        items: vec![RewardClaimItem {
                            item: placement.item.clone(),
                            count: placement.count,
                        }],
                        achievement: placement.achievement.clone(),
                        quest_transition: placement.quest_transition.clone(),
                        key_number: placement.key_number,
                    },
                    cell,
                    bundle_placement_key,
                    contents: placement.contents.clone(),
                    root_text: placement.root_text.clone(),
                    root_fluid_subtype: placement.root_fluid_subtype,
                    root_charges_remaining: placement.root_charges_remaining,
                }],
            });
        }
    }
    Ok(served)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Base;
    struct Tile;

    impl RewardClaimBase for Base {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            ((x, y, floor) == (32546, 31522, -11)).then_some(Tile)
        }
    }

    impl RewardClaimTile for Tile {
        fn depths(&self) -> &[u8] {
            &[0, 0]
        }
        fn unique(&self, _: u8) -> Option<UniqueEntry> {
            None
        }
        fn palette_id(&self, ordinal: u8) -> Option<u32> {
            (ordinal < 2).then_some(u32::from(ordinal))
        }
    }

    struct Elvenbane;
    impl RewardClaimBase for Elvenbane {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            ([
                (32588, 31644, -3),
                (32588, 31645, -3),
                (32590, 31647, -3),
                (32591, 31647, -3),
            ]
            .contains(&(x, y, floor)))
            .then_some(Tile)
        }
    }
    #[test]
    fn elvenbane_native_drawers_keep_both_filled_bags_and_all_children() -> Result<(), ClaimsError>
    {
        let drawer = |_, id, appearance| id == 1 && appearance == 2434;
        let claims = bind_standalone_chests(&Elvenbane, &|_| true, drawer)?;
        assert_eq!(claims.len(), 2);
        for claim in &claims {
            assert_eq!(
                claim.placements[0].placement.items[0].item.key().as_str(),
                "oteryn:item.tibia.i2853"
            );
        }
        assert_eq!(claims[0].placements[0].contents.len(), 3);
        assert_eq!(claims[1].placements[0].contents.len(), 2);
        let held = bind_standalone_chests(
            &Elvenbane,
            &|item| item.key().as_str() != "oteryn:item.tibia.i3147",
            drawer,
        )?;
        assert_eq!(held.len(), 1);
        assert!(held[0].key.as_str().ends_with(".bp2"));
        Ok(())
    }

    #[test]
    fn standalone_native_chests_require_one_root_and_every_admitted_reward()
    -> Result<(), ClaimsError> {
        let exact = |_, id, appearance| id == 1 && appearance == 1983;
        let served = bind_standalone_chests(&Base, &|_| true, exact)?;
        assert_eq!(served.len(), 1);
        let coffin = &served[0].placements[0];
        assert_eq!(coffin.contents.len(), 5);
        assert_eq!(
            coffin.placement.items[0].item.key().as_str(),
            "oteryn:item.tibia.i2867"
        );
        assert_eq!(coffin.contents[0].count, 13);
        assert!(bind_standalone_chests(&Base, &|_| true, |_, _, _| false)?.is_empty());
        assert!(bind_standalone_chests(&Base, &|_| true, |_, _, _| true)?.is_empty());
        assert!(
            bind_standalone_chests(
                &Base,
                &|item| item.key().as_str() != "oteryn:item.tibia.i239",
                exact
            )?
            .is_empty()
        );
        Ok(())
    }
}
