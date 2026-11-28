//! CONTAINER-MANUAL-SORT-1 (capability 510): a deposit naming its box slot takes that slot of
//! the box's view order (newest first) through the depot owner, in the same transaction and under
//! a container sort receipt; a replay with the same slot changes nothing and another slot
//! conflicts.
use super::*;
use oteryn_protocol_oteryn::item_view::ItemHandle;

const PAGE_FIXTURE_BOX: u8 = 16;

/// Items in display order.
async fn order(pool: &sqlx::PgPool, box_number: u8) -> TestResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT item_instance_id::text FROM game_item_character_depot_locations \
         WHERE town_id=0 AND box_number=$1 ORDER BY ordinal DESC",
    )
    .bind(i16::from(box_number))
    .fetch_all(pool)
    .await?)
}

fn text(item: [u8; 16]) -> String {
    let h: String = item.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    stacks: &[(ItemHandle, [u8; 16])],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let [(a_handle, a), (b_handle, b), (c_handle, c)] = stacks[..3] else {
        return Err("three carried stacks".into());
    };
    let deposit = |handle| DepotIntent {
        town: Some(0),
        box_number: 17,
        page: 0,
        action: DepotAction::Deposit(handle),
    };
    let outcome = |command_id, handle, item, slot| async move {
        Ok::<_, Box<dyn std::error::Error>>(
            owner
                .depot_at(
                    actor,
                    command(command_id),
                    deposit(handle),
                    slot,
                    Some(ItemKey::Instance(item)),
                )
                .await
                .ok_or("deposit")?
                .reply
                .outcome,
        )
    };
    assert_eq!(outcome(101, a_handle, a, None).await?, DepotOutcome::Moved);
    assert_eq!(order(pool, 17).await?, vec![text(a)]);
    let shown = Box::pin(owner.depot_at(
        actor,
        command(104),
        DepotIntent {
            action: DepotAction::Browse,
            ..deposit(a_handle)
        },
        None,
        None,
    ))
    .await
    .ok_or("displayed depot page")?;
    assert_eq!(shown.items.len(), 1);
    let (ordinal, expected) = shown.items[0];
    assert_eq!(expected, a);
    // A slot past the last entry places the deposit last.
    assert_eq!(
        outcome(102, b_handle, b, Some(5)).await?,
        DepotOutcome::Moved
    );
    assert_eq!(order(pool, 17).await?, vec![text(a), text(b)]);
    // The third deposit takes slot 1, between the earlier two.
    assert_eq!(
        outcome(103, c_handle, c, Some(1)).await?,
        DepotOutcome::Moved
    );
    assert_eq!(order(pool, 17).await?, vec![text(a), text(c), text(b)]);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_container_sort_receipts \
         WHERE permutation @> '[{\"depot\":{\"town\":0,\"box\":17}}]'",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(
        receipts, 2,
        "one depot ordinal receipt per rotated placement"
    );
    // The same command with the same slot replays; with another slot it conflicts.
    assert_eq!(
        outcome(103, c_handle, c, Some(1)).await?,
        DepotOutcome::Moved
    );
    assert_eq!(
        outcome(103, c_handle, c, Some(0)).await?,
        DepotOutcome::Rejected
    );
    assert_eq!(order(pool, 17).await?, vec![text(a), text(c), text(b)]);
    let stale = Box::pin(owner.depot_at(
        actor,
        command(105),
        DepotIntent {
            action: DepotAction::Withdraw(ordinal),
            ..deposit(a_handle)
        },
        None,
        Some(ItemKey::Instance(expected)),
    ))
    .await
    .ok_or("withdraw")?;
    assert_eq!(stale.reply.outcome, DepotOutcome::Stale);
    assert_eq!(order(pool, 17).await?, vec![text(a), text(c), text(b)]);
    // The receipt triggers admit no ordinal change outside a current depot move.
    let forged = sqlx::query(
        "UPDATE game_item_character_depot_locations SET ordinal=ordinal \
         WHERE item_instance_id=$1::uuid",
    )
    .bind(text(a))
    .execute(pool)
    .await;
    assert!(
        forged.is_err(),
        "an ordinal update without its receipt is refused"
    );
    Ok(())
}

pub(super) async fn deposit_page_fixture(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    item: [u8; 16],
    slot: Option<u32>,
) -> TestResult {
    let moved = owner
        .depot_at(
            actor,
            command,
            DepotIntent {
                town: Some(0),
                box_number: PAGE_FIXTURE_BOX,
                page: 0,
                action: DepotAction::Deposit(ItemHandle::MIN),
            },
            slot,
            Some(ItemKey::Instance(item)),
        )
        .await
        .ok_or("page fixture deposit")?;
    assert_eq!(moved.reply.outcome, DepotOutcome::Moved);
    Ok(())
}

pub(super) async fn stale_withdrawal(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    stacks: &[(ItemHandle, [u8; 16])],
) -> TestResult {
    use super::write_text_depot_session::{exchange, read_reply, send, send_at};
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 12, 14, 18, 37, 62, 153, 510],
    )
    .ok_or("manual sort selection")?;
    let browse = DepotIntent {
        town: Some(0),
        box_number: 17,
        page: 0,
        action: DepotAction::Browse,
    };
    let (_, added) = stacks[3];
    let storage_ordinal = async |item| -> TestResult<String> {
        Ok(sqlx::query_scalar(
            "SELECT ordinal::text FROM game_item_character_depot_locations WHERE item_instance_id=$1::uuid",
        )
        .bind(text(item))
        .fetch_one(pool)
        .await?)
    };
    let previous_ordinal = storage_ordinal(stacks[1].1).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let mut view = SessionItemView::resume(admitted.continuity.item_view).with_equip_drop();
    view.snapshot(
        crate::gameplay_transport::item_move::observe_inventory(
            owner,
            actor,
            admitted.game_session_id,
            &view,
        )
        .await
        .ok_or("current inventory")?,
        None,
    )
    .map_err(debug)?;
    let source = view
        .table()
        .handle(&ItemKey::Instance(added))
        .ok_or("last carried stack")?;
    let reordered = vec![
        text(stacks[0].1),
        text(stacks[2].1),
        text(stacks[1].1),
        text(added),
    ];
    let generation = admitted.continuity.connection_generation;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        exchange(owner, &mut admitted, async |client| {
            send(client, 1, browse).await?;
            let before = read_reply(client, 1).await?;
            assert_eq!(before.outcome, DepotOutcome::Opened);
            let old = before
                .entries
                .last()
                .ok_or("the formerly last entry")?
                .ordinal;
            send_at(
                client,
                2,
                DepotIntent {
                    action: DepotAction::Deposit(source),
                    ..browse
                },
                Some(5),
            )
            .await?;
            assert_eq!(read_reply(client, 2).await?.outcome, DepotOutcome::Moved);
            assert_eq!(order(pool, 17).await?, reordered);
            assert_eq!(storage_ordinal(added).await?, previous_ordinal);
            send(
                client,
                3,
                DepotIntent {
                    action: DepotAction::Withdraw(old),
                    ..browse
                },
            )
            .await?;
            let stale = read_reply(client, 3).await?;
            assert_eq!(stale.outcome, DepotOutcome::Stale);
            assert_eq!(order(pool, 17).await?, reordered);
            let current = stale.entries.last().ok_or("the added entry")?.ordinal;
            send(
                client,
                4,
                DepotIntent {
                    action: DepotAction::Withdraw(current),
                    ..browse
                },
            )
            .await?;
            assert_eq!(read_reply(client, 4).await?.outcome, DepotOutcome::Moved);
            assert_eq!(order(pool, 17).await?, reordered[..3]);
            stale_withdrawal_after_search(client, generation, pool).await
        }),
    )
    .await?
}

async fn stale_withdrawal_after_search(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    pool: &sqlx::PgPool,
) -> TestResult {
    use super::super::item_forge_fixtures::support::send_command;
    use super::write_text_depot_session::{read_reply, send};
    use oteryn_gameplay_items::depot::DEPOT_PAGE_ENTRIES;
    use oteryn_protocol_oteryn::{CommandStatus, depot_search};

    let browse = DepotIntent {
        town: Some(0),
        box_number: PAGE_FIXTURE_BOX,
        page: 1,
        action: DepotAction::Browse,
    };
    let before = order(pool, PAGE_FIXTURE_BOX).await?;
    let page_size = usize::try_from(DEPOT_PAGE_ENTRIES)?;
    assert_eq!(before.len(), page_size + 1);
    let target = &before[page_size];
    send(client, 5, browse).await?;
    let page = read_reply(client, 5).await?;
    assert_eq!(page.outcome, DepotOutcome::Opened);
    assert_eq!(page.entries.len(), 1);
    let ordinal = page.entries[0].ordinal;

    // Route the search on the live connection so its periodic work and commands stay serialized.
    // Retrieval changes custody without refreshing the command-36 depot view.
    let payload = depot_search::encode_request(depot_search::Request::Retrieve {
        client_id: 3031,
        all: false,
    })
    .map_err(debug)?;
    let (status, reply) =
        send_command(client, generation, 6, depot_search::COMMAND_TYPE_DEPOT_SEARCH, &payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let search = depot_search::decode_reply(&reply).map_err(debug)?;
    assert_eq!(
        (search.outcome, search.retrieved),
        (depot_search::Outcome::Retrieved, 100)
    );
    let shifted = order(pool, PAGE_FIXTURE_BOX).await?;
    assert_eq!(shifted.len(), page_size);
    assert_eq!(shifted.last(), Some(target));

    send(client, 7, DepotIntent {
        action: DepotAction::Withdraw(ordinal),
        ..browse
    })
    .await?;
    assert_eq!(read_reply(client, 7).await?.outcome, DepotOutcome::Stale);
    assert_eq!(order(pool, PAGE_FIXTURE_BOX).await?, shifted);
    Ok(())
}
