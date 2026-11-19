use super::TestResult;

pub(super) async fn ashmunrah(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [super::AdmittedSession],
    pool: &sqlx::PgPool,
) -> TestResult {
    use super::{
        annihilator_route_fixtures::{step, use_target},
        fixture::debug,
        l6_shared_gate_native_fixtures::relocate,
    };
    use crate::map::{
        overlay::TilePos,
        view::{MapFacts, placement_key},
    };
    use oteryn_protocol_oteryn::{
        world_object::{UseDisposition, WorldObjectTarget},
        world_spatial::StepDirection,
    };

    // Crystal 201189f8 actions_active_teleport_switches.lua:18-21,39-50
    // and movements_enter_ashmunrah_teleport_switche_done.lua:9-16.
    let switches = [
        ((33176, 32880), (33177, 32880)),
        ((33182, 32880), (33181, 32880)),
        ((33175, 32884), (33176, 32884)),
        ((33183, 32884), (33182, 32884)),
        ((33176, 32889), (33177, 32889)),
        ((33181, 32889), (33180, 32889)),
    ];
    let origin = (33181, 32884, 11);
    for (index, ((x, y), (from_x, from_y))) in switches.into_iter().enumerate() {
        let member = &mut admitted[usize::from(index == 4)];
        let actor = member.runtime_actor.ok_or("switch operator")?;
        relocate(owner, actor, member.game_session_id, (from_x, from_y, 11)).await?;
        let map = owner.bundle_map().ok_or("Ashmunrah map")?;
        let key = placement_key(TilePos { x, y, floor: -11 }, 2).ok_or("switch placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        assert_eq!(
            use_target(owner, member, &target).await?,
            UseDisposition::Committed
        );
        if index >= 4 {
            let member = &mut admitted[0];
            let actor = member.runtime_actor.ok_or("stepping actor")?;
            relocate(owner, actor, member.game_session_id, origin).await?;
            step(owner, member, StepDirection::East).await?;
            let at = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            assert_eq!(
                (at.x, at.y, at.floor),
                if index == 4 {
                    (33181, 32884, 11)
                } else {
                    (33198, 32885, 11)
                }
            );
        }
    }
    let per_character: Vec<i64> = sqlx::query_scalar(
        "SELECT count(*) AS switches FROM game_shared_switch_leases
         WHERE recipe='oteryn:quest-callback/ancient-tombs-ashmunrah-switch'
         GROUP BY character_id ORDER BY switches",
    )
    .fetch_all(pool)
    .await?;
    assert_eq!(per_character, [1, 5]);
    Ok(())
}

pub(super) async fn reversible_passage(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    member: &mut super::AdmittedSession,
    map: &crate::map::boot::BundleMap,
) -> TestResult {
    use super::annihilator_route_fixtures::{step, target, use_target};
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::{world_object::UseDisposition, world_spatial::StepDirection};
    use std::sync::atomic::Ordering;

    let binding = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("/annihilator-entry"))
        .ok_or("fixture lever")?;
    let mut authored = serde_json::to_value(binding)?;
    authored["effects"] = serde_json::json!({"passages_reset_after_ms": 100_000});
    authored["reversible"] = true.into();
    // Crystal 201189f8 action-1-first_seal_lever.lua:24-36. Use the fixture's
    // real lever and remote door to prove the shared world reset in dispatch.
    authored["passages"] = serde_json::json!([{
        "position": {"x": 33225, "y": 31659, "floor": -13}, "appearance": 5107
    }]);
    let mut close = authored.clone();
    close["key"] = "oteryn:quest-callback/fixture-trapdoor-step".into();
    close["trigger"] = "step_in".into();
    close["selector"] =
        serde_json::json!({"position": {"x":33224,"y":31671,"floor":-13}, "ordinal": 0});
    close["lever"] = serde_json::Value::Null;
    close["reversible"] = false.into();
    close["effects"] = serde_json::json!({"close_passages_after_ms": 3_000});
    let callback = serde_json::from_value(authored)?;
    let step_close = serde_json::from_value(close)?;
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback, step_close])
            .is_ok()
    );
    let wall = target(map, 33225, 31659)?;
    let wall_key = u64::from_be_bytes(wall.placement[32..].try_into()?);
    owner
        .states
        .test_owner_time
        .store(100_000_001, Ordering::Relaxed);
    assert_eq!(
        use_target(owner, member, &target(map, 33226, 31671)?).await?,
        UseDisposition::Committed
    );
    assert!(map.facts.base_hidden(wall_key));
    assert_eq!(map.facts.next_movement_decay_time(), Some(200_000_000));

    owner
        .states
        .test_owner_time
        .store(110_000_001, Ordering::Relaxed);
    assert_eq!(
        use_target(owner, member, &target(map, 33226, 31671)?).await?,
        UseDisposition::Committed
    );
    assert!(!map.facts.base_hidden(wall_key));

    owner
        .states
        .test_owner_time
        .store(120_000_001, Ordering::Relaxed);
    assert_eq!(
        use_target(owner, member, &target(map, 33226, 31671)?).await?,
        UseDisposition::Committed
    );
    assert!(map.facts.base_hidden(wall_key));
    map.facts.expire_movement_items(200_000_000);
    assert!(
        map.facts.base_hidden(wall_key),
        "old window cannot close the reopened wall"
    );
    map.facts.expire_movement_items(219_999_999);
    assert!(map.facts.base_hidden(wall_key));
    map.facts.expire_movement_items(220_000_000);
    assert!(!map.facts.base_hidden(wall_key));
    assert_eq!(map.facts.next_movement_decay_time(), None);

    // Crystal 201189f8 movement-1-first_seal_close_mw.lua:60-72.
    owner
        .states
        .test_owner_time
        .store(230_000_001, Ordering::Relaxed);
    assert_eq!(
        use_target(owner, member, &target(map, 33226, 31671)?).await?,
        UseDisposition::Committed
    );
    owner
        .states
        .test_owner_time
        .store(240_000_001, Ordering::Relaxed);
    assert_eq!(
        use_target(owner, member, &target(map, 33226, 31671)?).await?,
        UseDisposition::Committed
    );
    owner
        .states
        .test_owner_time
        .store(241_000_001, Ordering::Relaxed);
    step(owner, member, StepDirection::West).await?;
    assert_eq!(map.facts.next_movement_decay_time(), Some(244_000_000));
    map.facts.expire_movement_items(243_999_999);
    assert!(map.facts.base_hidden(wall_key));
    map.facts.expire_movement_items(244_000_000);
    assert!(!map.facts.base_hidden(wall_key));
    Ok(())
}

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const CATHEDRAL_START: Position = at(32598, 32382, -10);

pub(super) async fn cathedral(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    member: &mut super::AdmittedSession,
    map: &crate::map::boot::BundleMap,
    pool: &sqlx::PgPool,
) -> TestResult {
    use super::{fixture::debug, item_craft_fixtures, quest_world_callbacks_fixtures::input_item};
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use crate::map::{
        overlay::TilePos,
        view::{MapFacts, placement_key},
    };
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
        authored_bindings::Bindings, world_callbacks::Offering,
    };
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        encode_item_use_intent,
    };
    use sha2::{Digest, Sha256};
    use std::{
        num::NonZeroU64,
        sync::{Arc, atomic::Ordering},
    };

    const QUEST: &str = "oteryn:quest.the_thieves_guild_quest";
    const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/mission08";
    const INPUT: &str = "oteryn:item.tibia.i7873";
    const START_MISSION: &str =
        "oteryn:quest-transition/quest/u8_2/the_thieves_guild_quest/mission08/npc_1";
    const WALL: Position = at(32598, 32381, -10);
    const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_thieves_guild_quest/actions_note.lua:1-25";
    let callback = serde_json::json!({
        "key": "oteryn:quest-callback/cathedral-message-note",
        "quest": QUEST,
        "trigger": "use",
        "selector": {"position": WALL, "ordinal": 1, "appearance": 1451},
        "requires_world_admission": true,
        "lever": null, "reversible": false, "passages": [], "offerings": [],
        "return_on_failure": false,
        "item_on_target": {"item": INPUT, "count": 1, "selected_instance": true},
        "storage": [{"track": MISSION, "minimum": 1, "maximum": 1, "set": 2}],
        "effects": {"timed_display": {"position": WALL, "appearance": 7872, "after_ms": 300_000}},
        "sources": [SOURCE],
    });
    let recipe: String = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST, "recipe_sha256": recipe, "sources": [SOURCE],
            "tracks": [{"key": MISSION, "initial": 0, "minimum": 0, "maximum": 3}],
            "stages": [{"key": "post-note", "completes": false,
                        "trigger": {"kind": "map_use", "callback": callback}}],
        }],
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture quest catalogue")?;
    let catalogue = Arc::new(
        bindings
            .attach(&base)?
            .with_fixture_transition(QuestTransition {
                key: START_MISSION.into(),
                quest: QUEST.into(),
                world_change: None,
                effects: vec![QuestEffect {
                    track: MISSION.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "Cathedral quest fence")?
        .ok_or("Cathedral session")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: START_MISSION.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        member.game_session_id,
                        CommandId::new(901).map_err(debug)?,
                    )),
                },
                catalogue,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    owner
        .refresh_quest_session(member.game_session_id, false)
        .await;
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "Cathedral item fence")?
        .ok_or("Cathedral session")?;
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: CATHEDRAL_START,
            item: INPUT.into(),
            count: 1,
        },
        952,
        true,
    )
    .await?;
    member.item_fence = Some(
        owner
            .current_quest_fence(member.game_session_id)
            .await
            .map_err(|_| "Cathedral carried item fence")?
            .ok_or("Cathedral session")?
            .item_fence(),
    );
    let item = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", INPUT, "definition-r1"))
        .ok_or("selected note reference")?;
    let wall = TilePos {
        x: WALL.x,
        y: WALL.y,
        floor: WALL.floor,
    };
    let key = placement_key(wall, 1).ok_or("actual Cathedral wall")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
    })
    .map_err(debug)?;
    owner
        .states
        .test_owner_time
        .store(100_000_001, Ordering::Relaxed);
    let (updated, result, _) = item_craft_fixtures::exchange(
        owner,
        *member,
        member.continuity.next_command_id,
        &payload,
        Some(item),
    )
    .await?;
    *member = updated;
    assert_eq!(result, UseDisposition::Committed);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(MISSION), Some(&2));
    let retired: Vec<(i64, i16)> = sqlx::query_as(
        "SELECT quantity, lifecycle FROM game_item_instances WHERE definition_production_key=$1",
    )
    .bind(INPUT)
    .fetch_all(pool)
    .await?;
    assert_eq!(retired, [(0, 2)], "the selected carried note is consumed");
    let displayed = || {
        map.facts
            .movement_item_entries()
            .iter()
            .any(|(at, entry)| *at == wall && entry.facts.appearance_id == 7872)
    };
    assert!(displayed());
    let expiry = map
        .facts
        .next_movement_decay_time()
        .ok_or("Cathedral display deadline")?;
    assert_eq!(expiry, 400_000_000);
    map.facts.expire_movement_items(expiry - 1);
    assert!(displayed());
    map.facts.expire_movement_items(expiry);
    assert!(!displayed());
    Ok(())
}

pub(super) const LORD_START: Position = at(33263, 31831, -12);
const LORD_EXIT: Position = at(33265, 31838, -10);

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

pub(super) async fn lord(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [super::AdmittedSession],
    map: &crate::map::boot::BundleMap,
    entry_pool: Option<&sqlx::PgPool>,
) -> TestResult {
    use super::{
        annihilator_route_fixtures::use_target, fixture::debug, quest_lord_entry_fixtures as entry,
        quest_world_callbacks_fixtures::local,
    };
    use crate::map::{
        floor::BundleDoors,
        overlay::TilePos,
        view::{MapFacts, placement_key},
    };
    use oteryn_gameplay_quests::quest::world_callbacks::Callback;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    use std::sync::atomic::Ordering;

    // Crystal 201189f8 actions_lord_of_elements_lever.lua:1-30,63-99,143-188.
    let switches = [
        (
            at(33273, 31840, -12),
            at(33272, 31840, -12),
            at(33275, 31834, -12),
            at(33275, 31838, -12),
            5072,
            at(33275, 31836, -12),
        ),
        (
            at(33262, 31840, -12),
            at(33263, 31840, -12),
            at(33266, 31843, -12),
            at(33270, 31843, -12),
            5071,
            at(33268, 31843, -12),
        ),
        (
            at(33262, 31831, -12),
            LORD_START,
            at(33260, 31834, -12),
            at(33260, 31838, -12),
            5072,
            at(33260, 31836, -12),
        ),
        (
            at(33273, 31831, -12),
            at(33272, 31831, -12),
            at(33266, 31828, -12),
            at(33270, 31828, -12),
            5071,
            at(33268, 31828, -12),
        ),
    ];
    let window: Vec<_> = switches
        .iter()
        .map(|&(lever, station, from, to, appearance, sound)| {
            serde_json::json!({
                "lever": lever, "station": station, "wall_from": from, "wall_to": to,
                "wall_appearance": appearance,
                "wall_speech": {"position":sound, "text":"ZOOOOOOOOM"}
            })
        })
        .collect();
    let mut callbacks = Vec::new();
    // Crystal lines 31-60 list these two diagonals in this order.
    let start_magic: Vec<_> = (0..14)
        .map(|i| at(33261 + i, 31829 + i, -12))
        .chain((0..14).map(|i| at(33274 - i, 31829 + i, -12)))
        .map(|position| serde_json::json!({"position":position, "effect":12}))
        .collect();
    for (index, &(lever, _, _, _, _, _)) in switches.iter().enumerate() {
        callbacks.push(serde_json::from_value::<Callback>(serde_json::json!({
            "key": format!("oteryn:quest-callback/lord-of-elements-switch-{}", index + 1),
            "quest": "oteryn:quest.elemental_spheres_quest",
            "trigger": "use",
            "selector": {"position": lever, "appearance": 2772},
            "lever": [2772, 2773],
            "reversible": false,
            "passages": [], "storage": [], "offerings": [],
            "return_on_failure": false,
            "effects": {
                "encounter": {
                    "minimum_level": 1,
                    "players": [],
                    "monsters": [{"position": at(33267,31836,-12), "creature": "oteryn:creature.lord_of_the_elements"}],
                    "room_min": at(33238,31806,-12),
                    "room_max": at(33297,31865,-12)
                },
                "timed_group_transfer": {
                    "after_ms": 600_000, "destination": LORD_EXIT, "switches": window,
                    "start_speech": {
                        "position":at(33267,31836,-12),
                        "text":"You have 10 minutes from now on until you get teleported out."
                    },
                    "start_magic": start_magic,
                    "warning": {
                        "after_ms": 300_000, "position": at(33266,31835,-13),
                        "room_min": at(33261,31829,-12), "room_max": at(33274,31842,-12),
                        "text": "You have 5 minutes from now on until you get teleported out."
                    }
                }
            },
            "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_lord_of_elements_lever.lua:63-188"],
        }))?);
    }
    if entry_pool.is_some() {
        let entrance = entry::binding()?;
        let Err(refused) = oteryn_gameplay_quests::quest::world_callbacks::load_with_source_fixture(
            std::slice::from_ref(&entrance),
        ) else {
            return Err("an entrance without its source circuit must not load".into());
        };
        assert!(refused.contains("missing chamber window"), "{refused}");
        callbacks.push(entrance);
    }
    let callbacks =
        oteryn_gameplay_quests::quest::world_callbacks::load_with_source_fixture(&callbacks)?;
    assert!(owner.states.quest_callbacks.fixture.set(callbacks).is_ok());
    if let Some(pool) = entry_pool {
        entry::assert_entry(owner, members, map, pool).await?;
    }
    let speech_start = map
        .facts
        .movement_speech_since(0)
        .last()
        .map_or(0, |(seq, _)| *seq);
    let effect_start = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |(seq, _, _)| *seq);

    let target_for = |lever: Position| -> TestResult<WorldObjectTarget> {
        let tile = TilePos {
            x: lever.x,
            y: lever.y,
            floor: lever.floor,
        };
        let base = map
            .overlay
            .base()
            .tile(tile.x, tile.y, tile.floor)
            .ok_or("lever tile")?;
        let found = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .find_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let facts = map.facts.base_entry(tile, ordinal, *id)?;
                [2772, 2773]
                    .contains(&facts.appearance_id)
                    .then(|| placement_key(tile, ordinal))
                    .flatten()
            })
            .ok_or("source lever")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&found.to_be_bytes());
        Ok(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(found),
        })
    };

    // The east switch is deliberately used last. Only that use starts the 10-minute timer.
    let turns = if entry_pool.is_some() {
        [(2, 0), (3, 1), (1, 2), (0, 3)]
    } else {
        [(2, 0), (3, 1), (1, 1), (0, 0)]
    };
    for (turn, (which_switch, which_member)) in turns.into_iter().enumerate() {
        let (lever, station, wall, _, _, sound) = switches[which_switch];
        let member = &mut members[which_member];
        {
            let actor = member.runtime_actor.ok_or("actor")?;
            let mut runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(actor).map_err(debug)?;
            if expected.position() != local(station) {
                let doors = BundleDoors(&map.facts);
                let proof = crate::map::floor::prepare_quest_encounter_ejection(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    member.game_session_id,
                    expected,
                    local(station),
                    &doors,
                )
                .map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)?;
            }
        }
        owner.states.test_owner_time.store(
            100_000_001 + u64::try_from(turn)? * 1_000_000,
            Ordering::Relaxed,
        );
        assert_eq!(
            use_target(owner, member, &target_for(lever)?).await?,
            UseDisposition::Committed
        );
        let wall = TilePos {
            x: wall.x,
            y: wall.y,
            floor: wall.floor,
        };
        assert!(
            map.facts
                .quest_callback_blocked(map.overlay.base(), wall, false)
        );
        assert_eq!(
            owner.states.quest_callbacks.next_encounter_due_us(),
            (turn == 3).then_some(403_000_000)
        );
        let speech = map.facts.movement_speech_since(speech_start);
        assert_eq!(speech.len(), turn + 1 + usize::from(turn == 3));
        assert_eq!(speech[turn].1.talk_type, 36);
        assert_eq!(speech[turn].1.text, "ZOOOOOOOOM");
        assert_eq!(
            speech[turn].1.position,
            TilePos {
                x: sound.x,
                y: sound.y,
                floor: sound.floor
            }
        );
        let magic = map.facts.movement_effects_since(effect_start);
        if turn == 3 {
            assert_eq!(speech[4].1.talk_type, 37);
            assert_eq!(
                speech[4].1.text,
                "You have 10 minutes from now on until you get teleported out."
            );
            assert_eq!(
                speech[4].1.position,
                TilePos {
                    x: 33267,
                    y: 31836,
                    floor: -12
                }
            );
            let diagonal: Vec<_> = (0..14)
                .map(|i| (33261 + i, 31829 + i, -12, 12))
                .chain((0..14).map(|i| (33274 - i, 31829 + i, -12, 12)))
                .collect();
            assert_eq!(
                magic
                    .iter()
                    .map(|(_, at, effect)| (at.x, at.y, at.floor, *effect))
                    .collect::<Vec<_>>(),
                diagonal
            );
        } else {
            assert!(magic.is_empty());
        }
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len(),
        1
    );
    let mut speeches_before_warning = 5;
    if entry_pool.is_some() {
        // The synchronized window must refuse another USE even with no
        // spectators in the room (Crystal actions_lever.lua:39-43).
        for (member, (station, _, _, _)) in members.iter().zip(entry::PARTIES) {
            entry::move_member(owner, member, station).await?;
        }
        entry::assert_occupied(owner, &mut members[3], map).await?;
        speeches_before_warning += 1;
        for &(which_switch, which_member) in &turns {
            entry::move_member(owner, &members[which_member], switches[which_switch].1).await?;
        }
    }
    owner
        .states
        .test_owner_time
        .store(403_000_000, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        map.facts.movement_speech_since(speech_start).len(),
        speeches_before_warning
    );
    owner
        .states
        .test_owner_time
        .store(403_000_001, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    owner.drain_quest_encounter_deadlines().await;
    let speech = map.facts.movement_speech_since(speech_start);
    assert_eq!(speech.len(), speeches_before_warning + 1);
    let warning = &speech[speeches_before_warning].1;
    assert_eq!(
        warning.position,
        TilePos {
            x: 33266,
            y: 31835,
            floor: -13
        }
    );
    assert_eq!(warning.talk_type, 37);
    assert_eq!(
        warning.text,
        "You have 5 minutes from now on until you get teleported out."
    );
    assert_eq!(
        owner.states.quest_callbacks.next_encounter_due_us(),
        Some(703_000_000)
    );
    owner
        .states
        .test_owner_time
        .store(703_000_001, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    let runtime = owner.runtime.lock().await;
    for member in &*members {
        let actor = member.runtime_actor.ok_or("actor")?;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(LORD_EXIT)
        );
    }
    assert!(runtime.visible_entities().creatures.is_empty());
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), None);
    for (lever, _, wall, _, _, _) in switches {
        assert_eq!(target_for(lever)?.expected_revision, 2);
        let wall = TilePos {
            x: wall.x,
            y: wall.y,
            floor: wall.floor,
        };
        assert!(
            !map.facts
                .quest_callback_blocked(map.overlay.base(), wall, false)
        );
    }
    drop(runtime);
    if entry_pool.is_some() {
        for (member, (station, _, _, _)) in members.iter().zip(entry::PARTIES) {
            entry::move_member(owner, member, station).await?;
        }
        entry::assert_party_admitted(owner, members, map).await?;
    }
    Ok(())
}
