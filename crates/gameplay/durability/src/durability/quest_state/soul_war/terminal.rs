use super::*;
use sqlx::{Postgres, Transaction};

/// The shared encounter owner has proven a terminal unsuccessful Megalomania
/// attempt and retained its group-wide pair in this transaction. This is crate
/// private: neither NPC commands nor individual deaths can invoke a final reset.
/// PR2784 ANSWER6547488765; reference reconstruction, NOT_DONOR_CLOSED.
pub(crate) async fn apply_soul_war_failed_final_in_transaction(
    tx: &mut Transaction<'_, Postgres>, node: &NodeIncarnationProof,
    attempt: &[u8; 32], participants: &[CurrentCharacterGameplayFence], removed_bosses: u8,
) -> Result<()> {
    if participants.is_empty() || participants.len() > 5
        || removed_bosses.count_ones() != 2 || removed_bosses & !31 != 0
        || participants.windows(2).any(|pair| pair[0].character_id >= pair[1].character_id)
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let canonical = participants.iter().flat_map(|f| f.character_id.as_bytes().iter().copied()).collect::<Vec<_>>();
    let affected: Option<Vec<u8>> = sqlx::query_scalar("SELECT affected_participants FROM game_endgame_terminal_outcomes WHERE attempt=$1 AND encounter_kind=1 AND removed_flags=$2 AND participants=$3 AND created_xact_id=pg_current_xact_id()")
        .bind(attempt.as_slice()).bind(i16::from(removed_bosses)).bind(&canonical)
        .fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    let Some(affected) = affected else { return Err(CharacterProgressionError::AuthorityRejected); };
    if affected.is_empty() || affected.len() % 16 != 0
        || affected.as_chunks::<16>().0.iter().any(|member| !canonical.as_chunks::<16>().0.iter().any(|candidate| candidate == member))
    {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    let transition = SoulWarTransition::FailedMegalomania { removed_bosses };
    let binding = storage::binding(transition)?;
    for fence in participants {
        // ANSWER6547994219: the full roster is retained above. Expired members
        // have explicit no-op evidence in that outcome and must not consume the
        // selected pair or advance Character state a second time.
        if !affected.as_chunks::<16>().0.iter().any(|member| member == fence.character_id.as_bytes()) {
            continue;
        }
        let occurrence: [u8; 32] = Sha256::new().chain_update(b"oteryn:soul-war:failed-final:v1")
            .chain_update(attempt).chain_update(fence.character_id.as_bytes()).finalize().into();
        storage::lock_occurrence(tx, fence.character_id, &occurrence).await?;
        if storage::replay(tx, fence.character_id, &occurrence, &binding).await??.is_some() {
            continue;
        }
        let root = assert_gameplay_fence(tx, fence, node).await??;
        storage::commit_locked(tx, fence.character_id, &root, &occurrence, transition, None, Some(attempt)).await?;
    }
    Ok(())
}
