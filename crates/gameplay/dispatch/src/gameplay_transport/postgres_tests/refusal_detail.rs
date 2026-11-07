//! PARITY-REFUSAL-DETAIL-1: real session ingress on a bundle World answers an unreachable map
//! door `TOO_FAR`; with capability 66 field 15 names `TOO_FAR_AWAY`, without it the bytes are
//! the pre-66 payload.
use super::autowalk::{COIN, DOOR, at, bundle, object, pos, result, send};
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
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "refusal detail e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "refusal_detail", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
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
        let (bytes, pins) = bundle(reference(DOOR)?.get() - 1, reference(COIN)?.get() - 1)?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos(1, 1),
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: reference(key).ok()?,
                    solid: Some(key == DOOR),
                    blocks_projectile: key == DOOR,
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
            (1, 1, 7),
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
            .initialize_pinned_test_position(deps.actor, at(1, 1))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
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
