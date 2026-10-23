use super::*;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;

pub(super) fn binding(
    fence: &CurrentCharacterGameplayFence,
    request: &NpcPurchaseRequest,
) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(fence.character_id.as_bytes());
    hash.update(fence.runtime_scope.world_id().as_bytes());
    hash.update(request.account);
    hash.update(request.command.game_session_id().as_bytes());
    hash.update(request.command.command_id().get().to_be_bytes());
    hash.update(request.npc_identity);
    hash.update(request.npc_generation.to_be_bytes());
    hash.update(request.service.kind().to_be_bytes());
    for value in [&request.npc_key, request.service.key()] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    hash.finalize().to_vec()
}

pub(in crate::durability) async fn load_by_command(
    connection: &mut PgConnection,
    character: CharacterId,
    command: CommandRef,
) -> Result<Option<(Vec<u8>, PurchaseReceipt)>, DurabilityError> {
    let row = sqlx::query("SELECT request_binding,original_character_revision::text,committed_character_revision::text,npc_identity,npc_generation::text,service_key,cause_kind,fee_gold_units FROM game_character_npc_purchase_receipts WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
        .bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(connection).await?;
    row.map(|row| {
        let service = row.try_get("service_key")?;
        Ok((
            row.try_get("request_binding")?,
            PurchaseReceipt {
                character,
                original_revision: CharacterRevision::new(numeric_u64(
                    &row,
                    "original_character_revision",
                )?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                committed_revision: CharacterRevision::new(numeric_u64(
                    &row,
                    "committed_character_revision",
                )?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                npc_identity: row
                    .try_get::<Vec<u8>, _>("npc_identity")?
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                npc_generation: numeric_u64(&row, "npc_generation")?,
                service: match row.try_get::<i16, _>("cause_kind")? {
                    2 => PurchaseService::Promotion(service),
                    3 => PurchaseService::Blessing(service),
                    _ => return Err(DurabilityError::InvalidStoredState),
                },
                price: u64::try_from(row.try_get::<i64, _>("fee_gold_units")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            },
        ))
    })
    .transpose()
}

pub(super) async fn insert(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &NpcPurchaseRequest,
    fee: &FeeBurnRequest,
    blessing: Option<&str>,
    binding: &[u8],
) -> Result<(), DurabilityError> {
    let occurrence = request.occurrence;
    sqlx::query("INSERT INTO game_character_npc_purchase_receipts(purchase_occurrence_id,character_id,account_id,command_session_id,command_id,request_binding,cause_kind,npc_key,npc_identity,npc_generation,service_key,blessing_key,fee_transaction_id,fee_gold_units,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,s.character_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9::text::numeric,$10,$11,encode($12,'hex')::uuid,$13,s.character_revision,s.character_revision+1,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($14,'hex')::uuid")
        .bind(occurrence.as_slice()).bind(request.account.as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(binding).bind(request.service.kind()).bind(&request.npc_key).bind(request.npc_identity.as_slice()).bind(request.npc_generation.to_string()).bind(request.service.key()).bind(blessing).bind(fee.transaction_id.as_slice()).bind(i64::try_from(fee.fee_gold_units).map_err(|_|DurabilityError::InvalidStoredState)?).bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    Ok(())
}
