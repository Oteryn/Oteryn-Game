//! Forward migration of derived caches; actual immutable guards stay enabled.
use super::*;

#[test]
fn tibiadrome_category_migration_retires_old_tokens_and_preserves_revision_floors() -> TestResult {
    run("hsdromecategories", async |h| {
        type ResyncSecurity = (String, String, Option<String>, Option<Vec<String>>, bool);
        const RESYNC_SECURITY: &str = "SELECT p.oid::regprocedure::text,p.proowner::text,p.proacl::text,p.proconfig,p.prosecdef FROM pg_proc p WHERE p.oid IN('game_public_projection_resync_before_world_totals(text,boolean)'::regprocedure,'game_public_projection_resync(text,boolean)'::regprocedure) ORDER BY p.proname";
        let security_original: Vec<ResyncSecurity> =
            sqlx::query_as(RESYNC_SECURITY).fetch_all(&h.pool).await?;
        // Harness has1107 already. Restore the actual pre1107 resync source
        // as well as1105 bounds; all immutable guards remain untouched/enabled.
        let current_base:String=sqlx::query_scalar("SELECT pg_get_functiondef('game_public_projection_resync_before_world_totals(text,boolean)'::regprocedure)").fetch_one(&h.pool).await?;
        let metadata_filter =
            "SELECT world_id, category FROM game_public_highscores WHERE category <> 13";
        let snapshot_filter = " FROM game_world_highscore_snapshots WHERE category <> 13";
        assert_eq!(current_base.matches(metadata_filter).count(), 1);
        assert_eq!(current_base.matches(snapshot_filter).count(), 1);
        let old_base = current_base
            .replace(
                metadata_filter,
                "SELECT world_id, category FROM game_public_highscores",
            )
            .replace(snapshot_filter, " FROM game_world_highscore_snapshots");
        h.pool.execute(sqlx::AssertSqlSafe(old_base)).await?;
        // Restore1105 CHECK bounds, without disabling or replacing any guard.
        for table in [
            "game_world_highscore_snapshots",
            "game_world_highscore_heads",
            "game_public_highscores",
        ] {
            h.pool.execute(sqlx::AssertSqlSafe(format!("ALTER TABLE {table} DROP CONSTRAINT {table}_category_check, ADD CONSTRAINT {table}_category_check CHECK(category BETWEEN 1 AND 14)"))).await?;
        }
        // Restore the historical touch function too: today's 2801 function deliberately
        // withholds Loyalty 13, while the pre1107 fixture must seed both revision floors.
        let current_touch: String = sqlx::query_scalar("SELECT pg_get_functiondef('game_public_highscores_touch(uuid,smallint)'::regprocedure)")
            .fetch_one(&h.pool).await?;
        let withheld = "IF p_category = 13 THEN\n        RETURN;\n    END IF;";
        assert_eq!(current_touch.matches(withheld).count(), 1);
        h.pool
            .execute(sqlx::AssertSqlSafe(current_touch.replace(withheld, "")))
            .await?;
        let mut tokens = Vec::new();
        for category in [13_i32, 14] {
            let mut pair = Vec::new();
            for ordinal in 0..2 {
                let token: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,0,'pre1107-derived-cache',1) RETURNING snapshot_id")
                    .bind(id(WORLD).as_slice()).bind(category).fetch_one(&h.pool).await?;
                sqlx::query("INSERT INTO game_world_highscore_rows(snapshot_id,filter,position,rank,character_id,name,vocation,level,value_primary,value_progress) VALUES($1,0,1,1,encode($2,'hex')::uuid,'Old cache','knight',1,$3,0)")
                    .bind(token).bind(id(CHARACTER).as_slice()).bind(i64::from(ordinal+1)).execute(&h.pool).await?;
                pair.push(token);
            }
            sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category,current_snapshot_id,previous_snapshot_id) VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
                .bind(id(WORLD).as_slice()).bind(category).bind(pair[1]).bind(pair[0]).execute(&h.pool).await?;
            tokens.extend(pair);
        }
        let metadata_before: Vec<(i16,i64,i64)> = sqlx::query_as("SELECT category,projection_revision,revised_at FROM game_public_highscores WHERE world_id=encode($1,'hex')::uuid AND category IN(13,14) ORDER BY category")
            .bind(id(WORLD).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(metadata_before.len(), 2);
        assert!(metadata_before.iter().all(|row| row.1 == 2));
        h.pool.execute(sqlx::AssertSqlSafe(current_touch)).await?;
        dataset(h, 300, 1, "druid", 1000, world()).await?;
        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,floor(extract(epoch FROM statement_timestamp())/1209600)::bigint,encode($2,'hex')::uuid,7,1000)")
            .bind(id(WORLD).as_slice()).bind(generated(300).as_slice()).execute(&h.pool).await?;
        let scores_before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(s) ORDER BY world_id,rotation,character_id) FROM game_tibiadrome_scores s").fetch_one(&h.pool).await?;
        let security_before: Vec<ResyncSecurity> =
            sqlx::query_as(RESYNC_SECURITY).fetch_all(&h.pool).await?;
        assert_eq!(security_before.len(), 2);
        assert_eq!(
            security_before, security_original,
            "fixture source restoration preserves owner/ACL/config/security"
        );
        let wrapper_before:String=sqlx::query_scalar("SELECT pg_get_functiondef('game_public_projection_resync(text,boolean)'::regprocedure)").fetch_one(&h.pool).await?;
        let base_before:String=sqlx::query_scalar("SELECT pg_get_functiondef('game_public_projection_resync_before_world_totals(text,boolean)'::regprocedure)").fetch_one(&h.pool).await?;
        let guard_before: String = sqlx::query_scalar(
            "SELECT pg_get_functiondef('game_public_highscores_revision_guard()'::regprocedure)",
        )
        .fetch_one(&h.pool)
        .await?;
        let mut tx = h.pool.begin().await?;
        tx.execute(include_str!(
            "../../../migrations/1107_tibiadrome_highscore_category_allocation.sql"
        ))
        .await?;
        tx.commit().await?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_world_highscore_snapshots WHERE snapshot_id=ANY($1)"
            )
            .bind(&tokens)
            .fetch_one(&h.pool)
            .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_world_highscore_rows WHERE snapshot_id=ANY($1)"
            )
            .bind(&tokens)
            .fetch_one(&h.pool)
            .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_world_highscore_heads WHERE category IN(13,14)"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_public_highscore_outbox WHERE category IN(13,14)"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        );
        let metadata_after: Vec<(i16,i64,i64)> = sqlx::query_as("SELECT category,projection_revision,revised_at FROM game_public_highscores WHERE world_id=encode($1,'hex')::uuid AND category IN(13,14) ORDER BY category")
            .bind(id(WORLD).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            metadata_after, metadata_before,
            "retired13 metadata and14 floor remain untouched"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT pg_get_functiondef('game_public_highscores_revision_guard()'::regprocedure)"
            )
            .fetch_one(&h.pool)
            .await?,
            guard_before
        );
        let security_after: Vec<ResyncSecurity> =
            sqlx::query_as(RESYNC_SECURITY).fetch_all(&h.pool).await?;
        assert_eq!(
            security_after, security_before,
            "resync owner/ACL/search_path/security mode unchanged"
        );
        assert_eq!(sqlx::query_scalar::<_,String>("SELECT pg_get_functiondef('game_public_projection_resync(text,boolean)'::regprocedure)").fetch_one(&h.pool).await?,wrapper_before,"0977 wrapper remains byte-identical");
        let expected_base = base_before
            .replace(
                "SELECT world_id, category FROM game_public_highscores",
                "SELECT world_id, category FROM game_public_highscores WHERE category <> 13",
            )
            .replace(
                " FROM game_world_highscore_snapshots",
                " FROM game_world_highscore_snapshots WHERE category <> 13",
            );
        assert_ne!(expected_base, base_before);
        assert_eq!(sqlx::query_scalar::<_,String>("SELECT pg_get_functiondef('game_public_projection_resync_before_world_totals(text,boolean)'::regprocedure)").fetch_one(&h.pool).await?,expected_base,"only two intended retired-category source predicates change");
        for category in [13_i32, 15, 16] {
            assert!(sqlx::query("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,0,'reserved',0)").bind(id(WORLD).as_slice()).bind(category).execute(&h.pool).await.is_err());
            assert!(sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category) VALUES(encode($1,'hex')::uuid,$2)").bind(id(WORLD).as_slice()).bind(category).execute(&h.pool).await.is_err());
            // 2801 later admits Loyalty 13; 1107's historical checks here still refuse it.
            assert_eq!(
                served_category(u8::try_from(category)?),
                matches!(category, 13 | 16)
            );
        }
        for category in [15_i16, 16] {
            assert!(sqlx::query("INSERT INTO game_public_highscores(world_id,category,projection_revision,revised_at) VALUES(encode($1,'hex')::uuid,$2,1,0)").bind(id(WORLD).as_slice()).bind(category).execute(&h.pool).await.is_err());
        }
        // The historical1107 checks above still exclude16 and 13. Restore the current
        // forward migrations' category bounds before exercising today's publisher.
        let current = include_str!("../../../migrations/2727_boss_difficulty_victories.sql");
        let bounds = current
            .split_once("-- Phosphorus Record category admission.")
            .ok_or("2727 category bounds")?
            .1;
        h.pool.execute(bounds).await?;
        let loyalty = include_str!("../../../migrations/2801_loyalty_highscore_category.sql");
        let bounds = loyalty
            .split_once("\nALTER TABLE game_world_highscore_snapshots\n")
            .ok_or("2801 category bounds")?
            .1;
        h.pool
            .execute(sqlx::AssertSqlSafe(format!(
                "ALTER TABLE game_world_highscore_snapshots\n{bounds}"
            )))
            .await?;
        assert!(matches!(
            h.root
                .read_highscore_page(world(), 14, None, 1, u64::try_from(tokens[3])?, asking())
                .await,
            Err(SnapshotError::SnapshotExpired)
        ));
        assert!(matches!(
            h.root
                .read_highscore_page(world(), 13, None, 1, u64::try_from(tokens[1])?, asking())
                .await,
            Err(SnapshotError::SnapshotExpired)
        ));
        for family in ["highscores", "all"] {
            sqlx::query("SELECT game_public_projection_resync($1,false)")
                .bind(family)
                .execute(&h.pool)
                .await?;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_public_highscore_outbox WHERE category=13"
                )
                .fetch_one(&h.pool)
                .await?,
                0,
                "{family} resync must not poison the publisher with retired13"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT projection_revision FROM game_public_highscores WHERE world_id=encode($1,'hex')::uuid AND category=13").bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?,metadata_before[0].1);
        }
        let revision14_after_resync:i64=sqlx::query_scalar("SELECT projection_revision FROM game_public_highscores WHERE world_id=encode($1,'hex')::uuid AND category=14").bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision14_after_resync, metadata_before[1].1 + 2);
        h.root
            .refresh_highscore_schedule(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        for category in [14, 17] {
            let page = h
                .root
                .read_highscore_page(world(), category, None, 1, 0, asking())
                .await
                .map_err(debug)?;
            assert!(!tokens.contains(&i64::try_from(page.snapshot.snapshot_id)?));
            assert!(
                !page.rows.is_empty(),
                "correct source category {category} publishes"
            );
        }
        // A published Loyalty 13 snapshot never reaches the public projection.
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_public_highscore_outbox WHERE category=13"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT projection_revision FROM game_public_highscores WHERE world_id=encode($1,'hex')::uuid AND category=13").bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?,metadata_before[0].1);
        let revision14:i64=sqlx::query_scalar("SELECT projection_revision FROM game_public_highscores WHERE world_id=encode($1,'hex')::uuid AND category=14").bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            revision14,
            revision14_after_resync + 1,
            "correct14 publication advances retained floor"
        );
        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_agg(to_jsonb(s) ORDER BY world_id,rotation,character_id) FROM game_tibiadrome_scores s").fetch_one(&h.pool).await?,scores_before);
        eprintln!("TIBIADROME_1107_CATEGORY_MIGRATION POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}
