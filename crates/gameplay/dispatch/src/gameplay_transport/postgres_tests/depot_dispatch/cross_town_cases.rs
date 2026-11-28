use super::write_text_depot_session::{exchange, read_reply, send, send_request};
use super::write_text_ground_movement::{read_step, step};
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) const OTHER_TOWN: u16 = 20;

#[test]
fn depot_cross_town_wire_reads_and_withdraws_from_existing_town_custody() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_with_premium(false, false, false, false, false, None, Mode::CrossTown)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "cross-town depot thread panicked")?
        .map_err(Into::into)
}

#[test]
fn stash_cross_town_wire_stows_only_stackable_items() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_with_premium(
                true,
                false,
                false,
                false,
                false,
                None,
                Mode::StashEligibility,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "stash eligibility thread panicked")?
        .map_err(Into::into)
}

pub(super) async fn depot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    pool: &sqlx::PgPool,
    handle: ItemHandle,
    item: [u8; 16],
) -> TestResult {
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 18, 37, CAPABILITY_CROSS_TOWN_DEPOT_V1],
    )
    .ok_or("cross-town selection")?;
    let browse = DepotIntent {
        town: Some(0),
        box_number: 17,
        page: 0,
        action: DepotAction::Browse,
    };
    let remote = DepotRequest {
        intent: browse,
        slot: None,
        cross_town: true,
    };
    let mut ordinal = 0;
    exchange(owner, &mut admitted, async |client| {
        send(
            client,
            1,
            DepotIntent {
                action: DepotAction::Deposit(handle),
                ..browse
            },
        )
        .await?;
        let deposited = read_reply(client, 1).await?;
        assert_eq!(
            (deposited.outcome, deposited.town, deposited.total),
            (DepotOutcome::Moved, Some(0), 1)
        );
        let location: (i32, i16) = sqlx::query_as(
            "SELECT town_id,box_number FROM game_item_character_depot_locations
             WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_one(pool)
        .await?;
        assert_eq!(location, (0, 17));

        step(client, 2, StepDirection::East).await?;
        read_step(client, 2).await?;
        send_request(client, 3, remote).await?;
        assert_eq!(read_reply(client, 3).await?.outcome, DepotOutcome::NoLocker);

        step(client, 4, StepDirection::East).await?;
        read_step(client, 4).await?;
        send(client, 5, browse).await?;
        assert_eq!(read_reply(client, 5).await?.outcome, DepotOutcome::Stale);
        send(
            client,
            6,
            DepotIntent {
                town: Some(OTHER_TOWN),
                ..browse
            },
        )
        .await?;
        let local = read_reply(client, 6).await?;
        assert_eq!(
            (local.outcome, local.town, local.total),
            (DepotOutcome::Opened, Some(OTHER_TOWN), 0)
        );

        send_request(client, 7, remote).await?;
        let shown = read_reply(client, 7).await?;
        assert_eq!(
            (shown.outcome, shown.town, shown.total),
            (DepotOutcome::Opened, Some(0), 1)
        );
        assert_eq!(shown.entries, deposited.entries);
        ordinal = shown.entries[0].ordinal;
        send_request(
            client,
            8,
            DepotRequest {
                intent: DepotIntent {
                    action: DepotAction::Withdraw(ordinal),
                    ..browse
                },
                ..remote
            },
        )
        .await?;
        let withdrawn = read_reply(client, 8).await?;
        assert_eq!(
            (withdrawn.outcome, withdrawn.town, withdrawn.total),
            (DepotOutcome::Moved, Some(0), 0)
        );
        Ok(())
    })
    .await?;

    let actor = admitted.runtime_actor.ok_or("admitted actor")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 8,
        item_fence: admitted.item_fence,
    };
    let withdrawal = DepotRequest {
        intent: DepotIntent {
            action: DepotAction::Withdraw(ordinal),
            ..browse
        },
        ..remote
    };
    let replay = owner
        .depot_request(actor, command, withdrawal, None)
        .await
        .ok_or("durable cross-town replay")?;
    assert_eq!(replay.reply.outcome, DepotOutcome::Moved);
    let conflicting = DepotRequest::local(withdrawal.intent, None);
    let replay = owner
        .depot_request(actor, command, conflicting, None)
        .await
        .ok_or("conflicting cross-town replay")?;
    assert_eq!(replay.reply.outcome, DepotOutcome::Rejected);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_loop_move_receipts WHERE depot_town=0 AND depot_box=17",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(receipts, 2);
    let carried: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid)",
    ).bind(item.as_slice()).fetch_one(pool).await?;
    assert!(carried);
    Ok(())
}
