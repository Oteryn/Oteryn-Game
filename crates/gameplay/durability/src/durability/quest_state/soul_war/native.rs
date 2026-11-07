use super::*;
use super::super::super::character_progression::numeric_u64;
use crate::domain::CharacterId;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_gameplay_quests::quest::soul_war::{
    SoulWarBoss, qualifies_for_taint_experience, taint_experience,
};
use sqlx::{Postgres, Transaction};

/// The XP owner calls this for each receiver's own raw share, before hunting and
/// stamina modifiers and before accepting any boss-death taint. Its transaction
/// subsequently checks and locks that receiver's supplied CharacterRevision.
pub async fn soul_war_experience_in_transaction(
    tx: &mut Transaction<'_, Postgres>, character: CharacterId,
    creature: &str, raw_experience: i64,
) -> Result<i64> {
    if !qualifies_for_taint_experience(creature) { return Ok(raw_experience); }
    let state = storage::load(tx, character).await?;
    let now = storage::database_time(tx).await?;
    taint_experience(raw_experience, state.taint_count(now))
        .map_err(|_| CharacterProgressionError::InvalidInput)
}

/// Native party-death receipt consumer. `dealt_damage` comes exclusively from
/// the captured native damage map; receiving shared XP does not grant a taint.
/// The caller holds all member revision slots and the original gameplay fences,
/// and invokes this after XP writes but before the same transaction commits.
pub async fn apply_soul_war_kill_in_transaction(
    tx: &mut Transaction<'_, Postgres>, node: &NodeIncarnationProof,
    death_key: &[u8; 52], creature: &str,
    participants: &[(CurrentCharacterGameplayFence, bool)],
) -> Result<()> {
    let Some(boss) = SoulWarBoss::from_creature(creature) else { return Ok(()); };
    if participants.is_empty() || participants.len() > 50 {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let transition = SoulWarTransition::BossDefeated(boss);
    let binding = storage::binding(transition)?;
    for (fence, dealt_damage) in participants {
        if !dealt_damage { continue; }
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
            return Err(CharacterProgressionError::AuthorityRejected);
        };
        if death_key[..16] != world_id.as_bytes()[..]
            || death_key[16..32] != channel_id.as_bytes()[..]
            || death_key[32..40] != fence.scope_ownership_generation.get().to_be_bytes()
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let source_matches: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_kill_xp_receipts d JOIN game_party_kill_xp_members m USING(death_key) WHERE d.death_key=$1 AND d.creature_key=$2 AND m.character_id=encode($3,'hex')::uuid)")
            .bind(death_key.as_slice()).bind(creature).bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
        if !source_matches { return Err(CharacterProgressionError::AuthorityRejected); }
        let occurrence: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:soul-war:death:v1")
            .chain_update(death_key).chain_update(fence.character_id.as_bytes())
            .finalize().into();
        storage::lock_occurrence(tx, fence.character_id, &occurrence).await?;
        if storage::replay(tx, fence.character_id, &occurrence, &binding).await??.is_some() {
            continue;
        }
        let native_member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_kill_xp_receipts d JOIN game_party_kill_xp_members m USING(death_key) WHERE d.death_key=$1 AND d.created_xact_id=pg_current_xact_id() AND d.creature_key=$2 AND m.character_id=encode($3,'hex')::uuid)")
            .bind(death_key.as_slice()).bind(creature).bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
        if !native_member { return Err(CharacterProgressionError::AuthorityRejected); }
        // XP and forge dust can advance this root within the already-fenced
        // transaction. Rebind only its revision, then recheck session, connection,
        // Character lease, scope generation and node incarnation unchanged.
        let row = sqlx::query("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
        let mut current = *fence;
        current.expected_character_revision = CharacterRevision::new(numeric_u64(&row, "character_revision")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let root = assert_gameplay_fence(tx, &current, node).await??;
        storage::commit_locked(tx, fence.character_id, &root, &occurrence, transition, Some(death_key), None).await?;
    }
    Ok(())
}
