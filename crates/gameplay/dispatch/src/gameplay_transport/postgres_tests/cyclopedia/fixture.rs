use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;
async fn exchange_with_delta(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
    require_delta: bool,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1024);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                break (r.status, r.payload.to_vec());
            }
        };
        if require_delta {
            loop {
                let frame = read_frame(&mut client).await?;
                let env = decode_wire_envelope(&frame).map_err(debug)?;
                if env.message_type() == MessageType::StateDelta {
                    let delta =
                        oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                    assert_eq!(
                        delta.domain_id, 9,
                        "the bank-fee coin change refreshes the inventory"
                    );
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        harness_fixture::join_two(serving, driving),
    )
    .await?;
    match &ended {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) => {
            *admitted = *current
        }
        _ => {}
    }
    result
        .map_err(|error| format!("cyclopedia command {id}/{kind} ended {ended:?}: {error}").into())
}

pub(super) fn world(
    deps: &Dependencies,
    generation: &crate::content::ActiveGeneration,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{
        bundle,
        sector::{Attrs, Item, Tile},
        writer::Sector,
    };
    let index = generation
        .native_gameplay()
        .and_then(|n| n.item_index())
        .ok_or("item index")?;
    let backpack = index
        .definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
        .ok_or("backpack definition")?;
    let gold = index
        .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
        .ok_or("coin definition")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        ("oteryn:item.tibia.i2854", backpack.get() - 1),
        ("oteryn:item.tibia.i3031", gold.get() - 1),
    );
    let mut parsed = bundle::read(&bytes)?;
    let mut tiles = Vec::new();
    for y in 0..2 {
        for x in 0..32 {
            tiles.push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            });
        }
    }
    parsed.sectors = vec![Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles,
    }];
    let bytes =
        oteryn_world_bundle::writer::write(&parsed.manifest, &parsed.sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: true,
                door: false,
            })
        },
    )
    .map_err(debug)
}
pub(in super::super) async fn seed(h: &Harness, bundle: [u8; 32]) -> TestResult {
    // Explicit fixture seed, never a product writer. Runtime assertions below use the
    // normal restricted runtime role, including fee audit and revision constraints.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'fixture-cyclopedia',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),'2026-10')")
        .bind(id(42).as_slice()).bind(bundle.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(id(42).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&mut *tx)
        .await?;
    let uuid = |n| crate::durability::item_fee_burn::test_support::uuid(n);
    let seed = format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({}, {}, {},1,sha256('seed'),{},{},{},{},1,1000,0,1,sha256('seed'),0,{},{},{}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({},{},{},{},1,1000,0,1000,{}); INSERT INTO game_account_bank_balances VALUES({},{},1000,{});",
        uuid(80),
        uuid(81),
        uuid(82),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(43),
        uuid(83),
        uuid(84),
        uuid(70),
        uuid(85),
        uuid(81),
        uuid(40),
        uuid(42),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(85)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    let seed = format!(
        "{}{} INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES({},{},{},{}); INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES({},{},{},{},1,{});",
        crate::durability::item_fee_burn::test_support::item(
            70,
            "oteryn:item.tibia.i2854",
            "definition-r1",
            1
        ),
        crate::durability::item_fee_burn::test_support::item(
            72,
            "oteryn:item.tibia.i3031",
            "definition-r1",
            5
        ),
        uuid(41),
        uuid(70),
        uuid(42),
        uuid(74),
        uuid(41),
        uuid(70),
        uuid(72),
        uuid(42),
        uuid(75)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub(in super::super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    exchange_with_delta(owner, admitted, id, kind, payload, false).await
}
pub(super) async fn exchange_inventory(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    exchange_with_delta(owner, admitted, id, kind, payload, true).await
}
