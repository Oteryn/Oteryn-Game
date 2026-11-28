use super::{
    TestResult,
    fixture::{debug, id},
};
use oteryn_gameplay_quests::quest::world_callbacks;
use oteryn_world_bundle_compiler::{b3, source};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Deserialize)]
struct Startup {
    bindings: Vec<Binding>,
}

#[derive(Deserialize)]
struct Binding {
    category: String,
    position: [u16; 3],
    item: u16,
    attribute: String,
    value: u16,
}

#[derive(Deserialize)]
struct PlacementIndex {
    palette: Vec<PaletteEntry>,
}

#[derive(Deserialize)]
struct PaletteEntry {
    source_item_id: Option<u16>,
}

fn loaded_bindings(category: &str, count: usize) -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let capture: Startup = serde_json::from_slice(&std::fs::read(
        root.join("tools/world-bundle-compiler/data/crystal-startup-bindings.json"),
    )?)?;
    let selected = capture
        .bindings
        .iter()
        .filter(|row| row.category == category)
        .map(|row| (row.position[0] / 256, row.position[1] / 256))
        .collect::<BTreeSet<_>>();
    let project =
        source::project_regions(root.as_path(), &selected.into_iter().collect::<Vec<_>>())?;
    let index: PlacementIndex = serde_json::from_value(project.index)?;
    let mut budget = oteryn_world_bundle::bundle::BUNDLE_BUDGET;
    let mut regions = BTreeMap::new();
    for bytes in project.regions {
        let region = b3::decode_region(
            &bytes,
            oteryn_world_bundle::bundle::TILE_LIMITS,
            &mut budget,
        )?;
        regions.insert((u16::from(region.z), region.rx, region.ry), region);
    }
    let mut matched = 0;
    for binding in capture
        .bindings
        .iter()
        .filter(|row| row.category == category)
    {
        let [x, y, z] = binding.position;
        let key = (z, x / 256, y / 256);
        let tile = regions[&key]
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|tile| tile.x == x && tile.y == y)
            .ok_or("donor tile is missing")?;
        assert_eq!(tile.house, 0, "{category} at {x}/{y}/{z}");
        let items: Vec<_> = tile
            .items
            .iter()
            .filter(|item| {
                item.depth == 0
                    && index.palette[item.palette as usize].source_item_id == Some(binding.item)
            })
            .collect();
        let [item] = items.as_slice() else {
            return Err(format!(
                "expected exactly one appearance {} at {x}/{y}/{z}",
                binding.item
            )
            .into());
        };
        let actual = match binding.attribute.as_str() {
            "action" => item.attrs.action,
            "unique" => item.attrs.unique,
            other => return Err(format!("unexpected startup attribute {other}").into()),
        };
        assert_eq!(actual, Some(binding.value), "{category} at {x}/{y}/{z}");
        if category == "rookgaard" {
            assert_eq!(
                item.attrs.action, None,
                "Crystal supplies only a UID for this gate"
            );
        }
        matched += 1;
    }
    assert_eq!(matched, count);
    Ok(())
}

#[test]
fn cwdoor_level_doors_load_donor_actions() -> TestResult {
    loaded_bindings("level", 52)
}

#[test]
fn cwdoor_numbered_locks_load_donor_actions() -> TestResult {
    loaded_bindings("key", 45)
}

#[test]
fn cwdoor_citizenship_portals_load_donor_uniques() -> TestResult {
    loaded_bindings("citizen", 17)
}

#[test]
fn cwdoor_gray_beach_citizenship_portal_is_enterable() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("cwdoor-gray-beach", || {
        use crate::map::{overlay::TilePos, view::placement_key};

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let capture: Startup = serde_json::from_slice(&std::fs::read(
            root.join("tools/world-bundle-compiler/data/crystal-startup-bindings.json"),
        )?)?;
        let portal = capture
            .bindings
            .iter()
            .find(|row| row.category == "citizen" && row.item == 5022)
            .ok_or("Gray Beach startup binding")?;
        let [x, y, z] = portal.position;
        let pos = TilePos {
            x,
            y,
            floor: -i8::try_from(z)?,
        };
        let (compiled, pins) = compiled_regions(&root, &[(x / 256, y / 256)])?;
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let (controller, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
        let world = crate::map::boot::boot(
            &compiled.bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos,
            },
            world_id,
            channel,
            |key| crate::map::boot::native_item_definition(Some(native), key),
        )
        .map_err(debug)?;
        assert!(world.enterable(pos));
        assert!(
            (0..=u8::MAX)
                .filter_map(|ordinal| placement_key(pos, ordinal))
                .any(|key| {
                    world.facts().quest_callback_matches(
                        key,
                        Some(portal.value),
                        None,
                        Some(portal.item),
                    )
                })
        );
        Ok(())
    })
}

#[test]
fn cwdoor_rookgaard_loads_donor_unique() -> TestResult {
    loaded_bindings("rookgaard", 1)
}

fn compiled_regions(
    root: &Path,
    regions: &[(u16, u16)],
) -> TestResult<(
    oteryn_world_bundle_compiler::compile::Compiled,
    crate::map::BundlePins,
)> {
    use oteryn_world_bundle_compiler::{bundle, compile, project};
    let project = source::project_regions(root, regions)?;
    let (registry, palette) = source::registry(root, &project)?;
    let (fixture, mut pins) = crate::map::boot::tests::bundle();
    let identity = bundle::read(&fixture)?.manifest.identity;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    Ok((compiled, pins))
}

#[test]
fn cwdoor_galthen_gate_boots_as_a_storage_door() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("cwdoor-galthen", || {
        use crate::map::{
            door::DoorUse,
            overlay::TilePos,
            view::{MapFacts, placement_key},
        };
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let pos = TilePos {
            x: 32466,
            y: 32494,
            floor: -8,
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let (compiled, pins) = compiled_regions(&root, &[(pos.x / 256, pos.y / 256)])?;
        let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let world = crate::map::boot::boot(
            &compiled.bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    y: pos.y - 1,
                    ..pos
                },
            },
            world_id,
            channel,
            |key| crate::map::boot::native_item_definition(Some(native), key),
        )
        .map_err(debug)?;
        let tile = world
            .base()
            .tile(pos.x, pos.y, pos.floor)
            .ok_or("Galthen gate tile")?;
        let (key, ordinal, id) = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .find_map(|(ordinal, (&id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(pos, ordinal)?;
                (world.facts().base_entry(pos, ordinal, id)?.appearance_id == 36547)
                    .then_some((key, ordinal, id))
            })
            .ok_or("Galthen closed door admits appearance 36547")?;
        assert_eq!(world.facts().action_at(key), Some(47601));
        assert!(world.facts().doors().blocks(pos));
        assert_eq!(
            world.facts().doors().toggle(key, 0, false, 1, false),
            Some(DoorUse::Locked)
        );
        assert_eq!(
            world.facts().doors().toggle(key, 0, false, 1, true),
            Some(DoorUse::Toggled {
                revision: 1,
                open: true
            })
        );
        assert!(!world.facts().doors().blocks(pos));
        assert_eq!(
            world
                .facts()
                .base_entry(pos, ordinal, id)
                .ok_or("open gate")?
                .appearance_id,
            36548
        );
        Ok(())
    })
}

#[test]
fn cwdoor_svargrond_roof_loads_qualified_routes() -> TestResult {
    use world_callbacks::Position;
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let callbacks = world_callbacks::load()?;
    let routes: Vec<_> = callbacks
        .iter()
        .filter(|row| row.key.starts_with("oteryn:quest-callback/svargrond-roof-"))
        .collect();
    assert_eq!(routes.len(), 3);
    let track = oteryn_gameplay_quests::quest::barbarian_mead::TRACKS[0];
    let roof = Position {
        x: 32212,
        y: 31131,
        floor: -5,
    };
    let fallback = Position {
        x: 32210,
        y: 31134,
        floor: -7,
    };
    for (stage, destination) in [
        (None, fallback),
        (Some(7), fallback),
        (Some(8), roof),
        (Some(9), fallback),
    ] {
        let values = stage
            .map(|value| (track.to_owned(), value))
            .into_iter()
            .collect();
        let route = world_callbacks::select_storage_phase(&routes, &values, catalogue.catalogue())?
            .ok_or("qualified roof phase")?;
        assert_eq!(route.teleport, Some(destination));
        assert_eq!(route.facing, Some(1));
        assert_eq!(route.effects.teleport_effect, Some(11));
        assert_eq!(route.selector.action, Some(30032));
        assert_eq!(
            route.effects.player_speech.as_deref(),
            (destination == fallback).then_some(
                "You have to be a honorary barbarian to access the roof. Talk to the Jarl about it."
            )
        );
    }
    let legacy = BTreeMap::from([("oteryn:quest-progress/storage/41174".to_owned(), 8)]);
    let route = world_callbacks::select_storage_phase(&routes, &legacy, catalogue.catalogue())?
        .ok_or("legacy honorary roof phase")?;
    assert_eq!(route.teleport, Some(roof));
    assert!(route.is_read_only());
    loaded_bindings("roof", 1)
}

#[test]
fn cwdoor_drefia_switch_loads_quest_use() -> TestResult {
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let callbacks = world_callbacks::load()?;
    let route = callbacks
        .iter()
        .find(|row| row.key == "oteryn:quest-callback/gravedigger-switch")
        .ok_or("Drefia switch")?;
    assert_eq!(route.trigger, world_callbacks::Trigger::Use);
    assert_eq!(route.selector.action, Some(4650));
    let candidates = callbacks
        .iter()
        .filter(|row| {
            row.trigger == world_callbacks::Trigger::Use
                && row.selector.action == route.selector.action
                && row
                    .selector
                    .position
                    .is_none_or(|position| Some(position) == route.selector.position)
                && row
                    .selector
                    .appearance
                    .is_none_or(|appearance| Some(appearance) == route.selector.appearance)
        })
        .map(|row| row.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(candidates, [route.key.as_str()]);
    assert_eq!(route.success_message.as_deref(), Some("<sizzle> <fizz>"));
    let [before, after] = route.storage.as_slice() else {
        return Err("two Drefia progress facts".into());
    };
    assert!(before.track.ends_with("/mission39"));
    assert_eq!(
        (before.minimum, before.maximum, before.preserve),
        (1, 1, true)
    );
    assert!(after.track.ends_with("/mission40"));
    assert_eq!((after.minimum, after.maximum, after.set), (-1, 0, 1));
    let transition = world_callbacks::primary_transition(route)?;
    assert_eq!(
        catalogue.catalogue().transition(&transition.key),
        Some(&transition)
    );
    loaded_bindings("lever", 1)
}

#[test]
fn cwdoor_numbered_key_claims_load_the_donor_chests() -> TestResult {
    use crate::content::world_reward_claims::bind_embedded_reward_claims;

    let expected = [
        ((33377, 31321, -1), 909),
        ((32802, 31576, -7), 3002),
        ((32800, 31582, -2), 3008),
        ((32623, 32187, -9), 3304),
        ((32509, 32181, -13), 3600),
        ((32589, 32100, -14), 3610),
        ((32411, 32155, -15), 3620),
        ((32576, 32216, -15), 3667),
        ((32842, 32225, -8), 3700),
        ((32605, 31908, -3), 3800),
        ((32648, 31905, -3), 3801),
        ((32599, 31923, -6), 3802),
        ((32497, 31887, -7), 3899),
        ((32150, 32112, -12), 4601),
        ((32176, 32132, -9), 4603),
    ];
    let regions: BTreeSet<_> = expected
        .iter()
        .map(|&((x, y, _), _)| (x / 256, y / 256))
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (compiled, pins) = compiled_regions(&root, &regions.into_iter().collect::<Vec<_>>())?;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    let report = bind_embedded_reward_claims(&base, &|_| true)?;
    let keys: BTreeSet<_> = report
        .served
        .iter()
        .flat_map(|claim| &claim.placements)
        .flat_map(|placement| {
            let cell = placement.cell;
            placement
                .placement
                .key_number
                .into_iter()
                .chain(placement.contents.iter().filter_map(|item| item.key_number))
                .map(move |key| ((cell.x, cell.y, cell.floor), key))
        })
        .collect();
    assert!(
        expected.iter().all(|entry| keys.contains(entry)),
        "absent chest claims {:?}",
        expected
            .iter()
            .filter(|entry| !keys.contains(entry))
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn cwdoor_npc_key_sources_load_exact_cost_and_key_numbers() -> TestResult {
    let state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let callbacks =
        oteryn_gameplay_quests::quest::reward_callbacks::offered_for(state.catalogue())?;
    let sale = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/shirith-key-3033")
        .ok_or("Shirith key sale")?;
    assert_eq!(sale.npc, "oteryn:npc.shirith");
    assert!(sale.repeatable);
    let fee = sale.fee.as_ref().ok_or("Shirith fee")?;
    assert_eq!((fee.gold, fee.include_bank), (50, true));
    assert_eq!(
        sale.give.as_ref().and_then(|give| give.key_number),
        Some(3033)
    );
    let prisoner: Vec<_> = callbacks
        .iter()
        .filter(|c| {
            c.npc == "oteryn:npc.a_prisoner" && c.quest == "oteryn:quest.mad_mage_room_quest"
        })
        .collect();
    let [apples, grant] = prisoner.as_slice() else {
        return Err("expected one apple hand-in and one key grant for A Prisoner".into());
    };
    assert_eq!(
        apples.key,
        "oteryn:quest-reward/l6-mad-mage-prisoner-apples"
    );
    assert_eq!(apples.take.len(), 1);
    assert_eq!(
        (apples.take[0].item.as_str(), apples.take[0].count),
        ("oteryn:item.tibia.i3585", 7)
    );
    assert_eq!((apples.topic, apples.next_topic), (2, 3));
    assert_eq!(grant.key, "oteryn:quest-reward/l6-mad-mage-prisoner-key");
    assert_eq!(
        grant.give.as_ref().and_then(|give| give.key_number),
        Some(3666)
    );
    assert_eq!((grant.topic, grant.next_topic), (5, 0));
    let quiz = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/skjaar-quiz-donation")
        .ok_or("Skjaar quiz donation")?;
    let tuition = quiz.fee.as_ref().ok_or("Skjaar donation fee")?;
    assert_eq!(
        (
            tuition.gold,
            tuition.include_bank,
            quiz.topic,
            quiz.next_topic
        ),
        (1000, true, 2, 3)
    );
    let crypt = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/skjaar-key-3142")
        .ok_or("Skjaar crypt key")?;
    assert_eq!(
        crypt.give.as_ref().and_then(|give| give.key_number),
        Some(3142)
    );
    assert_eq!((crypt.topic, crypt.next_topic), (6, 6));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(126, 125), (126, 124), (127, 123)])?;
    for (npc, cell) in [
        ("oteryn:npc.a_prisoner", (32394, 32138, 13)),
        ("oteryn:npc.shirith", (32644, 31655, 9)),
        ("oteryn:npc.skjaar", (32450, 32039, 8)),
    ] {
        assert!(
            project
                .families
                .npc_placements
                .iter()
                .any(|entry| entry.npc == npc && entry.cell == cell),
            "{npc}"
        );
    }
    Ok(())
}

#[test]
fn cwdoor_refused_numbered_key_exchanges_clear_confirmation_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("cwdoor-refused-keys", || {
        super::full_world_support::world_scenario!(
            "cwdoor_refused_keys",
            |h,
             _request,
             seal,
             authority,
             world_id,
             native,
             world,
             cells,
             room,
             pin,
             deps,
             actor,
             session,
             owner,
             fence,
             admitted| {
                use super::classic_quests_fixtures::{install, talk, talk_expecting};
                use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

                let state =
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?;
                owner.quest_catalogue = Some(std::sync::Arc::new(state.catalogue().clone()));
                owner.refresh_quest_session(session, true).await;
                let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::offered_for(
                    state.catalogue(),
                )?;
                let [shirith, skjaar, prisoner] = install(
                    &owner,
                    &admitted,
                    [
                        "oteryn:npc.shirith",
                        "oteryn:npc.skjaar",
                        "oteryn:npc.a_prisoner",
                    ],
                )
                .await?;

                let offers: serde_json::Value = serde_json::from_str(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/quests/npc-offers/l6-stories.json"
                )))?;
                let riddle_answer = offers["offers"]
                    .as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .find(|row| row["npc"] == "oteryn:npc.a_prisoner" && row["topic"] == 1)
                    })
                    .and_then(|offer| offer["keywords"][0].as_str())
                    .ok_or("published riddle answer")?;
                sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
                    .execute(&h.pool)
                    .await?;
                // Spend the fixture's 100 gold through its two ordinary key purchases.
                for (index, line) in ["hi", "key", "yes", "key", "yes", "key"]
                    .into_iter()
                    .enumerate()
                {
                    talk(&mut owner, &admitted, shirith, 10 + u64::try_from(index)?, line).await?;
                }
                assert_eq!(gold(&ledger(&h).await?), 0);

                let mut refused_topics = Vec::new();
                for (npc, key, lines) in [
                    (shirith, "shirith-key-3033", &[][..]),
                    (skjaar, "skjaar-quiz-donation", &["hi", "key", "yes"][..]),
                    (
                        prisoner,
                        "l6-mad-mage-prisoner-apples",
                        &["hi", "riddle", riddle_answer][..],
                    ),
                ] {
                    let callback = callbacks
                        .iter()
                        .find(|callback| callback.key == format!("oteryn:quest-reward/{key}"))
                        .ok_or("published key callback")?;
                    let start = 20 + 10 * u64::try_from(refused_topics.len())?;
                    for (index, line) in lines.iter().enumerate() {
                        talk(&mut owner, &admitted, npc, start + u64::try_from(index)?, line).await?;
                    }
                    assert_eq!(
                        owner
                            .states
                            .npc_conversations
                            .fixture_quest_topic(session)?,
                        callback.topic
                    );
                    let refusal = talk_expecting(
                        &mut owner,
                        &admitted,
                        npc,
                        start + 3,
                        "yes",
                        NpcIntentDisposition::Rejected,
                    )
                    .await?;
                    assert_eq!(refusal, callback.failure_reply);
                    refused_topics.push(
                        owner
                            .states
                            .npc_conversations
                            .fixture_quest_topic(session)?,
                    );
                    talk(&mut owner, &admitted, npc, start + 4, "bye").await?;
                }
                assert_eq!(refused_topics, [0, 0, 0], "Shirith, Skjaar, A Prisoner");
                Ok(())
            }
        )
    })
}
