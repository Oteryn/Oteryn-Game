//! D1195 actual linked checkpoint, strict SQL shape and current-writer fencing.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::character_familiar::{
    DurableFamiliarState, FamiliarLifecycleWriter, FamiliarStateOccurrence, FamiliarStateOutcome,
    FamiliarStateRequest,
};
use crate::durability::character_progression::CharacterProgressionError;
/// Isolated native fixture ONLY: reconstruct an already-committed v1 receipt with
/// its original binding. The bypass is transaction-local and ends before real APIs.
pub(super) async fn seed_historical_legacy(
    h: &Harness,
    request: &FamiliarStateRequest,
) -> TestResult {
    use sha2::{Digest, Sha256};
    let writer = fence(1)?;
    let mut semantic = vec![1_u8];
    semantic.extend_from_slice(request.occurrence.as_bytes());
    semantic.extend_from_slice(writer.character_id.as_bytes());
    semantic.extend_from_slice(&1_u64.to_be_bytes());
    for value in [
        serde_json::to_string(&request.before)?,
        serde_json::to_string(&request.after)?,
        request.content_revision.clone(),
        request.policy_revision.clone(),
    ] {
        semantic.extend_from_slice(&(value.len() as u64).to_be_bytes());
        semantic.extend_from_slice(value.as_bytes());
    }
    semantic.extend_from_slice(&request.policy_digest);
    let mut binding = vec![1_u8];
    binding.extend_from_slice(&Sha256::digest(&semantic));
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for query in [
        "UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid",
        "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid",
    ] {
        sqlx::query(query)
            .bind(writer.character_id.as_bytes().as_slice())
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_character_familiar_receipts(familiar_occurrence_id,command_binding,policy_digest,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,state_before,state_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,$2,$3,s.character_id,1,2,s.level,s.level,s.total_experience,s.total_experience,$4::text::jsonb,$5::text::jsonb,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($6,'hex')::uuid")
        .bind(request.occurrence.as_bytes().as_slice()).bind(binding).bind(request.policy_digest.as_slice()).bind(serde_json::to_string(&request.before)?).bind(serde_json::to_string(&request.after)?).bind(writer.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_familiar_state(character_id,state,committed_character_revision,last_familiar_occurrence_id) VALUES(encode($1,'hex')::uuid,$2::text::jsonb,2,encode($3,'hex')::uuid)").bind(writer.character_id.as_bytes().as_slice()).bind(serde_json::to_string(&request.after)?).bind(request.occurrence.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
#[test]
fn familiar_v2_converts_linked_checkpoint_and_rejects_record_derived_writer() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "familiar_v2_checkpoint", true).await?;
        let result: TestResult = async {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let seconds = h.root.read_familiar_unix_time(&authority).await.map_err(debug)?.seconds();
            let legacy = DurableFamiliarState { saved_expiry_unix: seconds + 60, last_logout_unix: 1, ..Default::default() };
            let request = |tag, before, after| -> TestResult<FamiliarStateRequest> { Ok(FamiliarStateRequest {
                occurrence: FamiliarStateOccurrence::from_bytes(id(tag)).map_err(debug)?, before, after,
                content_revision: "content-1".into(), policy_revision: "policy-1".into(), policy_digest: [1;32],
            }) };
            let first = request(120, DurableFamiliarState::default(), legacy.clone())?;
            let refused = h.root.commit_character_familiar_state(&authority,&h.node,fence(1)?,first.clone()).await;
            assert!(matches!(refused,Err(CharacterProgressionError::InvalidInput)),"{refused:?}");
            seed_historical_legacy(&h, &first).await?;
            // Actual API replay occurs after fixture transaction restored trigger role.
            assert!(matches!(h.root.commit_character_familiar_state(&authority,&h.node,fence(1)?,first.clone()).await.map_err(debug)?, FamiliarStateOutcome::AlreadyCommitted(_)));
            let loaded = h.root.read_character_familiar_state(&authority,CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?).await.map_err(debug)?;
            let actual: i64 = sqlx::query_scalar("SELECT r.committed_at FROM game_character_familiar_state s JOIN game_character_familiar_receipts r ON r.familiar_occurrence_id=s.last_familiar_occurrence_id WHERE s.character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(loaded.checkpoint_committed_at_ms(),Some(actual));
            let writer = FamiliarLifecycleWriter::from_fence(&fence(2)?).map_err(debug)?;
            let after = loaded.state().for_next_write(Some(actual),&writer).map_err(debug)?;
            assert_eq!(after.lifecycle.as_ref().ok_or("missing v2")?.familiar_remaining_ms, u64::try_from((i128::from(seconds+60)*1000-i128::from(actual)).max(0))?);
            let json = serde_json::to_value(&after)?;
            let valid: bool = sqlx::query_scalar("SELECT game_character_familiar_state_valid($1::jsonb)").bind(&json).fetch_one(&h.pool).await?;
            assert!(valid);
            let mut malformed = json.clone(); malformed["lifecycle"]["unexpected"] = true.into();
            let invalid: bool = sqlx::query_scalar("SELECT game_character_familiar_state_valid($1::jsonb)").bind(&malformed).fetch_one(&h.pool).await?;
            assert!(!invalid);
            let mut forged = after.clone(); forged.lifecycle.as_mut().ok_or("missing v2")?.writer.connection_generation += 1;
            let rejected = h.root.commit_character_familiar_state(&authority,&h.node,fence(2)?,request(121,legacy.clone(),forged)?).await;
            assert!(matches!(rejected,Err(CharacterProgressionError::AuthorityRejected)),"{rejected:?}");
            let mut unsupported_clean_end = after.clone();
            unsupported_clean_end.lifecycle.as_mut().ok_or("missing v2")?.clean_end_game_session_id =
                Some(writer.game_session_id.clone());
            let rejected = h.root.commit_character_familiar_state(&authority, &h.node, fence(2)?,
                request(123, legacy.clone(), unsupported_clean_end)?).await;
            assert!(matches!(rejected, Err(CharacterProgressionError::AuthorityRejected)), "{rejected:?}");
            let second = request(122,legacy,after.clone())?;
            assert!(matches!(h.root.commit_character_familiar_state(&authority,&h.node,fence(2)?,second.clone()).await.map_err(debug)?,FamiliarStateOutcome::Committed(_)));
            let downgraded = h.root.commit_character_familiar_state(&authority,&h.node,fence(3)?,request(124,after.clone(),first.after.clone())?).await;
            assert!(matches!(downgraded,Err(CharacterProgressionError::InvalidInput)),"{downgraded:?}");
            let retained = h.root.reconcile_character_familiar_state(&authority, first.occurrence)
                .await.map_err(debug)?.ok_or("missing legacy receipt")?;
            assert_eq!(retained.before(), &DurableFamiliarState::default());
            assert_eq!(retained.after(), &first.after);
            assert!(retained.matches_request(&fence(1)?, &first));
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
