//! The committed Lighthouse map, authored progress and canonical once claims.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session_request};
use super::quest_world_callbacks_fixtures::wire;
use super::questpc8_authored_programmes_fixtures::object_target;
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, FirstEntryOutcome, TestResult};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::starter_grant::StarterTemplate;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelRuntimeV1, CharacterId, CharacterLease,
    MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ControlLossResult, ControllerBinding, FreshAdmissionAuthority, ResumeAttempt,
};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state, log::loader::load_embedded_quest_log,
};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_ITEM_USE_V1, UseDisposition, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection::{self, East, North, South, West};
use oteryn_protocol_oteryn::world_spatial::{
    CAPABILITY_PACED_MOVEMENT_V1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDisposition,
    decode_step_result_paced, encode_step_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
use std::sync::Arc;
use std::time::Duration;

const QUEST: &str = "oteryn:quest.thais_lighthouse_quest";
const STAGE: &str = "oteryn:quest-progress/authored/thais-lighthouse/stage";
const HAMMER: &str = "oteryn:item.tibia.i3305";
const SHIELD: &str = "oteryn:item.tibia.i3421";
const START: TilePos = tile(32227, 32280, 7);
const CAPABILITIES: &[u32] = &[
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_PACED_MOVEMENT_V1,
    CAPABILITY_ITEM_USE_V1,
];

const fn tile(x: u16, y: u16, z: i8) -> TilePos {
    TilePos { x, y, floor: -z }
}

fn local(p: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}

#[test]
fn thais_lighthouse_full_journey() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |_| {
                let h =
                    Harness::create_with_admission_seed(admin, "lighthouse", false, false).await?;
                let result = journey(&h).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Lighthouse PostgreSQL journey panicked")??;
    println!(
        "thais_lighthouse_full_journey POSTGRES_CASE_COMPLETED {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn journey(h: &Harness) -> TestResult {
    let loaded = load_embedded_quest_state("content-1").map_err(debug)?;
    assert!(
        loaded
            .catalogue()
            .authored_bindings()
            .is_some_and(|bindings| bindings.quests.iter().any(|quest| quest.quest == QUEST)),
        "the production catalogue is missing the authored Lighthouse journey"
    );
    let log = load_embedded_quest_log(&loaded)?;
    assert!(
        log.content
            .catalogues()
            .all(|catalogue| catalogue.quests().iter().all(|quest| quest.key != QUEST)),
        "this script-only quest remains absent from the quest log"
    );
    let catalogue = Arc::new(loaded.catalogue().clone());
    let recovery = RecoverySource::start(h).await?;
    let fresh = prepare_owned_session_request(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native catalogue")?;
    let world = super::source_world_crop::compiled_region(
        tile(32222, 32264, 10),
        tile(32234, 32286, 7),
        START,
        native,
    )?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let map = world.map_revision_digest();
    let start = local(START);
    let room = activated
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
        .ok_or("Lighthouse bundle room")?;
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        map,
        (start.x, start.y, start.floor),
    );
    let mut deps = Dependencies::new()?;
    deps.combat_room = Some(room);
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
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
    let slot = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(slot).map_err(debug)?;
    deps.admitted.runtime_actor = Some(deps.actor);
    deps.runtime = tokio::sync::Mutex::new(runtime);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;

    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.evidence = &recovery.evidence;
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(catalogue);
    let mut player = deps.admitted;
    let fence =
        super::death_ml_loss_composed_fixtures::current_fence(&owner, player.game_session_id)
            .await?;
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
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                player.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    player.first_entry = FirstEntryOutcome::Positioned;
    player.item_fence = Some(fence.item_fence());
    player.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), CAPABILITIES)
            .ok_or("capabilities")?;
    assert_eq!(
        player.continuity.selected_capabilities.as_slice(),
        CAPABILITIES
    );
    owner.admit_progression_session(&mut player).await;
    owner.admit_quest_session(&player).await;
    deps.initialize(&owner).await;
    assert_state(&owner, &player, 0, &[]).await?;

    let hammer = object_target(&owner, tile(32225, 32265, 10), 1)?;
    let shield = object_target(&owner, tile(32226, 32265, 10), 1)?;
    walk(
        &mut owner,
        &mut player,
        &[North, North, North, West, West, West, North, North],
    )
    .await?;
    assert_at(&owner, &player, tile(32224, 32275, 8)).await?;
    assert_state(&owner, &player, 1, &[]).await?;

    walk(&mut owner, &mut player, &[East, South]).await?;
    assert_at(&owner, &player, tile(32225, 32276, 8)).await?;
    let (_, disposition) = use_object(&mut owner, &mut player, &hammer).await?;
    assert_eq!(disposition, UseDisposition::TooFar);
    assert_state(&owner, &player, 1, &[]).await?;

    walk(&mut owner, &mut player, &[East, East, South]).await?;
    let ladder_lever = object_target(&owner, tile(32227, 32278, 8), 1)?;
    assert_eq!(
        use_object(&mut owner, &mut player, &ladder_lever).await?.1,
        UseDisposition::Committed
    );
    assert_state(&owner, &player, 1, &[]).await?;
    walk(&mut owner, &mut player, &[West, West, North]).await?;
    assert_at(&owner, &player, tile(32225, 32276, 9)).await?;
    assert_state(&owner, &player, 2, &[]).await?;

    walk(&mut owner, &mut player, &[South; 6]).await?;
    assert_at(&owner, &player, tile(32225, 32282, 9)).await?;
    assert_state(&owner, &player, 2, &[]).await?;
    walk(&mut owner, &mut player, &[North]).await?;
    let mut helper = plate_helper(h, &deps, &owner, &player).await?;
    walk(&mut owner, &mut helper, &[North]).await?;
    assert_at(&owner, &helper, tile(32225, 32268, 9)).await?;
    assert_state(&owner, &player, 2, &[]).await?;

    walk(&mut owner, &mut player, &[South, South]).await?;
    assert_at(&owner, &player, tile(32225, 32284, 10)).await?;
    assert_state(&owner, &player, 2, &[]).await?;
    let portal_lever = object_target(&owner, tile(32225, 32285, 10), 1)?;
    assert_eq!(
        use_object(&mut owner, &mut player, &portal_lever).await?.1,
        UseDisposition::Committed
    );
    assert_state(&owner, &player, 2, &[]).await?;
    walk(&mut owner, &mut player, &[North; 7]).await?;
    assert_at(&owner, &player, tile(32225, 32276, 9)).await?;
    walk(&mut owner, &mut player, &[East; 7]).await?;
    assert_at(&owner, &player, tile(32225, 32274, 10)).await?;
    assert_state(&owner, &player, 3, &[]).await?;
    walk(&mut owner, &mut player, &[North; 8]).await?;

    assert_eq!(
        use_object(&mut owner, &mut player, &hammer).await?,
        (CommandStatus::Accepted, UseDisposition::Committed)
    );
    assert_state(&owner, &player, 3, &[HAMMER]).await?;
    let terminal = player.continuity.next_command_id;
    assert_eq!(
        use_object(&mut owner, &mut player, &shield).await?,
        (CommandStatus::Accepted, UseDisposition::Committed)
    );
    let inventory = assert_state(&owner, &player, 3, &[HAMMER, SHIELD]).await?;

    assert_eq!(
        owner.lose_control(player, Duration::ZERO, None).await,
        ControlLossResult::Recorded
    );
    let token = RecoverySource::token(2)?;
    let mut resumed = owner
        .resume(ResumeAttempt {
            game_session_id: player.game_session_id,
            recovery_material: token.as_bytes(),
            transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
            last_applied_server_sequence: player.continuity.server_sequence,
            supported_capabilities: CAPABILITIES,
        })
        .await
        .map_err(debug)?;
    assert_eq!(resumed.game_session_id, player.game_session_id);
    assert_eq!(resumed.continuity.connection_generation, 2);
    assert_at(&owner, &resumed, tile(32225, 32266, 10)).await?;
    let replay = super::quest_callbacks_fixtures::wire_use_targets_retained(
        &owner,
        &mut resumed,
        &[(terminal, &shield)],
    )
    .await?;
    assert_eq!(
        replay,
        [(CommandStatus::DuplicateReplay, UseDisposition::Committed)]
    );
    assert_eq!(
        assert_state(&owner, &resumed, 3, &[HAMMER, SHIELD]).await?,
        inventory
    );
    for chest in [&shield, &hammer] {
        assert_eq!(
            use_object(&mut owner, &mut resumed, chest).await?.1,
            UseDisposition::NothingToUse
        );
        assert_eq!(
            assert_state(&owner, &resumed, 3, &[HAMMER, SHIELD]).await?,
            inventory
        );
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key IN ('oteryn:reward-claim.kv.lighthouse1', 'oteryn:reward-claim.kv.lighthouse2')",
    ).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 2);
    Ok(())
}

async fn plate_helper(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &AdmittedSession,
) -> TestResult<AdmittedSession> {
    let session =
        super::boss_rewards_fixtures::privacy::admit_gameplay_peer(h, owner.evidence, 1).await?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    let transport = current
        .current_transport()
        .ok_or("plate helper transport")?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("plate helper fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        // The second player begins beside the plate. Only STEP opens the stairs.
        runtime
            .initialize_pinned_test_position(actor, local(tile(32225, 32269, 9)))
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, actor, session).await;
    let account: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(account_id) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid",
    ).bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let mut peer = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: Some(ControllerBinding {
            transport,
            account_id: account.try_into().map_err(|_| "plate helper account")?,
        }),
        continuity: super::SessionContinuity {
            selected_capabilities: player.continuity.selected_capabilities,
            ..super::SessionContinuity::FRESH
        },
        item_fence: Some(fence),
        ..*player
    };
    owner.admit_progression_session(&mut peer).await;
    Ok(peer)
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    target: &WorldObjectTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    let command = player.continuity.next_command_id;
    super::quest_callbacks_fixtures::wire_use_targets_retained(owner, player, &[(command, target)])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| "missing USE result".into())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    steps: &[StepDirection],
) -> TestResult {
    for &direction in steps {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("clock")?;
        let command = player.continuity.next_command_id;
        let reply = wire(
            owner,
            player,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result_paced(&reply, true).map_err(debug)?,
            StepDisposition::Moved,
            "Lighthouse STEP command {command} {direction:?}"
        );
    }
    Ok(())
}

async fn assert_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &AdmittedSession,
    p: TilePos,
) -> TestResult {
    let actor = player.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(p)
    );
    Ok(())
}

async fn assert_state(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &AdmittedSession,
    stage: i64,
    expected_items: &[&str],
) -> TestResult<Vec<[u8; 16]>> {
    let character = player.item_fence.ok_or("player fence")?.character_id;
    let quest = owner
        .root
        .read_character_quest_state(owner.character, character)
        .await
        .map_err(debug)?;
    assert_eq!(quest.tracks().get(STAGE).copied().unwrap_or(0), stage);
    assert_eq!(
        quest
            .states()
            .get(QUEST)
            .is_some_and(|state| state.completed),
        expected_items.len() == 2
    );
    for (marker, item) in [
        ("oteryn:quest-progress/kv/lighthouse1/claimed", HAMMER),
        ("oteryn:quest-progress/kv/lighthouse2/claimed", SHIELD),
    ] {
        assert_eq!(
            quest.tracks().get(marker).copied().unwrap_or(0),
            i64::from(expected_items.contains(&item))
        );
    }
    let bag = owner
        .root
        .read_character_backpack(owner.character, character)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    let actual = bag
        .entries
        .iter()
        .map(|entry| {
            assert_eq!(entry.item.quantity, 1);
            entry.item.definition.production_key.as_str()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected_items);
    Ok(bag
        .entries
        .iter()
        .map(|entry| entry.item.item_instance_id)
        .collect())
}
