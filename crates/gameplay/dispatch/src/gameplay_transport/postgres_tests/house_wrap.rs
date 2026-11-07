//! HOUSE-ITEM-WIRE-1: framed commands, real composed House owner and runtime-role PG.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
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
    house_wrap as wire,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const CATALOGUE_OVEN: &str = "oteryn:item.tibia.i6355";
const POSITION: [i32; 3] = [32310, 32804, 7];

#[test]
fn house_wrap_dispatch_postgres_roundtrip_replay_and_refusal() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i6355",
        "oteryn:item.tibia.i23398",
        false,
        false,
        false,
    )
}

#[test]
fn srv7_house_construction_bookcase_dispatch_roundtrip_and_catalogue() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i6109",
        "oteryn:item.tibia.i23398",
        true,
        false,
        false,
    )
}

#[test]
fn srv7_house_present_kit_first_use_needs_no_prior_wrap_receipt() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i37547",
        "oteryn:item.tibia.i37562",
        false,
        true,
        false,
    )
}

#[test]
fn srv7_house_native_use_constructs_acquired_kit_and_replays() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i37547",
        "oteryn:item.tibia.i37562",
        false,
        true,
        true,
    )
}

#[test]
fn srv7_house_folded_rift_carpet_native_use_places_and_replays() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i22736",
        "oteryn:item.tibia.i22737",
        false,
        true,
        true,
    )
}

#[test]
fn srv7_house_modern_oven_wrap_uses_generic_decoration_kit_and_native_unwrap() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i6355",
        "oteryn:item.tibia.i23398",
        false,
        false,
        true,
    )
}
#[test]
fn srv7_house_modern_golden_display_wrap_uses_generic_decoration_kit_and_native_unwrap()
-> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i31510",
        "oteryn:item.tibia.i23398",
        false,
        false,
        true,
    )
}

fn on_large_stack(
    furniture: &'static str,
    kit: &'static str,
    catalogue: bool,
    first_use: bool,
    native_use: bool,
) -> TestResult {
    on_large_stack_mode(
        furniture,
        kit,
        catalogue,
        first_use,
        native_use,
        NativeWrapCase::None,
    )
}
#[derive(Clone, Copy)]
enum NativeWrapCase {
    Cask,
    Floor,
    Opaque,
    Carried,
    StackCarried,
    Look,
    Historical,
    None,
}
fn on_large_stack_mode(
    furniture: &'static str,
    kit: &'static str,
    catalogue: bool,
    first_use: bool,
    native_use: bool,
    wrap_case: NativeWrapCase,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async move |admin| {
                let started = std::time::Instant::now();
                let h =
                    Harness::create_with_admission_seed(admin, "house_wrap", true, false).await?;
                let result = scenario(
                    &h, furniture, kit, catalogue, first_use, native_use, wrap_case,
                )
                .await;

                h.cleanup().await?;
                result?;
                println!(
                    "PG_MARKER house-wrap roundtrip replay refusals elapsed={:?}",
                    started.elapsed()
                );
                Ok(())
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "house wrap thread panicked")??;
    Ok(())
}

#[test]
fn srx1_house_cask_charge_checkpoint_exhaustion_and_replay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h = Harness::create_with_admission_seed(admin, "srx1_house_cask", true, false)
                    .await?;
                let result = scenario(
                    &h,
                    CATALOGUE_OVEN,
                    "oteryn:item.tibia.i23398",
                    false,
                    false,
                    false,
                    NativeWrapCase::Cask,
                )
                .await;
                h.cleanup().await?;
                result?;
                eprintln!(
                    "SRX1_PG House cask checkpoint/exhaustion/replay elapsed={:?}",
                    started.elapsed()
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "House cask PG thread panicked")??;
    Ok(())
}

async fn scenario(
    h: &Harness,
    furniture: &str,
    kit: &str,
    catalogue: bool,
    first_use: bool,
    native_use: bool,
    wrap_case: NativeWrapCase,
) -> TestResult {
    let native_wrap = matches!(
        wrap_case,
        NativeWrapCase::Opaque | NativeWrapCase::Carried | NativeWrapCase::StackCarried
    );
    let carried_wrap = matches!(
        wrap_case,
        NativeWrapCase::Carried | NativeWrapCase::StackCarried
    );
    let original_quantity = if matches!(wrap_case, NativeWrapCase::StackCarried) {
        2_i64
    } else {
        1
    };
    let wrapped_look = matches!(wrap_case, NativeWrapCase::Look);
    let historical_wrap = matches!(wrap_case, NativeWrapCase::Historical);
    let cask = matches!(wrap_case, NativeWrapCase::Cask);
    let catalogue_query_case = furniture == CATALOGUE_OVEN
        && matches!(wrap_case, NativeWrapCase::None)
        && !first_use
        && !native_use;

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
    let world = wrap_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    let (controller, native_pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            base.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active content")?;
    let cells = world
        .movement_cells(native_pin.qualified_room().movement_cells())
        .map_err(debug)?;
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
    owner.movement_cells = &cells;
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    // This fixture boots the bounded command set it will carry through the real handoff.
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
            id: 11,
            requires: &[15, 18],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 15,
            requires: &[4],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 18,
            requires: &[4, 6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 28,
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
            id: 207,
            requires: &[49],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 350,
            requires: &[6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 387,
            requires: &[11, 86, 207, 350],
        },
    ];
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[11,18],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
    ).map_err(debug)?;

    deps.initialize(&owner).await;
    if native_use {
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    }

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
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        if native_wrap {
            &[4, 6, 11, 15, 18, 49, 86, 207, 350, 387]
        } else if wrapped_look {
            &[4, 6, 11, 15, 18, 28, 49, 86, 207]
        } else {
            &[4, 6, 11, 15, 18, 49, 86, 207]
        },
    )
    .ok_or("wrap offer")?;

    let request = wire::Request {
        action: wire::Action::Wrap,
        item: id(71),
        expected_revision: 1,
        acl_revision: 1,
        position: POSITION,
    };
    assert_eq!(
        exchange(&owner, &mut admitted, 1, request).await?,
        (
            CommandStatus::Rejected,
            wire::Reply::Refused(wire::Refusal::NotInHouse)
        )
    );
    let mut body = vec![8, 1, 18, HOUSE.len() as u8];
    body.extend(HOUSE.as_bytes());
    let (inside, _) = owner
        .transfer_house(admitted, 2, &body)
        .await
        .ok_or("house transfer outcome")?;
    let mut admitted = inside.ok_or("house transfer admission")?;

    // NX-1 refresh must use the exact named-House owner, including on ordinary players.
    let house_fence = admitted.item_fence.ok_or("House item fence")?;
    let status = owner
        .root
        .staff_live_status(owner.character, owner.holder, house_fence)
        .await
        .map_err(debug)?;
    assert_eq!(status, crate::durability::staff::StaffLiveStatus::default());
    let mut stale = house_fence;
    stale.connection_generation =
        crate::foundation::ConnectionGeneration::new(house_fence.connection_generation.get() + 1)
            .map_err(debug)?;
    assert!(
        owner
            .root
            .staff_live_status(owner.character, owner.holder, stale)
            .await
            .is_err()
    );

    if cask {
        return house_cask(&owner, h, admitted).await;
    }

    let actor_position = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("constructor House actor")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        [
            state.position.x,
            state.position.y,
            i32::from(state.position.floor),
        ]
    };
    // Seed a real placed oven and a backpack, then restore every constraint before commands.
    super::item_look::seed_main_backpack(h, &[]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$4,'definition-r1',$5,1,encode($3,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(73).as_slice()).bind(if first_use || historical_wrap {kit} else {furniture}).bind(original_quantity).execute(&mut *tx).await?;
    let position: Vec<u8> = POSITION.iter().flat_map(|v| v.to_be_bytes()).collect();
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(&position)
    .execute(&mut *tx)
    .await?;
    let actor_tile: Vec<u8> = actor_position
        .iter()
        .flat_map(|n| n.to_be_bytes())
        .collect();
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(actor_tile)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(74).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    if catalogue_query_case {
        // This donor wrap target has no XML name. The panel must still advertise
        // the real festive table using its current, source-qualified display name.
        let mut festive_position = POSITION;
        festive_position[0] += 1;
        let festive_position: Vec<u8> = festive_position
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect();
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i30231','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
        .bind(id(42).as_slice()).bind(HOUSE).bind(&festive_position).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&festive_position).bind(id(77).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(77).as_slice()).execute(&mut *tx).await?;
    }
    let old_house:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(location) FROM game_item_house_interior_locations location WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&mut *tx).await?;
    if first_use || carried_wrap || historical_wrap {
        sqlx::query("DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(71).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    }
    if historical_wrap {
        let historical_tx = id(75);
        sqlx::query("UPDATE game_item_container_entries SET placed_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).bind(historical_tx.as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_instances SET state_revision=2,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).bind(historical_tx.as_slice()).execute(&mut *tx).await?;
        let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i)||jsonb_build_object('definition_production_key',$2::text,'state_revision',1,'last_transaction_id',NULL) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).bind(furniture).fetch_one(&mut *tx).await?;
        let inventory:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(entry) FROM game_item_container_entries entry WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&mut *tx).await?;
        // Historical accepted legacy writer shape, not a production acquisition claim.
        // Every historical metadata field is retained; new API invocation is live/runtime-role.
        sqlx::query("INSERT INTO game_house_furniture_wrap_receipts(transaction_id,game_session_id,command_id,character_id,item_instance_id,world_id,house_key,intent,action,before_item,after_key,house_location,inventory_location) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,91,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,1,$8,$9,$10,$11)")
            .bind(historical_tx.as_slice()).bind(id(76).as_slice()).bind(id(41).as_slice()).bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(request.encode()).bind(before).bind(kit).bind(old_house).bind(inventory).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    if historical_wrap {
        let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(r) FROM game_house_furniture_wrap_receipts r WHERE transaction_id=encode($1,'hex')::uuid").bind(id(75).as_slice()).fetch_one(&h.pool).await?;
        let request = wire::Request {
            action: wire::Action::Unwrap,
            expected_revision: 2,
            ..request
        };
        assert_eq!(
            exchange_checked(&owner, &mut admitted, 1, request, true).await?,
            (CommandStatus::Accepted, wire::Reply::Committed(3)),
            "actual retained legacy wrapper must unwrap after new wrapping output changes"
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(r) FROM game_house_furniture_wrap_receipts r WHERE transaction_id=encode($1,'hex')::uuid").bind(id(75).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(after, before, "historical wrap metadata is immutable");
        assert_eq!(
            exchange_checked(&owner, &mut admitted, 1, request, false)
                .await?
                .1,
            wire::Reply::Committed(3)
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts")
                .fetch_one(&h.pool)
                .await?,
            2
        );
        return Ok(());
    }
    if first_use && native_use {
        // Entry is the closed entrance door. Walk into the real free interior
        // cell before constructing at the actor's current tile.
        let (step, _) = owner
            .house_paced_step(
                admitted.runtime_actor.ok_or("constructor actor")?,
                admitted.game_session_id,
                crate::gameplay_transport::world_spatial::StepDirection::East,
            )
            .await;
        assert_eq!(
            step.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved,
            "native House STEP from entrance to construction cell"
        );
        let state = owner
            .runtime
            .lock()
            .await
            .house_actor_state(
                admitted.runtime_actor.ok_or("constructor actor")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        assert_eq!(
            [
                state.position.x,
                state.position.y,
                i32::from(state.position.floor)
            ],
            POSITION
        );
    }
    admitted.continuity.next_command_id = 1;
    if matches!(wrap_case, NativeWrapCase::Floor) {
        let reference = active
            .native_gameplay()
            .and_then(|n| n.item_index())
            .and_then(|i| i.definition_ref("Item", kit, "definition-r1"))
            .ok_or("floor kit reference")?;
        let (status, payload, _, _) =
            opaque_peer::command(&owner, &mut admitted, 1, 9, vec![], Some(reference), false)
                .await?;
        assert_eq!(
            status,
            CommandStatus::Accepted,
            "actual MOVE9 places the independently source-qualified constructor"
        );
        assert_eq!(
            oteryn_protocol_oteryn::item_view::decode_item_move_result_for(
                &payload,
                oteryn_protocol_oteryn::item_view::ItemMoveSelection {
                    world_interaction: true,
                    ..Default::default()
                }
            )
            .map_err(debug)?,
            oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
        );
        assert_custody(h, kit, true, catalogue_query_case).await?;
        let (status, payload, _, _) =
            opaque_peer::command(&owner, &mut admitted, 2, 2, vec![], Some(reference), false)
                .await?;
        assert_eq!(
            status,
            CommandStatus::Accepted,
            "source House-floor constructor USE must be answered"
        );
        let result = oteryn_protocol_oteryn::world_object::decode_use_result_with_gate(&payload)
            .map_err(debug)?;
        assert_eq!(
            result.0,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        return Ok(());
    }
    if first_use {
        if native_use {
            let native = active.native_gameplay().ok_or("native")?;
            let source = native
                .item_index()
                .and_then(|index| index.definition_ref("Item", kit, "definition-r1"))
                .ok_or("kit ref")?;
            let initial = oteryn_protocol_oteryn::world_object::encode_item_use_intent(
                &oteryn_protocol_oteryn::world_object::UseIntent {
                    target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
                        std::num::NonZeroU64::new(1).ok_or("dummy handle")?,
                    ),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                },
            )
            .map_err(debug)?;
            let (mut current, disposition, payload) =
                super::rust_lottery::exchange(&owner, admitted, 1, &initial, Some(source), None)
                    .await?;
            assert_eq!(
                disposition,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            assert_custody(h, furniture, true, catalogue_query_case).await?;
            drop(owner);
            let mut owner = deps.owner(h, &authority);
            owner.movement_cells = &cells;
            owner.evidence = &evidence.evidence;
            owner.active_generation = Some(active);
            current.continuity.next_command_id = 1;
            let (_, disposition, _) =
                super::rust_lottery::exchange(&owner, current, 1, &payload, None, None).await?;
            assert_eq!(
                disposition,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            assert_custody(h, furniture, true, catalogue_query_case).await?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts WHERE use_intent IS NOT NULL").fetch_one(&h.pool).await?,1);
            return Ok(());
        }
        let request = wire::Request {
            action: wire::Action::Unwrap,
            ..request
        };
        assert_eq!(
            exchange_checked(&owner, &mut admitted, 1, request, true).await?,
            (CommandStatus::Accepted, wire::Reply::Committed(2))
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        drop(owner);
        let mut owner = deps.owner(h, &authority);
        owner.movement_cells = &cells;
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(active);
        admitted.continuity.next_command_id = 1;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, request).await?.1,
            wire::Reply::Committed(2)
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        return Ok(());
    }
    if native_wrap {
        let reference = active
            .native_gameplay()
            .ok_or("opaque native state")?
            .item_index()
            .and_then(|i| i.definition_ref("Item", furniture, "definition-r1"))
            .ok_or("opaque source definition")?;
        let (status, payload, frames, client_frame) = opaque_peer::command(
            &owner,
            &mut admitted,
            1,
            402,
            vec![],
            Some(reference),
            carried_wrap,
        )
        .await?;
        assert_eq!(
            (status, wire::Reply::decode(&payload)),
            (CommandStatus::Accepted, Some(wire::Reply::Committed(2)))
        );
        let original:Vec<u8>=sqlx::query_scalar("SELECT use_intent FROM game_house_furniture_wrap_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND action=1").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert!(
            wire::OpaqueWrapRequest::decode(&original).is_some()
                || wire::OpaqueWrapRequest::decode_carried(&original).is_some()
        );
        let (status, payload, _, _) =
            opaque_peer::command(&owner, &mut admitted, 1, 402, original, None, false).await?;
        assert!(matches!(
            status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ));
        assert_eq!(
            wire::Reply::decode(&payload),
            Some(wire::Reply::Committed(2))
        );
        assert!(!frames.is_empty());
        if let Some(path) = std::env::var_os("OTERYN_SRV7_OPAQUE_WRAP_CAPTURE") {
            let viewer = owner
                .runtime
                .lock()
                .await
                .house_actor_state(
                    admitted.runtime_actor.ok_or("wrap viewer")?,
                    admitted.game_session_id,
                )
                .map_err(debug)?;
            let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            let capture = serde_json::json!({"producer_test":"srv7_house_native_opaque_wrap_and_use_preserve_source_and_retired_handle_replay","source":{"crystal_sha":"201189f871d23597b82caf88fd611454a2ddefb8","path":"src/game/game.cpp","lines":[4899,4901]},"selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),"viewer":{"house":viewer.house,"position":[viewer.position.x,viewer.position.y,viewer.position.floor],"scope_generation":viewer.generation.get(),"lease_generation":viewer.lease.generation()},"client_command_envelope_hex":hex(&client_frame),"server_frames_hex":frames.iter().map(|f|hex(f)).collect::<Vec<_>>()});
            let mut path = std::path::PathBuf::from(path);
            path.set_extension(format!("{furniture}.{original_quantity}.json"));
            std::fs::write(path, serde_json::to_vec_pretty(&capture)?)?;
        }
    } else {
        assert_eq!(
            exchange_checked(&owner, &mut admitted, 1, request, true).await?,
            (CommandStatus::Accepted, wire::Reply::Committed(2))
        );
    }
    assert_custody(h, kit, false, catalogue_query_case).await?;
    if catalogue_query_case {
        let query_command = admitted.continuity.next_command_id;
        let (status, catalogue) = furniture_query(&owner, &mut admitted, query_command, 0).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            catalogue.first(),
            Some(&7),
            "native furniture catalogue must be served by the real House owner"
        );
        let catalogue = wire::Catalogue::decode(&catalogue).ok_or("native catalogue codec")?;
        assert_eq!(catalogue.acl_revision, 1);
        assert_eq!(catalogue.entries.len(), 2);
        assert!(catalogue.entries.iter().any(|entry| entry.item == id(75)
            && entry.action == wire::Action::Wrap
            && entry.name == "festive table"));
        assert_eq!(
            (
                catalogue.entries[0].item,
                catalogue.entries[0].revision,
                catalogue.entries[0].action
            ),
            (id(71), 2, wire::Action::Unwrap)
        );
        assert!(catalogue.entries[0].name.contains("oven"));
    }

    if native_wrap {
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?,1,"generic23398 is one nonstackable wrapper");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT (before_item->>'quantity')::bigint FROM game_house_furniture_wrap_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND action=1").bind(id(71).as_slice()).fetch_one(&h.pool).await?,original_quantity,"source C++AMOUNT is sealed by original receipt, not lost orrefilled");
    }
    if wrapped_look {
        let reference = active
            .native_gameplay()
            .ok_or("wrappedLook native")?
            .item_index()
            .and_then(|i| i.definition_ref("Item", kit, "definition-r1"))
            .ok_or("wrappedLook definition")?;
        let (status, payload, _, _) =
            opaque_peer::command(&owner, &mut admitted, 2, 30, vec![], Some(reference), false)
                .await?;
        assert_eq!(
            status,
            CommandStatus::Accepted,
            "actual House Look30 must retain its existing native text owner"
        );
        let result = oteryn_protocol_oteryn::item_look::decode_item_look_result_for(
            &payload,
            oteryn_protocol_oteryn::item_look::ItemLookSelection { extended: true },
        )
        .map_err(debug)?;
        let oteryn_protocol_oteryn::item_look::ItemLookResult::Text(text) = result else {
            return Err("wrapped Look text".into());
        };
        assert!(
            text.contains("Unwrap it in your own house to create a <oven>."),
            "actual wrapped source description: {text}"
        );
        assert_custody(h, kit, false, catalogue_query_case).await?;
        return Ok(());
    }
    if carried_wrap {
        // playerWrapableItem4855–4860 rejects context-Unwrap while carried.
        // This new carried-Wrap source proof therefore places the real wrapper
        // through native9 before native2; existing accepted inventory-first
        // legacy tests remain separate and unchanged.
        let reference = active
            .native_gameplay()
            .and_then(|n| n.item_index())
            .and_then(|i| i.definition_ref("Item", kit, "definition-r1"))
            .ok_or("carried wrapper reference")?;
        let place_command = admitted.continuity.next_command_id;
        let (status, payload, _, _) = opaque_peer::command(
            &owner,
            &mut admitted,
            place_command,
            9,
            vec![],
            Some(reference),
            false,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            oteryn_protocol_oteryn::item_view::decode_item_move_result_for(
                &payload,
                oteryn_protocol_oteryn::item_view::ItemMoveSelection {
                    world_interaction: true,
                    ..Default::default()
                }
            )
            .map_err(debug)?,
            oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
        );
        assert_custody(h, kit, true, catalogue_query_case).await?;
        let use_command = admitted.continuity.next_command_id;
        let (status, payload, _, _) = opaque_peer::command(
            &owner,
            &mut admitted,
            use_command,
            2,
            vec![],
            Some(reference),
            false,
        )
        .await?;
        assert_eq!(
            status,
            CommandStatus::Accepted,
            "placed generic wrapper USE is a separate source caller"
        );
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result_with_gate(&payload)
                .map_err(debug)?
                .0,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?,original_quantity);
        let original:Vec<u8>=sqlx::query_scalar("SELECT use_intent FROM game_house_furniture_wrap_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND action=2 AND command_id=$2::text::numeric").bind(id(71).as_slice()).bind(use_command.to_string()).fetch_one(&h.pool).await?;
        let (status, payload, _, _) =
            opaque_peer::command(&owner, &mut admitted, use_command, 2, original, None, false)
                .await?;
        assert!(matches!(
            status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ));
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result_with_gate(&payload)
                .map_err(debug)?
                .0,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        if original_quantity > 1 {
            // The first real Wrap4 -> MOVE9 -> USE2 restored AMOUNT onto the
            // House floor. Re-wrap that observed stack through World marker3;
            // no test changes its position, quantity, custody or revision.
            let floor_reference = active
                .native_gameplay()
                .and_then(|native| native.item_index())
                .and_then(|index| index.definition_ref("Item", furniture, "definition-r1"))
                .ok_or("restored floor stack reference")?;
            let revision: i64 = sqlx::query_scalar("SELECT state_revision::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            let floor_wrap_command = admitted.continuity.next_command_id;
            let (status, reply, _, original_floor_wrap) = opaque_peer::command(
                &owner,
                &mut admitted,
                floor_wrap_command,
                402,
                vec![],
                Some(floor_reference),
                false,
            )
            .await?;
            assert_eq!(
                status,
                CommandStatus::Accepted,
                "a restored source stack must remain World-Wrapable"
            );
            assert_eq!(
                wire::Reply::decode(&reply),
                Some(wire::Reply::Committed(u64::try_from(revision + 1)?))
            );
            assert_custody(h, kit, false, catalogue_query_case).await?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?, 1);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT (before_item->>'quantity')::bigint FROM game_house_furniture_wrap_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND action=1")
                .bind(admitted.game_session_id.as_bytes().as_slice()).bind(floor_wrap_command.to_string()).fetch_one(&h.pool).await?, original_quantity);
            let before_replay: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            let (status, replay, _, _) = opaque_peer::command(
                &owner,
                &mut admitted,
                floor_wrap_command,
                402,
                original_floor_wrap,
                None,
                false,
            )
            .await?;
            assert!(matches!(
                status,
                CommandStatus::Accepted | CommandStatus::DuplicateReplay
            ));
            assert_eq!(wire::Reply::decode(&replay), wire::Reply::decode(&reply));
            assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?, before_replay);
            let wrapper_reference = active
                .native_gameplay()
                .and_then(|native| native.item_index())
                .and_then(|index| index.definition_ref("Item", kit, "definition-r1"))
                .ok_or("re-wrapped source reference")?;
            let place_command = admitted.continuity.next_command_id;
            let (status, payload, _, _) = opaque_peer::command(
                &owner,
                &mut admitted,
                place_command,
                9,
                vec![],
                Some(wrapper_reference),
                false,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                oteryn_protocol_oteryn::item_view::decode_item_move_result_for(
                    &payload,
                    oteryn_protocol_oteryn::item_view::ItemMoveSelection {
                        world_interaction: true,
                        ..Default::default()
                    },
                )
                .map_err(debug)?,
                oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
            );
            let second_use = admitted.continuity.next_command_id;
            let (status, payload, _, _) = opaque_peer::command(
                &owner,
                &mut admitted,
                second_use,
                2,
                vec![],
                Some(wrapper_reference),
                false,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result_with_gate(&payload)
                    .map_err(debug)?
                    .0,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            assert_custody(h, furniture, true, catalogue_query_case).await?;
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?, original_quantity);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND action=1")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?, 2);
        }
        return Ok(());
    }
    if native_use {
        // The wrapped source has genuinely left its old House cell. The
        // existing native movement owner now admits that free interior tile.
        let (step, _) = owner
            .house_paced_step(
                admitted.runtime_actor.ok_or("dedicated wrapper actor")?,
                admitted.game_session_id,
                crate::gameplay_transport::world_spatial::StepDirection::East,
            )
            .await;
        assert_eq!(
            step.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved
        );
        let native = active.native_gameplay().ok_or("dedicated wrapper native")?;
        let source = native
            .item_index()
            .and_then(|index| index.definition_ref("Item", kit, "definition-r1"))
            .ok_or("dedicated wrapper Item reference")?;
        let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(
            &oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
                    std::num::NonZeroU64::MIN,
                ),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
        )
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let (current, outcome, raw) =
            super::rust_lottery::exchange(&owner, admitted, command, &payload, Some(source), None)
                .await?;
        assert_eq!(
            outcome,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
            "real native USE must reopen its exact generic-wrapper receipt"
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        if native_wrap {
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?,original_quantity,"native unwrap restores exact source amount");
        }
        let required_capabilities = std::sync::Arc::clone(&owner.required_capabilities);
        drop(owner);
        let mut owner = deps.owner(h, &authority);
        owner.movement_cells = &cells;
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(active);
        owner.required_capabilities = required_capabilities;
        // An empty command directory starts at1. Reconstruct its actual immutable
        // Wrap1 prefix before replaying USE2, rather than claiming a missing
        // directory already has high-water3 (prepare_quest_callback_resume).
        let mut recovered = current;
        recovered.continuity.next_command_id = 1;
        if native_wrap {
            let original:Vec<u8>=sqlx::query_scalar("SELECT use_intent FROM game_house_furniture_wrap_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=1").bind(recovered.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            let (status, reply, _, _) =
                opaque_peer::command(&owner, &mut recovered, 1, 402, original, None, false).await?;
            assert!(matches!(
                status,
                CommandStatus::Accepted | CommandStatus::DuplicateReplay
            ));
            assert_eq!(wire::Reply::decode(&reply), Some(wire::Reply::Committed(2)));
        } else {
            assert_eq!(
                exchange_checked(&owner, &mut recovered, 1, request, false)
                    .await?
                    .1,
                wire::Reply::Committed(2)
            );
        }
        assert_eq!(
            recovered.continuity.next_command_id, command,
            "actual prefix restores the next request; no cursor is invented"
        );
        let (_, outcome, _) =
            super::rust_lottery::exchange(&owner, recovered, command, &raw, None, None).await?;

        assert_eq!(
            outcome,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        assert_custody(h, furniture, true, catalogue_query_case).await?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts")
                .fetch_one(&h.pool)
                .await?,
            2
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts WHERE use_intent IS NOT NULL").fetch_one(&h.pool).await?,if native_wrap {2}else{1});
        return Ok(());
    }

    // New composed owner / command directory forces durable replay, bypassing cached ingress.
    drop(owner);
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    admitted.continuity.next_command_id = 1;
    assert_eq!(
        exchange(&owner, &mut admitted, 1, request).await?.1,
        wire::Reply::Committed(2)
    );
    assert_custody(h, kit, false, catalogue_query_case).await?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange(
            &owner,
            &mut admitted,
            command,
            wire::Request {
                item: id(70),
                ..request
            }
        )
        .await?
        .1,
        wire::Reply::Refused(wire::Refusal::NotWrapable)
    );
    let access_body: String = sqlx::query_scalar(
        "SELECT pg_get_functiondef('game_house_access(uuid,text,uuid)'::regprocedure)",
    )
    .fetch_one(&h.pool)
    .await?;
    sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC,guild_revisions JSONB) LANGUAGE sql AS $$ SELECT 'GUEST'::text,false,1::numeric,NULL::jsonb $$").execute(&h.pool).await?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange(
            &owner,
            &mut admitted,
            command,
            wire::Request {
                expected_revision: 2,
                action: wire::Action::Unwrap,
                ..request
            }
        )
        .await?
        .1,
        wire::Reply::Refused(wire::Refusal::NoAccess)
    );
    // Restores only the function definition read from this isolated test database.
    sqlx::raw_sql(sqlx::AssertSqlSafe(access_body))
        .execute(&h.pool)
        .await?;
    let unwrap = wire::Request {
        action: wire::Action::Unwrap,
        expected_revision: 2,
        ..request
    };
    for (bad, refusal) in [
        (
            wire::Request {
                acl_revision: 2,
                ..unwrap
            },
            wire::Refusal::Stale,
        ),
        (
            wire::Request {
                position: [0, 0, 7],
                ..unwrap
            },
            wire::Refusal::Blocked,
        ),
    ] {
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut admitted, command, bad).await?.1,
            wire::Reply::Refused(refusal)
        );
    }
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange_checked(&owner, &mut admitted, command, unwrap, true)
            .await?
            .1,
        wire::Reply::Committed(3)
    );
    assert_custody(h, furniture, true, catalogue_query_case).await?;
    if catalogue {
        // Every donor pair is checked by the actual SQL admission function; a
        // Rust-only catalogue is insufficient evidence that a kit can be used.
        for &(kit, output) in crate::gameplay_transport::furniture_catalog::CONSTRUCTION_KITS {
            let allowed: bool = sqlx::query_scalar(
                "SELECT game_house_furniture_wrap_pair(encode($1,'hex')::uuid,$2,$3,2::smallint)",
            )
            .bind(id(71).as_slice())
            .bind(format!("oteryn:item.tibia.i{kit}"))
            .bind(format!("oteryn:item.tibia.i{output}"))
            .fetch_one(&h.pool)
            .await?;
            assert!(allowed, "construction kit {kit} -> {output}");
        }
        for &(begin, end, kit) in crate::gameplay_transport::furniture_catalog::WRAP_TRANSFORMS {
            for furniture in begin..=end {
                let allowed: bool = sqlx::query_scalar("SELECT game_house_furniture_wrap_pair(encode($1,'hex')::uuid,$2,$3,1::smallint)")
                    .bind(id(71).as_slice())
                    .bind(format!("oteryn:item.tibia.i{furniture}"))
                    .bind(format!("oteryn:item.tibia.i{kit}"))
                    .fetch_one(&h.pool).await?;
                assert!(allowed, "wrap {furniture} -> {kit}");
            }
        }
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts")
            .fetch_one(&h.pool)
            .await?,
        2
    );
    Ok(())
}
async fn assert_custody(h: &Harness, key: &str, house: bool, catalogue_source: bool) -> TestResult {
    let (stored,placed,carried):(String,bool,bool)=sqlx::query_as("SELECT definition_production_key,EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id),EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((stored.as_str(), placed, carried), (key, house, !house));
    let mint: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(minted_transaction_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(mint, id(73));

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?,
        2 + i64::from(catalogue_source)
    );
    Ok(())
}
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    request: wire::Request,
) -> TestResult<(CommandStatus, wire::Reply)> {
    exchange_checked(owner, admitted, command, request, false).await
}
async fn exchange_checked(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    request: wire::Request,
    expect_delta: bool,
) -> TestResult<(CommandStatus, wire::Reply)> {
    let body = request.encode();
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type: wire::COMMAND_WRAP,
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
    let mut inventory_delta = false;
    while cursor < bytes.len() {
        let n = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&bytes[cursor + 4..cursor + 4 + n]).map_err(debug)?;
        cursor += 4 + n;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            reply = Some((
                result.status,
                wire::Reply::decode(result.payload).ok_or("wrap result")?,
            ));
        } else if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            inventory_delta |= delta.domain_id
                == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY;
        }
    }
    if expect_delta {
        assert!(
            inventory_delta,
            "committed House conversion must publish inventory delta"
        );
    }
    reply.ok_or_else(|| format!("wrap command {command}: {end:?}").into())
}

pub(super) fn wrap_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    for tile in &mut sector.tiles {
        if tile.x == 7 {
            tile.house = 3437;
            tile.items.truncate(1);
        }
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    entrance.x = 32309;
    entrance.y = 32805;
    sector.tiles.push(entrance);
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        fixture::door_items,
    )
    .map_err(debug)
}

async fn furniture_query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    page: u16,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let body = [3, page.to_be_bytes()[0], page.to_be_bytes()[1]];
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type: wire::COMMAND_WRAP,
            payload: &body,
        },
    )?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = end
    {
        *admitted = current;
    }
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut at = 0;
    while at < bytes.len() {
        let count = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&bytes[at + 4..at + 4 + count])?;
        at += 4 + count;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            return Ok((result.status, result.payload.to_vec()));
        }
    }
    Err("missing furniture catalogue result".into())
}

async fn house_cask(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget,
    };
    // Historical fixture with real named-House placement/provenance and a partial counter.
    super::item_look::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i283", 2),
            ("oteryn:item.tibia.i283", 2),
            ("oteryn:item.tibia.i25904", 1),
        ],
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i25880','definition-r1',1,1,game_character_uuid_v7())")
        .bind(id(80).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
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
        .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(81).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(81).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,3,1)")
        .bind(id(80).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_definitions VALUES('Item','oteryn:item.tibia.i25904','definition-r1',500,NULL,false) ON CONFLICT DO NOTHING").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,498,1)").bind(id(73).as_slice()).execute(&mut *tx).await?;
    let mut keg_position = POSITION;
    keg_position[0] += 1;
    let keg_position: Vec<u8> = keg_position
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect();
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(&keg_position)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    house_keg_custody(owner, h, admitted, keg_position).await?;
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("flask handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            std::num::NonZeroU64::new(2).ok_or("cask handle")?,
        )),
        use_with_position: None,
    };
    let command = |id| crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: id,
        item_fence: admitted.item_fence,
    };
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let first = owner
        .use_dispenser_action(
            actor,
            command(1),
            intent.clone(),
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(id(80))),
        )
        .await
        .map_err(|_| "House first use unknown")?
        .ok_or("House dispenser production route")?;
    assert_eq!(first.disposition, UseDisposition::Committed);
    assert_eq!(sqlx::query_scalar::<_,i32>("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?,1);
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let last = owner
        .use_dispenser_action(
            actor,
            command(2),
            intent.clone(),
            Some(ItemKey::Instance(id(72))),
            Some(ItemKey::Instance(id(80))),
        )
        .await
        .map_err(|_| "House last use unknown")?
        .ok_or("House exhaustion route")?;
    assert_eq!(last.disposition, UseDisposition::Committed);
    let item:(i64,i16)=sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(item, (0, 2));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid)+(SELECT count(*) FROM game_item_house_reclaim_provenance WHERE item_instance_id=encode($1,'hex')::uuid)").bind(id(80).as_slice()).fetch_one(&h.pool).await?,0);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i236' AND lifecycle=1").fetch_one(&h.pool).await?,3);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid"
        )
        .bind(id(72).as_slice())
        .fetch_one(&h.pool)
        .await?,
        1
    );
    for number in [1, 2] {
        let replay = owner
            .use_dispenser_action(actor, command(number), intent.clone(), None, None)
            .await
            .map_err(|_| "House replay unknown")?
            .ok_or("House replay receipt")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill'").fetch_one(&h.pool).await?,2);
    Ok(())
}

async fn house_keg_custody(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    position: Vec<u8>,
) -> TestResult {
    use crate::durability::house_interior::{
        HouseBackpackFacts, HouseCharacterSource, HouseItemAction, HouseItemOutcome,
        HouseItemRequest,
    };
    let fence = admitted.item_fence.ok_or("House inventory fence")?;
    let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    let ordinal:String=sqlx::query_scalar("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    let mut request = HouseItemRequest {
        command: crate::foundation::CommandRef::new(
            fence.game_session_id,
            crate::foundation::CommandId::new(900).map_err(debug)?,
        ),
        house: HouseId::new(admitted.world_id, HOUSE).ok_or("House")?,
        item_instance_id: id(73),
        action: HouseItemAction::Place {
            source: HouseCharacterSource::Inventory {
                parent: id(70),
                ordinal: ordinal.parse()?,
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
    let place = owner
        .root
        .freeze_house_item(owner.character, owner.holder, fence, request.clone())
        .await
        .map_err(debug)?;
    let receipt = match owner
        .root
        .commit_house_item(owner.character, owner.holder, fence, &place)
        .await
        .map_err(debug)?
    {
        HouseItemOutcome::Committed(receipt) => receipt,
        other => return Err(format!("fresh House placement: {other:?}").into()),
    };
    assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?,before);
    assert!(matches!(
        owner
            .root
            .commit_house_item(owner.character, owner.holder, fence, &place)
            .await
            .map_err(debug)?,
        HouseItemOutcome::AlreadyCommitted(_)
    ));
    request.command = crate::foundation::CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(901).map_err(debug)?,
    );
    request.action = HouseItemAction::Take {
        placement: receipt.transaction_id,
        provenance_revision: 1,
    };
    request.backpack = Some(HouseBackpackFacts {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        capacity: 20,
    });
    let take = owner
        .root
        .freeze_house_item(owner.character, owner.holder, fence, request)
        .await
        .map_err(debug)?;
    assert!(matches!(
        owner
            .root
            .commit_house_item(owner.character, owner.holder, fence, &take)
            .await
            .map_err(debug)?,
        HouseItemOutcome::Committed(_)
    ));
    assert!(matches!(
        owner
            .root
            .commit_house_item(owner.character, owner.holder, fence, &take)
            .await
            .map_err(debug)?,
        HouseItemOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?,before);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?,0);
    eprintln!("SRX1_PG House keg placement/take/replay preserves 498 charges");
    Ok(())
}

#[test]
fn srv7_house_historical_oven_6371_receipt_unwraps_after_generic_wrap_migration() -> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i6355",
        "oteryn:item.tibia.i6371",
        false,
        false,
        false,
        NativeWrapCase::Historical,
    )
}
#[test]
fn srv7_house_historical_golden_26054_receipt_unwraps_after_generic_wrap_migration() -> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i31510",
        "oteryn:item.tibia.i26054",
        false,
        false,
        false,
        NativeWrapCase::Historical,
    )
}
#[test]
fn srv7_house_native_bookcase_6115_construction_is_independent_of_wrap_output() -> TestResult {
    on_large_stack(
        "oteryn:item.tibia.i6109",
        "oteryn:item.tibia.i6115",
        false,
        true,
        true,
    )
}

#[path = "house_wrap/opaque_wrap_peer.rs"]
mod opaque_peer;
#[test]
fn srv7_house_native_opaque_wrap_and_use_preserve_source_and_retired_handle_replay() -> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i6355",
        "oteryn:item.tibia.i23398",
        false,
        false,
        true,
        NativeWrapCase::Opaque,
    )
}

#[test]
fn srv7_house_wrapped_oven_native_look_retains_source_description() -> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i6355",
        "oteryn:item.tibia.i23398",
        false,
        false,
        false,
        NativeWrapCase::Look,
    )
}

#[test]
fn srv7_house_native_carried_easel_wrap_and_unwrap_preserve_source_identity() -> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i14687",
        "oteryn:item.tibia.i23398",
        false,
        false,
        true,
        NativeWrapCase::Carried,
    )
}

#[test]
fn srv7_house_native_carried_carpet_stack_wrap_preserves_hidden_amount_and_restores_two()
-> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i36951",
        "oteryn:item.tibia.i23398",
        false,
        false,
        true,
        NativeWrapCase::StackCarried,
    )
}

#[test]
fn srv7_house_native_floor_bookcase_6115_move_then_use_constructs_in_place() -> TestResult {
    on_large_stack_mode(
        "oteryn:item.tibia.i6109",
        "oteryn:item.tibia.i6115",
        false,
        true,
        true,
        NativeWrapCase::Floor,
    )
}

pub(super) async fn capture_peer(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: u64,
    kind: u32,
    body: Vec<u8>,
    observed: Option<std::num::NonZeroU32>,
    carried: bool,
) -> TestResult<(
    oteryn_protocol_oteryn::CommandStatus,
    Vec<u8>,
    Vec<Vec<u8>>,
    Vec<u8>,
)> {
    opaque_peer::command(owner, admitted, command, kind, body, observed, carried).await
}
