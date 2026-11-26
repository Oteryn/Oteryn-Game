//! PARITY-REFUSAL-DETAIL-1: real session ingress on a bundle World answers an unreachable map
//! door `TOO_FAR`; with capability 66 field 15 names `TOO_FAR_AWAY`, without it the bytes are
//! the pre-66 payload.
use super::autowalk_fixtures::{COIN, DOOR, at, bundle_with_scenery, object, pos, result, send};
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::map::boot::{BootPins, boot};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::refusal_detail::CAPABILITY_REFUSAL_DETAIL_V1;
use oteryn_protocol_oteryn::world_object::encode_use_intent;
use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

#[test]
fn refusal_detail_bundle_world_door_too_far_with_and_without_capability_66() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario(false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "refusal detail e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn shrine_use_requires_reach_and_knowledge_and_door_still_opens() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "shrine use scenario panicked")?
        .map_err(Into::into)
}

const SHRINE: &str = "oteryn:item.tibia.i25060";

fn scenario(shrine_use: bool) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "refusal_detail", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        deps.room = pin.qualified_room().clone();
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key: &str| {
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("item ref")
        };
        let scenery = if shrine_use { SHRINE } else { COIN };
        let start = if shrine_use { (2, 5) } else { (1, 1) };
        let (bytes, pins) = bundle_with_scenery(
            reference(DOOR)?.get() - 1,
            (scenery, reference(scenery)?.get() - 1),
        )?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos(start.0, start.1),
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: reference(key).ok()?,
                    solid: Some(key == DOOR || key == SHRINE),
                    blocks_projectile: key == DOOR || key == SHRINE,
                    pickupable: key == COIN,
                    door: key == DOOR,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let channel_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (i32::from(start.0), i32::from(start.1), 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at(i32::from(start.0), i32::from(start.1)))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let digest = world.base().digest();
        if shrine_use {
            deps.initialize(&owner).await;
            tokio::select! {
                result = shrine_and_door(&h, &owner, admitted, digest) => result?,
                result = super::item_craft_fixtures::maintain_ready(&owner) => result?,
            }
        } else {
            // The bundle World offers capability 66; the door at (12, 1) has no route.
            for (command_id, requested, expected) in [
                (
                    1,
                    &[4, 6, 11, 15, 18, 31, CAPABILITY_REFUSAL_DETAIL_V1][..],
                    &[0x08, 0x05, 0x78, 0x02][..],
                ),
                (2, &[4, 6, 11, 15, 18, 31][..], &[0x08, 0x05][..]),
            ] {
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(owner.offered_capabilities(), requested)
                        .ok_or("selection")?;
                assert_eq!(admitted.continuity.selected_capabilities.as_slice(), requested);
                admitted = refused_use(&owner, admitted, digest, command_id, expected).await?;
                // A refused world action keeps its 200 ms cooldown.
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn refused_use(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    digest: [u8; 32],
    command_id: u64,
    expected: &[u8],
) -> TestResult<AdmittedSession> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_flow = async {
        let outcome = client_scenario(&mut client, digest, command_id, expected).await;
        client.shutdown().await?;
        outcome
    };
    let server_flow = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS);
    let (end, outcome) = join_two(server_flow, client_flow).await;
    outcome?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    Ok(admitted)
}

async fn client_scenario(
    client: &mut DuplexStream,
    digest: [u8; 32],
    command_id: u64,
    expected: &[u8],
) -> TestResult {
    loop {
        let len = tokio::time::timeout(Duration::from_secs(20), client.read_u32()).await??;
        let mut frame = vec![0; usize::try_from(len)?];
        client.read_exact(&mut frame).await?;
        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    send(
        client,
        command_id,
        2,
        &encode_use_intent(&object(digest, pos(12, 1), 0)?).map_err(debug)?,
    )
    .await?;
    let (refused, steps) = result(client, command_id).await?;
    assert_eq!(steps, 0);
    assert_eq!(refused, expected);
    Ok(())
}

async fn shrine_and_door(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    digest: [u8; 32],
) -> TestResult {
    use crate::gameplay_transport::connection::test_world_object_use_route;
    use oteryn_protocol_oteryn::imbuement::{
        CAPABILITY_IMBUE_V1, CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1, ImbuementResult,
        decode_imbuement_reply_with_success,
    };
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};

    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 11, 15, 18, 31, CAPABILITY_IMBUE_V1, CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1],
    ).ok_or("shrine capabilities")?;
    let shrine = object(digest, pos(1, 5), 0)?;
    let far = object(digest, pos(7, 5), 0)?;
    let bytes = test_world_object_use_route(owner, admitted, 1, &shrine).await?;
    assert_eq!(decode_use_result(&bytes).map_err(debug)?, UseDisposition::RequirementNotMet);

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .bind("oteryn:quest-progress/quest/u11_02/forgotten_knowledge/tomes")
        .bind("oteryn:quest.forgotten_knowledge_quest")
        .execute(&mut *tx).await?;
    tx.commit().await?;

    let bytes = test_world_object_use_route(owner, admitted, 2, &far).await?;
    assert_eq!(decode_use_result(&bytes).map_err(debug)?, UseDisposition::TooFar);

    let bytes = test_world_object_use_route(owner, admitted, 3, &shrine).await?;
    let (reply, _) = decode_imbuement_reply_with_success(&bytes).map_err(debug)?;
    assert_eq!(reply.result, ImbuementResult::Ok);
    let window = reply.window.ok_or("shrine opens the item chooser")?;
    assert_eq!(window.slot_count, 0);

    let facts = &owner.bundle_map().ok_or("bundle map")?.facts;
    let door_tile = pos(1, 6);
    assert!(facts.doors().blocks(door_tile));
    let door = object(digest, door_tile, 0)?;
    let bytes = test_world_object_use_route(owner, admitted, 4, &door).await?;
    assert_eq!(decode_use_result(&bytes).map_err(debug)?, UseDisposition::Committed);
    assert!(!facts.doors().blocks(door_tile));
    Ok(())
}
