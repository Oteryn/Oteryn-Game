//! Crystal 201189f8 data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:31-58.
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire, world_with_cells};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
};

const QUEST: &str = "oteryn:quest.fixture_solo_room";
const TRACK: &str = "oteryn:quest-progress/fixture/solo-room";
const ENTRY: Position = Position {
    x: 33039,
    y: 32103,
    floor: -12,
};
const ROOM: Position = Position {
    x: 33023,
    y: 32106,
    floor: -12,
};
const EXIT: Position = Position {
    x: 33056,
    y: 32004,
    floor: -11,
};
const AFTER_MS: u32 = 5 * 60 * 1_000;

#[test]
fn solo_quest_room_ejects_the_admitted_player_after_the_last_monster_dies() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "solo_quest_room_ejection",
                    false,
                    false,
                )
                .await?;
                let result = Box::pin(exercise(&h)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "solo quest room case panicked")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:31-58";
    let callback_json = serde_json::json!({
        "key":"oteryn:quest-callback/fixture/solo-room", "quest":QUEST, "trigger":"step_in",
        "selector":{"position":ENTRY,"appearance":100,"native":{"position":ENTRY,"appearance":100,"ordinal":1}},
        "requires_world_admission":true, "reversible":false, "passages":[], "offerings":[],
        "storage":[{"track":TRACK,"minimum":9,"maximum":9,"set":10}],
        "effects":{"encounter":{
            "minimum_level":1, "players":[{"from":ENTRY,"to":ROOM}],
            "monsters":[{"creature":"oteryn:creature.rat","position":{"x":33034,"y":32107,"floor":-12}}],
            "room_min":{"x":33011,"y":32097,"floor":-12},
            "room_max":{"x":33051,"y":32117,"floor":-12},
            "ejection":{"after_ms":AFTER_MS,"destination":EXIT}
        }},
        "return_on_failure":true, "failure_teleport":EXIT, "sources":[source]
    });
    let callback: Callback = serde_json::from_value(callback_json.clone())?;
    let bindings: Bindings = serde_json::from_value(serde_json::json!({
        "schema":1,"quests":[{
            "quest":QUEST,
            "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(source).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tracks":[{"key":TRACK,"initial":9,"minimum":9,"maximum":10}],
            "stages":[{"key":"s1","completes":false,"trigger":{"kind":"step_in","callback":callback_json}}],
            "sources":[source]
        }]
    }))?;
    let catalogue =
        bindings.attach(&QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)?)?;
    let world = world_with_cells(&callback, ENTRY, &[EXIT])?;
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let start = local(Position {
        x: ENTRY.x - 1,
        ..ENTRY
    });
    let frame = activated.frame_binding().digest();
    let digest = generation.identity().server_artifact_digest();
    let client_digest = generation.identity().client_artifact_digest();
    let map = world.map_revision_digest();
    deps.combat_room = Some(
        activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                map,
                crate::content::NativeEntryStart {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .ok_or("solo room bundle")?,
    );
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        digest,
        client_digest,
        frame,
        map,
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue));

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    admitted.item_fence = Some(fence.item_fence());
    deps.runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice())
        .bind(client_digest.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            digest,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;

    let enter_at = 1_000_000;
    owner
        .states
        .test_owner_time
        .store(enter_at + 1, Ordering::Relaxed);
    let reply = wire(
        &owner,
        &mut admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let read_progress = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        ).bind(id(41).as_slice()).bind(TRACK).fetch_one(&h.pool).await
    };
    assert_eq!(read_progress().await?, 10);
    {
        let mut runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(ROOM)
        );
        let creatures = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(creatures.len(), 1);
        for creature in creatures {
            runtime.remove_encounter_actor(creature).map_err(debug)?;
        }
    }
    let due = enter_at + u64::from(AFTER_MS) * 1_000;
    assert_eq!(
        owner.states.quest_callbacks.next_encounter_due_us(),
        Some(due)
    );
    owner.states.test_owner_time.store(due, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        local(ROOM)
    );
    owner
        .states
        .test_owner_time
        .store(due + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        local(EXIT)
    );
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), None);
    assert_eq!(read_progress().await?, 10);
    Ok(())
}
