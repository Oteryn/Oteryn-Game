//! Exercise the Goshnar blood-use route with two durable player sessions.
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterLease, ExactActorRef, GameSessionId,
    MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

// The fixture assigns the donor AID and uses the source's south-tile selection.
const GRAVESTONE: Position = Position {
    x: 32790,
    y: 32330,
    floor: -10,
};
const GATHERING: Position = Position {
    y: 32331,
    ..GRAVESTONE
};
const LANDING: Position = Position {
    x: 32791,
    y: 32332,
    floor: -10,
};
const EAST: Position = Position {
    x: 32792,
    ..LANDING
};
const WEST: Position = Position {
    x: 32790,
    ..LANDING
};
const VIAL: &str = "oteryn:item.tibia.i2874";

#[test]
fn grptrans_goshnar_blood_use_relocates_both_players_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("grptrans-goshnar", move || {
        run(async move |_| {
            let h = Harness::create_with_admission_seed(admin, "grptrans_goshnar", false, false)
                .await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let (peer, _) =
        crate::world_party_runtime_postgres_cases::committed_fresh_player_with_operation(
            &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            80,
            70,
            now,
        )
        .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active generation")?;
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/goshnar-gravestone",
        "quest": "oteryn:quest.the_pits_of_inferno_quest",
        "trigger": "use",
        "selector": {"position": null, "unique": null, "action": 2023, "appearance": null},
        "reversible": false,
        "passages": [],
        "teleport": null,
        "storage": [],
        "offerings": [],
        "return_on_failure": false,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/fluids.lua:157-169"]
    }))?;
    let world = super::quest_world_callbacks::world_with_cells(
        &callback,
        GRAVESTONE,
        &[GATHERING, LANDING, EAST, WEST],
    )?;
    let cells = world
        .movement_cells(activation.qualified_room().movement_cells())
        .map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        activation.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        world.frame_binding_digest(deps.admitted.world_id),
        world.map_revision_digest(),
        (i32::from(GATHERING.x), i32::from(GATHERING.y), 10),
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
    let fence = crate::gameplay_transport::item_fence_of(
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0,
    )
    .ok_or("current item fence")?;
    let lease = CharacterLease::new(
        oteryn_protocol_oteryn::CharacterId::decode(fence.character_id.as_bytes())
            .map_err(debug)?,
        fence.character_lease_generation,
    )
    .map_err(debug)?;
    let peer_session = peer.current_game_session_id();
    let mut actors = Vec::new();
    for (member, lease, at) in [
        (session, lease, GATHERING),
        (peer_session, peer.current_character_lease(), GATHERING),
    ] {
        let reservation = runtime.reserve_fresh_session(member).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, member, lease)
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, local(at))
            .map_err(debug)?;
        actors.push(actor);
    }
    let [entrant, occupant] = actors.as_slice() else {
        return Err("two admitted actors".into());
    };
    deps.actor = *entrant;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        for (actor, member) in [(*entrant, session), (*occupant, peer_session)] {
            states
                .initialize(
                    &runtime,
                    actor,
                    member,
                    crate::spell::cast::CharacterCastFacts {
                        level: 120,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("live character facts")?;
        }
    }
    super::item_look::seed_main_backpack(h, &[(VIAL, 1)]).await?;
    let vessel = id(71);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,5,game_character_uuid_v7())")
        .bind(vessel.as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    let map = owner.bundle_map().ok_or("fixture bundle map")?;
    let key = crate::map::view::placement_key(
        TilePos {
            x: GRAVESTONE.x,
            y: GRAVESTONE.y,
            floor: GRAVESTONE.floor,
        },
        1,
    )
    .ok_or("gravestone placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend(key.to_be_bytes());
    let result = owner
        .use_fluid_on_map(
            *entrant,
            UseCommand {
                game_session_id: session,
                command_id: 1,
                item_fence: Some(fence),
            },
            ItemKey::Instance(vessel),
            WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await
        .ok_or("blood-on-target USE outcome")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    {
        let runtime = owner.runtime.lock().await;
        for actor in actors.iter().copied() {
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(LANDING),
                "the source USE must land both players",
            );
        }
    }
    let fluid: Option<i16> = sqlx::query_scalar("SELECT fluid_subtype FROM game_item_fluid_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(vessel.as_slice()).fetch_optional(&h.pool).await?;
    assert_eq!(fluid, None, "blood was consumed");
    step(&mut owner, *entrant, session, StepDirection::East, EAST).await?;
    step(
        &mut owner,
        *occupant,
        peer_session,
        StepDirection::West,
        WEST,
    )
    .await
}

fn local(at: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: StepDirection,
    expected: Position,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let (result, _) = owner
        .ordinary_paced_step(actor, session, direction, Some(before), None)
        .await;
    assert_eq!(result.disposition, StepDisposition::Moved);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(expected),
        "Goshnar landing exit",
    );
    Ok(())
}
