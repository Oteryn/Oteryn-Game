//! Dormant capability server qualification through the real composed command route.
use super::house_transfer as house;
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{
    FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::podium::{self as wire, Display, Request};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

fn assert_window(
    bytes: &[u8],
    revision: u64,
    display: Display,
    palette: &[[u8; 3]],
    mounts: &[u16],
) -> TestResult {
    assert_eq!(&bytes[..2], &[0, 0]);
    assert_eq!(&bytes[2..10], &revision.to_be_bytes());
    assert_eq!(
        wire::decode_display(&bytes[10..25]).map_err(debug)?,
        display
    );
    assert_eq!(&bytes[25..28], &[1, 0, 133]);
    assert_eq!(&bytes[28..427], palette.as_flattened());
    // The independently authored male fixture owns look 128 and starter addon 1,
    // but not addon 2. A window must not advertise the unowned addon.
    assert_eq!(&bytes[427..432], &[0, 1, 0, 128, 1]);
    assert_eq!(&bytes[432..434], &(mounts.len() as u16).to_be_bytes());
    let encoded_mounts: Vec<_> = mounts.iter().flat_map(|v| v.to_be_bytes()).collect();
    assert_eq!(&bytes[434..], encoded_mounts);
    Ok(())
}

#[test]
fn podium_postgres_real_dispatch_mutable_rotation_reload_and_refusals() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "podium scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    let admin =
        crate::bestiary_postgres_harness::configured_admin().ok_or("routed PostgreSQL required")?;
    let rt = crate::bestiary_postgres_harness::runtime()?;
    for guest in [false, true] {
        let h = rt.block_on(Harness::create_appearance(
            admin.clone(),
            if guest {
                "podium_guest"
            } else {
                "podium_owner"
            },
        ))?;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rt.block_on(case(&h, guest))
        }));
        let cleanup = rt.block_on(h.cleanup());
        match outcome {
            Ok(result) => {
                result?;
                cleanup?;
            }
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    request: &Request,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let body = wire::encode_request(request).map_err(debug)?;
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: id,
            command_type: 154,
            payload: &body,
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
    match &end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(state)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(state, _) => {
            *admitted = *state
        }
        _ => {}
    }
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut at = 0;
    while at < bytes.len() {
        let n = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
        let env =
            crate::foundation::decode_wire_envelope(&bytes[at + 4..at + 4 + n]).map_err(debug)?;
        at += 4 + n;
        if env.message_type() == crate::foundation::MessageType::CommandResult {
            let result = decode_command_result(env.payload()).map_err(debug)?;
            return Ok((result.status, result.payload.to_vec()));
        }
    }
    Err(format!("missing podium command result: {end:?}").into())
}

async fn direct_exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    id: u64,
    request: &Request,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let body = wire::encode_request(request).map_err(debug)?;
    crate::gameplay_transport::podium::dispatch(
        owner,
        admitted,
        oteryn_protocol_oteryn::ClientCommandView {
            command_id: id,
            command_type: 154,
            payload: &body,
        },
    )
    .await
    .ok_or_else(|| "direct podium route".into())
}
async fn case(h: &Harness, guest: bool) -> TestResult {
    reset();
    let source = house::RecoverySource::start(h).await?;
    house::prepare_owned_session(h).await?;
    if guest {
        h.seed_appearance_observer().await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let world = house::transfer_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    let cells = world
        .movement_cells(base.room.movement_cells())
        .map_err(debug)?;
    let mut packet = cosmetics_test_packet();
    packet["authoring"]["document"]["content_revision"] = "content-1".into();
    let mount_key = packet["authoring"]["document"]["mounts"][0]["key"]
        .as_str()
        .ok_or("mount key")?
        .to_owned();
    let document = &mut packet["authoring"]["document"];
    if document["grant_mappings"].is_null() {
        document["grant_mappings"] = serde_json::json!([]);
    }
    document["grant_mappings"].as_array_mut().ok_or("grant mappings")?.push(serde_json::json!({
        "grant_key":"podium-fixture-mount","target":{"kind":"Mount","mount_key":mount_key},"compatible_provenance":["content-1"]
    }));
    document["grant_mappings"].as_array_mut().ok_or("grant mappings")?.push(serde_json::json!({
        "grant_key":"podium-fixture-incompatible-mount","target":{"kind":"Mount","mount_key":mount_key},"compatible_provenance":["content-0"]
    }));
    let mappings = packet["authoring"]["document"]["grant_mappings"].clone();
    let mut promotion = packet["authoring"]["promotions"][0].clone();
    promotion["path"] = "/grant_mappings".into();
    promotion["value"] = mappings;
    packet["authoring"]["promotions"]
        .as_array_mut()
        .ok_or("promotions")?
        .push(promotion);
    let (controller, pin) = activated_with_optional_cosmetics(
        base.admitted.world_id,
        None,
        Some(&serde_json::to_vec(&packet)?),
    );
    let room = pin.qualified_room().clone();
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let deps = Dependencies::with_content(channel_pin, room, base.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
    owner.evidence = &source.evidence;
    owner.active_generation = controller.active();
    deps.initialize(&owner).await;
    let content = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.cosmetics())
        .ok_or("cosmetics")?;
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
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!("podium/owned_house.sql"))
        .bind(id(42).as_slice())
        .bind(HOUSE)
        .bind(id(if guest { 101 } else { 41 }).as_slice())
        .bind(content.source_digest().as_slice())
        .execute(&h.pool)
        .await?;
    if guest {
        // Independent historical invitation. Production mutation re-reads this ACL.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_house_acl_entries VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
            .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,1").bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    assert!(matches!(
        h.root
            .assign_house_scope(HouseScopeAssignmentRequest {
                operation_key: OperationKey::from_bytes([83; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                actor_role: "oteryn_test_admin".into(),
                command: HouseScopeAssignmentCommand::Assign {
                    house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                    target: h.node.fact()
                }
            })
            .await
            .map_err(debug)?,
        HouseScopeAssignmentOutcome::Committed(_)
    ));
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    assert!(!owner.offered_capabilities().iter().any(|c| c.id == 133));
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[86]).ok_or("selection")?;
    let (mut inside, status, _) = house::drive(&owner, admitted, false).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let state = owner
        .runtime
        .lock()
        .await
        .house_actor_state(deps.actor, inside.game_session_id)
        .map_err(debug)?;
    let position = [
        state.position.x,
        state.position.y,
        i32::from(state.position.floor),
    ]
    .iter()
    .flat_map(|v| v.to_be_bytes())
    .collect::<Vec<_>>();
    sqlx::query("INSERT INTO game_house_tiles(world_id,house_key,spatial_position) VALUES(encode($1,'hex')::uuid,$2,$3)").bind(id(42).as_slice()).bind(HOUSE).bind(&position).execute(&h.pool).await?;
    // Historical carried Item fixture only. Placement below uses the real House writer.
    let item = id(180);
    let backpack = id(181);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (item, key) in [
        (item, "oteryn:item.tibia.i35973"),
        (backpack, "oteryn:item.tibia.i2854"),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,game_character_uuid_v7())").bind(item.as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())").bind(id(41).as_slice()).bind(backpack.as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,game_character_uuid_v7())").bind(item.as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(backpack.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    use crate::durability::house_interior::{
        HouseCharacterSource, HouseItemAction, HouseItemOutcome, HouseItemRequest,
    };
    let fence = inside.item_fence.ok_or("house fence")?;
    let place = HouseItemRequest {
        command: CommandRef::new(inside.game_session_id, CommandId::new(150).map_err(debug)?),
        house: HouseId::new(inside.world_id, HOUSE).ok_or("house")?,
        item_instance_id: item,
        action: HouseItemAction::Place {
            source: HouseCharacterSource::Inventory {
                parent: backpack,
                ordinal: 1,
            },
            position,
        },
        expected_acl_revision: 1,
        expected_guild_revisions: None,
        backpack: None,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let frozen = h
        .root
        .freeze_house_item(&authority, &h.node, fence, place)
        .await
        .map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_house_item(&authority, &h.node, fence, &frozen)
            .await
            .map_err(debug)?,
        HouseItemOutcome::Committed(_)
    ));
    let writer_root = runtime_root(h).await?;
    let writer_authority = writer_root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    owner.root = &writer_root;
    owner.character = &writer_authority;
    let mut request = Request {
        house: HOUSE.into(),
        item,
        action: 0,
        expected_revision: 0,
        display: Display::default(),
    };
    let (status, bytes) = exchange(&owner, &mut inside, 1, &request).await?;
    assert_ne!(status, CommandStatus::Accepted);
    assert!(bytes.is_empty());
    // The capability is deliberately unoffered. Inject the future negotiated selection only
    // at this test boundary to qualify framed House ingress and the real SQL writer.
    inside.continuity.selected_capabilities = SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: 86,
                requires: &[],
            },
            OfferedCapability {
                id: 133,
                requires: &[],
            },
        ],
        &[86, 133],
    )
    .ok_or("test selection")?;
    // The House snapshot independently binds the retained transfer selection. Seed
    // the same future selection in that historical fixture; the real transfer above
    // remains qualified only for capability 86. This does not prove negotiation of 133.
    let mut receipt = h
        .root
        .house_transfer_recovery(inside.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("house receipt")?;
    receipt.ack.selection = vec![86, 133];
    let acknowledgement =
        oteryn_protocol_oteryn::house_transfer::encode_house_transfer_ack(&receipt.ack)
            .map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query("UPDATE game_house_transfer_receipts SET selection=ARRAY[86,133],acknowledgement=$2 WHERE destination_game_session_id=encode($1,'hex')::uuid")
        .bind(inside.game_session_id.as_bytes().as_slice()).bind(acknowledgement)
        .execute(&mut *tx).await?.rows_affected();
    assert_eq!(changed, 1);
    tx.commit().await?;
    let (status, bytes) = exchange(&owner, &mut inside, 2, &request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_window(&bytes, 0, Display::default(), content.palette(), &[])?;
    request.action = 2;
    let (status, bytes) = exchange(&owner, &mut inside, 3, &request).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(&bytes[..2], &1634_u16.to_be_bytes());
    let look = content
        .look_type(
            &content
                .fallback(crate::domain::appearance::Sex::Male)
                .outfit_key,
            crate::domain::appearance::Sex::Male,
        )
        .ok_or("look")?;
    request.action = 1;
    request.display = Display {
        look_type: u16::try_from(look)?,
        colours: [1, 2, 3, 132],
        show_outfit: true,
        ..Display::default()
    };
    let (status, bytes) = exchange(&owner, &mut inside, 4, &request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        bytes,
        wire::encode_result(0, 1, request.display).map_err(debug)?
    );
    let stored = request.display;
    request.expected_revision = 1;
    request.display.look_type = u16::MAX;
    let (status, bytes) = exchange(&owner, &mut inside, 5, &request).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(&bytes[..2], &1635_u16.to_be_bytes());
    let mut unowned_mount = request.clone();
    unowned_mount.display = stored;
    let mount = content.mount_metadata(&mount_key).ok_or("mount fixture")?.0;
    unowned_mount.display.mount = u16::try_from(mount)?;
    unowned_mount.display.show_mount = true;
    let (status, bytes) = exchange(&owner, &mut inside, 6, &unowned_mount).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(&bytes[..2], &1635_u16.to_be_bytes());
    // A historical unlock with incompatible provenance must neither be offered nor
    // authorize configuration. Keep the same house, session, Item and mount identity.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'podium-fixture-incompatible-mount','content-1',encode($3,'hex')::uuid,'quest',$4)")
        .bind(id(192).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).bind(id(193).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'podium-fixture-incompatible-mount',encode($2,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(192).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let window_request = Request {
        action: 0,
        expected_revision: 0,
        display: Display::default(),
        ..request.clone()
    };
    let (status, bytes) = exchange(&owner, &mut inside, 7, &window_request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_window(&bytes, 1, stored, content.palette(), &[])?;
    let (status, bytes) = exchange(&owner, &mut inside, 8, &unowned_mount).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(&bytes[..2], &1635_u16.to_be_bytes());
    // Independently seeded historical compatible quest grant, consumed through the real
    // account appearance owner. This case does not qualify the quest reward writer itself.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'podium-fixture-mount','content-1',encode($3,'hex')::uuid,'quest',$4)")
        .bind(id(190).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).bind(id(191).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'podium-fixture-mount',encode($2,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(190).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut stale = inside;
    stale
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .character_lease_generation += 1;
    // A stale server-side fence cannot be supplied by a client. Exercise this invariant
    // at the composed handler boundary; all ordinary commands use framed House ingress.
    let (status, bytes) = direct_exchange(&owner, &stale, 41, &unowned_mount).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(&bytes[..2], &1631_u16.to_be_bytes());
    request.action = 2;
    request.display = Display::default();
    let (status, bytes) = exchange(&owner, &mut inside, 9, &request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let rotated = Display {
        direction: 3,
        ..stored
    };
    assert_eq!(bytes, wire::encode_result(0, 2, rotated).map_err(debug)?);
    let (status, replay) = exchange(&owner, &mut inside, 9, &request).await?;
    assert_eq!(status, CommandStatus::DuplicateReplay);
    assert_eq!(replay, bytes);
    // Also bypass the transport cache to retain qualification of the durable receipt:
    // a delivery retry after loss of the connection result cannot rotate twice.
    let (status, durable_replay) = direct_exchange(&owner, &inside, 9, &request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(durable_replay, bytes);
    // Complete the four-direction cycle, preserving every non-direction field.
    for (command, revision, direction) in [(10, 2, 0), (11, 3, 1), (12, 4, 2)] {
        request.expected_revision = revision;
        let (status, bytes) = exchange(&owner, &mut inside, command, &request).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            bytes,
            wire::encode_result(
                0,
                revision + 1,
                Display {
                    direction,
                    ..stored
                }
            )
            .map_err(debug)?
        );
    }
    // A second configuration proves mutable state and preservation of hidden selections.
    request.action = 1;
    request.expected_revision = 5;
    let changed = Display {
        mount: u16::try_from(mount)?,
        show_mount: true,
        mount_colours: [132, 1, 2, 3],
        colours: [132, 3, 2, 1],
        show_outfit: false,
        show_podium: false,
        ..stored
    };
    request.display = changed;
    let (status, bytes) = exchange(&owner, &mut inside, 13, &request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(bytes, wire::encode_result(0, 6, changed).map_err(debug)?);
    let (status, bytes) = exchange(&owner, &mut inside, 14, &request).await?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(&bytes[..2], &1637_u16.to_be_bytes());
    request.display = Display::default();
    request.action = 0;
    request.expected_revision = 0;
    // Reopen the native runtime role and its recovery capability. No podium cache survives
    // or is used by the fresh root; session, Item and house owners remain live.
    let reopened = runtime_root(h).await?;
    let recovered_authority = reopened
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    owner.root = &reopened;
    owner.character = &recovered_authority;
    // A fresh query reads durable state rather than a process-local podium cache.
    let (status, bytes) = exchange(&owner, &mut inside, 15, &request).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_window(
        &bytes,
        6,
        changed,
        content.palette(),
        &[u16::try_from(mount)?],
    )?;
    let audits:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_podium_records WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(audits, 6);
    if guest {
        // Revoke precisely the invitation fact after a successful guest mutation.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND character_id=encode($3,'hex')::uuid")
            .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (status, bytes) = exchange(&owner, &mut inside, 16, &request).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert_eq!(&bytes[..2], &1632_u16.to_be_bytes());
    }
    Ok(())
}

async fn runtime_root(h: &Harness) -> TestResult<crate::durability::DurabilityRoot> {
    let admin = crate::bestiary_postgres_harness::configured_admin().ok_or("admin route")?;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&h.pool)
        .await?;
    let prefix = admin.strip_suffix("/postgres").ok_or("admin route")?;
    let url = format!("{prefix}/{database}?options=-c%20role%3Doteryn_game_runtime");
    let reopened = crate::durability::DurabilityRoot::connect_test_runtime(&url)?;
    assert!(reopened.maintain_ready_once().await?);
    Ok(reopened)
}
