//! N--Q source chests through the production USE dispatcher and durable item owner.
//! These cases qualify reward execution from committed map selectors; they do not
//! certify the entire source quest journey.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world, wire_use_targets};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

struct Chest {
    claim: &'static str,
    position: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    reward: &'static str,
    quantity: u32,
    contents: &'static [(&'static str, i64, i32)],
    text_digest: Option<&'static str>,
}

struct Gate {
    quest: &'static str,
    position: (u16, u16, i8),
    appearance: u16,
    level: u32,
}

const NAGINATA_GATE: Gate = Gate {
    quest: "Naginata Quest",
    position: (32353, 32073, -11),
    appearance: 1646,
    level: 40,
};
const NOBLE_ARMOR_GATE: Gate = Gate {
    quest: "Noble Armor Quest",
    position: (32448, 32042, -8),
    appearance: 6264,
    level: 35,
};
const ORC_FORTRESS_GATE: Gate = Gate {
    quest: "Orc Fortress Quest",
    position: (32981, 31760, -9),
    appearance: 6264,
    level: 40,
};
const PANPIPE_DOOR: (u16, u16, i8) = (32643, 32128, -8);

const NAGINATA_ROCK: (u16, u16, i8) = (32356, 32074, -10);

// Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
// data-global/startup/tables/chest.lua:1602-1616,1627-1634,2877-2885.
const NAGINATA: Chest = Chest {
    claim: "oteryn:reward-claim.kv.naginata",
    position: (32346, 32063, -12),
    unique: 6135,
    appearance: 2472,
    reward: "oteryn:item.tibia.i3314",
    quantity: 1,
    contents: &[],
    text_digest: None,
};
const NIGHTMARE_TEDDY: Chest = Chest {
    claim: "oteryn:reward-claim.kv.nightmare_teddy_quest",
    position: (33444, 32605, -11),
    unique: 6305,
    appearance: 2433,
    reward: "oteryn:item.tibia.i21982",
    quantity: 1,
    contents: &[],
    text_digest: None,
};
const NOBLE_ARMOR: Chest = Chest {
    claim: "oteryn:reward-claim.kv.noblearmor1",
    position: (32451, 32048, -8),
    unique: 6132,
    appearance: 2472,
    reward: "oteryn:item.tibia.i3380",
    quantity: 1,
    contents: &[],
    text_digest: None,
};
const CROWN_HELMET: Chest = Chest {
    claim: "oteryn:reward-claim.kv.noblearmor2",
    position: (32455, 32048, -8),
    unique: 6133,
    appearance: 2472,
    reward: "oteryn:item.tibia.i3385",
    quantity: 1,
    contents: &[],
    text_digest: None,
};

const PANPIPE_ACCESS_KEY: Chest = Chest {
    claim: "oteryn:reward-claim.quest.key.id4055",
    position: (32652, 32107, -7),
    unique: 5001,
    appearance: 1777,
    reward: "oteryn:item.tibia.i2969",
    quantity: 1,
    contents: &[],
    text_digest: None,
};

// Pinned Crystal startup chest table; exact roots and source-ordered children.
const PHASE_A_LEAVES: &[Chest] = &[
    Chest {
        claim: "oteryn:reward-claim.kv.orcfortress1",
        position: (32980, 31727, -9),
        unique: 6020,
        appearance: 2469,
        reward: "oteryn:item.tibia.i3318",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.orcfortress2",
        position: (32981, 31727, -9),
        unique: 6021,
        appearance: 2469,
        reward: "oteryn:item.tibia.i3370",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.orcfortress3",
        position: (32985, 31727, -9),
        unique: 6022,
        appearance: 2469,
        reward: "oteryn:item.tibia.i3280",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.platearmor",
        position: (33327, 32182, -7),
        unique: 6173,
        appearance: 2474,
        reward: "oteryn:item.tibia.i3357",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.purpletome1",
        position: (32424, 31591, -15),
        unique: 6128,
        appearance: 2436,
        reward: "oteryn:item.tibia.i2822",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.purpletome2",
        position: (32427, 31591, -15),
        unique: 6129,
        appearance: 2436,
        reward: "oteryn:item.tibia.i2823",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.purpletome3",
        position: (32421, 31594, -15),
        unique: 6130,
        appearance: 2438,
        reward: "oteryn:item.tibia.i2848",
        quantity: 1,
        contents: &[],
        text_digest: None,
    },
];
const PHASE_A_CONTAINERS: &[Chest] = &[
    Chest {
        claim: "oteryn:reward-claim.kv.orcshaman",
        position: (33089, 32030, -9),
        unique: 6150,
        appearance: 2469,
        reward: "oteryn:item.tibia.i2853",
        quantity: 1,
        contents: &[
            ("oteryn:item.tibia.i3046", 1, 0),
            ("oteryn:item.tibia.i3092", 1, 0),
            ("oteryn:item.tibia.i3147", 1, 0),
        ],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.panpipe",
        position: (32644, 32131, -8),
        unique: 6065,
        appearance: 2472,
        reward: "oteryn:item.tibia.i2853",
        quantity: 1,
        contents: &[
            ("oteryn:item.tibia.i3033", 2, 0),
            ("oteryn:item.tibia.i3050", 1, 0),
            ("oteryn:item.tibia.i2953", 1, 0),
        ],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.poisondaggers",
        position: (33155, 31880, -11),
        unique: 6068,
        appearance: 2473,
        reward: "oteryn:item.tibia.i2854",
        quantity: 1,
        contents: &[
            ("oteryn:item.tibia.i3448", 30, 0),
            ("oteryn:item.tibia.i3299", 1, 0),
            ("oteryn:item.tibia.i3299", 1, 0),
        ],
        text_digest: None,
    },
    Chest {
        claim: "oteryn:reward-claim.kv.powerbolt1",
        position: (32816, 32279, -8),
        unique: 6170,
        appearance: 4240,
        reward: "oteryn:item.tibia.i2853",
        quantity: 1,
        contents: &[
            ("oteryn:item.tibia.i3450", 5, 0),
            ("oteryn:item.tibia.i3449", 12, 0),
        ],
        text_digest: None,
    },
];
const PHASE_A_KEY_CONTAINER: &[Chest] = &[Chest {
    claim: "oteryn:reward-claim.quest.u7_2.parchment_room.bag",
    position: (33063, 31624, -15),
    unique: 6076,
    appearance: 2474,
    reward: "oteryn:item.tibia.i2853",
    quantity: 1,
    contents: &[
        ("oteryn:item.tibia.i2972", 1, 6010),
        ("oteryn:item.tibia.i3114", 1, 0),
        ("oteryn:item.tibia.i3034", 2, 0),
        ("oteryn:item.tibia.i3049", 1, 0),
        ("oteryn:item.tibia.i3115", 1, 0),
    ],
    text_digest: None,
}];

const PHASE_A_BOOK: &[Chest] = &[Chest {
    claim: "oteryn:reward-claim.kv.powerbolt2",
    position: (32814, 32281, -8),
    unique: 6112,
    appearance: 4240,
    reward: "oteryn:item.tibia.i2821",
    quantity: 1,
    contents: &[],
    text_digest: Some("9ea894f92d1b6e4cf60555cd08f5e552b8c5ce59c11c13eb6d35f7ff07abb036"),
}];

fn phase_a_chests() -> impl Iterator<Item = &'static Chest> {
    PHASE_A_LEAVES
        .iter()
        .chain(PHASE_A_CONTAINERS)
        .chain(PHASE_A_BOOK)
        .chain(PHASE_A_KEY_CONTAINER)
}

#[test]
fn quest_reward_claims_phase_a_leaf_cohort() -> TestResult {
    execute_cohort(PHASE_A_LEAVES)
}

#[test]
fn quest_reward_claims_phase_a_container_cohort() -> TestResult {
    execute_cohort(PHASE_A_CONTAINERS)
}

#[test]
fn quest_reward_claims_phase_a_book_cohort() -> TestResult {
    execute_cohort(PHASE_A_BOOK)
}

#[test]
fn quest_reward_claims_phase_a_key_container_cohort() -> TestResult {
    execute_cohort(PHASE_A_KEY_CONTAINER)
}

#[test]
fn quest_reward_claims_batch_one_orc_fortress_all_rewards_and_level_gate() -> TestResult {
    execute_gate(&ORC_FORTRESS_GATE)?;
    execute_cohort(&PHASE_A_LEAVES[..3])
}

#[test]
fn quest_reward_claims_batch_one_orc_shaman_bag_exact_contents() -> TestResult {
    execute(&PHASE_A_CONTAINERS[0])
}

#[test]
fn quest_reward_claims_batch_one_panpipe_bag_exact_contents() -> TestResult {
    execute(&PHASE_A_CONTAINERS[1])
}

#[test]
fn quest_reward_claims_batch_one_plate_armor_coffin_once_and_replay() -> TestResult {
    execute(&PHASE_A_LEAVES[3])
}

#[test]
fn quest_reward_claims_batch_one_purple_tome_all_three_bookcases() -> TestResult {
    execute_cohort(&PHASE_A_LEAVES[4..])
}

#[test]
fn quest_reward_claims_batch_two_poison_daggers_two_instances_once_and_replay() -> TestResult {
    execute(&PHASE_A_CONTAINERS[2])
}

#[test]
fn quest_reward_claims_batch_two_power_bolts_book_and_ammunition() -> TestResult {
    execute(&PHASE_A_CONTAINERS[3])?;
    execute_cohort(PHASE_A_BOOK)
}

#[test]
fn quest_reward_claims_batch_two_parchment_room_key_bag() -> TestResult {
    execute_cohort(PHASE_A_KEY_CONTAINER)
}

#[test]
fn quest_reward_claims_panpipe_key_4055_once_and_replay() -> TestResult {
    execute(&PANPIPE_ACCESS_KEY)
}

#[test]
fn quest_reward_claims_panpipe_locked_door_to_reward_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(&PHASE_A_CONTAINERS[1], None, false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Panpipe access scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED Panpipe locked door to reward elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn execute_cohort(chests: &'static [Chest]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let mut failures = Vec::new();
    for chest in chests {
        if let Err(error) = execute(chest) {
            failures.push(format!("{}: {error}", chest.claim));
        }
    }
    if failures.is_empty() {
        eprintln!(
            "POSTGRES_CASE_COMPLETED quest reward claims cohort first={} claims={} elapsed_ms={}",
            chests[0].claim,
            chests.len(),
            started.elapsed().as_millis()
        );
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}

fn committed_attrs(
    position: (u16, u16, i8),
    appearance: u16,
) -> TestResult<oteryn_world_bundle::sector::Attrs> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (x, y, floor) = position;
    let path = root.join(format!(
        "content/world/placements/region-z{:02}-x{:03}-y{:03}.b3",
        -floor,
        x / 256,
        y / 256
    ));
    let mut budget = oteryn_world_bundle::bundle::BUNDLE_BUDGET;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        &std::fs::read(path)?,
        oteryn_world_bundle::bundle::TILE_LIMITS,
        &mut budget,
    )?;
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let entries: Vec<_> = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .filter(|tile| tile.x == x && tile.y == y)
        .flat_map(|tile| &tile.items)
        .filter(|item| {
            item.depth == 0
                && index["palette"][item.palette as usize]["source_item_id"] == appearance
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "one committed source entry at {position:?}"
    );
    Ok(entries[0].attrs.clone())
}

#[test]
fn quest_reward_claims_committed_map_selectors_match_source() -> TestResult {
    assert_eq!(committed_attrs(NAGINATA_ROCK, 1791)?.action, Some(40031));
    for chest in [
        &NAGINATA,
        &NIGHTMARE_TEDDY,
        &NOBLE_ARMOR,
        &CROWN_HELMET,
        &PANPIPE_ACCESS_KEY,
    ]
    .into_iter()
    .chain(phase_a_chests())
    {
        assert_eq!(
            committed_attrs(chest.position, chest.appearance)?.unique,
            Some(chest.unique),
            "{}",
            chest.claim
        );
    }
    for gate in [&NAGINATA_GATE, &NOBLE_ARMOR_GATE, &ORC_FORTRESS_GATE] {
        assert_eq!(
            committed_attrs(gate.position, gate.appearance)?.action,
            Some(u16::try_from(1000 + gate.level)?),
            "{}",
            gate.quest
        );
    }
    Ok(())
}

#[test]
fn quest_reward_claims_source_selectors_bind_exactly() -> TestResult {
    for chest in [
        &NAGINATA,
        &NIGHTMARE_TEDDY,
        &NOBLE_ARMOR,
        &CROWN_HELMET,
        &PANPIPE_ACCESS_KEY,
    ]
    .into_iter()
    .chain(phase_a_chests())
    {
        for (unique, appearance, offset, expected) in [
            (chest.unique, chest.appearance, 0, true),
            (chest.unique + 1, chest.appearance, 0, false),
            (chest.unique, chest.appearance + 1, 0, false),
            (chest.unique, chest.appearance, 1, false),
        ] {
            let world = source_world(chest.position, unique, appearance, offset)?;
            let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
                world.base().as_ref(),
                &|_| true,
            )?;
            assert_eq!(
                report
                    .served
                    .iter()
                    .any(|row| row.key.as_str() == chest.claim),
                expected,
                "{}: unique={unique}, appearance={appearance}, offset={offset}",
                chest.claim
            );
        }
    }
    Ok(())
}

#[test]
fn quest_reward_claims_naginata_use_once_and_replay() -> TestResult {
    execute(&NAGINATA)
}

#[test]
fn quest_reward_claims_naginata_pick_six_shared_hits_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(&NAGINATA, None, true, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "quest reward claims pick scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED quest reward claims Naginata pick elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn quest_reward_claims_nightmare_teddy_use_once_and_replay() -> TestResult {
    execute(&NIGHTMARE_TEDDY)
}

#[test]
fn quest_reward_claims_noble_armor_use_once_and_replay() -> TestResult {
    execute(&NOBLE_ARMOR)
}

#[test]
fn quest_reward_claims_noble_armor_crown_helmet_use_once_and_replay() -> TestResult {
    execute(&CROWN_HELMET)
}

fn execute(chest: &'static Chest) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(chest, None, false, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "quest reward claims reward scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED quest reward claims {} elapsed_ms={}",
        chest.claim,
        started.elapsed().as_millis()
    );
    Ok(())
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

// Use the actual committed entry and its complete attributes, with a small
// approach corridor so the ordinary production map owner handles the action.
fn committed_world(
    position: (u16, u16, i8),
    appearance: u16,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let attrs = committed_attrs(position, appearance)?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    let index = native.item_index().ok_or("native item index")?;
    let key = format!("oteryn:item.tibia.i{appearance}");
    let reference = index
        .definition_ref("Item", &key, "definition-r1")
        .ok_or("committed item definition")?;
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        key,
        family: bundle::Family::Item,
        id: reference.get() - 1,
        terrain: None,
    });
    let (x, y, floor) = position;
    data.manifest.world = bundle::Extent {
        min_x: x - 1,
        min_y: y,
        max_x: x + 2,
        max_y: y + 1,
        floors: vec![floor],
    };
    let mut sectors = std::collections::BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for tile_x in x - 1..=x + 1 {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: sector::Attrs::default(),
        }];
        if tile_x == x {
            items.push(sector::Item {
                palette: 1,
                depth: 0,
                attrs: attrs.clone(),
            });
        }
        sectors
            .entry((floor, tile_x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x: tile_x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items,
            });
    }
    if position == NAGINATA_ROCK {
        // Source positions and complete selectors for the first special access
        // step, two floor changes, level gate and final chest. Only the approach
        // corridors are reduced; none of the mechanic steps is bypassed.
        for (appearance, terrain) in [(1646, false), (2472, false), (385, true), (386, true)] {
            data.manifest.palette.push(if terrain {
                bundle::PaletteEntry {
                    key: format!("oteryn:terrain.tibia.i{appearance}"),
                    family: bundle::Family::Terrain,
                    id: if appearance == 385 { 1 } else { 2 },
                    terrain: Some(bundle::Terrain {
                        kind: bundle::TerrainKind::Ground,
                        walkable: Some(true),
                        ground_speed: Some(100),
                        blocks_projectile: None,
                    }),
                }
            } else {
                let key = format!("oteryn:item.tibia.i{appearance}");
                bundle::PaletteEntry {
                    id: index
                        .definition_ref("Item", &key, "definition-r1")
                        .ok_or("journey Item")?
                        .get()
                        - 1,
                    key,
                    family: bundle::Family::Item,
                    terrain: None,
                }
            });
        }
        let mut route = vec![
            (32355, 32074, -10),
            (32356, 32074, -10),
            (32356, 32075, -10),
            (32356, 32074, -11),
            (32355, 32074, -11),
            (32354, 32074, -11),
            (32354, 32073, -11),
            (32353, 32073, -11),
            (32352, 32073, -11),
            (32351, 32073, -11),
            (32350, 32073, -11),
            (32350, 32074, -11),
            (32350, 32075, -11),
            (32346, 32063, -12),
        ];
        route.extend((32064..=32074).map(|y| (32350, y, -12)));
        route.extend((32346..32350).map(|x| (x, 32064, -12)));
        sectors.clear();
        for (x, y, floor) in route {
            let ground = match (x, y, floor) {
                (32356, 32074, -10) | (32350, 32074, -11) => 4,
                (32356, 32074, -11) | (32350, 32074, -12) => 5,
                _ => 0,
            };
            let mut items = vec![sector::Item {
                palette: ground,
                depth: 0,
                attrs: Default::default(),
            }];
            let placed = match (x, y, floor) {
                NAGINATA_ROCK => Some((1, 1791)),
                (32353, 32073, -11) => Some((2, 1646)),
                (32346, 32063, -12) => Some((3, 2472)),
                _ => None,
            };
            if let Some((palette, appearance)) = placed {
                items.push(sector::Item {
                    palette,
                    depth: 0,
                    attrs: committed_attrs((x, y, floor), appearance)?,
                });
            }
            sectors
                .entry((floor, x / 32, y / 32))
                .or_default()
                .push(sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items,
                });
        }
        data.manifest.world = bundle::Extent {
            min_x: 32345,
            min_y: 32062,
            max_x: 32358,
            max_y: 32077,
            floors: vec![-12, -11, -10],
        };
    }
    if position == PANPIPE_DOOR {
        let chest = &PHASE_A_CONTAINERS[1];
        let chest_key = format!("oteryn:item.tibia.i{}", chest.appearance);
        data.manifest.palette.push(bundle::PaletteEntry {
            id: index
                .definition_ref("Item", &chest_key, "definition-r1")
                .ok_or("Panpipe chest Item")?
                .get()
                - 1,
            key: chest_key,
            family: bundle::Family::Item,
            terrain: None,
        });
        sectors.clear();
        // Start north of the source door; its only route south reaches the source chest.
        // The long dungeon approach is outside this access/reward scenario.
        for (x, y) in [
            (32643, 32127),
            (32643, 32128),
            (32643, 32129),
            (32643, 32130),
            (32644, 32130),
            (32644, 32131),
        ] {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (x, y) == (32643, 32128) {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: attrs.clone(),
                });
            } else if (x, y) == (32644, 32131) {
                items.push(sector::Item {
                    palette: 2,
                    depth: 0,
                    attrs: committed_attrs(chest.position, chest.appearance)?,
                });
            }
            sectors
                .entry((floor, x / 32, y / 32))
                .or_default()
                .push(sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items,
                });
        }
        data.manifest.world = bundle::Extent {
            min_x: 32642,
            min_y: 32126,
            max_x: 32646,
            max_y: 32133,
            floors: vec![-8],
        };
    }
    let sectors: Vec<_> = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: if position == PANPIPE_DOOR {
                crate::map::overlay::TilePos { x, y: y - 1, floor }
            } else {
                crate::map::overlay::TilePos { x: x - 1, y, floor }
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            use crate::content::{ReferenceItemField, ReferenceItemType};
            let policy = native.item_policy(key, "definition-r1")?.record();
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: matches!(&policy.semantics.physical, ReferenceItemField::Known(p)
                    if p.pickupable == ReferenceItemField::Known(true)),
                door: matches!(&policy.semantics.classification, ReferenceItemField::Known(c)
                    if c.item_type == ReferenceItemField::Known(ReferenceItemType::Door)),
            })
        },
    )
    .map_err(debug)
}

#[test]
fn quest_reward_claims_naginata_level_gate_requires_40() -> TestResult {
    execute_gate(&NAGINATA_GATE)
}

#[test]
fn quest_reward_claims_noble_armor_level_gate_requires_35() -> TestResult {
    execute_gate(&NOBLE_ARMOR_GATE)
}

#[test]
fn quest_reward_claims_phase_a_orc_fortress_level_gate_requires_40() -> TestResult {
    execute_gate(&ORC_FORTRESS_GATE)
}

fn execute_gate(gate: &'static Gate) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario(&NAGINATA, Some(gate), false, false).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "quest reward claims gate scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED quest reward claims {} level_gate elapsed_ms={}",
        gate.quest,
        started.elapsed().as_millis()
    );
    Ok(())
}

fn journey_target(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    position: (u16, u16, i8),
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let key =
        oteryn_world_bundle::bundle::placement_key(position.2, position.0, position.1, ordinal)
            .ok_or("journey placement")?;
    let map = owner.bundle_map().ok_or("journey map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn journey_step(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    direction: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction).to_vec(),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    Ok(())
}

async fn duplicate_pick(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    payload: &[u8],
) -> TestResult<crate::gameplay_transport::connection::AdmittedSession> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::AsyncWriteExt;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async {
        loop {
            let bytes = super::item_look::read_frame(&mut client).await?;
            if decode_wire_envelope(&bytes)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: 1,
                command_type: 2,
                payload,
            },
        )?;
        for expected in [CommandStatus::Accepted, CommandStatus::DuplicateReplay] {
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let bytes = super::item_look::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&bytes)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                    assert_eq!(result.command_id, 1);
                    assert_eq!(result.status, expected);
                    assert_eq!(
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?,
                        UseDisposition::Committed
                    );
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, replied) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(
            async {
                let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
                drop(server);
                end
            },
            peer,
        ),
    )
    .await?;
    replied?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => Ok(next),
        _ => Err("pick duplicate connection did not disconnect normally".into()),
    }
}

async fn journey_use(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

// The separately qualified hollow-stone case proves the real Key4055 mint.
// This case seeds carried custody only, then drives every door/reward action over the wire.
async fn panpipe_access(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, encode_item_use_intent,
    };
    use oteryn_protocol_oteryn::world_spatial::StepDirection::{East, South};
    let door = journey_target(owner, PANPIPE_DOOR, 1)?;
    let map = owner.bundle_map().ok_or("Panpipe map")?;
    let position = crate::map::overlay::TilePos {
        x: PANPIPE_DOOR.0,
        y: PANPIPE_DOOR.1,
        floor: PANPIPE_DOOR.2,
    };
    assert!(map.facts.doors().blocks(position));
    assert_eq!(
        journey_use(owner, admitted, &door).await?,
        UseDisposition::Locked
    );
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(door)),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("Panpipe index")?;
    for (item, expected) in [
        ("oteryn:item.tibia.i2968", UseDisposition::KeyMismatch),
        ("oteryn:item.tibia.i2969", UseDisposition::Committed),
    ] {
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        let command = admitted.continuity.next_command_id;
        let (next, result, actual) = super::item_craft::exchange(
            owner,
            *admitted,
            command,
            &payload,
            index.definition_ref("Item", item, "definition-r1"),
        )
        .await?;
        assert_eq!(result, expected);
        *admitted = next;
        if expected == UseDisposition::Committed {
            let (next, replay, _) =
                super::item_craft::exchange(owner, *admitted, command, &actual, None).await?;
            *admitted = next;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(
                map.facts.doors().revision(
                    oteryn_world_bundle::bundle::placement_key(-8, 32643, 32128, 1)
                        .ok_or("Panpipe door key")?
                ),
                1
            );
        } else {
            assert!(map.facts.doors().blocks(position));
        }
    }
    assert!(!map.facts.doors().blocks(position));
    for direction in [South, South, South, East] {
        journey_step(owner, admitted, direction).await?;
    }
    let chest = journey_target(owner, PHASE_A_CONTAINERS[1].position, 1)?;
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    assert_eq!(
        journey_use(owner, admitted, &chest).await?,
        UseDisposition::Committed
    );
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    assert_eq!(
        journey_use(owner, admitted, &chest).await?,
        UseDisposition::NothingToUse
    );
    Ok(())
}

async fn naginata_journey(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection::{East, North, South, West};
    journey_step(owner, admitted, East).await?;
    let actor = admitted.runtime_actor.ok_or("journey actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
            .floor,
        11
    );
    // Rope back out and descend again. This uses the carried Rope and committed
    // rope spot, not a fixture teleport or direct position overwrite.
    let rope = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i3003", "definition-r1"))
        .ok_or("journey rope")?;
    let rope_target = journey_target(owner, (32356, 32074, -11), 0)?;
    let intent = oteryn_protocol_oteryn::world_object::UseIntent {
        target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(rope),
        use_with: None,
        use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
            rope_target,
        )),
        use_with_position: None,
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        None,
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "rope ascends to the dragon lair"
    );
    *admitted = next;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
            .floor,
        10
    );
    journey_step(owner, admitted, North).await?;
    for direction in [West, West, North] {
        journey_step(owner, admitted, direction).await?;
    }
    let gate = journey_target(owner, NAGINATA_GATE.position, 1)?;
    assert_eq!(
        journey_use(owner, admitted, &gate).await?,
        UseDisposition::Committed
    );
    for direction in [West, West, West, West, South] {
        journey_step(owner, admitted, direction).await?;
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
            .floor,
        12
    );
    for _ in 0..10 {
        journey_step(owner, admitted, North).await?;
    }
    for _ in 0..4 {
        journey_step(owner, admitted, West).await?;
    }
    let chest = journey_target(owner, NAGINATA.position, 1)?;
    assert_eq!(
        journey_use(owner, admitted, &chest).await?,
        UseDisposition::Committed
    );
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    assert_eq!(
        journey_use(owner, admitted, &chest).await?,
        UseDisposition::NothingToUse
    );
    Ok(())
}

fn scenario(chest: &Chest, gate: Option<&Gate>, pick: bool, panpipe: bool) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "questpc2_reward", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native generation")?;
        deps.room = activated.qualified_room().clone();
        let (position, appearance) = if panpipe {
            assert_eq!(committed_attrs(PANPIPE_DOOR, 1631)?.action, Some(4055));
            (PANPIPE_DOOR, 1631)
        } else if pick {
            (NAGINATA_ROCK, 1791)
        } else {
            gate.map_or((chest.position, chest.appearance), |gate| {
                (gate.position, gate.appearance)
            })
        };
        let world = committed_world(position, appearance, native)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let (x, y, floor) = position;
        let start = if panpipe {
            (i32::from(x), i32::from(y) - 1, -i16::from(floor))
        } else {
            (i32::from(x) - 1, i32::from(y), -i16::from(floor))
        };
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            start,
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: start.0,
                    y: start.1,
                    floor: start.2,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(world.frame_binding_digest(deps.admitted.world_id).as_slice()).execute(&harness.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness
            .root
            .initialize_admission_equipment(
                &authority,
                &harness.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: definition("oteryn:item.tibia.i2854"),
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        if panpipe {
            super::item_look::seed_main_backpack(
                &harness,
                &[
                    ("oteryn:item.tibia.i2969", 1),
                    ("oteryn:item.tibia.i2968", 1),
                ],
            )
            .await?;
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (item, number) in [(71, 4055), (72, 4056)] {
                sqlx::query("INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id) VALUES(encode($1,'hex')::uuid,$2,game_character_uuid_v7())")
                    .bind(id(item).as_slice()).bind(number).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        } else if pick {
            super::item_look::seed_main_backpack(
                &harness,
                &[
                    ("oteryn:item.tibia.i3456", 1),
                    ("oteryn:item.tibia.i3003", 1),
                ],
            )
            .await?;
        } else {
            harness
                .root
                .grant_starter_items(
                    &authority,
                    &harness.node,
                    &fence,
                    &NativeItemWeights(native),
                    &template,
                )
                .await
                .map_err(debug)?;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key =
            oteryn_world_bundle::bundle::placement_key(floor, x, y, 1).ok_or("placement key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        if panpipe {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[4, 6, 11, 15, 18],
                )
                .ok_or("Panpipe capabilities")?;
            panpipe_access(&owner, &mut admitted).await?;
            let rewards: Vec<(String,i64)> = sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i3033','oteryn:item.tibia.i3050','oteryn:item.tibia.i2953') AND lifecycle=1 ORDER BY definition_production_key")
                .fetch_all(&harness.pool).await?;
            assert_eq!(
                rewards,
                vec![
                    ("oteryn:item.tibia.i2953".into(), 1),
                    ("oteryn:item.tibia.i3033".into(), 2),
                    ("oteryn:item.tibia.i3050".into(), 1)
                ]
            );
            let claims:i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1").bind(chest.claim).fetch_one(&harness.pool).await?;
            assert_eq!(claims, 1);
            let keys:Vec<(i32,i64)> = sqlx::query_as("SELECT k.key_number,i.quantity FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id) ORDER BY key_number").fetch_all(&harness.pool).await?;
            assert_eq!(keys, vec![(4055, 1), (4056, 1)], "keys are retained");
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        if pick {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            {
                use crate::durability::character_progression::{
                    ExperienceAwardRequest, ExperienceRewardOccurrence,
                };
                let binding = owner
                    .progression_binding(admitted.game_session_id)
                    .ok_or("pick progression")?;
                let current = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "pick fence")?
                    .ok_or("pick current fence")?;
                let before = owner
                    .root
                    .read_character_progression(&authority, current.character_id)
                    .await
                    .map_err(debug)?
                    .ok_or("pick progression state")?;
                let threshold = binding
                    .policy
                    .thresholds
                    .iter()
                    .find(|t| t.level == 40)
                    .ok_or("level40 threshold")?
                    .minimum_experience
                    .get();
                let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
                slot.commit_experience(
                    owner.root,
                    &authority,
                    owner.holder,
                    current,
                    ExperienceAwardRequest {
                        occurrence: ExperienceRewardOccurrence::from_bytes(id(140))
                            .map_err(debug)?,
                        amount: oteryn_simulation_determinism::ExactI64::new(
                            threshold
                                .checked_sub(before.total_experience.get())
                                .ok_or("pick XP")?,
                        ),
                        context: binding.context.clone(),
                        policy_revision: binding.policy_revision.clone(),
                        reward_revision: binding.reward_revision.clone(),
                        policy: binding.policy.clone(),
                    },
                    None,
                )
                .await
                .map_err(debug)?;
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        admitted.game_session_id,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(current.character_id.as_bytes())
                                .map_err(debug)?,
                            current.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
            }
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[4, 6, 11, 15, 18],
                )
                .ok_or("pick USE capabilities")?;
            {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        deps.actor,
                        admitted.game_session_id,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            level: 40,
                            max_health: 1000,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .ok_or("pick health facts")?;
            }
            let tool = native
                .item_index()
                .ok_or("pick item index")?
                .definition_ref("Item", "oteryn:item.tibia.i3456", "definition-r1")
                .ok_or("pick definition")?;
            let original = admitted;
            let mut first_payload = None;
            let mut health = 1000;
            for hit in 0..6 {
                let intent = oteryn_protocol_oteryn::world_object::UseIntent {
                    target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(tool),
                    use_with: None,
                    use_with_map_item: Some(
                        oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
                            WorldObjectTarget {
                                expected_revision: hit,
                                ..target.clone()
                            },
                        ),
                    ),
                    use_with_position: None,
                };
                let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                    .map_err(debug)?;
                if hit == 0 {
                    admitted = duplicate_pick(&owner, admitted, &payload).await?;
                    first_payload = Some(payload);
                } else {
                    let (next, result, _) =
                        super::item_craft::exchange(&owner, admitted, hit + 1, &payload, None)
                            .await?;
                    assert_eq!(result, UseDisposition::Committed, "pick hit {hit}");
                    admitted = next;
                }
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let after = states
                    .get(&runtime, deps.actor, admitted.game_session_id)
                    .ok_or("live pick health")?
                    .vitals()
                    .health;
                assert!(
                    (31..=39).contains(&(health - after)),
                    "each pick deals source damage"
                );
                health = after;
                assert_eq!(world.facts().object_revision(key), hit + 1);
                assert_eq!(
                    world.enterable(crate::map::overlay::TilePos { x, y, floor }),
                    hit == 5
                );
            }
            let payload = first_payload.ok_or("first pick payload")?;
            {
                let runtime = owner.runtime.lock().await;
                assert_eq!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .get(&runtime, deps.actor, original.game_session_id)
                        .ok_or("replay health")?
                        .vitals()
                        .health,
                    health
                );
            }
            naginata_journey(&owner, &mut admitted).await?;
            let rewards: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3314' AND lifecycle=1")
                .fetch_one(&harness.pool).await?;
            assert_eq!(
                rewards, 1,
                "the completed journey mints exactly one Naginata"
            );
            drop(owner);
            // A new connection owner keeps the same Channel map receipt, even
            // though it has no memory of this command's ingress acknowledgement.
            let mut restarted = deps.owner(&harness, &authority);
            restarted.active_generation = Some(generation);
            restarted.movement_cells = (&cells).into();
            deps.initialize(&restarted).await;
            let (_, replay, _) =
                super::item_craft::exchange(&restarted, original, 1, &payload, None).await?;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(world.facts().object_revision(key), 6);
            let quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3456' AND lifecycle=1")
                .fetch_one(&harness.pool).await?;
            assert_eq!(quantity, 1, "the pick is retained");
            drop(restarted);
            let mut altered = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                original.continuity.selected_capabilities.as_slice(),
                &payload,
            )
            .map_err(debug)?;
            let Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(target)) =
                altered.use_with_map_item.as_mut()
            else {
                return Err("pick map target".into());
            };
            target.expected_revision = 1;
            let changed = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&altered)
                .map_err(debug)?;
            let mut changed_owner = deps.owner(&harness, &authority);
            changed_owner.active_generation = Some(generation);
            changed_owner.movement_cells = (&cells).into();
            deps.initialize(&changed_owner).await;
            let (_, refusal, _) =
                super::item_craft::exchange(&changed_owner, original, 1, &changed, None).await?;
            assert_eq!(
                refusal,
                UseDisposition::Rejected,
                "a receipt cannot authorize changed bytes"
            );
            assert_eq!(world.facts().object_revision(key), 6);
            drop(changed_owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        if let Some(gate) = gate {
            use crate::durability::character_progression::{
                ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
            };
            for (index, level) in [gate.level - 1, gate.level].into_iter().enumerate() {
                let mut owner = deps.owner(&harness, &authority);
                owner.active_generation = Some(generation);
                owner.movement_cells = (&cells).into();
                admitted.continuity.selected_capabilities =
                    crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                        owner.offered_capabilities(),
                        &[4, 6, 11, 15, 18],
                    )
                    .ok_or("door USE capabilities")?;
                owner.admit_progression_session(&mut admitted).await;
                let binding = owner
                    .progression_binding(admitted.game_session_id)
                    .ok_or("progression binding")?;
                let experience = binding
                    .policy
                    .thresholds
                    .iter()
                    .find(|threshold| threshold.level == level)
                    .ok_or("level threshold")?
                    .minimum_experience
                    .get();
                let current_fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "current gameplay fence")?
                    .ok_or("gameplay fence")?;
                let current = owner
                    .root
                    .read_character_progression(&authority, current_fence.character_id)
                    .await
                    .map_err(debug)?
                    .ok_or("progression state")?;
                let amount = experience
                    .checked_sub(current.total_experience.get())
                    .ok_or("experience amount")?;
                assert!(amount > 0);
                let mut slot = owner
                    .revision_sequencer
                    .acquire(current_fence.character_id)
                    .await;
                let outcome = slot
                    .commit_experience(
                        owner.root,
                        &authority,
                        owner.holder,
                        current_fence,
                        ExperienceAwardRequest {
                            occurrence: ExperienceRewardOccurrence::from_bytes(id(u8::try_from(
                                90 + index,
                            )?))
                            .map_err(debug)?,
                            amount: oteryn_simulation_determinism::ExactI64::new(amount),
                            context: binding.context.clone(),
                            policy_revision: binding.policy_revision.clone(),
                            reward_revision: binding.reward_revision.clone(),
                            policy: binding.policy.clone(),
                        },
                        None,
                    )
                    .await
                    .map_err(debug)?;
                assert!(
                    matches!(outcome, ExperienceCommitOutcome::Committed(_)),
                    "{outcome:?}"
                );
                drop(slot);
                let stored = owner
                    .root
                    .read_character_progression(&authority, current_fence.character_id)
                    .await
                    .map_err(debug)?
                    .ok_or("committed progression")?;
                assert_eq!(stored.level, level);
                // Admit this fixture actor from the level actually committed by
                // the progression owner. The generic fixture otherwise seeds
                // one unrelated spell-test level for every character.
                {
                    let runtime = owner.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    runtime
                        .player_control_facts(deps.actor, admitted.game_session_id)
                        .map_err(|error| format!("gate level {level} control: {error:?}"))?;
                    states
                        .initialize(
                            &runtime,
                            deps.actor,
                            admitted.game_session_id,
                            crate::spell::cast::CharacterCastFacts {
                                vocation: crate::spell::Vocation::Monk,
                                level: stored.level,
                                ..crate::gameplay_transport::actor_spell::tests::FACTS
                            },
                            (3, 7_000_000),
                            owner.owner_now(),
                        )
                        .ok_or("live level facts")?;
                }
                let expected = if level < gate.level {
                    UseDisposition::Locked
                } else {
                    UseDisposition::Committed
                };
                if level < gate.level {
                    let map = owner.bundle_map().ok_or("bundle map")?;
                    assert_eq!(
                        crate::gameplay_transport::map_door::use_door(
                            &map.overlay,
                            &map.facts,
                            generation.identity().client_artifact_digest(),
                            crate::map::overlay::TilePos { x: x - 1, y, floor },
                            |_| false,
                            stored.level,
                            false,
                            &target,
                        )
                        .disposition,
                        UseDisposition::Locked,
                        "source level gate is bound before dispatch"
                    );
                }
                assert_eq!(
                    super::quest_callbacks::wire_uses(&owner, admitted, &[1], &target).await?,
                    [expected],
                    "{} at level {level}",
                    gate.quest
                );
                assert_eq!(
                    world.enterable(crate::map::overlay::TilePos { x, y, floor }),
                    level >= gate.level
                );
            }
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(receipts, 0, "door use cannot mint a chest reward");
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        // A stale current-character fence must refuse before reserving the claim.
        {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut stale = admitted;
            let mut stale_fence = fence;
            stale_fence.character_lease_generation += 1;
            stale.item_fence = Some(stale_fence);
            assert_eq!(
                wire_use_targets(&owner, stale, &[(1, &target)]).await?,
                [(CommandStatus::Rejected, UseDisposition::Rejected)]
            );
            let reservations: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(reservations, 0);
        }
        let mut original_item = None;
        for cold_replay in [false, true] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let commands = if cold_replay {
                vec![(1, &target)]
            } else {
                vec![(1, &target), (1, &target), (2, &target)]
            };
            let expected = if cold_replay {
                vec![(CommandStatus::Accepted, UseDisposition::Committed)]
            } else {
                vec![
                    (CommandStatus::Accepted, UseDisposition::Committed),
                    (CommandStatus::DuplicateReplay, UseDisposition::Committed),
                    (CommandStatus::Accepted, UseDisposition::NothingToUse),
                ]
            };
            assert_eq!(
                wire_use_targets(&owner, admitted, &commands).await?,
                expected,
                "{}",
                chest.claim
            );
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("backpack")?;
            assert_eq!(bag.entries.len(), 1);
            let item = &bag.entries[0].item;
            assert_eq!(item.definition, definition(chest.reward));
            assert_eq!(item.quantity, chest.quantity);
            if chest.claim == PANPIPE_ACCESS_KEY.claim {
                let key: (i32, bool, bool) = sqlx::query_as(
                    "SELECT k.key_number,k.minted_transaction_id=r.transaction_id, \
                     k.created_xact_id=r.created_xact_id FROM game_item_keys k \
                     JOIN game_reward_claim_mint_receipts r USING(item_instance_id) \
                     WHERE k.item_instance_id=encode($1,'hex')::uuid",
                )
                .bind(item.item_instance_id.as_slice())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(key, (4055, true, true), "key and receipt commit atomically");
            }
            let children: Vec<(String, i64, i32)> = sqlx::query_as(
                "SELECT i.definition_production_key,i.quantity,COALESCE(k.key_number,0) \
                 FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) \
                 LEFT JOIN game_item_keys k USING(item_instance_id) \
                 WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal",
            )
            .bind(item.item_instance_id.as_slice())
            .fetch_all(&harness.pool)
            .await?;
            assert_eq!(
                children,
                chest
                    .contents
                    .iter()
                    .map(|(key, count, number)| ((*key).into(), *count, *number))
                    .collect::<Vec<_>>(),
                "{} contents",
                chest.claim
            );
            if let Some(digest) = chest.text_digest {
                let text: (String, i64) = sqlx::query_as(
                    "SELECT encode(sha256(convert_to(text,'UTF8')),'hex'),text_revision::bigint \
                     FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
                )
                .bind(item.item_instance_id.as_slice())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(text, (digest.into(), 1));
            }
            if let Some(original) = original_item {
                assert_eq!(item.item_instance_id, original);
            } else {
                original_item = Some(item.item_instance_id);
            }
            let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT \
                (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), \
                (SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1), \
                (SELECT count(*) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid)), \
                (SELECT count(DISTINCT created_xact_id::text) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid))")
                .bind(chest.claim).bind(item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(counts, (1, 1, 1 + i64::try_from(chest.contents.len())?, 1));
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
