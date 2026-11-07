//! Task Board must not make the shared quest copy unreadable at its existing limits.
use super::*;
use oteryn_gameplay_quests::quest::{
    QUESTSTATE0_RL_01, QUESTSTATE0_RL_05, QUESTSTATE0_RL_08_STATES, QUESTSTATE0_RL_08_TRACKS,
    monk_shrines,
};

const FIXTURE_PREFIX: &str = "oteryn:taskboard-capacity-fixture/";

async fn seed_boundary(
    harness: &Harness,
    tracks: usize,
    states: usize,
    monk_flag: bool,
    fund_upgrade: bool,
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Previously committed unrelated quests/tracks are historical fixture input.
    // Every tested command below runs with normal receipt/custody/fence guards.
    let special = usize::from(monk_flag) + usize::from(fund_upgrade);
    let ordinary = tracks
        .checked_sub(special)
        .ok_or("capacity fixture shape")?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT encode($1,'hex')::uuid,$2||'track/'||n::text,$2||'quest',1,2 FROM generate_series(1,$3::bigint) n")
        .bind(id(41).as_slice()).bind(FIXTURE_PREFIX).bind(i64::try_from(ordinary)?)
        .execute(&mut *tx).await?;
    if fund_upgrade {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,5,2)")
            .bind(id(41).as_slice()).bind(task_board::BOUNTY_POINTS).bind(task_board::QUEST)
            .execute(&mut *tx).await?;
    }
    if monk_flag {
        let flag = format!("{}{}", monk_shrines::FLAG_PREFIX, monk_shrines::NAMES[0]);
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,1,2)")
            .bind(id(41).as_slice()).bind(flag).bind(format!("{FIXTURE_PREFIX}quest"))
            .execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) SELECT encode($1,'hex')::uuid,$2||'quest/'||n::text,'content-1',decode(repeat('00',32),'hex'),2,NULL,2 FROM generate_series(1,$3::bigint) n")
        .bind(id(41).as_slice()).bind(FIXTURE_PREFIX).bind(i64::try_from(states)?)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn remove_boundary(harness: &Harness) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let flag = format!("{}{}", monk_shrines::FLAG_PREFIX, monk_shrines::NAMES[0]);
    sqlx::query("DELETE FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND (left(track_key,length($2))=$2 OR track_key IN($3,$4))")
        .bind(id(41).as_slice()).bind(FIXTURE_PREFIX).bind(task_board::BOUNTY_POINTS).bind(flag)
        .execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND left(quest_key,length($2))=$2")
        .bind(id(41).as_slice()).bind(FIXTURE_PREFIX).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Snapshot all affected value families, excluding the newly retained refusal.
async fn values(harness: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('revision',r.character_revision::text,'progression',to_jsonb(p),'board',(SELECT to_jsonb(b) FROM game_character_task_boards b WHERE b.character_id=r.character_id),'tracks',(SELECT jsonb_object_agg(t.track_key,to_jsonb(t)) FROM game_character_quest_tracks t WHERE t.character_id=r.character_id),'states',(SELECT jsonb_object_agg(s.quest_key,to_jsonb(s)) FROM game_character_quest_states s WHERE s.character_id=r.character_id),'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY i.item_instance_id) FROM game_item_instances i WHERE i.world_id=r.world_id),'quest_receipts',(SELECT count(*) FROM game_character_quest_receipts q WHERE q.character_id=r.character_id),'xp_obligations',(SELECT count(*) FROM game_character_quest_xp_obligations q WHERE q.character_id=r.character_id),'item_grants',(SELECT count(*) FROM game_task_board_item_grants),'stash_sources',(SELECT count(*) FROM game_task_board_stash_sources),'fee_proofs',(SELECT count(*) FROM game_task_board_fee_proofs),'mint_reservations',(SELECT count(*) FROM game_reward_claim_inbox_mint_reservations),'item_audits',(SELECT count(*) FROM game_item_audit_outbox),'stash_audits',(SELECT count(*) FROM game_character_stash_audit_outbox)) FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?)
}

pub(super) async fn global_quest_capacity(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let initial: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid)")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(
        initial,
        (0, 0),
        "run before the first accepted Task Board mutation"
    );
    let character = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "capacity current fence")?
        .ok_or("capacity current fence")?
        .character_id;
    let track_limit = QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS);
    let state_limit = QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES);
    for (tracks, states, monk_flag, action) in [
        (track_limit, 0, false, 18_u8),
        (track_limit - 1, 0, true, 5_u8),
        (1, state_limit, false, 18_u8),
    ] {
        seed_boundary(harness, tracks, states, monk_flag, action == 18).await?;
        let copy = owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)?;
        assert_eq!(copy.tracks().len(), tracks + usize::from(monk_flag));
        assert_eq!(copy.states().len(), states);
        if monk_flag {
            assert_eq!(copy.tracks().get(monk_shrines::COUNT_TRACK), Some(&1));
        }
        let before = values(harness).await?;
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_task_board_receipts")
            .fetch_one(&harness.pool)
            .await?;
        let command = admitted.continuity.next_command_id;
        let replies = serve_from(owner, admitted, command, &[full_request(action, 0, "")]).await?;
        assert_eq!(replies[0].1, CommandStatus::Accepted);
        let bytes = replies[0].2.clone();
        let refused = wire::full::decode_reply(&bytes).map_err(debug)?;
        assert_eq!(
            refused.refusal, 16,
            "global quest capacity must be a durable semantic refusal"
        );
        if action == 18 {
            assert_eq!(
                refused.bounty_points, 5,
                "the affordable upgrade cannot spend before capacity admission"
            );
            assert!(refused.talisman.iter().all(|path| path.level == 0));
        }
        assert!(!refused.weekly.started);
        assert_eq!(values(harness).await?, before);
        let stored: serde_json::Value = sqlx::query_scalar("SELECT refusal FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND committed_character_revision IS NULL AND effects='null'::jsonb")
            .bind(id(41).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string())
            .fetch_one(&harness.pool).await?;
        assert_eq!(stored, serde_json::json!("CapacityExceeded"));
        let replay = serve_from(owner, admitted, command, &[full_request(action, 0, "")]).await?;
        assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
        assert_eq!(replay[0].2, bytes);
        assert_eq!(values(harness).await?, before);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_receipts")
                .fetch_one(&harness.pool)
                .await?,
            receipts + 1
        );
        let after = owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)?;
        assert_eq!(after.tracks(), copy.tracks());
        assert_eq!(after.states(), copy.states());

        remove_boundary(harness).await?;
        let cleared = values(harness).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        // Releasing capacity cannot turn the retained failed command into a new mutation.
        let replay = serve_from(owner, admitted, command, &[full_request(action, 0, "")]).await?;
        assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
        assert_eq!(replay[0].2, bytes);
        assert_eq!(values(harness).await?, cleared);
    }
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: global_quest_capacity_preserves_tracks_states_monk_projection_and_replay"
    );
    // The existing scenario immediately starts a new board with a fresh command,
    // exercising the successful path after the historical capacity input is removed.
    Ok(())
}
