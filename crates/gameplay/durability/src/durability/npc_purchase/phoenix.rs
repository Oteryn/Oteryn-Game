//! Kawill's free preparation shares the admitted Character fence and replay key.
use super::*;

pub(super) async fn prepare(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &NpcPurchaseRequest,
    binding: &[u8],
) -> Result<PurchaseOutcome, DurabilityError> {
    if request.npc_key != "oteryn:npc.kawill" { return Ok(PurchaseOutcome::Refused(PurchaseRefusal::NotEligible)); }
    let prepared: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_phoenix_prerequisites WHERE character_id=encode($1,'hex')::uuid AND active)")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    if prepared { return Ok(PurchaseOutcome::Refused(PurchaseRefusal::AlreadyHeld)); }
    sqlx::query("INSERT INTO game_character_npc_stage_receipts(purchase_occurrence_id,character_id,account_id,command_session_id,command_id,request_binding,npc_key,npc_identity,npc_generation,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,s.character_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8::text::numeric,s.character_revision,s.character_revision+1,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($9,'hex')::uuid")
        .bind(request.occurrence.as_slice()).bind(request.account.as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(binding).bind(&request.npc_key).bind(request.npc_identity.as_slice()).bind(request.npc_generation.to_string()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_character_phoenix_prerequisites(character_id,grant_occurrence_id,active) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,true) ON CONFLICT(character_id) DO UPDATE SET grant_occurrence_id=EXCLUDED.grant_occurrence_id,active=true,consumed_by_purchase=NULL")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    let receipt = load_by_command(tx,fence.character_id,request.command).await?.ok_or(DurabilityError::InvalidStoredState)?.1;
    Ok(PurchaseOutcome::Committed(receipt))
}
