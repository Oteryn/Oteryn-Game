//! Real command436 persists the full current-owner projection before immutable replay.
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::*;
#[test]
fn srx3_drome_postgres_large_hazard_receipt_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::reaudit_arena_lifecycle::physical_hazards().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "large Drome receipt test panicked")??;
    eprintln!(
        "SRX3_DROME POSTGRES_CASE_COMPLETED large_hazard_receipt_exact_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
pub(super) async fn large_hazards_receipt(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    harness: &super::fixture::Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::arena as wire;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(16))
        .ok_or("test owner clock")?;
    {
        let mut runtime = owner.runtime.lock().await;
        crate::gameplay_transport::tibiadrome::tick(owner, &mut runtime);
    }
    let snapshot =
        crate::gameplay_transport::tibiadrome::modifiers::snapshot(owner, admitted.game_session_id)
            .ok_or("current Drome hazard owner")?;
    assert_eq!(snapshot.selected, [5, 8]);
    assert_eq!(
        snapshot
            .entries
            .iter()
            .map(|entry| entry.positions.len())
            .sum::<usize>(),
        100,
        "source engine must produce actual100 walkable marks before receipt write"
    );
    let proof = wire::append_drome_hazards(
        wire::DromeResult {
            disposition: 1,
            phase: 2,
            ..Default::default()
        }
        .encode(),
        &snapshot,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert!(proof.len() > 128);
    eprintln!(
        "SRX3_DROME ACTUAL_CURRENT_OWNER_PROJECTION selected={:?} marks=100 bytes={} before command436 immutable save",
        snapshot.selected,
        proof.len()
    );
    let body = wire::encode_match_request(wire::ArenaAction::Drome(wire::DromeAction::Status))
        .map_err(|error| format!("{error:?}"))?;
    let constraint: String = sqlx::query_scalar("SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conrelid='game_tibiadrome_receipts'::regclass AND conname='game_tibiadrome_receipts_reply_check'").fetch_one(&harness.pool).await?;
    eprintln!("SRX3_DROME ACTUAL_POSTGRES_RECEIPT_CONSTRAINT {constraint}");
    let command = admitted.continuity.next_command_id;
    let (status, payload) = super::guild_wire::exchange(
        owner,
        admitted,
        command,
        wire::COMMAND_TYPE_ARENA_MATCH,
        &body,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert!(payload.len() > 128);
    let received = wire::decode_drome_hazards(&payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(received.selected, snapshot.selected);
    assert_eq!(received.entries, snapshot.entries);
    let stored:Vec<u8>=sqlx::query_scalar("SELECT reply FROM game_tibiadrome_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::numeric").bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string()).fetch_one(&harness.pool).await?;
    assert_eq!(
        stored, payload,
        "field13 must be in the first immutable stored reply"
    );
    let replay = owner
        .arena_match_command(*admitted, command, &body)
        .await
        .ok_or("large reply exact replay")?;
    assert_eq!(
        replay, payload,
        "a replay must not append or recompute hazards after receipt lookup"
    );
    Ok(())
}
