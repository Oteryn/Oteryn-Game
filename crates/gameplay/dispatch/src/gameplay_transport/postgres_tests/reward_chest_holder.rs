//! Verify bounded pre-issue checkout and receipt recovery on framed chest USE.

use super::fixture::{Harness, debug};
use super::quest_callbacks_fixtures::wire_use_targets;
use super::quest_container_chests_fixtures::{Case, scenario};
use super::TestResult;
use crate::durability::reward_claim_mint::with_stalled_reward_chest_commit;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::bundle_chest::{
    with_busy_reward_mint_holder, with_expiring_reward_preflight,
};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::time::Duration;

#[derive(Clone, Copy)]
pub(super) enum Scenario {
    Brief,
    Exhausted,
    LostCommitReply,
}

#[test]
fn reward_chest_use_waits_for_brief_holder_contention_and_mints_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    run(Scenario::Brief)
}

#[test]
fn reward_chest_use_replays_busy_refusal_and_next_use_mints_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    run(Scenario::Exhausted)
}

#[test]
fn reward_chest_use_recovers_lost_commit_reply_from_original_receipt() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    run(Scenario::LostCommitReply)
}

fn run(test: Scenario) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(Case::MintContention(test)).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "reward chest holder scenario panicked")??;
    Ok(())
}

pub(super) async fn exercise(
    case: Scenario,
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    target: &WorldObjectTarget,
    claim: &str,
    reward_key: &str,
) -> TestResult {
    let committed = (CommandStatus::Accepted, UseDisposition::Committed);
    let (command_id, charged_units) = match case {
        Scenario::Brief => {
            let (results, waited) = with_busy_reward_mint_holder(
                Duration::from_millis(200),
                wire_use_targets(owner, admitted, &[(1, target), (1, target)]),
            )
            .await;
            assert!(waited.is_some_and(|elapsed| elapsed >= Duration::from_millis(200)));
            assert_eq!(
                results?,
                [committed, (CommandStatus::DuplicateReplay, UseDisposition::Committed)],
            );
            (1_i64, 1_i16)
        }
        Scenario::Exhausted => {
            let (results, waited) = with_busy_reward_mint_holder(
                Duration::from_millis(1_200),
                wire_use_targets(owner, admitted, &[(1, target), (1, target), (2, target)]),
            )
            .await;
            assert!(
                waited.is_some_and(|elapsed| elapsed >= Duration::from_secs(1)),
                "the busy refusal must wait for the full checkout budget",
            );
            assert_eq!(
                results?,
                [
                    (CommandStatus::Rejected, UseDisposition::Rejected),
                    (CommandStatus::DuplicateReplay, UseDisposition::Rejected),
                    committed,
                ],
                "the next command is accepted on the same connection",
            );
            (2, 1)
        }
        Scenario::LostCommitReply => {
            lost_commit_reply(harness, owner, admitted, target, claim).await?;
            (1, 2)
        }
    };

    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM game_reward_claim_mint_reservations WHERE claim_production_key=$1),
            (SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1),
            (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1),
            (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$2),
            (SELECT count(*) FROM game_item_audit_outbox a
              JOIN game_reward_claim_mint_receipts r USING (event_id)
             WHERE r.claim_production_key=$1)",
    )
    .bind(claim)
    .bind(reward_key)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(counts, (1, 1, 1, 1, 1), "exactly one complete durable reward");
    let (reserved_command, work_units, minted_item): (i64, i16, Vec<u8>) = sqlx::query_as(
        "SELECT r.command_id::bigint, r.work_units_used, uuid_send(c.item_instance_id)
           FROM game_reward_claim_mint_reservations r
           JOIN game_reward_claim_mint_receipts c USING (game_session_id, command_id)
          WHERE r.claim_production_key=$1",
    )
    .bind(claim)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!((reserved_command, work_units), (command_id, charged_units));

    let fence = admitted.item_fence.ok_or("admitted item fence")?;
    let bag = harness
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("equipped reward backpack")?;
    assert_eq!(bag.entries.len(), 1);
    assert_eq!(bag.entries[0].item.item_instance_id.as_slice(), minted_item);
    assert_eq!(bag.entries[0].item.quantity, 1);
    Ok(())
}

async fn lost_commit_reply(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    target: &WorldObjectTarget,
    claim: &str,
) -> TestResult {
    use super::cyclopedia_fixtures::support::exchange;
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use oteryn_protocol_oteryn::world_object::{
        CAPABILITY_GAMEPLAY_OCCURRENCES_V1, COMMAND_TYPE_USE_INTENT, decode_use_reward_result,
        encode_use_intent,
    };

    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[CAPABILITY_GAMEPLAY_OCCURRENCES_V1],
    )
    .ok_or("reward occurrence capability")?;
    let payload = encode_use_intent(target).map_err(debug)?;
    // The existing USE preflight watchdog expires while COMMIT's response
    // is withheld. The issued mint still has to finish receipt recovery.
    let (result, stalled) = with_expiring_reward_preflight(with_stalled_reward_chest_commit(
        exchange(owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, &payload),
    ))
    .await;
    assert!(stalled, "PostgreSQL COMMIT must land before its response is withheld");
    let (status, wire) = result?;
    assert_eq!(status, CommandStatus::Accepted);
    let event: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(event_id) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(claim)
    .fetch_one(&harness.pool)
    .await?;
    let event: [u8; 16] = event.try_into().map_err(|_| "reward event ID length")?;
    assert_eq!(
        decode_use_reward_result(&wire),
        Ok((UseDisposition::Committed, None, Some(event))),
    );
    assert_eq!(
        exchange(owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, &payload).await?,
        (CommandStatus::DuplicateReplay, wire),
    );
    Ok(())
}
