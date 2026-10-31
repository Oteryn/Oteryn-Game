use super::*;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};

pub(super) async fn transaction(h: &Harness) -> TestResult<Type2Transaction<'_>> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='5s'").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL lock_timeout='5s'").execute(&mut *tx).await?;
    Type2Transaction::open(tx).await.map_err(debug)
}

pub(super) async fn save_entry(h: &Harness, source: GameSessionId) -> TestResult {
    sqlx::query("INSERT INTO game_character_last_positions(character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence) SELECT character_id,world_id,32375,31158,-8,decode(repeat('11',32),'hex'),decode(repeat('22',32),'hex'),decode(repeat('33',32),'hex'),character_lease_generation,1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(source.as_bytes().as_slice()).execute(&h.pool).await?;
    Ok(())
}

pub(super) async fn prepare(tx: &mut Type2Transaction<'_>, request: &SoulpitEntryRequest, source: GameSessionId, scope: RuntimeScopeRefV1) -> TestResult {
    let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else { return Err("Instance scope".into()); };
    sqlx::query("INSERT INTO game_activity_handoff_groups(handoff_id,world_id,instance_id,activity_key,template_revision,content_revision,ownership_generation,participant_count,state,prepared_at) SELECT encode($1,'hex')::uuid,s.world_id,encode($3,'hex')::uuid,'oteryn:activity.soulpit','template-1',r.content_revision,1,1,1,0 FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id) WHERE s.game_session_id=encode($2,'hex')::uuid")
        .bind(request.handoff_id.as_slice()).bind(source.as_bytes().as_slice()).bind(instance_id.as_slice()).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_activity_handoff_participants(handoff_id,world_id,character_id,account_id,origin_channel_id,source_game_session_id,destination_game_session_id,source_connection_generation,source_lease_generation,source_ownership_generation,profile_revision,ruleset_revision,content_revision,starter_template_revision,entry_x,entry_y,entry_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest) SELECT encode($1,'hex')::uuid,s.world_id,s.character_id,s.account_id,s.runtime_scope_channel_id,s.game_session_id,encode($3,'hex')::uuid,s.current_generation,s.character_lease_generation,s.scope_ownership_generation,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,p.tile_x,p.tile_y,p.tile_floor,p.coordinate_frame_digest,p.map_revision_digest,p.server_artifact_digest FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id) JOIN game_character_last_positions p USING(character_id) WHERE s.game_session_id=encode($2,'hex')::uuid")
        .bind(request.handoff_id.as_slice()).bind(source.as_bytes().as_slice()).bind(id(93).as_slice()).execute(&mut ***tx).await?;
    Ok(())
}

pub(super) async fn runtime_cannot_capture_commit(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
    let error = sqlx::query("INSERT INTO game_soulpit_entry_handoff_commits(handoff_id) VALUES(encode($1,'hex')::uuid)")
        .bind(id(92).as_slice()).execute(&mut *tx).await.expect_err("runtime captured a fake shared handoff decision");
    assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("42501"));
    tx.rollback().await?;
    Ok(())
}

pub(super) async fn null_consumption_is_not_a_free_entry(
    h: &Harness,
    request: &SoulpitEntryRequest,
    source: GameSessionId,
    scope: RuntimeScopeRefV1,
) -> TestResult {
    let mut tx = transaction(h).await?;
    prepare(&mut tx, request, source, scope).await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut **tx).await?;
    let error = sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,soul_core_proof) SELECT game_character_uuid_v7(),game_character_uuid_v7(),encode($1,'hex')::uuid,2,e.character_id,i.world_id,decode('01','hex'),'oteryn:soulpit.entry',i.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,to_jsonb(i),'[]'::jsonb,1,sha256('fixture'),true,jsonb_build_object('inputs',jsonb_build_array(jsonb_build_object('item',i.item_instance_id::text,'before',to_jsonb(i),'parent',e.parent_item_instance_id::text,'ordinal',e.placement_ordinal::text,'nested',false,'consume',NULL)),'delivery',jsonb_build_object('kind','handoff','handoff',encode($2,'hex')),'channel',encode($3,'hex'),'scope_generation','1') FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($4,'hex')::uuid")
        .bind(source.as_bytes().as_slice()).bind(request.handoff_id.as_slice()).bind(id(43).as_slice()).bind(request.source.as_slice())
        .execute(&mut **tx).await.expect_err("JSON null authorized a free Soulpit core debit");
    assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
    assert!(error.to_string().contains("invalid Soul Core input proof"), "wrong proof refusal: {error}");
    tx.into_inner().rollback().await?;
    Ok(())
}

/// The same real control writer/readiness publication used by boss_encounter's allocation
/// probe. Readiness is deliberately insufficient to grant any participant authority.
pub(super) async fn allocate_instance(h: &Harness) -> TestResult<RuntimeScopeRefV1> {
    use super::super::fresh_fixture::fresh::Source;
    use crate::durability::admission_authority_guards::GuardPublicationDisposition;
    use crate::durability::runtime_scope_assignment::{AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey, RuntimeScopeAssignmentWriter};
    use crate::foundation::admission_authority_publication::{AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1};
    let scope = RuntimeScopeRefV1::instance(super::super::fixture::fence(1)?.runtime_scope.world_id(), id(88)).map_err(debug)?;
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(scope.world_id().as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer").await.map_err(debug)?;
    let outcome = writer.submit(&AssignmentRequest {
        operation_key: OperationKey::from_bytes([88;32]), actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        command: AssignmentCommand::Assign { scope, target: h.node.fact() },
    }).await.map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = outcome else { return Err(format!("Instance allocation refused: {outcome:?}").into()); };
    assert_eq!(receipt.assignment.scope, scope);
    assert_eq!(receipt.assignment.ownership_generation, 1);
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
    let mut source = Source::new(now)?;
    source.rows.retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    assert_eq!(source.rows.len(), 1);
    source.rows[0].key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    assert_eq!(h.root.publish_runtime_readiness(&h.node, &publication).await.map_err(debug)?, GuardPublicationDisposition::Applied);
    Ok(scope)
}
