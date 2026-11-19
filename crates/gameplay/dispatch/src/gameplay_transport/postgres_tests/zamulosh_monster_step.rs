//! Crystal 201189f8, ferumbras_ascension/movements_lovely_monsters.lua:1-28.
use super::{ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) const CASE: &str = "ferumbras-zamulosh-lovely-monster-step";
pub(super) const BLOSSOM: &str = "first-dragon-fertility-blossom-step";
pub(super) const HABITAT: &str = "ferumbras-zamulosh-grass-habitat";
const BLOCKED_SPAWN: &str = "ferumbras-zamulosh-grass-habitat-blocked-spawn";
const MISSING_BASIN: &str = "ferumbras-zamulosh-grass-habitat-missing-basin";
const STALE_HABITAT: &str = "ferumbras-zamulosh-grass-habitat-stale";
const FRAZZLEMAW: &str = "oteryn:creature.lovely_frazzlemaw";
const DEER: &str = "oteryn:creature.lovely_deer";
const SPIRIT: &str = "oteryn:creature.spirit_of_fertility";
const PLANT: &str = "oteryn:creature.angry_plant";
const MATCHED_CUE: &str = "That seemed correct!";
const THUNDER: u16 = 73; // Crystal src/utils/utils_definitions.hpp:128.
const ENERGY_AREA: u16 = 38; // Crystal src/utils/utils_definitions.hpp:93.
const FERUMBRAS: &str = "oteryn:quest.ferumbras_ascension_quest";
const GRASS: &str = "oteryn:quest-progress/quest/u10_90/ferumbras_ascension/habitats/grass";
const ALL_HABITATS: &str =
    "oteryn:quest-progress/quest/u10_90/ferumbras_ascension/habitats/all_habitats";
const HABITAT_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/actions_habitat_grass.lua:15-27,93,393-420,439-472";
// Mount the source UID on a tile in the finale fixture. Crystal selects its
// appearance and UID, while the map asset determines the coordinates.
const FLOWER: Position = Position {
    x: 33625,
    y: 31022,
    floor: -14,
};
// Crystal config selects AID 34315 and checks its original creature. The mounted
// World places that AID beside its source spectator centre (33618, 32666, 12).
const PORTAL: Position = Position {
    x: 33619,
    y: 32666,
    floor: -12,
};
// The donor registers AID 34306. The fixture places it beside the source basin.
const HABITAT_LEVER: Position = Position {
    x: 33632,
    y: 32667,
    floor: -12,
};
const BASIN: Position = Position {
    x: 33631,
    y: 32667,
    floor: -12,
};
const HABITAT_SPAWN: Position = Position {
    x: 33642,
    y: 32666,
    floor: -12,
};
const HABITAT_TILES: [(Position, u16); 3] = [
    (
        Position {
            x: 33641,
            y: 32664,
            floor: -12,
        },
        4515,
    ),
    (HABITAT_SPAWN, 4515),
    (
        Position {
            x: 33642,
            y: 32667,
            floor: -12,
        },
        4518,
    ),
];
// The donor clears whatever terrain occupies these cells. The mounted World uses 417.
const PREVIOUS_TERRAIN: u16 = 417;

#[test]
fn zamulosh_monster_step_retires_only_the_creature_paired_with_the_portal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, PORTAL, false)
    })
}

#[test]
fn spirit_of_fertility_replaces_itself_and_consumes_only_the_source_dry_flower() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BLOSSOM, || {
        super::quest_world_callbacks::scenario(BLOSSOM, FLOWER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_changes_basin_at_use_and_terrain_after_two_seconds() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(HABITAT, || {
        super::quest_world_callbacks::scenario(HABITAT, HABITAT_LEVER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_refuses_an_unavailable_spawn_without_awarding_credit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BLOCKED_SPAWN, || {
        super::quest_world_callbacks::scenario(BLOCKED_SPAWN, HABITAT_LEVER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_drops_the_whole_batch_when_one_terrain_revision_changes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(STALE_HABITAT, || {
        super::quest_world_callbacks::scenario(STALE_HABITAT, HABITAT_LEVER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_skips_the_absent_basin_and_its_cue() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(MISSING_BASIN, || {
        super::quest_world_callbacks::scenario(MISSING_BASIN, HABITAT_LEVER, false)
    })
}

pub(super) fn is_habitat(name: &str) -> bool {
    matches!(
        name,
        HABITAT | BLOCKED_SPAWN | MISSING_BASIN | STALE_HABITAT
    )
}

pub(super) fn habitat_callback() -> TestResult<Callback> {
    let delayed_edits = HABITAT_TILES.map(|(position, appearance)| {
        serde_json::json!({
            "position":position,"remove":PREVIOUS_TERRAIN,"create":appearance,"passable":true
        })
    });
    let document = serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/ferumbras/zamulosh/grass-habitat",
        "quest":FERUMBRAS,
        "trigger":"use","selector":{"position":HABITAT_LEVER,"action":34306,"appearance":9125},
        "lever":[9125,9126],"reversible":false,"passages":[],"offerings":[],
        "requires_world_admission":true,
        "return_on_failure":false,
        "storage":[{"track":GRASS,"minimum":-1,"maximum":0,"set":1}],
        "state_operations":[{"track":ALL_HABITATS,"from":{"kind":"any"},
            "effect":{"kind":"add","value":1}}],
        "effects":{"magic":{"position":BASIN,"effect":ENERGY_AREA},"scene":{
            "open":[{"position":BASIN,"remove":11114,"create":22196,"passable":true,
                "allow_missing":true}],
            "close":[],"delayed_open":{"delay_ms":2000,"edits":delayed_edits,
                "spawn":{"position":HABITAT_SPAWN,"creature":FRAZZLEMAW}}
        }},
        "sources":[HABITAT_SOURCE]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into()
        .map_err(|_| "one grass habitat callback")?;
    Ok(callback)
}

fn habitat_close_callback() -> TestResult<Callback> {
    let document = json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/ferumbras/zamulosh/grass-habitat-close",
        "quest":FERUMBRAS,"trigger":"use",
        "selector":{"position":HABITAT_LEVER,"action":34306,"appearance":9126},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"transform":9125},"sources":[HABITAT_SOURCE]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into()
        .map_err(|_| "one grass habitat closing callback")?;
    Ok(callback)
}

pub(super) fn habitat_catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    let digest = format!("{:x}", Sha256::digest(HABITAT_SOURCE));
    let document = json!({"schema":1,"quests":[{
        "quest":FERUMBRAS,"recipe_sha256":digest,"sources":[HABITAT_SOURCE],
        "tracks":[
            {"key":GRASS,"initial":-1,"minimum":-1,"maximum":1},
            {"key":ALL_HABITATS,"initial":-1,"minimum":-1,"maximum":8}
        ],
        "stages":[{"key":"grass-habitat","completes":false,
            "trigger":{"kind":"map_use","callback":callback}}]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) fn habitat_world(
    callback: &Callback,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    let mut objects = Vec::new();
    if name != MISSING_BASIN {
        objects.push((BASIN, 11114));
    }
    objects.extend(HABITAT_TILES.map(|(position, _)| (position, PREVIOUS_TERRAIN)));
    let mut appearances = vec![22196];
    appearances.extend(HABITAT_TILES.map(|(_, appearance)| appearance));
    super::quest_world_callbacks::world_with_scene_cells(
        callback,
        HABITAT_LEVER,
        &objects,
        &appearances,
        &[],
    )
}

pub(super) async fn exercise_habitat(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    callback: Callback,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback, habitat_close_callback()?])
        .map_err(|_| "callback installed")?;
    let map = owner.bundle_map().ok_or("habitat map")?;
    let has = |at: Position, appearance: u16| -> TestResult<bool> {
        let pos = crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        };
        let tile = crate::map::view::compose_tile(&map.overlay, &map.facts, pos)
            .map_err(debug)?
            .ok_or("habitat tile")?;
        Ok(tile
            .entries
            .iter()
            .any(|entry| entry.facts.appearance_id == appearance))
    };
    let before = owner.owner_now().get();
    let progress = super::questpc7_ferumbras::value;
    assert_eq!(progress(owner, admitted, GRASS).await?, -1);
    assert_eq!(progress(owner, admitted, ALL_HABITATS).await?, -1);
    if name == BLOCKED_SPAWN {
        let pos = crate::map::overlay::TilePos {
            x: HABITAT_SPAWN.x,
            y: HABITAT_SPAWN.y,
            floor: HABITAT_SPAWN.floor,
        };
        let key = crate::map::view::placement_key(pos, 1).ok_or("spawn ground placement")?;
        let facts = map
            .facts
            .quest_callback_replacement(key, PREVIOUS_TERRAIN)
            .ok_or("spawn ground facts")?;
        {
            let _channel = owner.runtime.lock().await;
            assert!(map.facts.apply_quest_callback(
                key,
                map.facts.object_revision(key),
                facts,
                &[],
                &[(key, facts, false)],
                false,
                None,
            ));
        }
        assert_eq!(
            super::questpc7_ferumbras::use_at(owner, admitted, HABITAT_LEVER, 9125).await?,
            UseDisposition::Rejected,
        );
        assert_eq!(progress(owner, admitted, GRASS).await?, -1);
        assert_eq!(progress(owner, admitted, ALL_HABITATS).await?, -1);
        assert!(has(HABITAT_LEVER, 9125)?);
        assert!(has(BASIN, 11114)?);
        assert!(!has(BASIN, 22196)?);
        assert_eq!(map.facts.next_movement_decay_time(), None);
        let channel = owner.runtime.lock().await;
        assert!(
            !channel
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .any(|actor| channel.matches_live_creature_identity(actor, FRAZZLEMAW.as_bytes()))
        );
        return Ok(());
    }
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, HABITAT_LEVER, 9125).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(owner, admitted, GRASS).await?, 1);
    assert_eq!(progress(owner, admitted, ALL_HABITATS).await?, 0);
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("deferred terrain")?;
    assert!(due >= before + 2_000_000 && due <= owner.owner_now().get() + 2_000_000);
    assert_eq!(has(BASIN, 22196)?, name != MISSING_BASIN);
    assert!(!has(BASIN, 11114)?);
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .into_iter()
            .filter(
                |(_, at, effect)| (at.x, at.y, at.floor) == (BASIN.x, BASIN.y, BASIN.floor)
                    && *effect == ENERGY_AREA
            )
            .count(),
        usize::from(name != MISSING_BASIN)
    );
    let channel = owner.runtime.lock().await;
    let frazzlemaws = || -> TestResult<Vec<ExactActorRef>> {
        Ok(channel
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|actor| channel.matches_live_creature_identity(*actor, FRAZZLEMAW.as_bytes()))
            .collect())
    };
    let [frazzlemaw] = frazzlemaws()?
        .try_into()
        .map_err(|_| "one frazzlemaw after USE")?;
    assert_eq!(
        channel
            .read_actor_position(frazzlemaw)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(HABITAT_SPAWN)
    );
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    for (at, next) in HABITAT_TILES {
        assert!(has(at, PREVIOUS_TERRAIN)?);
        assert!(!has(at, next)?);
    }
    let stale = name == STALE_HABITAT;
    if stale {
        let (at, _) = HABITAT_TILES[0];
        let pos = crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        };
        let key = crate::map::view::placement_key(pos, 1).ok_or("fixture terrain placement")?;
        let replacement = map
            .facts
            .quest_callback_replacement(key, PREVIOUS_TERRAIN)
            .ok_or("fixture terrain facts")?;
        // A concurrent map writer published a new revision of this exact placement.
        assert!(map.facts.apply_quest_callback(
            key,
            map.facts.object_revision(key),
            replacement,
            &[],
            &[],
            false,
            None,
        ));
    }
    assert_eq!(map.facts.expire_movement_items(due), usize::from(!stale));
    for (at, next) in HABITAT_TILES {
        assert_eq!(has(at, PREVIOUS_TERRAIN)?, stale);
        assert_eq!(has(at, next)?, !stale);
    }
    assert_eq!(map.facts.expire_movement_items(due), 0);
    assert_eq!(map.facts.next_movement_decay_time(), None);
    assert_eq!(frazzlemaws()?, [frazzlemaw]);
    drop(channel);
    if stale {
        // Restore the lever after the stale batch discarded its map edits. The
        // original placement is eligible again, but the character already used it.
        assert_eq!(
            super::questpc7_ferumbras::use_at(owner, admitted, HABITAT_LEVER, 9126).await?,
            UseDisposition::Committed,
        );
        assert_eq!(
            super::questpc7_ferumbras::use_at(owner, admitted, HABITAT_LEVER, 9125).await?,
            UseDisposition::Rejected,
        );
        assert_eq!(progress(owner, admitted, GRASS).await?, 1);
        assert_eq!(progress(owner, admitted, ALL_HABITATS).await?, 0);
        assert_eq!(map.facts.next_movement_decay_time(), None);
        let runtime = owner.runtime.lock().await;
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(
            actors
                .into_iter()
                .filter(
                    |actor| runtime.matches_live_creature_identity(*actor, FRAZZLEMAW.as_bytes())
                )
                .collect::<Vec<_>>(),
            [frazzlemaw]
        );
    }
    Ok(())
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    // Crystal 201189f8, the_first_dragon/movements_heaven_blossom.lua:3-20.
    let row = if name == BLOSSOM {
        serde_json::json!({
            "key":"oteryn:quest-callback/first-dragon/fertility-blossom",
            "quest":"oteryn:quest.the_first_dragon_quest",
            "trigger":"step_in","actor_kind":"monster",
            "selector":{"position":FLOWER,"unique":1066,"appearance":3657},
            "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
            "effects":{"monster_step":{"creature":SPIRIT,"replace_with":PLANT,"consume_source":true}},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/movements_heaven_blossom.lua:3-20"]
        })
    } else {
        serde_json::json!({
            "key":"oteryn:quest-callback/ferumbras/zamulosh/lovely-frazzlemaw",
            "quest":"oteryn:quest.ferumbras_ascension_quest",
            "trigger":"step_in","actor_kind":"monster",
            "selector":{"action":34315},
            "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
            "effects":{"monster_step":{"creature":FRAZZLEMAW,
                "retirement_cue":{"text":MATCHED_CUE,"effect":THUNDER}}},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/movements_lovely_monsters.lua:1-49"]
        })
    };
    let data = serde_json::json!({"schema":1,"callbacks":[row]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one Zamulosh callback")?;
    Ok(callback)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    callback: Callback,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let position = callback.selector.position.unwrap_or(PORTAL);
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native source")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let candidates = if position == FLOWER {
        vec![(DEER, position, true), (SPIRIT, position, false)]
    } else {
        let adjacent = Position {
            x: position.x - 1,
            ..position
        };
        vec![
            (FRAZZLEMAW, adjacent, true),
            (DEER, position, true),
            (FRAZZLEMAW, position, false),
        ]
    };
    for (creature, destination, survives) in candidates {
        let approach = super::quest_world_callbacks::local(Position {
            y: destination.y - 1,
            ..destination
        });
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let policy = runtime.companion_policy(creature).map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(creature, policy.maximum_health, approach)
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, creature)
                .map_err(debug)?;
            actor
        };
        step_monster(owner, h, actor, destination).await?;
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.contains_live_creature(actor), survives);
        if position == PORTAL {
            let speech: Vec<_> = map
                .facts
                .movement_speech_since(0)
                .into_iter()
                .filter(|(_, cue)| cue.speaker == actor)
                .collect();
            if survives {
                assert!(speech.is_empty());
            } else {
                let [(_, cue)] = speech.as_slice() else {
                    return Err("expected one paired-creature cue".into());
                };
                assert_eq!(cue.text, MATCHED_CUE);
                assert_eq!(cue.talk_type, 36);
                assert_eq!(
                    (cue.position.x, cue.position.y, cue.position.floor),
                    (position.x, position.y, position.floor)
                );
                assert!(cue.captured_name.is_some());
            }
        }
        if survives {
            runtime.remove_encounter_actor(actor).map_err(debug)?;
        }
    }
    if position == PORTAL {
        assert_eq!(
            map.facts
                .movement_effects_since(0)
                .into_iter()
                .filter(|(_, _, effect)| *effect == THUNDER)
                .map(|(_, at, _)| (at.x, at.y, at.floor))
                .collect::<Vec<_>>(),
            [(PORTAL.x, PORTAL.y, PORTAL.floor)]
        );
    }
    if position == FLOWER {
        use crate::map::view::MapFacts;
        let map = owner.bundle_map().ok_or("map")?;
        let tile = crate::map::overlay::TilePos {
            x: FLOWER.x,
            y: FLOWER.y,
            floor: FLOWER.floor,
        };
        let key = crate::map::view::placement_key(tile, 1).ok_or("dry flower key")?;
        assert!(map.facts.base_hidden(key));
        let runtime = owner.runtime.lock().await;
        let plants: Vec<_> = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|actor| runtime.matches_live_creature_identity(*actor, PLANT.as_bytes()))
            .collect();
        let [plant] = plants.try_into().map_err(|_| "one angry plant")?;
        assert_eq!(
            runtime
                .read_actor_position(plant)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(FLOWER)
        );
    }
    Ok(())
}

pub(super) async fn step_monster(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    position: Position,
) -> TestResult {
    use crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction;
    use crate::movement::{CardinalStep, monster_map::CurrentMonsterMap};
    let permit = owner.spell_lane_permit().await.ok_or("creature map lane")?;
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("creature room")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native source")?;
    let objects = owner.door.lock().await;
    let mut tx = h.pool.begin().await?;
    let authority = assert_spell_item_scope_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation().get(),
    )
    .await
    .map_err(debug)?;
    let mut map = CurrentMonsterMap::new(&mut tx, &authority, &runtime, room, &objects, native);
    let to = super::quest_world_callbacks::local(position);
    assert_eq!(map.read(&runtime, to).await, Some(true));
    let moved = map
        .view()
        .step(
            &mut runtime,
            room.movement_cells(),
            actor,
            before,
            CardinalStep::South,
        )
        .map_err(debug)?;
    assert_eq!(moved.position(), to);
    drop(map);
    tx.rollback().await?;
    let states = owner.states.spell_states.lock().await;
    crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
        &mut runtime,
        &states,
        owner.bundle_map().ok_or("map")?,
        owner.owner_now().get(),
        owner.callback_content(),
    );
    Ok(())
}
