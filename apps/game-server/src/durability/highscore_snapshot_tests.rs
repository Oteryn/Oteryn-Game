//! Real native tables/guards and Root projection APIs. Administrative source dataset rows
//! are committed fixture DATA, not gameplay writes or current authority for those Characters.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::bestiary_postgres_harness as harness;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence,
};
use harness::{CHARACTER, Harness, TestResult, WORLD, debug, id};
use sqlx::Executor;

struct Formula;
impl BuildFormula for Formula {
    fn required(&self, _: &str, _: usize, _: u16) -> Option<u64> {
        Some(1_000_000)
    }
    fn digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn content_revision(&self) -> &str {
        "content-1"
    }
}
fn world() -> WorldId {
    WorldId::decode(&id(WORLD)).unwrap()
}
fn asking() -> CharacterId {
    CharacterId::decode(&id(CHARACTER)).unwrap()
}
fn generated(index: u32) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&index.to_be_bytes());
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let admin = harness::configured_admin().ok_or("PostgreSQL routing absent; no qualification")?;
    harness::runtime()?.block_on(async {
        let h = Harness::create(admin, tag, true).await?;
        let server: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(server, "170006");
        eprintln!("HIGHSCORES-NATIVE-PG server_version_num={server} fixture={tag}");
        h.pool.execute(SNAPSHOT_SCHEMA_PROPOSAL).await?;
        // One genuinely session/Node/recovery-fenced native build writer creates the source
        // template; later bulk datasets are explicitly admin fixtures, never active actors.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = DurableBuildState::default();
        let after = DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).map_err(debug)?;
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                harness::fence(1)?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(60)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before,
                    after,
                    pruned_stance: None,
                },
                &Formula,
            )
            .await
            .map_err(debug)?;
        drop(authority);
        drop(seal);
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}

/// Seed actual owning tables with every deferred SQL guard enabled. This is not a
/// bootstrap/admission producer, nor proof that these synthetic Characters can log in.
/// The native build template remains unchanged; each source dataset clone has fresh ids,
/// native root revision/XP/build consistency, and actual same-xid build receipt provenance.
async fn dataset(
    h: &Harness,
    start: u32,
    count: u32,
    vocation: &str,
    xp: u64,
    w: WorldId,
) -> TestResult {
    let rows: Vec<_> = (start..start + count)
        .map(|n| {
            let mut letters = String::from("Rank ");
            let mut v = n;
            for _ in 0..7 {
                letters.push(char::from(b'A' + (v % 26) as u8));
                v /= 26;
            }
            serde_json::json!({"id":hex(&generated(n)),"occurrence":hex(&generated(n+1_000_000)),
            "name":letters,"world":hex(w.as_bytes()),"vocation":vocation,"xp":xp})
        })
        .collect();
    let data = serde_json::to_string(&rows)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT x.id::uuid,r.account_id,x.world::uuid,1,1,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,x.name FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,world TEXT,name TEXT) CROSS JOIN game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid")
        .bind(&data).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_progression_state SELECT (jsonb_populate_record(NULL::game_character_progression_state,to_jsonb(p)||jsonb_build_object('character_id',x.id,'character_revision',2,'total_experience',x.xp))).* FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,xp BIGINT) CROSS JOIN game_character_progression_state p WHERE p.character_id=encode($2,'hex')::uuid")
        .bind(&data).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_build_receipts SELECT (jsonb_populate_record(NULL::game_character_build_receipts,to_jsonb(b)||jsonb_build_object('character_id',x.id,'build_occurrence_id',x.occurrence,'experience_before',x.xp,'experience_after',x.xp,'vocation_after',x.vocation,'created_xact_id',pg_current_xact_id()::text))).* FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,occurrence TEXT,xp BIGINT,vocation TEXT) CROSS JOIN game_character_build_receipts b WHERE b.character_id=encode($2,'hex')::uuid")
        .bind(&data).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_build_state SELECT (jsonb_populate_record(NULL::game_character_build_state,to_jsonb(b)||jsonb_build_object('character_id',x.id,'last_build_occurrence_id',x.occurrence,'vocation',x.vocation))).* FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,occurrence TEXT,vocation TEXT) CROSS JOIN game_character_build_state b WHERE b.character_id=encode($2,'hex')::uuid")
        .bind(&data).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=2 WHERE character_id IN(SELECT id::uuid FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT))")
        .bind(&data).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn page(
    h: &Harness,
    category: u8,
    filter: Option<&str>,
    number: u32,
    token: u64,
) -> TestResult<SnapshotPage> {
    h.root
        .read_highscore_page(world(), category, filter, number, token, asking())
        .await
        .map_err(|e| debug(e).into())
}

#[test]
fn committed_native_sources_cover_nine_categories_without_source_writes() -> TestResult {
    run("hssource", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut skills = [(10, 0); 7];
        for (i, slot) in skills.iter_mut().enumerate() {
            *slot = (10 + u16::try_from(i)?, u64::try_from(i + 1)? * 11);
        }
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                harness::fence(2)?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(61)).map_err(debug)?,
                    cause: BuildCause::Training,
                    before: DurableBuildState::new("knight", (0, 0), [(10, 0); 7])
                        .map_err(debug)?,
                    after: DurableBuildState::new("knight", (7, 31), skills).map_err(debug)?,
                    pruned_stance: None,
                },
                &Formula,
            )
            .await
            .map_err(debug)?;
        drop(authority);
        drop(seal);
        // Native no-build seed means vocation none; it must not enter any supported list.
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,world_id,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Unchosen Hero' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(id(71).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        let revision = h.root_revision().await?;
        let ids = h
            .root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        assert_eq!(ids.len(), 9);
        assert!(ids.iter().all(|s| s.source_cut == ids[0].source_cut));
        for category in 1..=9 {
            let p = page(h, category, None, 1, 0).await?;
            assert_eq!(p.rows.len(), 1);
            assert_eq!(p.rows[0].level, 50);
            let expected = match category {
                1 => (1000, 0),
                2 => (7, 31),
                _ => {
                    let index = usize::from(category - 3);
                    (u64::from(skills[index].0), skills[index].1)
                }
            };
            assert_eq!(p.rows[0].value, expected);
            assert_eq!(p.own_row.as_ref(), p.rows.first());
            assert!(!p.has_more);
        }
        assert_eq!(h.root_revision().await?, revision);
        assert_eq!(h.count("game_character_build_receipts").await?, 2);
        assert_eq!(h.count("game_character_xp_receipts").await?, 0);
        Ok(())
    })
}

#[test]
fn pinned_pages_retain_exact_previous_then_expire_after_three_generations() -> TestResult {
    run("hspages", async |h| {
        dataset(h, 100, 60, "knight", 1000, world()).await?;
        let first = h
            .root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?[0]
            .snapshot_id;
        let a = page(h, 1, None, 1, first).await?;
        let b = page(h, 1, None, 2, first).await?;
        assert_eq!((a.rows.len(), b.rows.len()), (50, 11));
        assert!(a.has_more);
        assert!(!b.has_more);
        assert!(a.rows.iter().chain(&b.rows).all(|r| r.rank == 1));
        assert!(
            a.rows
                .iter()
                .chain(&b.rows)
                .map(|r| r.name.as_str())
                .is_sorted()
        );
        dataset(h, 200, 1, "paladin", 2000, world()).await?;
        let second = h
            .root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?[0]
            .snapshot_id;
        assert_eq!(page(h, 1, None, 1, first).await?, a);
        assert_eq!(page(h, 1, None, 2, first).await?, b);
        assert_eq!(page(h, 1, None, 1, 0).await?.snapshot.snapshot_id, second);
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .read_highscore_page(world(), 1, None, 1, first, asking())
                .await,
            Err(SnapshotError::SnapshotExpired)
        ));
        assert_eq!(
            page(h, 1, None, 1, second).await?.snapshot.snapshot_id,
            second
        );
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 18);
        assert!(page(h, 1, None, u32::MAX, second).await?.rows.is_empty());
        Ok(())
    })
}

#[test]
fn filtered_list_is_capped_after_filtering_and_keeps_global_rank_below_all_cap() -> TestResult {
    run("hsfilter", async |h| {
        dataset(h, 100, 1000, "elite_knight", 10_000, world()).await?;
        dataset(h, 2000, 1, "royal_paladin", 1, world()).await?;
        let started = Instant::now();
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        eprintln!(
            "HS-PUBLISH:1002eligible,ninecategories,sixlists,{:?}",
            started.elapsed()
        );
        let target = CharacterId::decode(&generated(2000)).map_err(debug)?;
        let p = h
            .root
            .read_highscore_page(world(), 1, Some("paladin"), 1, 0, target)
            .await
            .map_err(debug)?;
        assert_eq!(p.rows.len(), 1);
        assert_eq!(p.rows[0].rank, 1002);
        assert_eq!(p.rows[0].vocation, "royal_paladin");
        assert_eq!(p.own_row.as_ref(), p.rows.first());
        let all = h
            .root
            .read_highscore_page(world(), 1, None, 20, 0, target)
            .await
            .map_err(debug)?;
        assert_eq!(all.rows.len(), 50);
        assert!(!all.has_more);
        assert!(all.own_row.is_none());
        assert!(page(h, 1, None, 21, 0).await?.rows.is_empty());
        assert!(page(h, 1, Some("knight"), 20, 0).await?.rows.len() == 50);
        assert_eq!(h.root_revision().await?, "2");
        Ok(())
    })
}

#[test]
fn failed_mid_generation_publication_preserves_previous_headers_rows_and_source() -> TestResult {
    run("hsrollback", async |h| {
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        let before = page(h, 1, None, 1, 0).await?;
        let count = h.count("game_world_highscore_rows").await?;
        for (query, message) in [
            (
                "UPDATE game_world_highscore_rows SET name='Changed'",
                "immutable highscore projection row",
            ),
            (
                "DELETE FROM game_world_highscore_rows",
                "snapshot rows only delete with their header",
            ),
            (
                "INSERT INTO game_world_highscore_rows SELECT snapshot_id,filter,position+1,rank,'00000000-0000-7000-8000-000000000002'::uuid,name,vocation,level,value_primary,value_progress FROM game_world_highscore_rows LIMIT 1",
                "published snapshot cannot acquire rows",
            ),
        ] {
            let error = h
                .pool
                .execute(query)
                .await
                .expect_err("guard must reject mutation");
            let database = error.as_database_error().expect("database guard error");
            assert_eq!(database.code().as_deref(), Some("23514"));
            assert_eq!(database.message(), message);
            assert_eq!(page(h, 1, None, 1, 0).await?, before);
        }
        h.pool.execute("CREATE FUNCTION hs_fixture_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF EXISTS(SELECT 1 FROM game_world_highscore_snapshots WHERE snapshot_id=NEW.snapshot_id AND category=5) THEN RAISE EXCEPTION 'injected snapshot failure'; END IF;RETURN NEW;END;$$;CREATE TRIGGER hs_fixture_fail BEFORE INSERT ON game_world_highscore_rows FOR EACH ROW EXECUTE FUNCTION hs_fixture_fail();").await?;
        assert!(h.root.publish_highscore_world(world()).await.is_err());
        assert_eq!(page(h, 1, None, 1, 0).await?, before);
        assert_eq!(h.count("game_world_highscore_rows").await?, count);
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 9);
        assert_eq!(h.root_revision().await?, "2");
        h.pool.execute("DROP TRIGGER hs_fixture_fail ON game_world_highscore_rows;DROP FUNCTION hs_fixture_fail();").await?;
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 18);
        Ok(())
    })
}

#[test]
fn repeatable_read_source_cut_and_stale_publication_conflict_are_real_postgres() -> TestResult {
    run("hscut", async |h| {
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        let mut conn = h.pool.acquire().await?;
        let mut tx = begin_projection(
            &mut conn,
            Instant::now() + std::time::Duration::from_secs(2),
        )
        .await?;
        assert_eq!(read_source(&mut tx, world()).await.map_err(debug)?.len(), 1);
        dataset(h, 100, 1, "paladin", 1000, world()).await?;
        assert_eq!(read_source(&mut tx, world()).await.map_err(debug)?.len(), 1);
        tx.rollback().await?;
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        assert_eq!(page(h, 1, None, 1, 0).await?.rows.len(), 2);
        let mut stale = begin_projection(
            &mut conn,
            Instant::now() + std::time::Duration::from_secs(2),
        )
        .await?;
        let winner = h
            .root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        assert!(matches!(
            publish_in_transaction(&mut stale, world()).await,
            Err(SnapshotError::RetryableConflict)
        ));
        stale.rollback().await?;
        assert_eq!(
            page(h, 1, None, 1, 0).await?.snapshot.snapshot_id,
            winner[0].snapshot_id
        );
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 18);
        Ok(())
    })
}

#[test]
fn world_category_token_isolation_empty_world_and_query_boundaries() -> TestResult {
    run("hsquery", async |h| {
        let token = h
            .root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?[0]
            .snapshot_id;
        let foreign = WorldId::decode(&id(90)).map_err(debug)?;
        h.root
            .publish_highscore_world(foreign)
            .await
            .map_err(debug)?;
        let empty = h
            .root
            .read_highscore_page(foreign, 1, None, 1, 0, asking())
            .await
            .map_err(debug)?;
        assert!(empty.rows.is_empty());
        assert!(empty.own_row.is_none());
        assert!(!empty.has_more);
        for (w, category, snapshot) in [
            (foreign, 1, token),
            (world(), 2, token),
            (world(), 1, u64::MAX),
        ] {
            assert!(matches!(
                h.root
                    .read_highscore_page(w, category, None, 1, snapshot, asking())
                    .await,
                Err(SnapshotError::SnapshotExpired)
            ));
        }
        for category in [0, 12, 13, 255] {
            assert!(matches!(
                h.root
                    .read_highscore_page(world(), category, None, 1, 0, asking())
                    .await,
                Err(SnapshotError::NotSupported)
            ));
        }
        assert!(matches!(
            h.root
                .read_highscore_page(world(), 1, None, 0, 0, asking())
                .await,
            Err(SnapshotError::InvalidQuery)
        ));
        assert!(matches!(
            h.root
                .read_highscore_page(world(), 1, Some("elite_knight"), 1, 0, asking())
                .await,
            Err(SnapshotError::InvalidQuery)
        ));
        assert_eq!(page(h, 1, None, 1, token).await?.rows.len(), 1);
        Ok(())
    })
}

#[test]
fn two_real_roots_publish_one_coherent_generation_or_typed_retry_without_lost_retention()
-> TestResult {
    run("hsconcurrent", async |h| {
        h.root
            .publish_highscore_world(world())
            .await
            .map_err(debug)?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let mut first = std::pin::pin!(h.root.publish_highscore_world(world()));
        let mut second = std::pin::pin!(other.publish_highscore_world(world()));
        let mut a = None;
        let mut b = None;
        std::future::poll_fn(|cx| {
            if a.is_none()
                && let std::task::Poll::Ready(v) = std::future::Future::poll(first.as_mut(), cx)
            {
                a = Some(v);
            }
            if b.is_none()
                && let std::task::Poll::Ready(v) = std::future::Future::poll(second.as_mut(), cx)
            {
                b = Some(v);
            }
            if a.is_some() && b.is_some() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        let results = [a.unwrap(), b.unwrap()];
        assert!(results.iter().any(|r| r.is_ok()));
        assert!(
            results
                .iter()
                .all(|r| matches!(r, Ok(_) | Err(SnapshotError::RetryableConflict)))
        );
        let current: Vec<i64> = sqlx::query_scalar(
            "SELECT current_snapshot_id FROM game_world_highscore_heads ORDER BY category",
        )
        .fetch_all(&h.pool)
        .await?;
        assert!(
            results
                .iter()
                .filter_map(|r| r.as_ref().ok())
                .any(|generation| generation
                    .iter()
                    .map(|s| s.snapshot_id)
                    .eq(current.iter().map(|id| id.unsigned_abs())))
        );
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 18);
        assert_eq!(h.root_revision().await?, "2");
        Ok(())
    })
}

fn complete_catalogues() -> TestResult<SnapshotCatalogues> {
    SnapshotCatalogues::new(
        AchievementCatalogue::embedded().map_err(debug)?,
        [BestiaryCharmEntry {
            race: crate::domain::bestiary::BestiaryRace::new(
                "oteryn:creature.rat",
                "definition-r1",
                vec![1, 2, 3],
            )
            .map_err(debug)?,
            charm_points: 25,
        }],
    )
    .map_err(|e| debug(e).into())
}

#[test]
fn account_representatives_and_earned_charm_points_use_native_sources() -> TestResult {
    run("hsextra", async |h| {
        dataset(h, 100, 2, "paladin", 1000, world()).await?;
        // Genuine append-only owner tables and their deferred request/fact guard, with no
        // projection-fed account points. Every fixture Character belongs to the same account.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_account_achievement_grant_requests(source_kind,source_event_id,achievement_key,achievement_revision,account_id,character_id,earned_at) SELECT 'highscore-fixture',$1,'oteryn:achievement/a_friend_in_need','1',account_id,character_id,1 FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(vec![1_u8]).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_achievements(account_id,achievement_key,source_kind,source_event_id) SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests")
            .execute(&mut *tx).await?;
        tx.commit().await?;
        // Three real current-fenced kill commits, completing the owning Bestiary definition.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for index in 0..3_u8 {
            h.root.commit_bestiary_kill(
                &authority, &h.node, harness::fence(u64::from(index) + 2)?,
                crate::durability::bestiary_progress::BestiaryKillRequest {
                    occurrence: crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(80 + index)).map_err(debug)?,
                    race: complete_catalogues()?.bestiary.values().next().ok_or("missing race")?.race.clone(),
                    context: harness::context(), policy_revision: "policy-1".into(), reward_revision: "reward-1".into(),
                },
            ).await.map_err(debug)?;
        }
        drop(authority);
        drop(seal);
        let ids = h
            .root
            .refresh_highscore_world(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        assert_eq!(ids.len(), 11);
        assert!(ids.iter().all(|s| s.source_cut == ids[0].source_cut));
        let accounts = page(h, 10, None, 1, 0).await?;
        assert_eq!(accounts.rows.len(), 1);
        assert_eq!(accounts.rows[0].character_id.as_bytes(), &generated(100));
        assert_eq!(accounts.rows[0].value, (2, 0));
        // The account's representative is selected before a vocation filter; the asking
        // knight does not become a second representative of the same account.
        assert!(page(h, 10, Some("knight"), 1, 0).await?.rows.is_empty());
        assert!(accounts.own_row.is_none());
        let charms = page(h, 11, None, 1, 0).await?;
        assert_eq!(charms.rows.len(), 3);
        assert_eq!(charms.rows[0].character_id, asking());
        assert_eq!(charms.rows[0].value, (25, 0));
        assert!(charms.rows[1..].iter().all(|r| r.value == (0, 0)));
        assert!(page(h, 12, None, 1, 0).await.is_err());
        Ok(())
    })
}

#[test]
fn periodic_due_state_survives_restart_and_reset_forces_publication() -> TestResult {
    run("hsdue", async |h| {
        let first = h
            .root
            .refresh_highscore_world(world(), complete_catalogues()?, false)
            .await
            .map_err(debug)?;
        assert_eq!(first.len(), 11);
        // A separately connected genuine Root supplies no process-local schedule cache.
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let job = job::HighscoreWorldJob::new(
            std::sync::Arc::new(other),
            world(),
            complete_catalogues()?,
        );
        let next = job.periodic_tick().await.map_err(debug)?;
        assert!(next.is_empty());
        let reset = job.after_planned_reset().await.map_err(debug)?;
        assert_eq!(reset.len(), 11);
        assert!(
            reset
                .iter()
                .zip(&first)
                .all(|(a, b)| a.snapshot_id > b.snapshot_id)
        );
        assert_eq!(
            page(h, 10, None, 1, first[9].snapshot_id)
                .await?
                .snapshot
                .snapshot_id,
            first[9].snapshot_id
        );
        Ok(())
    })
}

#[test]
fn public_outbox_attachment_commits_and_rolls_back_with_all_categories() -> TestResult {
    // This branch predates 0091. The CP executor must first stage that exact owning migration
    // from protected main; missing setup fails explicitly instead of skipping this case.
    let migration_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations/0091_public_projections.sql");
    let migration = std::fs::read_to_string(&migration_path).map_err(|error| {
        format!(
            "required owning 0091 public producer migration absent at {}: {error}",
            migration_path.display()
        )
    })?;
    run("hsoutbox", async |h| {
        // The native harness may already apply 0091 after the slot merges main.
        let installed: bool = sqlx::query_scalar(
            "SELECT to_regprocedure('game_public_highscores_snapshot_trigger()') IS NOT NULL",
        )
        .fetch_one(&h.pool)
        .await?;
        if !installed {
            h.pool.execute(migration.as_str()).await?;
        }
        h.pool
            .execute(PUBLIC_PROJECTION_ATTACHMENT_PROPOSAL)
            .await?;
        h.root
            .refresh_highscore_world(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        assert_eq!(h.count("game_public_highscore_outbox").await?, 11);
        h.pool.execute("CREATE FUNCTION highscore_test_fail_category() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NEW.category=2 THEN RAISE EXCEPTION 'fixture publication failure'; END IF; RETURN NEW; END;$$; CREATE TRIGGER highscore_test_fail_category BEFORE INSERT ON game_world_highscore_snapshots FOR EACH ROW EXECUTE FUNCTION highscore_test_fail_category();").await?;
        assert!(
            h.root
                .refresh_highscore_world(world(), complete_catalogues()?, true)
                .await
                .is_err()
        );
        assert_eq!(h.count("game_public_highscore_outbox").await?, 11);
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
        Ok(())
    })
}
