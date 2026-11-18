//! PostgreSQL dispatch regressions for the Banshee seal callbacks.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NativeEntryStart, channel_content_pin::ChannelContentPin};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn banshee_logic_use_flips_lever_without_overwriting_a_discordant_campfire() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("banshee-logic", || {
        run(async |admin| exercise(admin, Seal::Logic).await)
    })
}

#[test]
fn banshee_sacrifice_step_consumes_only_blood_and_commits_both_flags() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("banshee-sacrifice", || {
        run(async |admin| exercise(admin, Seal::Sacrifice).await)
    })
}

#[test]
fn banshee_queen_dialogue_refuses_a_pz_locked_kiss_and_clears_confirmation() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("banshee-queen", || {
        run(async |admin| exercise(admin, Seal::Queen).await)
    })
}

enum Seal {
    Logic,
    Queen,
    Sacrifice,
}

async fn exercise(admin: String, seal_case: Seal) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "banshee_logic", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active generation")?;
    let lever = Position {
        x: 32308,
        y: 31975,
        floor: -13,
    };
    let campfire = Position {
        x: 32309,
        y: 31975,
        floor: -13,
    };
    let flame = (32250, 31892, -14);
    let logic_flame = (32311, 31978, -13);
    let logic_destination = (32261, 31856, -15);
    let (source, objects) = if matches!(seal_case, Seal::Sacrifice) {
        (
            flame,
            vec![
                (32243, 31892, -14, 0, 431),
                (32250, 31892, -14, 35016, 1959),
                (32261, 31849, -15, 0, 1959),
            ],
        )
    } else {
        (
            (lever.x, lever.y, lever.floor),
            vec![
                (lever.x, lever.y, lever.floor, 30018, 2772),
                (campfire.x, campfire.y, campfire.floor, 0, 1996),
                (32309, 31976, -13, 0, 1996),
                (32311, 31975, -13, 0, 1996),
                (32311, 31976, -13, 0, 1996),
                (32313, 31975, -13, 0, 1998),
                (32313, 31976, -13, 0, 1998),
                (logic_flame.0, logic_flame.1, logic_flame.2, 35018, 1959),
                (
                    logic_destination.0,
                    logic_destination.1,
                    logic_destination.2,
                    0,
                    1959,
                ),
            ],
        )
    };
    let world = super::quest_callbacks::source_world_spatial(source, &objects)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let start = NativeEntryStart {
        x: i32::from(source.0) - 1,
        y: i32::from(source.1),
        floor: -i16::from(source.2),
    };
    let room = activated
        .qualified_room()
        .bundle_view(cells.clone(), frame, world.map_revision_digest(), start)
        .ok_or("bundle room")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .map_err(debug)?;
    let deps = Dependencies {
        actor,
        runtime: tokio::sync::Mutex::new(runtime),
        ..deps
    };
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            session,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    deps.initialize(&owner).await;
    let admitted = super::AdmittedSession {
        runtime_actor: Some(actor),
        ..deps.admitted
    };
    match seal_case {
        Seal::Logic => exercise_logic(&mut owner, &admitted, fence, lever, campfire).await?,
        Seal::Queen => exercise_queen(&mut owner, &h, &admitted, fence).await?,
        Seal::Sacrifice => exercise_sacrifice(&mut owner, &h, admitted, frame, start).await?,
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn exercise_logic(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    lever: Position,
    campfire: Position,
) -> TestResult {
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/banshees-logic-lever-30018",
        "quest": "oteryn:quest.the_queen_of_the_banshees_quest",
        "trigger": "use",
        "selector": { "position": lever, "unique": 30018 },
        "lever": [2772, 2773],
        "reversible": true,
        "storage": [],
        "offerings": [],
        "passages": [{
            "position": campfire,
            "appearance": 1998,
            "replacement": 1996,
            "if_present": true
        }],
        "return_on_failure": false,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-6-sixth_seal_lever.lua:3"]
    }))?;
    let layout = [32309, 32311, 32313]
        .into_iter()
        .flat_map(|x| {
            [31975, 31976].map(move |y| {
                serde_json::json!({
                    "position": { "x": x, "y": y, "floor": -13 },
                    "appearance": if x == 32313 { 1998 } else { 1996 },
                })
            })
        })
        .collect::<Vec<_>>();
    let flame: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/banshees-logic-flame",
        "quest": "oteryn:quest.the_queen_of_the_banshees_quest",
        "trigger": "step_in",
        "selector": { "unique": 35018 },
        "teleport": { "x": 32261, "y": 31856, "floor": -15 },
        "reversible": false,
        "storage": [],
        "offerings": [],
        "passages": [],
        "return_on_failure": true,
        "world_requirements": layout,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-6-sixth_seal_flame.lua:3"]
    }))?;
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback, flame])
            .is_ok()
    );
    let placement_key = |p: Position| {
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("placement")
    };
    let campfire_key = placement_key(campfire)?;
    let lever_key = placement_key(lever)?;
    let map = owner.bundle_map().ok_or("bundle map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&lever_key.to_be_bytes());
    let initial = map.facts.object_revision(campfire_key);
    let flame_origin = MovementLocalPosition {
        x: 32310,
        y: 31978,
        floor: 13,
    };
    for (command_id, changed) in [(1, false), (2, true), (3, true)] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("use clock")?;
        let map = owner.bundle_map().ok_or("bundle map")?;
        let outcome = owner
            .use_object(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence.item_fence()),
                },
                WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision: map.facts.object_revision(lever_key),
                },
            )
            .await;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert_eq!(map.facts.object_revision(campfire_key) != initial, changed);
        if command_id == 1 {
            continue;
        }
        move_fixture(owner, admitted, flame_origin).await?;
        let (step, _) = owner.paced_step(actor, session, StepDirection::East).await;
        assert_eq!(
            step.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
        let after = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        if command_id == 2 {
            assert_eq!(
                after.position(),
                flame_origin,
                "one incorrect campfire must refuse the layout"
            );
            move_fixture(
                owner,
                admitted,
                MovementLocalPosition {
                    x: 32307,
                    y: 31975,
                    floor: 13,
                },
            )
            .await?;
        } else {
            assert_eq!(
                after.position(),
                MovementLocalPosition {
                    x: 32261,
                    y: 31856,
                    floor: 15
                }
            );
        }
    }
    Ok(())
}

async fn exercise_sacrifice(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: super::AdmittedSession,
    frame: [u8; 32],
    start: NativeEntryStart,
) -> TestResult {
    const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
    const TRACK: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fourth_seal";
    const DOOR: &str =
        "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fourth_seal_door";
    const KEY: &str = "oteryn:quest-callback/banshees-fourth-seal";
    const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-4-fourth_seal_flame.lua:3";
    let generation = owner.active_generation.ok_or("active generation")?;
    let tracks = [TRACK, DOOR].map(|key| {
        serde_json::json!({
            "key": key, "initial": -1, "minimum": -1, "maximum": 1
        })
    });
    let storage = [TRACK, DOOR].map(|track| {
        serde_json::json!({
            "track": track, "minimum": -1, "maximum": 0, "set": 1
        })
    });
    let recipe_sha256 = <sha2::Sha256 as sha2::Digest>::digest(SOURCE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let bindings: Bindings = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": recipe_sha256,
            "sources": [SOURCE],
            "tracks": tracks,
            "stages": [{
                "key": "sacrifice-flame",
                "completes": false,
                "trigger": { "kind": "step_in", "callback": {
                    "key": KEY,
                    "quest": QUEST,
                    "trigger": "step_in",
                    "selector": { "position": {"x": 32250, "y": 31892, "floor": -14}, "unique": 35016 },
                    "teleport": {"x": 32261, "y": 31849, "floor": -15},
                    "reversible": false,
                    "return_on_failure": true,
                    "requires_world_admission": true,
                    "offerings": [],
                    "passages": [],
                    "storage": storage,
                    "effects": {
                        "splash_offering": { "position": {"x": 32243, "y": 31892, "floor": -14}, "subtype": 5 }
                    },
                    "sources": [SOURCE]
                }}
            }]
        }]
    }))?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(std::sync::Arc::new(bindings.attach(&base)?));
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let (outcome, _) = owner
        .paced_step(actor, admitted.game_session_id, StepDirection::East)
        .await;
    assert_eq!(
        outcome.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        after.position(),
        MovementLocalPosition {
            x: start.x,
            y: start.y,
            floor: start.floor
        }
    );
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 0);
    pour_sacrifice_fluids(owner, h, &admitted, start).await?;

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("step clock")?;
    let (outcome, _) = owner
        .paced_step(actor, admitted.game_session_id, StepDirection::East)
        .await;
    assert_eq!(
        outcome.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        after.position(),
        MovementLocalPosition {
            x: 32261,
            y: 31849,
            floor: 15
        }
    );
    let committed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key IN ($2,$3) AND value=1",
    ).bind(QUEST).bind(TRACK).bind(DOOR).fetch_one(&h.pool).await?;
    assert_eq!(committed, 2);
    let remaining = sqlx::query_scalar::<_, i16>(
        "SELECT fluid_subtype FROM game_item_fluid_splashes ORDER BY fluid_subtype LIMIT 3",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(remaining, [1], "water stays when the StepIn consumes blood");
    Ok(())
}

async fn pour_sacrifice_fluids(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &super::AdmittedSession,
    start: NativeEntryStart,
) -> TestResult {
    use crate::durability::starter_grant::{StarterGrantOutcome, StarterTemplate};
    use crate::gameplay_transport::{item_ref_admission::NativeItemWeights, item_view::ItemKey};

    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?
        .item_fence();
    let generation = owner.active_generation.ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native policies")?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            owner.character,
            &h.node,
            &fence,
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let backpack = h
        .root
        .grant_starter_items(
            owner.character,
            &h.node,
            &fence,
            &NativeItemWeights(native),
            &StarterTemplate::main(),
        )
        .await
        .map_err(debug)?;
    let StarterGrantOutcome::Minted(backpack) = backpack else {
        return Err(format!("starter backpack: {backpack:?}").into());
    };

    // A filled vial is a precondition. The only splash writes use the native USE_WITH owner.
    const VIAL: &str = "oteryn:item.tibia.i2874";
    let revision = native
        .item_index()
        .and_then(|index| index.revision(VIAL))
        .ok_or("vial")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for (n, fluid) in [(130_u8, 1_i16), (134, 5)] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(id(n).as_slice()).bind(id(42).as_slice()).bind(VIAL).bind(revision).bind(id(n+1).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
            .bind(id(n).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(backpack.as_slice()).bind(if fluid==1 { 1_i32 } else { 2 }).bind(id(n+2).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)")
            .bind(id(n).as_slice()).bind(id(42).as_slice()).bind(fluid).bind(id(n+3).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;

    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-14, 32243, 31892, 1).ok_or("altar")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let spot = MovementLocalPosition {
        x: 32244,
        y: start.y,
        floor: start.floor,
    };
    let origin = MovementLocalPosition {
        x: start.x,
        y: start.y,
        floor: start.floor,
    };
    for (n, command_id, expected_before) in [(130, 20, vec![1]), (134, 21, vec![1, 5])] {
        move_fixture(owner, admitted, spot).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("pour clock")?;
        let outcome = owner
            .use_fluid_on_map(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence),
                },
                ItemKey::Instance(id(n)),
                WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision: 0,
                },
            )
            .await
            .ok_or("typed pour onto altar terrain")?;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        let before = sqlx::query_scalar::<_, i16>(
            "SELECT fluid_subtype FROM game_item_fluid_splashes ORDER BY fluid_subtype LIMIT 3",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(before, expected_before);
        move_fixture(owner, admitted, origin).await?;
        if n == 130 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("step clock")?;
            owner.paced_step(actor, session, StepDirection::East).await;
            let at = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            assert_eq!(at.position(), origin, "water cannot admit the player");
        }
    }
    Ok(())
}

async fn move_fixture(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        admitted.game_session_id,
        expected,
        destination,
        false,
        &doors,
        None,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn exercise_queen(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &super::AdmittedSession,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, classic_talk::QUEEN,
    };
    const KEY: &str = "oteryn:quest-transition/test/banshee-kiss-prerequisites";
    const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
    const PREFIX: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/";
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let catalogue = std::sync::Arc::new(
        catalogue
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: KEY.into(),
                quest: QUEST.into(),
                effects: ["fourth", "first", "second", "third", "fifth", "sixth"]
                    .into_iter()
                    .map(|seal| QuestEffect {
                        track: format!("{PREFIX}{seal}_seal"),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(1),
                    })
                    .collect(),
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
    let session = admitted.game_session_id;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: KEY.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(101).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner.admit_quest_session(admitted).await;

    let [queen] = super::classic_quests::install(owner, admitted, [QUEEN]).await?;
    super::classic_quests::talk(owner, admitted, queen, 1, "hi").await?;
    super::classic_quests::talk(owner, admitted, queen, 2, "seventh seal").await?;
    for command in 3..=8 {
        super::classic_quests::talk(owner, admitted, queen, command, "yes").await?;
    }
    let offer = super::classic_quests::talk(owner, admitted, queen, 9, "kiss").await?;
    assert!(
        offer
            .iter()
            .any(|line| line.contains("prepared to receive my kiss"))
    );
    let actor = admitted.runtime_actor.ok_or("actor")?;
    {
        let mut spells = owner.states.spell_states.lock().await;
        crate::gameplay_transport::pvp_live::lock_mut(&mut spells, actor, session)
            .on_aggression(owner.owner_now());
    }
    let refused = super::classic_quests::talk(owner, admitted, queen, 10, "yes").await?;
    assert!(
        refused
            .iter()
            .any(|line| line.contains("spilled too much blood"))
    );
    let repeated = super::classic_quests::talk(owner, admitted, queen, 11, "yes").await?;
    assert!(
        !repeated
            .iter()
            .any(|line| line.contains("spilled too much blood"))
    );
    let writes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE track_key IN ($1,$2,$3) AND value=1",
    )
    .bind(format!("{PREFIX}last_seal"))
    .bind(format!("{PREFIX}last_seal_door"))
    .bind(format!("{PREFIX}banshee_door"))
    .fetch_one(&h.pool).await?;
    assert_eq!(writes, 0);
    let arrivals: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pending_arrivals")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(arrivals, 0);
    Ok(())
}
