use super::*;
use oteryn_protocol_oteryn::{
    self as protocol, CommandStatus, MessageType, container_batch as wire, item_view::ItemHandle,
};
use std::num::NonZeroU64;
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn stash_container_postgres_dispatch_requires_preview_and_stows_nested_stacks_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Mode::StashContainer, None)
}

#[test]
fn managed_container_move_postgres_dispatch_confirms_recursive_plan_and_conserves_units()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(
        admin,
        Mode::ManagedContainerMove,
        Some(EntitlementState::Active),
    )
}

#[test]
fn managed_container_move_postgres_prefers_armor_over_equipment() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    run_case(admin, Mode::ManagedArmor, Some(EntitlementState::Active))
}

fn run_case(_admin: String, mode: Mode, premium: Option<EntitlementState>) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario_with_premium(true, false, false, false, false, premium, mode)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "container batch dispatch thread")?
        .map_err(Into::into)
}

pub(super) async fn nest_fixture(pool: &sqlx::PgPool, bag: [u8; 16], item: [u8; 16]) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "WITH old AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid             RETURNING item_instance_id,world_id,placed_transaction_id)          INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)          SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,placed_transaction_id FROM old",
    ).bind(item.as_slice()).bind(bag.as_slice()).execute(&mut *tx).await?;
    assert_eq!(inserted.rows_affected(), 1, "the fixture must nest a live stack");
    tx.commit().await?;
    Ok(())
}

pub(super) async fn command(
    client: &mut DuplexStream,
    id: u64,
    command_type: u32,
    request: wire::Request,
) -> TestResult<(CommandStatus, Option<wire::Reply>)> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type,
            payload: &request.encode(),
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = super::super::item_look_fixtures::read_frame(client).await?;
        let wire = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if wire.message_type() == MessageType::CommandResult {
            let result = protocol::decode_command_result(wire.payload()).map_err(debug)?;
            assert_eq!(result.command_id, id);
            return Ok((result.status, wire::Reply::decode(result.payload)));
        }
    }
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    source: ItemHandle,
    stacks: &[[u8; 16]],
    mode: Mode,
) -> TestResult {
    let (command_type, capability, quantity) = if mode == Mode::StashContainer {
        stash_cases::runtime_role(owner.root).await?;
        (
            wire::COMMAND_TYPE_STASH_CONTAINER,
            wire::CAPABILITY_STASH_CONTAINER_V1,
            200,
        )
    } else {
        let destination = stacks.get(1).ok_or("assigned bag")?;
        sqlx::query("INSERT INTO game_character_quick_loot_settings(character_id,unassigned_container_id,committed_character_revision) SELECT character_id,encode($1,'hex')::uuid,character_revision FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(destination.as_slice()).bind(id(41).as_slice()).execute(pool).await?;
        if mode == Mode::ManagedArmor {
            use oteryn_gameplay_items::quick_loot::LootCategory;
            // These are the durable destinations selected through Quick Loot.
            sqlx::query("INSERT INTO game_character_quick_loot_categories(character_id,category,container_id)
                         VALUES (encode($1,'hex')::uuid,$2,NULL),
                                (encode($1,'hex')::uuid,$3,encode($4,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(LootCategory::Equipment as i16)
                .bind(LootCategory::Armor as i16).bind(destination.as_slice()).execute(pool).await?;
        }
        stash_cases::runtime_role(owner.root).await?;
        (
            wire::COMMAND_TYPE_MANAGED_CONTAINER_MOVE,
            wire::CAPABILITY_MANAGED_CONTAINER_MOVE_V1,
            2,
        )
    };
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 14, 18, 37, 62, 98, 153, capability],
    )
    .ok_or("batch capabilities")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(capability)
    );

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let exercise = async {
        let confirm = |preview| {
            NonZeroU64::new(preview)
                .map(|preview| wire::Request::Confirm { preview })
                .ok_or("preview id")
        };
        assert_eq!(
            command(&mut client, 1, command_type, confirm(1)?).await?,
            (CommandStatus::Rejected, None)
        );
        let first = if matches!(mode, Mode::ManagedContainerMove | Mode::ManagedArmor) {
            let direct = wire::Request::Preview {
                source,
                recursive: false,
            };
            assert_eq!(
                command(&mut client, 2, command_type, direct).await?,
                (CommandStatus::Accepted, Some(wire::Reply::Preview(1)))
            );
            3
        } else {
            2
        };
        let preview = wire::Request::Preview {
            source,
            recursive: true,
        };
        assert_eq!(
            command(&mut client, first, command_type, preview).await?,
            (
                CommandStatus::Accepted,
                Some(wire::Reply::Preview(quantity))
            )
        );

        let nested = stacks.last().ok_or("nested fixture")?;
        let nested_source = stacks.first().ok_or("nested source bag")?;
        let parent_of = || async {
            sqlx::query_scalar::<_, Vec<u8>>("SELECT uuid_send(parent_item_instance_id) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(nested.as_slice()).fetch_optional(pool).await
        };
        assert_eq!(parent_of().await?, Some(nested_source.to_vec()));
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_nested_entries SET placement_ordinal=2 WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(nested.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        assert_eq!(
            command(&mut client, first + 1, command_type, confirm(first)?).await?,
            (CommandStatus::Rejected, None)
        );
        assert_eq!(parent_of().await?, Some(nested_source.to_vec()));
        let first = first + 2;
        assert_eq!(
            command(&mut client, first, command_type, preview).await?,
            (
                CommandStatus::Accepted,
                Some(wire::Reply::Preview(quantity))
            )
        );
        let confirm = confirm(first)?;
        assert_eq!(
            command(&mut client, first + 1, command_type, confirm).await?,
            (
                CommandStatus::Accepted,
                Some(wire::Reply::Committed(quantity))
            )
        );
        assert_eq!(
            command(&mut client, first + 1, command_type, confirm).await?,
            (
                CommandStatus::DuplicateReplay,
                Some(wire::Reply::Committed(quantity))
            )
        );
        if mode == Mode::StashContainer {
            assert_eq!(parent_of().await?, None);
            let balance: i64 = sqlx::query_scalar("SELECT quantity FROM game_character_stash_balances WHERE definition_key='oteryn:item.tibia.i3028'").fetch_one(pool).await?;
            assert_eq!(balance, 200);
            let child_count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_stash_operations WHERE container_ordinal>0",
            )
            .fetch_one(pool)
            .await?;
            assert_eq!(child_count, 2);
        } else {
            let destination = stacks.get(1).ok_or("assigned bag")?;
            let units: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid").bind(destination.as_slice()).fetch_one(pool).await?;
            assert_eq!(units, if mode == Mode::ManagedArmor { 2 } else { 80 });
            let child_count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_loop_move_receipts WHERE quick_loot_ordinal>0",
            )
            .fetch_one(pool)
            .await?;
            assert_eq!(child_count, 2);
        }
        client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::join!(
        crate::gameplay_transport::connection::serve_admitted(
            &mut server,
            admitted,
            owner,
            SCENARIO_LIVENESS
        ),
        exercise,
    );
    result
}
