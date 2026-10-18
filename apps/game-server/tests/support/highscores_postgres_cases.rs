//! RANK-1 HS-1 (migration 0097): real native tables/guards and Root projection APIs, run in the
//! protected PostgreSQL lane on the CHARM-2 harness. Administrative source dataset rows are
//! committed fixture DATA, not gameplay writes or current authority for those Characters.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::bestiary_postgres_harness as harness;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::account_achievement::EarnedAchievement;
use crate::durability::character_authority::CharacterAuthorityError;
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::charm_state::{BestiaryCharmEntry, BestiaryCharmFacts};
use crate::durability::highscore_snapshot::reply::{HighscoresReply, answer_highscores};
use crate::durability::highscore_snapshot::{
    AchievementPoints, SnapshotCatalogues, SnapshotError, SnapshotPage, activated_charm_points,
    begin_projection, job, publish_in_transaction, read_source, served_categories,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{CharacterId, FoundationProtocolError, WorldId};
use harness::{CHARACTER, Harness, TestResult, WORLD, debug, id};
use oteryn_protocol_oteryn::highscores::{
    HighscoresQuery, HighscoresResult, decode_highscores_query, decode_highscores_result,
    encode_highscores_query,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::Executor;
use std::time::Instant;

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
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async {
        let h = Harness::create(admin, tag, true).await?;
        let server: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(server, "170006");
        eprintln!("HIGHSCORES-NATIVE-PG server_version_num={server} fixture={tag}");
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
        let token = p.snapshot.snapshot_id;
        assert_eq!(p.rows.len(), 1);
        assert_eq!(p.rows[0].rank, 1002);
        assert_eq!(p.rows[0].vocation, "royal_paladin");
        assert_eq!(p.own_row.as_ref(), p.rows.first());
        let all = h
            .root
            .read_highscore_page(world(), 1, None, 20, token, target)
            .await
            .map_err(debug)?;
        assert_eq!(all.rows.len(), 50);
        assert!(!all.has_more);
        assert!(all.own_row.is_none());
        assert!(page(h, 1, None, 21, token).await?.rows.is_empty());
        assert!(page(h, 1, Some("knight"), 20, token).await?.rows.len() == 50);
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
                .read_highscore_page(world(), 1, None, 2, 0, asking())
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

/// The achievement point owner of the fixture: one known key worth two points; any other key is
/// unknown to it, as a retired catalogue's missing entry is to the real one.
struct FixtureAchievementPoints;
impl AchievementPoints for FixtureAchievementPoints {
    fn total_points(&self, earned: &[EarnedAchievement]) -> Option<u32> {
        earned.iter().try_fold(0_u32, |sum, fact| {
            (fact.achievement_key == "oteryn:achievement/a_friend_in_need")
                .then_some(2)
                .and_then(|points| sum.checked_add(points))
        })
    }
}

pub(crate) fn complete_catalogues() -> TestResult<SnapshotCatalogues> {
    SnapshotCatalogues::new(
        std::sync::Arc::new(FixtureAchievementPoints),
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
        use crate::domain::charm::{
            CharmCatalogue, CharmCategory, CharmCurrency, CharmDefinition, CharmKey,
        };
        use crate::durability::charm_state::{
            CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmProgressionReadRequest,
        };
        let charm = CharmKey::new("oteryn:charm.wound").map_err(debug)?;
        let catalogue = CharmCatalogue::new([CharmDefinition {
            key: charm.clone(),
            category: CharmCategory::Major,
            stage_costs: [5, 5, 5],
        }])
        .map_err(debug)?;
        let outcome = h
            .root
            .commit_charm_command(
                &authority,
                &h.node,
                harness::fence(5)?,
                CharmCommandRequest {
                    occurrence: CharmCommandOccurrence::from_bytes(id(85)).map_err(debug)?,
                    command: CharmCommand::UnlockNextStage { charm },
                    catalogue_revision: "content-1".into(),
                    catalogue: catalogue.clone(),
                },
                BestiaryCharmFacts::new(complete_catalogues()?.bestiary.into_values())
                    .map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            outcome,
            crate::durability::charm_state::CharmCommandOutcome::Committed(
                crate::durability::charm_state::CommittedCharmCommand {
                    effect: crate::durability::charm_state::CharmCommandEffect::Unlocked {
                        cost: 5,
                        ..
                    },
                    ..
                }
            )
        ));
        let progression = h
            .root
            .read_character_charm_progression(
                &authority,
                &h.node,
                harness::fence(6)?,
                CharmProgressionReadRequest {
                    catalogue_revision: "content-1".into(),
                    catalogue,
                    races: Vec::new(),
                },
                BestiaryCharmFacts::new(complete_catalogues()?.bestiary.into_values())
                    .map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(progression.balance.points_earned, 25);
        assert_eq!(progression.balance.points_spent, 5);
        assert_eq!(
            progression.balance.available(CharmCurrency::CharmPoints),
            20
        );
        // Five points were durably spent, but the highscore below still reports 25 earned.
        assert_eq!(h.count("game_character_charm_unlocks").await?, 1);
        drop(authority);
        drop(seal);
        assert_eq!(h.root_revision().await?, "6");
        assert_eq!(h.count("game_character_bestiary_kill_receipts").await?, 3);
        // Reconciliation checks every root's creation provenance. Complete the native
        // gameplay writes before adding admin-only ranking rows, which lack that provenance.
        dataset(h, 100, 2, "paladin", 1000, world()).await?;
        // Genuine append-only owner tables and their deferred request/fact guard, with no
        // projection-fed account points. Every fixture Character belongs to the same account.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_account_achievement_grant_requests(source_kind,source_event_id,achievement_key,achievement_revision,account_id,character_id,earned_at) SELECT 'highscore-fixture',$1,'oteryn:achievement/a_friend_in_need','1',account_id,character_id,1 FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(vec![1_u8]).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_achievements(account_id,achievement_key,source_kind,source_event_id) SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests")
            .execute(&mut *tx).await?;
        tx.commit().await?;
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
        // Projection readability does not make the synthetic roots eligible for authority.
        // Preserve the exact guard that rejected the original fixture ordering.
        let seal = h.recovery.seal_current().map_err(debug)?;
        assert!(matches!(
            h.root.open_character_authority(&seal).await,
            Err(CharacterAuthorityError::Unavailable(
                DurabilityError::Unavailable
            ))
        ));
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
        let next = other
            .refresh_highscore_world(world(), complete_catalogues()?, false)
            .await
            .map_err(debug)?;
        assert!(next.is_empty());
        let reset = other
            .refresh_highscore_world(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
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
    // Migration 0097 attaches 0091's snapshot trigger; no fixture SQL installs it.
    run("hsoutbox", async |h| {
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

#[test]
fn eleven_category_publishers_keep_one_complete_generation_and_retention() -> TestResult {
    run("hsconcurrentfull", async |h| {
        h.root
            .refresh_highscore_world(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let mut first = std::pin::pin!(h.root.refresh_highscore_world(
            world(),
            complete_catalogues()?,
            true
        ));
        let mut second =
            std::pin::pin!(other.refresh_highscore_world(world(), complete_catalogues()?, true));
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
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 22);
        assert_eq!(h.root_revision().await?, "2");
        Ok(())
    })
}

#[test]
fn account_representatives_include_vocation_none_and_choose_level_before_identity() -> TestResult {
    run("hsrepresentatives", async |h| {
        // Administrative committed native source DATA with all native guards enabled. None
        // means an absent build row; each initialized XP row remains at root revision one.
        let rows = serde_json::to_string(&serde_json::json!([
            {"id":hex(&generated(200)),"account":hex(&id(40)),"level":60,"name":"Higher Hero"},
            {"id":hex(&generated(201)),"account":hex(&generated(400)),"level":70,"name":"Other Hero"},
            {"id":hex(&generated(199)),"account":hex(&id(40)),"level":55,"name":"Lower Hero"}
        ]))?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
            .bind(generated(400).as_slice())
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT x.id::uuid,x.account::uuid,r.world_id,1,1,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,x.name FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,account TEXT,name TEXT) CROSS JOIN game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid")
            .bind(&rows).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_progression_state SELECT (jsonb_populate_record(NULL::game_character_progression_state,to_jsonb(p)||jsonb_build_object('character_id',x.id,'character_revision',1,'level',x.level))).* FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,level BIGINT) CROSS JOIN game_character_progression_state p WHERE p.character_id=encode($2,'hex')::uuid")
            .bind(&rows).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        // Migration 0009 keeps a bootstrap-only Character (revision one, no progression row)
        // valid. One shares the account of the initialized Characters and sorts first by
        // identity; another is its account's only Character. Neither has a level to rank.
        let bootstrap = serde_json::to_string(&serde_json::json!([
            {"id":hex(&generated(198)),"account":hex(&id(40)),"name":"Bootstrap Hero"},
            {"id":hex(&generated(202)),"account":hex(&generated(401)),"name":"Bootstrap Only"}
        ]))?;
        sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
            .bind(generated(401).as_slice())
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT x.id::uuid,x.account::uuid,r.world_id,1,1,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,x.name FROM jsonb_to_recordset($1::jsonb) AS x(id TEXT,account TEXT,name TEXT) CROSS JOIN game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid")
            .bind(&bootstrap).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let published = h
            .root
            .refresh_highscore_world(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        assert_eq!(published.len(), 11);
        let accounts = page(h, 10, None, 1, 0).await?;
        // The highest initialized Character represents its account, the bootstrap-only
        // account has no representative, and the whole cut published.
        assert_eq!(accounts.rows.len(), 2);
        assert!(accounts.rows.iter().all(|row| row.vocation == "none"));
        assert!(
            accounts
                .rows
                .iter()
                .any(|row| row.character_id.as_bytes() == &generated(200) && row.level == 60)
        );
        assert!(
            accounts
                .rows
                .iter()
                .any(|row| row.character_id.as_bytes() == &generated(201) && row.level == 70)
        );
        assert!(
            accounts
                .rows
                .iter()
                .all(|row| ![generated(198), generated(202)].contains(row.character_id.as_bytes()))
        );
        assert!(page(h, 10, Some("knight"), 1, 0).await?.rows.is_empty());
        assert_eq!(page(h, 1, None, 1, 0).await?.rows.len(), 1);
        assert_eq!(page(h, 11, None, 1, 0).await?.rows.len(), 1);
        Ok(())
    })
}

/// Seed valid retained derived headers with a short remaining interval. This fixture changes
/// no source, bypasses no trigger, and supplies no current gameplay authority.
async fn seed_near_due_headers(h: &Harness, remaining_ms: i64) -> TestResult {
    sqlx::query("WITH inserted AS (INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) SELECT encode($1,'hex')::uuid,category,floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1800000+$2,pg_current_snapshot()::text,0 FROM generate_series(1,11) category RETURNING world_id,category,snapshot_id) INSERT INTO game_world_highscore_heads(world_id,category,current_snapshot_id) SELECT world_id,category,snapshot_id FROM inserted")
        .bind(world().as_bytes().as_slice()).bind(remaining_ms).execute(&h.pool).await?;
    Ok(())
}

async fn drive_real_job(
    job: &job::HighscoreWorldJob,
    resets: tokio::sync::mpsc::Receiver<()>,
    observer: impl std::future::Future<Output = TestResult>,
) -> TestResult {
    let mut runner = std::pin::pin!(job.run(resets));
    let mut observer = std::pin::pin!(observer);
    std::future::poll_fn(|cx| {
        if std::future::Future::poll(runner.as_mut(), cx).is_ready() {
            return std::task::Poll::Ready(Err("job ended before observer completed".into()));
        }
        std::future::Future::poll(observer.as_mut(), cx)
    })
    .await
}

async fn wait_for_passes(job: &job::HighscoreWorldJob, count: u64) {
    while job.completed_passes() < count {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[test]
fn actual_job_restart_preserves_durable_deadline_after_a_failed_reset() -> TestResult {
    run("hsjobdeadline", async |h| {
        let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(root.maintain_ready_once().await?);
        seed_near_due_headers(h, 3000).await?;
        let job =
            job::HighscoreWorldJob::new(std::sync::Arc::new(root), world(), complete_catalogues()?);
        let (sender, resets) = tokio::sync::mpsc::channel(1);
        let observer = async {
            wait_for_passes(&job, 1).await;
            // The immediate restart check truly skipped the not-yet-due retained cut.
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
            // The job writes as its own role (SET LOCAL ROLE): the fixture grants it the counter.
            h.pool.execute("CREATE SEQUENCE highscore_job_failure_seen; GRANT USAGE ON SEQUENCE highscore_job_failure_seen TO oteryn_game_highscore_job; CREATE FUNCTION highscore_job_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN PERFORM nextval('highscore_job_failure_seen'); RAISE EXCEPTION 'fixture reset failure'; END;$$; CREATE TRIGGER highscore_job_fail BEFORE INSERT ON game_world_highscore_snapshots FOR EACH ROW EXECUTE FUNCTION highscore_job_fail();").await?;
            sender.send(()).await?;
            wait_for_passes(&job, 2).await;
            let failed: bool =
                sqlx::query_scalar("SELECT is_called FROM highscore_job_failure_seen")
                    .fetch_one(&h.pool)
                    .await?;
            assert!(failed);
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
            h.pool
                .execute("DROP TRIGGER highscore_job_fail ON game_world_highscore_snapshots")
                .await?;
            while h.count("game_world_highscore_snapshots").await? != 22 {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            assert!(job.completed_passes() >= 3);
            // Uses the actual run-loop timer. The old process-relative +30min scheduler
            // times out here; neither the skipped restart nor the reset error may postpone it.
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 22);
            let ids: Vec<i64> = sqlx::query_scalar(
                "SELECT current_snapshot_id FROM game_world_highscore_heads ORDER BY category",
            )
            .fetch_all(&h.pool)
            .await?;
            assert!(ids.iter().all(|id| *id > 11));
            drop(sender);
            Ok(())
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(12),
            drive_real_job(&job, resets, observer),
        )
        .await??;
        Ok(())
    })
}

#[test]
fn actual_job_periodic_failure_keeps_prior_cut_and_backs_off_without_busy_loop() -> TestResult {
    run("hsjobfailure", async |h| {
        seed_near_due_headers(h, -1).await?;
        h.pool.execute("CREATE FUNCTION highscore_job_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'fixture periodic failure'; END;$$; CREATE TRIGGER highscore_job_fail BEFORE INSERT ON game_world_highscore_snapshots FOR EACH ROW EXECUTE FUNCTION highscore_job_fail();").await?;
        let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(root.maintain_ready_once().await?);
        let job =
            job::HighscoreWorldJob::new(std::sync::Arc::new(root), world(), complete_catalogues()?);
        let (sender, resets) = tokio::sync::mpsc::channel(1);
        let observer = async {
            wait_for_passes(&job, 1).await;
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
            h.pool
                .execute("DROP TRIGGER highscore_job_fail ON game_world_highscore_snapshots")
                .await?;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            assert_eq!(job.completed_passes(), 1);
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
            drop(sender);
            Ok(())
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            drive_real_job(&job, resets, observer),
        )
        .await??;
        Ok(())
    })
}

#[test]
fn missing_achievement_catalogue_key_keeps_complete_previous_cut() -> TestResult {
    run("hssourcefailure", async |h| {
        let before = h
            .root
            .refresh_highscore_world(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        // Administrative stored-data corruption fixture: native append-only SQL guards still
        // apply, but this account fact's key is absent from the active owning catalogue.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_account_achievement_grant_requests(source_kind,source_event_id,achievement_key,achievement_revision,account_id,character_id,earned_at) SELECT 'highscore-fixture',$1,'oteryn:achievement/highscore_missing','1',account_id,character_id,1 FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(vec![2_u8]).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_achievements(account_id,achievement_key,source_kind,source_event_id) SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests")
            .execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(matches!(
            h.root
                .refresh_highscore_world(world(), complete_catalogues()?, true)
                .await,
            Err(SnapshotError::Unavailable(
                DurabilityError::InvalidStoredState
            ))
        ));
        assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
        for snapshot in before {
            assert_eq!(
                page(h, snapshot.category, None, 1, 0)
                    .await?
                    .snapshot
                    .snapshot_id,
                snapshot.snapshot_id
            );
        }
        assert_eq!(h.root_revision().await?, "2");
        Ok(())
    })
}

fn xp_award(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
    let context = harness::context();
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag)).map_err(debug)?,
        amount: ExactI64::new(5),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

/// The client's command type 14 round trip: the encoded query is decoded exactly as the
/// gameplay transport decodes it and answered by the handler it dispatches to.
async fn client_reply(
    h: &Harness,
    category: u32,
    vocation: u32,
    page: u32,
    snapshot_id: u64,
) -> TestResult<HighscoresReply> {
    let payload = encode_highscores_query(HighscoresQuery {
        category,
        vocation,
        page,
        snapshot_id,
    })
    .map_err(debug)?;
    let query = decode_highscores_query(&payload).map_err(debug)?;
    Ok(answer_highscores(&h.root, world(), asking(), query).await)
}

async fn client_query(
    h: &Harness,
    vocation: u32,
    page: u32,
    snapshot_id: u64,
) -> TestResult<HighscoresResult> {
    match client_reply(h, 1, vocation, page, snapshot_id).await? {
        HighscoresReply::Result(result) => {
            decode_highscores_result(&result).map_err(|e| debug(e).into())
        }
        other => Err(format!("highscores query was not accepted: {other:?}").into()),
    }
}

fn standing(result: &HighscoresResult) -> Vec<(u64, u64)> {
    result
        .rows
        .iter()
        .map(|row| (row.rank, row.value))
        .collect()
}

/// RANK-1 end to end: the World's actual snapshot job publishes on its immediate check, the
/// asking Character gains experience through the real writer and other players join, and the
/// client's highscore request returns the ordered persisted page with its own row once the job
/// publishes again on a planned reset.
#[test]
fn experience_gain_reaches_the_client_through_the_snapshot_job() -> TestResult {
    run("hsendtoend", async |h| {
        let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(root.maintain_ready_once().await?);
        let job =
            job::HighscoreWorldJob::new(std::sync::Arc::new(root), world(), complete_catalogues()?);
        let (planned_resets, resets) = tokio::sync::mpsc::channel(1);
        let observer = async {
            wait_for_passes(&job, 1).await;
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 11);
            let first = client_query(h, 0, 1, 0).await?;
            assert_eq!(standing(&first), [(1, 1000)]);
            let own = first.own_row.clone().ok_or("own row missing")?;
            assert_eq!(
                (own.rank, own.name.as_str(), own.value),
                (1, "Fixture Hero", 1000)
            );
            assert!(!first.has_more);

            // The asking Character gains experience through the session-fenced native writer.
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let award = h
                .root
                .commit_character_experience(&authority, &h.node, harness::fence(2)?, xp_award(80)?)
                .await
                .map_err(debug)?;
            assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));
            drop(authority);
            drop(seal);
            // Admin-only ranking rows lack creation provenance, so they follow every native
            // gameplay write (see `account_representatives_and_earned_charm_points_...`).
            dataset(h, 1, 3, "knight", 1003, world()).await?;
            dataset(h, 4, 2, "sorcerer", 900, world()).await?;
            // The persisted snapshot is a cut: the changes are invisible until the job publishes.
            assert_eq!(standing(&client_query(h, 0, 1, 0).await?), standing(&first));

            planned_resets.send(()).await?;
            wait_for_passes(&job, 2).await;
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 22);
            let second = client_query(h, 0, 1, 0).await?;
            assert_ne!(second.snapshot_id, first.snapshot_id);
            assert_eq!(
                standing(&second),
                [
                    (1, 1005),
                    (2, 1003),
                    (2, 1003),
                    (2, 1003),
                    (5, 900),
                    (5, 900)
                ]
            );
            assert_eq!(second.rows[0].name, "Fixture Hero");
            assert_eq!(
                second.own_row.map(|row| (row.rank, row.value)),
                Some((1, 1005))
            );
            // The previous token stays answerable; a filter keeps the global ranks.
            assert_eq!(
                standing(&client_query(h, 0, 1, first.snapshot_id).await?),
                standing(&first)
            );
            let sorcerers = client_query(h, 3, 1, 0).await?;
            assert_eq!(standing(&sorcerers), [(5, 900), (5, 900)]);
            assert!(sorcerers.own_row.is_none());
            drop(planned_resets);
            Ok(())
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            drive_real_job(&job, resets, observer),
        )
        .await??;
        Ok(())
    })
}

/// HIGHSCORES0 §3: with no activated per-race Charm point source (CHARM-6) the serve job's
/// catalogues publish categories 1 to 10 only, and the client's category 11 query through the
/// dispatched handler is `NOT_SUPPORTED` rather than a ranking of zero points.
#[test]
fn category_eleven_is_withheld_without_an_activated_charm_point_source() -> TestResult {
    run("hscharmsource", async |h| {
        assert!(activated_charm_points().is_none());
        assert_eq!(served_categories(), 10);
        let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(root.maintain_ready_once().await?);
        let catalogues =
            SnapshotCatalogues::activated(std::sync::Arc::new(FixtureAchievementPoints))
                .map_err(debug)?;
        let job = job::HighscoreWorldJob::new(std::sync::Arc::new(root), world(), catalogues);
        let (planned_resets, resets) = tokio::sync::mpsc::channel(1);
        let observer = async {
            wait_for_passes(&job, 1).await;
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 10);
            let charm_heads: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_world_highscore_heads WHERE category=11",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(charm_heads, 0);
            assert_eq!(
                client_reply(h, 11, 0, 1, 0).await?,
                HighscoresReply::Terminal(FoundationProtocolError::NotSupported)
            );
            let HighscoresReply::Result(accounts) = client_reply(h, 10, 0, 1, 0).await? else {
                return Err("category 10 query was not accepted".into());
            };
            assert_eq!(
                decode_highscores_result(&accounts).map_err(debug)?.category,
                10
            );
            // A planned reset publishes the same ten categories again.
            planned_resets.send(()).await?;
            wait_for_passes(&job, 2).await;
            assert_eq!(h.count("game_world_highscore_snapshots").await?, 20);
            drop(planned_resets);
            Ok(())
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            drive_real_job(&job, resets, observer),
        )
        .await??;
        Ok(())
    })
}

/// Migration 0097's guard on the cluster-global job group, as the migration runs it.
fn job_role_guard() -> &'static str {
    const MIGRATION: &str = include_str!("../../migrations/0097_world_highscores.sql");
    let start = MIGRATION
        .find("DO $$")
        .expect("0097 opens with its role guard");
    let end = start
        + MIGRATION[start..]
            .find("END $$;")
            .expect("the guard block ends")
        + 7;
    &MIGRATION[start..end]
}

#[test]
fn job_role_guard_refuses_a_pre_existing_group_with_any_membership() -> TestResult {
    run("hsjobmembership", async |h| {
        let guard = job_role_guard();
        assert!(guard.contains("pg_auth_members"));
        // The provisioned group is a plain NOLOGIN group that is a member of no role.
        h.pool.execute(guard).await?;
        let probe = format!("bp_hsjobmember_{}", std::process::id());
        // The membership exists only in this transaction, so no other database on the shared
        // cluster ever sees the group as a member of a role.
        let mut tx = h.pool.begin().await?;
        for statement in [
            format!("CREATE ROLE {probe} NOLOGIN"),
            format!("GRANT {probe} TO oteryn_game_highscore_job"),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&mut *tx)
                .await?;
        }
        let refused = (&mut *tx).execute(guard).await;
        let code = refused
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(|error| error.code());
        assert_eq!(code.as_deref(), Some("42501"), "{refused:?}");
        assert!(
            format!("{refused:?}").contains("is a member of another role"),
            "{refused:?}"
        );
        tx.rollback().await?;
        let memberships: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.member \
             WHERE r.rolname = 'oteryn_game_highscore_job'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(memberships, 0);
        h.pool.execute(guard).await?;
        Ok(())
    })
}
