use super::*;
const PROVENANCE: &str = rules::PROVENANCE;
fn uuid(bytes: &[u8; 16]) -> String {
    sqlx::types::Uuid::from_bytes(*bytes).to_string()
}
pub(super) fn grant_keys(
    plan: &plan::Prepared,
    state: &Snapshot,
    content: &impl AppearanceContent<String>,
) -> Result<Vec<String>, DurabilityError> {
    let mut keys = Vec::new();
    for grant in &plan.grants {
        if content.earned_source_look_addon(&grant.key, PROVENANCE, grant.look, grant.addon)
            != UnlockFact::Compatible
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if !state
            .owned_look_addon(grant.look, grant.addon)
            .map_err(|_| DurabilityError::InvalidStoredState)?
            && !keys.contains(&grant.key)
        {
            keys.push(grant.key.clone())
        }
    }
    if plan.stampor {
        let key = "oteryn:appearance-grant/npc/chondur/stampor/mount";
        let mount = "oteryn:content.mount.stampor".to_owned();
        if content.earned_unlock(key, PROVENANCE, EarnedCosmetic::Mount(&mount))
            != UnlockFact::Compatible
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if !state
            .owned_mount(11)
            .map_err(|_| DurabilityError::InvalidStoredState)?
        {
            keys.push(key.to_owned());
        }
    }
    if plan.primal_prize {
        let key = "oteryn:appearance-grant/npc/gnomadness/noxious_ripptor/mount";
        let mount = "oteryn:content.mount.noxious_ripptor".to_owned();
        if content.earned_unlock(key, PROVENANCE, EarnedCosmetic::Mount(&mount))
            != UnlockFact::Compatible
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if !state
            .owned_mount(202)
            .map_err(|_| DurabilityError::InvalidStoredState)?
        {
            keys.push(key.to_owned());
        }
    }
    if keys.len() > 2 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(keys)
}
pub(super) fn plan_proof(
    plan: &plan::Prepared,
    grants: &[String],
) -> Result<Value, DurabilityError> {
    let mut costs = std::collections::BTreeMap::<(String, String, String), u32>::new();
    for cost in &plan.costs {
        let key = (
            cost.definition.family.clone(),
            cost.definition.production_key.clone(),
            cost.definition.revision_ref.clone(),
        );
        let count = costs.entry(key).or_default();
        *count = count
            .checked_add(cost.count)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    let costs:Vec<_>=costs.into_iter().map(|((family,key,revision),count)|json!({"family":family,"key":key,"revision":revision,"count":count})).collect();
    let mut proof = json!({"source_key":plan.source_key,"changes":plan.changes,"grant_keys":grants,"costs":costs,"effects":plan.transition.effects});
    if plan.paid {
        let (npc, transition) = plan
            .source_key
            .strip_prefix("oteryn:npc-cosmetic/")
            .and_then(|source| source.split_once('/'))
            .ok_or(DurabilityError::InvalidStoredState)?;
        if transition != plan.transition.id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let service = format!("oteryn:service.npc-cosmetic.{npc}.{transition}");
        let gold = source_fee_price(&service).ok_or(DurabilityError::InvalidStoredState)?;
        let cash_minimum = source_fee_cash_minimum(&service)
            .filter(|minimum| *minimum <= gold)
            .ok_or(DurabilityError::InvalidStoredState)?;
        proof["fee"] = json!({"service":service,"gold":gold,"cash_minimum":cash_minimum});
    }
    Ok(proof)
}
#[allow(
    clippy::too_many_arguments,
    reason = "seals independent Node possession, actor/session fence, occurrence and native source plan"
)]
pub(super) async fn arm(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &NpcCosmeticRequest,
    transaction: &str,
    revision: u64,
    before: Value,
    plan: &Value,
) -> Result<Vec<u8>, DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let context = json!({"source_key":plan["source_key"],"npc_key":request.npc,"game_session_id":uuid(fence.game_session_id.as_bytes()),"character_id":uuid(fence.character_id.as_bytes()),"world_id":uuid(world_id.as_bytes()),"channel_id":uuid(channel_id.as_bytes()),"connection_generation":fence.connection_generation.get().to_string(),"lease_generation":fence.character_lease_generation.to_string(),"scope_generation":fence.scope_ownership_generation.get().to_string(),"original_revision":revision.to_string(),"command_id":request.command.command_id().get().to_string(),"npc_transaction_id":transaction,"request_binding":binding(fence.character_id,request)?.iter().map(|byte|format!("{byte:02x}")).collect::<String>()});
    let hash: Vec<u8> = sqlx::query_scalar(
        "SELECT game_npc_arm_source(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6,$7)",
    )
    .bind(node.fact().node_id().as_bytes().as_slice())
    .bind(node.fact().registration_revision().to_string())
    .bind(node.sql_possession_secret())
    .bind(context)
    .bind(source::digest().as_slice())
    .bind(before)
    .bind(plan)
    .fetch_one(&mut ***tx)
    .await?;
    if hash.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(hash)
}
#[allow(
    clippy::too_many_arguments,
    reason = "immutable receipt binds source plan, Character revision, item-cost child and account grants"
)]
pub(super) async fn write(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcCosmeticRequest,
    account: sqlx::types::Uuid,
    transaction: &str,
    next: u64,
    plan: &plan::Prepared,
    grants: &[String],
    cost: Option<&str>,
    fee: Option<&str>,
    outcome: &NpcCosmeticOutcome,
    hash: &[u8],
) -> Result<(), DurabilityError> {
    let changes =
        serde_json::to_value(&plan.changes).map_err(|_| DurabilityError::InvalidStoredState)?;
    let encoded = serde_json::to_value(outcome).map_err(|_| DurabilityError::InvalidStoredState)?;
    let changed=sqlx::query("INSERT INTO game_npc_cosmetic_receipts(transaction_id,character_id,account_id,game_session_id,command_id,npc_key,source_key,request_binding,outcome,track_changes,grant_keys,cost_transaction_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,program_sha256,plan_sha256,fee_transaction_id) SELECT $1::uuid,s.character_id,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9,$10,$11::uuid,s.character_revision,$12::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,$14,$15,$16::uuid FROM game_character_progression_state s WHERE s.character_id=encode($13,'hex')::uuid")
 .bind(transaction).bind(account).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(&request.npc).bind(plan.source_key).bind(binding(fence.character_id,request)?).bind(encoded).bind(changes).bind(grants).bind(cost).bind(next.to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(source::digest().as_slice()).bind(hash).bind(fee).execute(&mut ***tx).await?.rows_affected();
    if changed != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    account: sqlx::types::Uuid,
    transaction: &str,
    revision: u64,
    plan: &plan::Prepared,
    grants: &[String],
) -> Result<(), DurabilityError> {
    for change in &plan.changes {
        let changed=sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision WHERE game_character_quest_tracks.quest_key=EXCLUDED.quest_key AND game_character_quest_tracks.value=$6")
   .bind(character.as_bytes().as_slice()).bind(&change.key).bind(&change.quest).bind(change.after).bind(revision.to_string()).bind(change.before).execute(&mut ***tx).await?.rows_affected();
        if changed != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let event = sqlx::types::Uuid::parse_str(transaction)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    for key in grants {
        let occurrence: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut ***tx)
            .await?;
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,'npc',$6)").bind(occurrence).bind(account).bind(key).bind(PROVENANCE).bind(character.as_bytes().as_slice()).bind(event.as_bytes().as_slice()).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)").bind(account).bind(key).bind(occurrence).execute(&mut ***tx).await?;
    }
    let refs: Vec<_> = plan.achievements.iter().map(String::as_str).collect();
    super::super::super::account_achievement::grant_missing(
        tx,
        character,
        &refs,
        "npc",
        event.as_bytes(),
    )
    .await?;
    for (key, target) in &plan.progress {
        super::super::super::account_achievement::add_progress(
            tx,
            character,
            key,
            *target,
            "npc",
            event.as_bytes(),
        )
        .await?;
    }
    Ok(())
}
