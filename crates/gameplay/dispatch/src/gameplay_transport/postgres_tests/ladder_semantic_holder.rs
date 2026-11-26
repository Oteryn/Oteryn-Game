//! Adjacent native ladder USE while an unrelated semantic pass owns the holder.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, MovementLocalPosition, WorldId};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, SessionContinuity,
    regression_world_use_with_busy_holder,
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::{self as wire, world_object};

#[test]
fn use_ladder_with_semantic_holder_busy_commits_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("ladder-semantic-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "ladder_semantic_holder", false, false)
                    .await?;
            scenario(&h).await?;
            h.cleanup().await?;
            Ok(())
        })
    })
}

async fn scenario(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let start = Position {
        x: 32320,
        y: 32220,
        floor: -7,
    };
    let (world, _) = super::annihilator_route_fixtures::source_world(native, start, false)?;
    let cells = world
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = super::rust_lottery_fixtures::bundle_action_room(
        pin.qualified_room(),
        &world,
        world_id,
        &cells,
    )?;
    let content = ChannelContentPin::from_activation(
        world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (i32::from(start.x), i32::from(start.y), 7),
    );
    let mut deps = Dependencies::with_content(content, room.clone(), native.spell_book().clone())?;
    deps.combat_room = Some(room.clone());
    let actor = deps.actor;
    let game_session_id = deps.admitted.game_session_id;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(game_session_id)
        .await?
        .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, game_session_id, current.current_character_lease())
            .map_err(debug)?;
    }
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;

    let map = owner.bundle_map().ok_or("ladder bundle map")?;
    let spot = crate::map::overlay::TilePos {
        x: 32321,
        y: 32220,
        floor: -7,
    };
    let base = map
        .overlay
        .base()
        .tile(spot.x, spot.y, spot.floor)
        .ok_or("ladder tile")?;
    let ordinal = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let entry = map.facts.base_entry(spot, ordinal, *id)?;
            (entry.appearance_id == 1948).then_some(ordinal)
        })
        .ok_or("authored appearance 1948")?;
    let key = crate::map::view::placement_key(spot, ordinal).ok_or("ladder placement")?;
    let target = world_object::WorldObjectTarget {
        placement: map
            .overlay
            .base()
            .digest()
            .into_iter()
            .chain(key.to_be_bytes())
            .collect(),
        expected_revision: map.facts.object_revision(key),
    };
    let admitted = AdmittedSession {
        item_fence: Some(fence),
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("current transport")?,
            account_id: id(40),
        }),
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 6],
            )
            .ok_or("world USE capabilities")?,
            ..SessionContinuity::FRESH
        },
        ..deps.admitted
    };
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .command_selected(world_object::COMMAND_TYPE_USE_INTENT)
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    tokio::time::timeout(
        std::time::Duration::from_secs(20),
        regression_world_use_with_busy_holder(&owner, admitted, &mut server, target),
    )
    .await??;
    drop(server);
    loop {
        let frame = super::item_look_fixtures::read_frame(&mut client).await?;
        let envelope = wire::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == wire::MessageType::CommandResult {
            let reply = wire::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.command_id, admitted.continuity.next_command_id);
            assert_eq!(reply.status, wire::CommandStatus::Accepted);
            assert_eq!(
                world_object::decode_use_result(reply.payload).map_err(debug)?,
                world_object::UseDisposition::Committed
            );
            break;
        }
    }
    assert_eq!(
        super::full_world_support::position(&owner, actor).await?,
        MovementLocalPosition {
            x: 32321,
            y: 32221,
            floor: 6
        },
    );
    Ok(())
}
