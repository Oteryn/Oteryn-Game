//! Source startup selectors on the served map. None item means the tile's ground.
//! This restores metadata only; World/quest owners still authorize every action.
use super::*;
use serde::Deserialize;
use std::sync::OnceLock;

type Key = ([u16; 3], Option<u16>);
pub(super) type Bindings = BTreeMap<Key, Binding>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    bindings: Vec<Binding>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    position: [u16; 3],
    item: Option<u16>,
    action: Option<u16>,
    unique: Option<u16>,
    source: Source,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Source {
    Donor(DonorSource),
    Authored(AuthoredSource),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DonorSource {
    repository: String,
    revision: String,
    path: String,
    line: u32,
    sha256: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredSource {
    policy: String,
    sha256: String,
    region: String,
    region_sha256: String,
}
impl Source {
    fn valid(&self) -> bool {
        let hex = |s: &str, len| s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit());
        match self {
            Self::Donor(s) => {
                s.line > 0
                    && !s.repository.is_empty()
                    && s.repository.len() <= 160
                    && !s.path.is_empty()
                    && s.path.len() <= 512
                    && hex(&s.revision, 40)
                    && hex(&s.sha256, 64)
            }
            Self::Authored(s) => {
                s.policy.starts_with("content/quests/")
                    && s.policy.len() <= 512
                    && !s.policy.split('/').any(|p| p == "..")
                    && hex(&s.sha256, 64)
                    && s.region.starts_with("content/world/placements/")
                    && s.region.len() <= 512
                    && !s.region.split('/').any(|p| p == "..")
                    && hex(&s.region_sha256, 64)
            }
        }
    }
}

/// Resolve declared event projection items even when no base tile places them.
pub(super) fn projection_appearances() -> Vec<u16> {
    let Ok(policy) = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/world-event-object-policy.json"
    )) else {
        return Vec::new();
    };
    policy["projections"]
        .as_array()
        .filter(|rows| rows.len() <= 64)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row["appearance"]
                .as_u64()
                .and_then(|id| u16::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

pub(super) fn load() -> Result<&'static Bindings, FactsRefusal> {
    static DATA: OnceLock<Option<Bindings>> = OnceLock::new();
    DATA.get_or_init(|| {
        parse(include_str!(
            "../../../../../../content/quests/native-object-bindings.json"
        ))
    })
    .as_ref()
    .ok_or(FactsRefusal::ObjectBindings)
}

fn parse(raw: &str) -> Option<Bindings> {
    let doc: Document = serde_json::from_str(raw).ok()?;
    if doc.schema != 1 || doc.bindings.len() > 256 {
        return None;
    }
    let mut result = Bindings::new();
    for binding in doc.bindings {
        let source = &binding.source;
        if binding.position[2] > 15
            || binding.item == Some(0)
            || binding.action == Some(0)
            || binding.unique == Some(0)
            || (binding.action.is_none() && binding.unique.is_none())
            || !source.valid()
        {
            return None;
        }
        if result
            .insert((binding.position, binding.item), binding)
            .is_some()
        {
            return None;
        }
    }
    Some(result)
}

pub(super) fn apply(
    bindings: &Bindings,
    pos: TilePos,
    ordinal: usize,
    appearance: u16,
    attrs: &mut Attrs,
) {
    let Ok(z) = u16::try_from(-i16::from(pos.floor)) else {
        return;
    };
    let position = [pos.x, pos.y, z];
    let binding = bindings.get(&(position, Some(appearance))).or_else(|| {
        (ordinal == 0)
            .then(|| bindings.get(&(position, None)))
            .flatten()
    });
    let Some(binding) = binding else {
        return;
    };
    if let Some(action) = binding.action {
        attrs.action = Some(action);
    }
    if let Some(unique) = binding.unique {
        attrs.unique = Some(unique);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_public_nests_require_closed_provenance() -> Result<(), Box<dyn std::error::Error>> {
        let raw = include_str!("../../../../../../content/quests/native-object-bindings.json");
        let bindings = parse(raw).ok_or("bindings")?;
        assert_eq!(
            bindings
                .values()
                .filter(|b| matches!(b.source, Source::Authored(_)))
                .count(),
            2
        );
        let mut document: serde_json::Value = serde_json::from_str(raw)?;
        let row = document["bindings"]
            .as_array_mut()
            .ok_or("array")?
            .iter_mut()
            .find(|b| b["source"]["policy"].is_string())
            .ok_or("authored")?;
        row["source"]["policy"] = "content/quests/../outside.json".into();
        assert!(parse(&serde_json::to_string(&document)?).is_none());
        Ok(())
    }

    #[test]
    fn source_jars_and_preplaced_walls_project_open_close_and_reset()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::map::boot::{self, BootPins};
        use oteryn_world_bundle::{
            sector::{Item, Tile},
            writer,
        };
        let (original, mut bundle) = boot::tests::bundle_with_item_states(
            ("oteryn:item.tibia.i12511", 12510),
            ("item:coin", 1),
            &[
                ("oteryn:item.tibia.i12506", 12505),
                ("oteryn:item.tibia.i1603", 1602),
                ("oteryn:item.tibia.i3514", 3513),
            ],
        );
        let mut decoded = writer::read(&original)?;
        decoded.manifest.world = writer::Extent {
            min_x: 32936,
            min_y: 32721,
            max_x: 32992,
            max_y: 32779,
            floors: vec![-12, -11, -10, -9, -8],
        };
        let mut groups = BTreeMap::new();
        let mut add = |pos: TilePos, palettes: &[u32]| {
            groups
                .entry((pos.floor, pos.y / 32, pos.x / 32))
                .or_insert_with(Vec::new)
                .push(Tile {
                    x: pos.x,
                    y: pos.y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: palettes
                        .iter()
                        .map(|&palette| Item {
                            palette,
                            depth: 0,
                            attrs: Attrs::default(),
                        })
                        .collect(),
                });
        };
        let start = TilePos {
            x: 32990,
            y: 32721,
            floor: -8,
        };
        add(start, &[0]);
        for binding in load()
            .map_err(|e| format!("{e:?}"))?
            .values()
            .filter(|b| b.item == Some(12511))
        {
            add(
                TilePos {
                    x: binding.position[0],
                    y: binding.position[1],
                    floor: -i8::try_from(binding.position[2])?,
                },
                &[0, 3],
            );
        }
        for x in 32941..=32944 {
            add(
                TilePos {
                    x,
                    y: 32754,
                    floor: -12,
                },
                &[0, 6],
            );
        }
        let sectors: Vec<_> = groups
            .into_iter()
            .map(|((floor, sy, sx), mut tiles)| {
                tiles.sort_by_key(|t| (t.y, t.x));
                writer::Sector {
                    floor,
                    sx,
                    sy,
                    tiles,
                }
            })
            .collect();
        let bytes = writer::write(&decoded.manifest, &sectors, &Default::default())?;
        bundle.digest = writer::read(&bytes)?.digest;
        let revision = bundle
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let pins = BootPins {
            bundle,
            map_revision: format!("sha256:{revision}"),
            start,
        };
        let item = |key: &str| {
            let id = key
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|s| s.parse::<u32>().ok());
            match id {
                Some(id) => Some(ItemDefinition {
                    reference: std::num::NonZeroU32::new(id)?,
                    solid: Some(true),
                    blocks_projectile: true,
                    pickupable: false,
                    door: false,
                }),
                None => boot::tests::items(key),
            }
        };
        let world = boot::boot(
            &bytes,
            &pins,
            crate::foundation::WorldId::decode(&[
                1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
            ])?,
            crate::foundation::ChannelId::decode(&[
                1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
            ])?,
            item,
        )?;
        let facts = world.facts();
        assert_eq!(facts.event_objects.len(), 5);
        let object = crate::map::facts::EventObjectProjection {
            position: TilePos {
                x: 32941,
                y: 32754,
                floor: -12,
            },
            appearance: 1603,
            sub_type: 0,
            visible: true,
        };
        assert!(facts.project_event_objects(world.base(), "test-a", &[object]));
        assert!(facts.project_event_objects(world.base(), "test-b", &[object]));
        assert_eq!(
            facts
                .callbacks
                .lock()
                .map_err(|_| "projection lock")?
                .movement_items
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            2
        );
        assert!(facts.project_event_objects(world.base(), "test-a", &[]));
        assert_eq!(
            facts
                .callbacks
                .lock()
                .map_err(|_| "projection lock")?
                .movement_items
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            1
        );
        assert!(facts.project_event_objects(world.base(), "test-b", &[]));
        assert!(
            facts
                .callbacks
                .lock()
                .map_err(|_| "projection lock")?
                .movement_items
                .values()
                .all(Vec::is_empty)
        );

        assert!(facts.project_mummy_objects(world.base(), 5, false, false));
        for &(aid, key) in &facts.event_objects {
            assert!(facts.quest_callback_matches(key, None, Some(aid), None));
            assert_eq!(
                facts
                    .callbacks
                    .lock()
                    .map_err(|_| "projection lock")?
                    .changed[&key]
                    .1
                    .appearance_id,
                12506
            );
        }
        {
            let at = TilePos {
                x: 32941,
                y: 32754,
                floor: -12,
            };
            let reservation = facts
                .reserve_encounter_room(world.base(), at, at)
                .expect("room reservation");
            assert!(facts.quest_callback_blocked(world.base(), at, false));
            assert!(facts.reserve_encounter_room(world.base(), at, at).is_none());
            reservation.release();
            assert!(!facts.quest_callback_blocked(world.base(), at, false));
            let reservation = facts
                .reserve_encounter_room(world.base(), at, at)
                .expect("retry reservation");
            drop(reservation);
            assert!(!facts.quest_callback_blocked(world.base(), at, false));
        }
        for x in 32941..=32944 {
            let pos = TilePos {
                x,
                y: 32754,
                floor: -12,
            };
            assert!(!facts.quest_callback_blocked(world.base(), pos, true));
        }
        assert!(facts.project_mummy_objects(world.base(), 5, false, true));
        assert!(facts.quest_callback_blocked(
            world.base(),
            TilePos {
                x: 32941,
                y: 32754,
                floor: -12
            },
            false
        ));
        assert!(facts.project_mummy_objects(world.base(), 0, true, false));
        for &(_, key) in &facts.event_objects {
            assert_eq!(
                facts
                    .callbacks
                    .lock()
                    .map_err(|_| "projection lock")?
                    .changed[&key]
                    .1
                    .appearance_id,
                12511
            );
        }
        assert!(facts.quest_callback_blocked(
            world.base(),
            TilePos {
                x: 32941,
                y: 32754,
                floor: -12
            },
            false
        ));
        Ok(())
    }
    #[test]
    fn a_missing_tomb_tile_does_not_publish_partial_jar_changes()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::map::boot::{self, BootPins};
        let (bytes, bundle) = boot::tests::bundle_with_item_states(
            ("oteryn:item.tibia.i12511", 12510),
            ("item:coin", 1),
            &[("oteryn:item.tibia.i12506", 12505)],
        );
        let revision = bundle
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let pins = BootPins {
            bundle,
            map_revision: format!("sha256:{revision}"),
            start: TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
        };
        let item = |key: &str| {
            let id = match key {
                "oteryn:item.tibia.i12511" => 12511,
                "oteryn:item.tibia.i12506" => 12506,
                _ => return boot::tests::items(key),
            };
            Some(ItemDefinition {
                reference: std::num::NonZeroU32::new(id)?,
                solid: Some(true),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        };
        let world = boot::boot(
            &bytes,
            &pins,
            crate::foundation::WorldId::decode(&[
                1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
            ])?,
            crate::foundation::ChannelId::decode(&[
                1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
            ])?,
            item,
        )?;
        let decoded = oteryn_world_bundle::bundle::read(&bytes)?;
        let mut facts =
            BundleFacts::palette(&decoded.manifest, item).map_err(|e| format!("{e:?}"))?;
        let mut blocked = BTreeSet::new();
        for sector in &decoded.sectors {
            facts.push(&decoded.manifest, sector, &mut blocked);
        }
        let key = placement_key(
            TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
            1,
        )
        .ok_or("jar key")?;
        facts.event_objects.push((50006, key));
        assert!(!facts.project_mummy_objects(world.base(), 1, false, false));
        assert!(
            facts
                .callbacks
                .lock()
                .map_err(|_| "projection lock")?
                .changed
                .is_empty()
        );
        Ok(())
    }
    #[test]
    fn source_selectors_restore_only_the_bound_item_or_ground() -> Result<(), FactsRefusal> {
        let bindings = load()?;
        assert_eq!(bindings.len(), 16);
        let mut attrs = Attrs::default();
        apply(
            bindings,
            TilePos {
                x: 32991,
                y: 32721,
                floor: -8,
            },
            3,
            12511,
            &mut attrs,
        );
        assert_eq!(attrs.action, Some(50006));
        let mut other = Attrs::default();
        apply(
            bindings,
            TilePos {
                x: 32991,
                y: 32721,
                floor: -8,
            },
            2,
            12491,
            &mut other,
        );
        assert!(other.action.is_none());
        apply(
            bindings,
            TilePos {
                x: 32056,
                y: 31088,
                floor: -7,
            },
            1,
            6679,
            &mut other,
        );
        assert!(other.action.is_none());
        apply(
            bindings,
            TilePos {
                x: 32056,
                y: 31088,
                floor: -7,
            },
            0,
            4602,
            &mut other,
        );
        assert_eq!(other.action, Some(50000));
        Ok(())
    }
}
