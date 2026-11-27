use super::fixture::{Harness, debug, id};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::NativeEntryStart;
use crate::durability::starter_grant::StarterTemplate;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, NodeId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) enum Quest {
    GreatDragonHunt,
    LostBrother,
}

pub(super) fn scenario(quest: Quest) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "adventurers_guild", false, false).await?;
        let result = exercise(&h, quest).await;
        let cleanup = h.cleanup().await;
        result.and(cleanup)
    })
}

async fn exercise(h: &Harness, quest: Quest) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (world, start) = match quest {
        Quest::GreatDragonHunt => (
            super::great_dragon_hunt_fixtures::world(native)?,
            super::great_dragon_hunt_fixtures::START,
        ),
        Quest::LostBrother => (
            native_crop(
                native,
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z04-x128-y127.b3"
                    )),
                    region: (4, 128, 127),
                    bounds: (32956, 32671, 32961, 32676),
                    start: (32958, 32674),
                    expected: vec![ExpectedTile {
                        position: (32959, 32674),
                        ids: vec![410],
                        counted_entry: None,
                    }],
                },
            )?,
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 32958,
                y: 32674,
                floor: -4,
            },
        ),
    };
    let origin = super::quest_world_callbacks_fixtures::local(start);
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let room = activated
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            world.map_revision_digest(),
            NativeEntryStart {
                x: origin.x,
                y: origin.y,
                floor: origin.floor,
            },
        )
        .ok_or("native Bundle room")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (origin.x, origin.y, origin.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        5,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, origin)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    owner
        .runtime
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
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let template = StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some()
    );
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;
    match quest {
        Quest::GreatDragonHunt => {
            super::great_dragon_hunt_fixtures::exercise(&mut owner, &admitted, h, &authority).await
        }
        Quest::LostBrother => {
            super::lost_brother_fixtures::exercise(&mut owner, &admitted, h, &authority).await
        }
    }
}
