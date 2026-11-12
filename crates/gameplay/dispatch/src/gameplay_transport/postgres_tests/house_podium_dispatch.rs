//! W18: house-podium commands through framed ingress, composed dispatch, and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MessageType, MovementLocalPosition,
    decode_wire_envelope,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    house_podium as wire,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const POSITION: [i32; 3] = [32310, 32804, 7];
const RAT: &str = "oteryn:creature.rat";

#[derive(Clone, Copy)]
enum Case {
    Roundtrip,
    Refusals,
    MissingCapability,
}

#[test]
fn house_podium_dispatch_query_set_projection_roundtrip_postgres() -> TestResult {
    postgres_case("house_podium_roundtrip", Case::Roundtrip)
}

#[test]
fn house_podium_dispatch_refusals_leave_display_unchanged_postgres() -> TestResult {
    postgres_case("house_podium_refusals", Case::Refusals)
}

#[test]
fn house_podium_dispatch_without_capability_150_is_rejected_postgres() -> TestResult {
    postgres_case("house_podium_without_capability", Case::MissingCapability)
}

fn postgres_case(name: &'static str, case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
                let result = scenario(&h, case).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "house podium dispatch thread panicked")??;
    Ok(())
}

async fn scenario(h: &Harness, case: Case) -> TestResult {
    super::reset();
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let world = super::house_wrap::wrap_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    let cells = world
        .movement_cells(base.room.movement_cells())
        .map_err(debug)?;
    let mut packet = cosmetics_test_packet();
    packet["authoring"]["document"]["content_revision"] = "content-1".into();
    let outfit_key = packet["authoring"]["document"]["outfits"][0]["key"]
        .as_str()
        .ok_or("cosmetics outfit key")?
        .to_owned();
    let bytes = serde_json::to_vec(&packet)?;
    let (controller, _) =
        activated_with_optional_cosmetics(base.admitted.world_id, None, Some(&bytes));
    let active = controller.active().ok_or("active content")?;
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        [2; 32],
        [3; 32],
        [4; 32],
        (32309, 32805, 7),
    );
    let deps = Dependencies::with_content(pin, base.room, base.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    const OFFERS: &[crate::gameplay_transport::capabilities::OfferedCapability] = &[
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 6,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 49,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 86,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 150,
            requires: &[49],
        },
    ];
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[], crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id), 1, "policy-1", OFFERS,
    ).map_err(debug)?;
    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    let selected: &[u32] = if matches!(case, Case::MissingCapability) {
        &[4, 6, 49, 86]
    } else {
        &[4, 6, 49, 86, 150]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), selected)
            .ok_or("house podium offer")?;
    let mut body = vec![8, 1, 18, HOUSE.len() as u8];
    body.extend(HOUSE.as_bytes());
    let (inside, _) = owner
        .transfer_house(admitted, 2, &body)
        .await
        .ok_or("house transfer outcome")?;
    let mut admitted = inside.ok_or("house transfer admission")?;

    super::item_look::seed_main_backpack(h, &[]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i42367','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(73).as_slice()).execute(&mut *tx).await?;
    let position: Vec<u8> = POSITION
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect();
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(&position)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(74).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.continuity.next_command_id = 1;

    let set = wire::SetRequest {
        item: id(71),
        expected_revision: 0,
        display_kind: 3,
        direction: 2,
        key: RAT.into(),
    }
    .encode()
    .ok_or("rat set payload")?;
    if matches!(case, Case::MissingCapability) {
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set).await?,
            Some((CommandStatus::Rejected, Vec::new()))
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_podium_displays")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        return Ok(());
    }

    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(&owner, &mut admitted, command, wire::COMMAND_QUERY, &id(71))
        .await?
        .ok_or("query command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(wire::Projection::decode(&payload), Some(None));
    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set)
        .await?
        .ok_or("set command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::SetResult::decode(&payload),
        Some(wire::SetResult::Accepted { revision: 1 })
    );
    let expected = Some(wire::Projection {
        podium_kind: 3,
        display_kind: 3,
        direction: 2,
        revision: 1,
        key: RAT.into(),
    });

    if matches!(case, Case::Roundtrip) {
        let command = admitted.continuity.next_command_id;
        let result = exchange(
            &owner,
            &mut admitted,
            command,
            wire::COMMAND_PROJECTION,
            &id(71),
        )
        .await?
        .ok_or("projection command result")?;
        assert_eq!(result.0, CommandStatus::Accepted);
        assert_eq!(wire::Projection::decode(&result.1), Some(expected));
        let stored: (String, String) =
            sqlx::query_as("SELECT revision::text,display_key FROM game_house_podium_displays")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(stored, ("1".into(), RAT.into()));
        return Ok(());
    }

    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(&owner, &mut admitted, command, wire::COMMAND_SET, &set)
        .await?
        .ok_or("stale set command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::SetResult::decode(&payload),
        Some(wire::SetResult::Refused(wire::SetRefusal::Changed))
    );
    let wrong_display = wire::SetRequest {
        item: id(71),
        expected_revision: 1,
        display_kind: 1,
        direction: 2,
        key: outfit_key,
    }
    .encode()
    .ok_or("outfit set payload")?;
    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(
        &owner,
        &mut admitted,
        command,
        wire::COMMAND_SET,
        &wrong_display,
    )
    .await?
    .ok_or("wrong-display command result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::SetResult::decode(&payload),
        Some(wire::SetResult::Refused(wire::SetRefusal::WrongDisplay))
    );
    let command = admitted.continuity.next_command_id;
    let (status, payload) = exchange(
        &owner,
        &mut admitted,
        command,
        wire::COMMAND_PROJECTION,
        &id(71),
    )
    .await?
    .ok_or("unchanged projection result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(wire::Projection::decode(&payload), Some(expected));
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult<Option<(CommandStatus, Vec<u8>)>> {
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    let mut reply = None;
    while cursor < bytes.len() {
        let n = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&bytes[cursor + 4..cursor + 4 + n]).map_err(debug)?;
        cursor += 4 + n;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            reply = Some((result.status, result.payload.to_vec()));
        }
    }
    Ok(reply)
}
