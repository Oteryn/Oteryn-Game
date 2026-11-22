use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session, reset};

use crate::durability::character_last_position::LastPositionWrite;
use crate::durability::premium_fence::EntitlementState;
use crate::foundation::{AuthenticatedTransportRefV1, ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::map::overlay::TilePos;

// One walkable cell from Crystal's specialtiles.xml rectangle, retained in the admitted B3.
const SAVED: TilePos = TilePos {
    x: 32359,
    y: 32240,
    floor: -7,
};
const THAIS: TilePos = TilePos {
    x: 32369,
    y: 32241,
    floor: -7,
};
const START: TilePos = TilePos {
    x: 32369,
    y: 32240,
    floor: -7,
};

#[test]
fn p20_lapsed_free_hometown_character_on_premium_tile_enters_free_temple() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_large_stack(admin, EntitlementState::Expired)
}

#[test]
fn p20_premium_character_keeps_same_saved_tile() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_large_stack(admin, EntitlementState::Active)
}

fn on_large_stack(admin: String, state: EntitlementState) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(Box::pin(scenario(&admin, state)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "P20 saved tile PostgreSQL thread panicked")??;
    Ok(())
}

async fn scenario(admin: &str, state: EntitlementState) -> TestResult {
    reset();
    let tag = if state == EntitlementState::Expired {
        "p20_tile_expired"
    } else {
        "p20_tile_current"
    };
    let h = Harness::create_with_admission_seed(admin, tag, false, false).await?;
    let request = prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let scope = super::fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let (controller, pin) = production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let bundle = super::source_world_crop::compiled_region(SAVED, THAIS, START, native)?;
    let towns = &bundle.base().subareas;
    let default_town = towns.default_town.ok_or("default town")?;
    let temple = towns.town(8).and_then(|town| town.temple);
    assert_eq!(
        temple.map(|tile| (tile.x, tile.y, tile.floor)),
        Some((THAIS.x, THAIS.y, THAIS.floor))
    );
    assert!(bundle.enterable(SAVED));
    assert!(bundle.enterable(THAIS));
    let cells = bundle
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let start = position(START);
    let room = pin
        .qualified_room()
        .bundle_view(
            cells.clone(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("bundle room")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(channel_pin.server_artifact_digest().as_slice())
        .bind(channel_pin.client_artifact_digest().as_slice())
        .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;

    let account = id(40);
    let (consumer, clock, activation, mut proof) = purchase_fixture::current(account);
    if state == EntitlementState::Expired {
        proof.state = state;
        proof.authority_revision += 1;
        consumer.prove_for_test(proof);
    }
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us,
        )
        .await?;
    let coordinator = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
    deps.combat_room = Some(deps.room.clone());
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.premium_coordinator = Some(&coordinator);

    let session = deps.admitted.game_session_id;
    let initial_fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("gameplay fence before citizenship")?;
    assert_eq!(
        h.root
            .character_home_town(
                &authority,
                owner.holder,
                initial_fence,
                default_town,
                Some(8),
                None
            )
            .await?,
        8,
    );
    assert_eq!(owner.home_town(session, None).await, Some(8));
    let saved = {
        let runtime = deps.runtime.lock().await;
        owner.placed_record(&runtime, position(SAVED))
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("gameplay fence")?;
    assert_eq!(
        h.root
            .write_character_last_position(
                &authority,
                owner.holder,
                fence,
                saved.write_sequence,
                saved.record
            )
            .await?,
        LastPositionWrite::Written,
    );

    let (outcome, _, _) = Box::pin(owner.initialize_first_entry(
        &request,
        session,
        AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
        deps.actor,
    ))
    .await;
    assert_eq!(outcome, FirstEntryOutcome::Positioned);
    let expected = if state == EntitlementState::Expired {
        THAIS
    } else {
        SAVED
    };
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        position(expected),
    );
    let stored = h
        .root
        .read_character_last_position(&authority, owner.holder, fence)
        .await?
        .ok_or("durable position")?;
    assert_eq!(
        (
            stored.record.cell.x,
            stored.record.cell.y,
            stored.record.cell.floor
        ),
        (
            i32::from(expected.x),
            i32::from(expected.y),
            i16::from(-expected.floor)
        )
    );
    assert_eq!(owner.home_town(session, None).await, Some(8));
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn position(tile: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(tile.x),
        y: i32::from(tile.y),
        floor: i16::from(-tile.floor),
    }
}
