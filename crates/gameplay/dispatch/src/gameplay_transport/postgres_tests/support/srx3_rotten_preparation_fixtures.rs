use super::TestResult;

use super::fixture::{Harness, debug, id};

/// Defensive boundary ONLY, not a claimed normal play journey. The initial
/// historical global1/unlit state models a source activation whose public visual
/// has not been published. Its unrelated historical Character has no live actor.
/// All current USE/flag/burn/feedback work below still uses the real owners.
pub(super) async fn seed_historical_defensive_global_phase(
    h: &Harness,
    deps: &super::Dependencies,
) -> TestResult {
    let (scope, generation, activation, server, client, frame, map) = {
        let runtime = deps.runtime.lock().await;
        let pin = runtime.content_pin();
        (
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation().get(),
            pin.activation_sequence(),
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            pin.map_revision_digest(),
        )
    };
    let world = scope.world_id();
    let channel = scope.channel_id().ok_or("historical public Channel")?;
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let hash = catalogue
        .catalogue()
        .definition_hash("oteryn:quest/rotten-blood")
        .ok_or("historical actual Quest hash")?;
    // Reuse the established historical bootstrap fixture: a root without its
    // immutable creation receipt is correctly rejected by global recovery.
    crate::durability::guild_chat_fixture::historical_character(
        &h.pool,
        105,
        crate::domain::CharacterId::from_bytes(id(105)).map_err(debug)?,
        id(107),
        *world.as_bytes(),
        "Historical Flame Holder",
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1")
        .bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,2,1,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) VALUES(encode($1,'hex')::uuid,'use',encode($2,'hex')::uuid,1,'oteryn:quest-callback/rotten-blood/prepare-murcion','oteryn:quest/rotten-blood',$3,'content-1',$4,false,ARRAY['oteryn:quest-progress/storage/52280'],ARRAY[1]::bigint[],ARRAY[0]::bigint[],1,2,1,1,0,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1',0)")
        .bind(id(105).as_slice()).bind(id(106).as_slice()).bind([1_u8;33].as_slice()).bind(hash.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_tracks VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/storage/52280','oteryn:quest/rotten-blood',0,2)")
        .bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_states VALUES(encode($1,'hex')::uuid,'oteryn:quest/rotten-blood','content-1',$2,2,NULL,2)")
        .bind(id(105).as_slice()).bind(hash.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_rotten_bak_preparations(character_id,cause_id,cause_ordinal,transition_key,request_binding,world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,server_digest,client_digest,frame_digest,map_digest,flavor,source_phase_ordinal,committed_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'oteryn:quest-callback/rotten-blood/prepare-murcion',$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric(20,0),0,$7::text::numeric(20,0),$8,$9,$10,$11,0,0,2)")
        .bind(id(105).as_slice()).bind(id(106).as_slice()).bind([1_u8;33].as_slice())
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.to_string()).bind(activation.to_string())
        .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).bind(map.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
