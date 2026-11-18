//! Exercise the Demon Oak occupancy guard through the real StepIn dispatcher.
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterLease, ExactActorRef, GameSessionId,
    MovementLocalPosition, NodeId,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const KICK: Position = Position {
    x: 32716,
    y: 32340,
    floor: -7,
};
const GROVE: Position = Position {
    x: 32716,
    y: 32347,
    floor: -7,
};
const EDGE: Position = Position {
    x: 32725,
    y: 32351,
    floor: -7,
};
const OUTSIDE: Position = Position { x: 32726, ..EDGE };

#[test]
fn grptrans_demon_oak_step_refuses_occupied_grove_until_player_leaves_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("grptrans-demon-oak", move || {
        run(async move |_| {
            let h =
                Harness::create_with_admission_seed(admin, "grptrans_oak", false, false).await?;
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
    // Crystal registers UID 9000. The fixture places that UID next to the kick
    // tile to drive StepIn and uses the donor's actual room bounds and landings.
    let trigger = Position {
        y: KICK.y + 1,
        ..KICK
    };
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/demon-oak-entrance",
        "quest": "oteryn:quest.demon_oak",
        "trigger": "step_in",
        "selector": {"position": null, "unique": 9000, "action": null, "appearance": null},
        "reversible": false,
        "passages": [],
        "teleport": GROVE,
        "storage": [],
        "offerings": [],
        "return_on_failure": true,
        "failure_teleport": KICK,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/demon_oak/movements_entrance.lua:21-49"]
    }))?;
    let world =
        super::quest_world_callbacks::world_with_cells(&callback, trigger, &[KICK, EDGE, OUTSIDE])?;
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
        (i32::from(KICK.x), i32::from(KICK.y), 7),
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
        (session, lease, KICK),
        (peer_session, peer.current_character_lease(), EDGE),
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
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(debug)?;
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
    step(&mut owner, *entrant, session, StepDirection::South, KICK).await?;
    step(
        &mut owner,
        *occupant,
        peer_session,
        StepDirection::East,
        OUTSIDE,
    )
    .await?;
    step(&mut owner, *entrant, session, StepDirection::South, GROVE).await
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
        "Demon Oak StepIn destination",
    );
    Ok(())
}
