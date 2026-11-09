use super::*;
use crate::bestiary_postgres_harness as pg;

const QUEST: &str = "oteryn:quest.the_ice_islands_quest";

#[test]
fn m01b_completed_quests_postgres_requires_own_completion_and_locks_it() -> pg::TestResult {
    let Some(admin) = pg::configured_admin() else {
        return Ok(());
    };
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            pg::runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let h = pg::Harness::create(admin, "m01b_completed", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|e: Box<dyn std::error::Error>| e.to_string())
        })?
        .join()
        .map_err(|_| "completion PG worker panicked")?;

    result.map_err(Into::into)
}

async fn exercise(h: &pg::Harness) -> pg::TestResult {
    let _version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;

    let fence = pg::fence(2)?;
    let keys = vec![QUEST.to_string()];
    let mut tx = h.pool.begin().await?;
    assert!(
        !require(&mut tx, fence, &keys).await?,
        "absent is not completed"
    );
    tx.rollback().await?;
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(pg::debug)?;
    let hash = catalogue
        .catalogue()
        .definition_hash(QUEST)
        .ok_or("canonical prerequisite")?;
    // Fixture arrangement only. The production callback never creates prerequisite progress.
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=greatest(character_revision,2) WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).execute(&mut *setup).await?;
    let inserted=sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$2,'content-1',$3,character_revision,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(QUEST).bind(hash.as_slice()).execute(&mut *setup).await?;
    assert_eq!(inserted.rows_affected(), 1);
    setup.commit().await?;
    let mut tx = h.pool.begin().await?;
    assert!(
        !require(&mut tx, fence, &keys).await?,
        "started is not completed"
    );
    tx.rollback().await?;
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("UPDATE game_character_quest_states SET completed_character_revision=committed_character_revision WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(QUEST).execute(&mut *setup).await?;
    setup.commit().await?;
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(s)) FROM game_character_quest_states s")
            .fetch_one(&h.pool)
            .await?;
    let root_before = h.root_revision().await?;
    let mut tx = h.pool.begin().await?;
    assert!(require(&mut tx, fence, &keys).await?);
    let mut foreign = fence;
    foreign.character_id = crate::domain::CharacterId::from_bytes(pg::id(60)).map_err(pg::debug)?;
    assert!(
        !require(&mut tx, foreign, &keys).await?,
        "another character cannot inherit completion"
    );
    assert!(
        !require(
            &mut tx,
            fence,
            &[
                QUEST.into(),
                "oteryn:quest.the_spirit_will_get_you_quest".into()
            ]
        )
        .await?,
        "every prerequisite must be complete"
    );
    let mut concurrent = h.pool.begin().await?;
    // Isolate the row lock from the separate writer-permit trigger guard.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *concurrent)
        .await?;
    sqlx::query("SET LOCAL lock_timeout='25ms'")
        .execute(&mut *concurrent)
        .await?;
    let conflict=sqlx::query("UPDATE game_character_quest_states SET completed_character_revision=completed_character_revision WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(QUEST).execute(&mut *concurrent).await;
    assert_eq!(
        conflict
            .err()
            .and_then(|e| e
                .as_database_error()
                .and_then(|d| d.code())
                .map(|s| s.into_owned()))
            .as_deref(),
        Some("55P03"),
        "completion stays locked until callback transaction ends"
    );
    concurrent.rollback().await?;
    tx.commit().await?;
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(s)) FROM game_character_quest_states s")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(before, after, "completion predicate is read-only");
    assert_eq!(root_before, h.root_revision().await?);
    Ok(())
}
