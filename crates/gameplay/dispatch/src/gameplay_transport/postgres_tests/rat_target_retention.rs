//! The placed Rat keeps its original victim when a failed chase becomes a reachable melee.
use super::fixture::{Harness, debug, id};
use super::loop_monster_fixtures::fixture::run_case;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::analyser::{
    AnalyserDamageSource, AnalyserFact, decode_attributed_analyser_facts,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::time::Duration;

const RAT: &str = "oteryn:creature.rat";
const RAT_AT: MovementLocalPosition = cell(1, 1);
const CASTER_AT: MovementLocalPosition = cell(2, 2);
const CASTER_OUT: MovementLocalPosition = cell(3, 2);
const OTHER_AT: MovementLocalPosition = cell(3, 1);
const OTHER_TARGET_AT: MovementLocalPosition = cell(3, 0);
const FAR_AT: MovementLocalPosition = cell(5, 2);
const FAR_TARGET_AT: MovementLocalPosition = cell(6, 2);

const fn cell(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

type Player = (ExactActorRef, GameSessionId);

#[test]
fn placed_rat_keeps_reachable_victim_and_lands_exact_melee_among_other_monsters() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(scenario)
}

async fn scenario(harness: &Harness) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let world_id = deps.admitted.world_id;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world_id, false);
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let world = bundle_world(&deps, native)?;
    let entry = pin.qualified_room();
    let cells = world
        .movement_cells(entry.movement_cells())
        .map_err(debug)?;
    let room = entry
        .bundle_view(
            cells,
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: CASTER_AT.x,
                y: CASTER_AT.y,
                floor: CASTER_AT.floor,
            },
        )
        .ok_or("bundle combat room")?;
    let channel_pin = ChannelContentPin::from_activation(
        world_id,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (CASTER_AT.x, CASTER_AT.y, CASTER_AT.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin.clone(),
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;

    // The two unpositioned reservations are committed first to select a reproducible tie
    // order. The caster loses that tie if the failed path forces a new nearest search.
    let a = player(&mut runtime, deps.admitted.game_session_id)?;
    let b = player(&mut runtime, GameSessionId::decode(&id(51)).map_err(debug)?)?;
    let (caster, other) = if a.0.placement_identity() > b.0.placement_identity() {
        (a, b)
    } else {
        (b, a)
    };
    let far = player(&mut runtime, GameSessionId::decode(&id(52)).map_err(debug)?)?;
    for ((actor, session), at) in [
        (caster, CASTER_AT),
        (other, OTHER_TARGET_AT),
        (far, FAR_TARGET_AT),
    ] {
        runtime
            .initialize_source_pinned_lab_player_position(actor, session, at)
            .map_err(debug)?;
    }
    let near_rat = runtime
        .realize_native_qualification_spawn_for_source(40_000, channel_pin.clone(), RAT, OTHER_AT)
        .map_err(debug)?
        .actor;
    let far_rat = runtime
        .realize_native_qualification_spawn_for_source(40_001, channel_pin.clone(), RAT, FAR_AT)
        .map_err(debug)?
        .actor;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.room = room;
    deps.combat_room = Some(deps.room.clone());
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();

    initialize(&owner, &[other, far]).await?;
    let mut now = SemanticTimeMicros::from_micros(0);
    owner.drain_monster_melee_at(now).await;
    assert_eq!(target(&owner, near_rat).await, Some(other.0));
    assert_eq!(target(&owner, far_rat).await, Some(far.0));

    initialize(&owner, &[caster]).await?;
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(caster.0);
    admitted.game_session_id = caster.1;
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 10, 486])
            .ok_or("analyser attribution capabilities")?;
    owner
        .analyser_snapshot(admitted)
        .await
        .ok_or("caster analyser")?;
    let rat = owner
        .runtime
        .lock()
        .await
        .realize_native_qualification_spawn_for_source(40_002, channel_pin, RAT, RAT_AT)
        .map_err(debug)?
        .actor;

    advance(&owner, &mut now).await?;
    assert_eq!(
        target(&owner, rat).await,
        Some(caster.0),
        "late placement selects the adjacent caster"
    );
    assert_eq!(target(&owner, near_rat).await, Some(other.0));
    assert_eq!(target(&owner, far_rat).await, Some(far.0));
    step(&owner, caster.0, CASTER_OUT).await?;
    advance(&owner, &mut now).await?;
    assert_eq!(target(&owner, rat).await, Some(caster.0));
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(rat)
            .map_err(debug)?
            .position(),
        RAT_AT
    );

    // The Rat has no cardinal exit from its island. Both victims are now in diagonal
    // melee range. Only the original target is entitled to the held melee occurrence.
    step(&owner, caster.0, CASTER_AT).await?;
    step(&owner, other.0, cell(2, 0)).await?;
    let exact = EntityRef {
        identity: rat.placement_identity(),
        generation: rat.actor_local_generation(),
    };
    let interval_us = {
        let states = owner.states.spell_states.lock().await;
        states
            .monster_melee
            .creature_ai
            .state(rat)
            .ok_or("admitted Rat")?
            .profile()
            .behavior()
            .attacks
            .first()
            .ok_or("authored Rat melee")?
            .interval_ms
            * 1_000
    };
    // The authored range includes zero. Allow one further attack interval for a zero roll.
    let deadline = now.get() + 2 * interval_us;
    while now.get() < deadline {
        advance(&owner, &mut now).await?;
        assert_eq!(
            target(&owner, rat).await,
            Some(caster.0),
            "a stale path failure dropped an adjacent victim"
        );
        while let Some(payload) = owner.analyser_next(admitted).await {
            if decode_attributed_analyser_facts(&payload)
                .map_err(debug)?
                .iter()
                .any(|event| {
                    event.attacker == Some(exact)
                        && matches!(
                            event.fact,
                            AnalyserFact::DamageInput {
                                source: AnalyserDamageSource::Creature(_),
                                ..
                            }
                        )
                })
            {
                return Ok(());
            }
        }
    }
    Err(
        format!("Rat {exact:?} landed no attributed melee within two authored attack intervals")
            .into(),
    )
}

fn player(runtime: &mut ChannelRuntimeV1, session: GameSessionId) -> TestResult<Player> {
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    Ok((
        runtime.commit_fresh_session(reservation).map_err(debug)?,
        session,
    ))
}

async fn initialize(owner: &ComposedFreshAdmission<'_, '_, '_>, players: &[Player]) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    for &(actor, session) in players {
        states
            .initialize(
                &runtime,
                actor,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize combat vitals")?;
    }
    Ok(())
}

async fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> Option<ExactActorRef> {
    owner
        .states
        .spell_states
        .lock()
        .await
        .monster_melee
        .creature_ai
        .state(actor)?
        .target()
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    to: MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let from = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(from, to)
        .map_err(debug)?;
    Ok(())
}

async fn advance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    now: &mut SemanticTimeMicros,
) -> TestResult {
    let interval = Duration::from_millis(crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS);
    // SQL deadlines retain the real clock. Advance only the owner time between turns.
    tokio::time::pause();
    let start = tokio::time::Instant::now();
    tokio::time::advance(interval).await;
    *now = SemanticTimeMicros::from_micros(now.get() + u64::try_from(start.elapsed().as_micros())?);
    tokio::time::resume();
    owner.drain_monster_melee_at(*now).await;
    Ok(())
}

fn bundle_world(
    deps: &Dependencies,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::Tile;
    let item_id = |key: &str| {
        crate::map::boot::native_item_definition(Some(native), key)
            .map(|item| item.reference.get() - 1)
            .ok_or_else(|| format!("missing fixture item definition {key}"))
    };
    let bag = "oteryn:item.tibia.i2854";
    let coin = "oteryn:item.tibia.i3031";
    let (bytes, mut pins) =
        crate::map::boot::tests::bundle_with_item_ids((bag, item_id(bag)?), (coin, item_id(coin)?));
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    bundle.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    let sector = bundle.sectors.first_mut().ok_or("fixture sector")?;
    let ground = sector.tiles.first().ok_or("fixture ground")?.items.clone();
    sector.tiles = [
        cell(2, 0),
        OTHER_TARGET_AT,
        RAT_AT,
        OTHER_AT,
        CASTER_AT,
        CASTER_OUT,
        FAR_AT,
        FAR_TARGET_AT,
    ]
    .into_iter()
    .map(|at| {
        Ok(Tile {
            x: u16::try_from(at.x)?,
            y: u16::try_from(at.y)?,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: ground.clone(),
        })
    })
    .collect::<TestResult<_>>()?;
    let bytes =
        oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
            .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: u16::try_from(CASTER_AT.x)?,
                y: u16::try_from(CASTER_AT.y)?,
                floor: -7,
            },
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(Into::into)
}
