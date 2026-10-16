// GAME-PUBLIC-PROJ-PRODUCER-1 (migration 0088): the public projections' profile and
// highscores outboxes, epoch, guards, operator resync and least privilege (SQL cases on
// their own database), and the producer's store reads over a real bootstrap
// (`bootstrap_reads`, called from `bootstrap_audit_flow` with its reconciled authority).

use super::durability::DurabilityRoot;
use super::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_game_server::native_admission_source::public_projections::{
    FILTERS, OnlineState, VOCATION_IDS, encode_highscores, encode_profile, online_digest,
};
use sqlx::{Connection, Executor};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn uuid(seed: u8) -> String {
    format!("'{seed:02x}020304-0506-7008-800a-0b0c0d0e0f{seed:02x}'::uuid")
}

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("pubproj_{suffix}");
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        admin.close().await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{name}");
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = sqlx::PgConnection::connect(&self.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        admin.close().await?;
        Ok(())
    }
}

fn root(character: u8, account: u8, name: &str) -> String {
    format!(
        "INSERT INTO game_character_account_guards VALUES ({account}) ON CONFLICT DO NOTHING; \
         INSERT INTO game_character_roots VALUES ({character},{account},{world},1,1,\
           'profile-1','ruleset-1','content-1','starter-1','{name}');",
        account = uuid(account),
        character = uuid(character),
        world = uuid(90),
    )
}

fn progression(character: u8, level: u32) -> String {
    format!(
        "INSERT INTO game_character_progression_state (character_id, character_revision, level, \
           total_experience, profile_revision, ruleset_revision, content_revision, \
           simulation_revision, evidence_revision, declaration_revision, policy_revision, \
           reward_revision) \
         VALUES ({},1,{level},0,'profile-1','ruleset-1','content-1','simulation-1',\
           'evidence-1','declaration-1','policy-1','reward-1');",
        uuid(character)
    )
}

/// A HIGHSCORES-0 snapshot run (HS-1's job): the snapshot insert fires the attached trigger.
fn snapshot(id: i64, world: u8, category: i16) -> String {
    format!(
        "INSERT INTO game_world_highscore_snapshots VALUES ({id},{},{category},0);",
        uuid(world)
    )
}

async fn attempt(pool: &sqlx::PgPool, script: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

fn sqlstate(result: &Result<(), sqlx::Error>) -> Option<String> {
    result
        .as_ref()
        .err()
        .and_then(|error| error.as_database_error())
        .and_then(|error| error.code())
        .map(|code| code.into_owned())
}

type ProfileRows = Vec<(i32, i64, i64, Option<String>)>;

/// (Character seed byte, epoch, revision, vacated name key) of every profile outbox row,
/// plus each Character's revision.
async fn profiles(pool: &sqlx::PgPool) -> TestResult<(ProfileRows, Vec<(i32, i64)>)> {
    let outbox = sqlx::query_as(
        "SELECT get_byte(uuid_send(character_id), 0), projection_epoch, projection_revision, \
                vacated_name_key \
           FROM game_public_character_profile_outbox ORDER BY 1, 2, 3",
    )
    .fetch_all(pool)
    .await?;
    let revisions = sqlx::query_as(
        "SELECT get_byte(uuid_send(character_id), 0), projection_revision \
           FROM game_public_character_profiles ORDER BY 1",
    )
    .fetch_all(pool)
    .await?;
    Ok((outbox, revisions))
}

/// (World seed byte, category, epoch, revision) of every highscores outbox row.
async fn highscores(pool: &sqlx::PgPool) -> TestResult<Vec<(i32, i16, i64, i64)>> {
    Ok(sqlx::query_as(
        "SELECT get_byte(uuid_send(world_id), 0), category, projection_epoch, \
                projection_revision \
           FROM game_public_highscore_outbox ORDER BY 1, 2, 3, 4",
    )
    .fetch_all(pool)
    .await?)
}

async fn epoch(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT projection_epoch FROM game_public_projection_epoch")
            .fetch_one(pool)
            .await?,
    )
}

async fn resync(pool: &sqlx::PgPool, family: &str, raise: bool) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT game_public_projection_resync($1, $2)")
            .bind(family)
            .bind(raise)
            .fetch_one(pool)
            .await?,
    )
}

async fn acked(pool: &sqlx::PgPool) -> TestResult<Vec<bool>> {
    Ok(
        sqlx::query_scalar("SELECT acked FROM game_public_world_online ORDER BY world_id")
            .fetch_all(pool)
            .await?,
    )
}

#[test]
fn public_projections_outboxes_epoch_and_privileges() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin).await?;
            let result = projection_cases(&database).await;
            database.cleanup().await?;
            result
        })
}

#[allow(clippy::too_many_lines)]
async fn projection_cases(database: &Database) -> TestResult {
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let runtime = |sql: &str| format!("SET LOCAL ROLE oteryn_game_runtime; {sql}");
    assert_eq!(epoch(&pool).await?, 1);
    assert_eq!(profiles(&pool).await?, (vec![], vec![]));

    // A rolled-back Character write leaves no revision and no outbox row.
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(root(0x11, 0xa0, "Aldric")))
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    assert_eq!(profiles(&pool).await?, (vec![], vec![]));

    // Committed: the root insert advances the Character's revision in the same transaction.
    attempt(&pool, &root(0x11, 0xa0, "Aldric")).await?;
    assert_eq!(
        profiles(&pool).await?,
        (vec![(0x11, 1, 1, None)], vec![(0x11, 1)])
    );
    // The level (progression) is a projected field: its insert advances the revision.
    attempt(&pool, &progression(0x11, 8)).await?;
    assert_eq!(
        profiles(&pool).await?,
        (
            vec![(0x11, 1, 1, None), (0x11, 1, 2, None)],
            vec![(0x11, 2)]
        )
    );
    // A refused Character write (taken name, 23505) queues nothing.
    let refused = attempt(&pool, &root(0x12, 0xa0, "al dric")).await;
    assert_eq!(sqlstate(&refused).as_deref(), Some("23505"), "{refused:?}");
    assert_eq!(profiles(&pool).await?.1, vec![(0x11, 2)]);
    attempt(&pool, &root(0x12, 0xa1, "Bera")).await?;
    assert_eq!(profiles(&pool).await?.1, vec![(0x11, 2), (0x12, 1)]);
    // The build vocation and the progression level have their triggers (the build row's
    // insert needs the full build state; the shared function is covered above).
    let triggers: Vec<String> = sqlx::query_scalar(
        "SELECT tgname::text FROM pg_trigger WHERE NOT tgisinternal \
            AND tgname LIKE 'game_public_profile_%' ORDER BY 1",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        triggers,
        vec![
            "game_public_profile_build_insert_delete",
            "game_public_profile_build_update",
            "game_public_profile_progression_insert_delete",
            "game_public_profile_progression_update",
            "game_public_profile_revision_guard",
            "game_public_profile_root_insert_delete",
            "game_public_profile_root_update",
        ]
    );
    // The created_at of a queued row is its transaction start, never in the future.
    let future: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_public_character_profile_outbox \
          WHERE created_at > floor(extract(epoch FROM clock_timestamp()) * 1000)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(future, 0);

    // Highscores: no HIGHSCORES-0 source yet, so a resync queues nothing.
    assert_eq!(resync(&pool, "highscores", false).await?, 1);
    assert_eq!(highscores(&pool).await?, vec![]);
    // The HS-1 interface (contract §11): a snapshot insert advances the (World, category)
    // revision in the job's own transaction, whatever role the job runs as.
    attempt(
        &pool,
        "CREATE TABLE game_world_highscore_snapshots (snapshot_id BIGINT PRIMARY KEY, \
           world_id UUID NOT NULL, category SMALLINT NOT NULL, computed_at_ms BIGINT NOT NULL); \
         CREATE TRIGGER game_public_highscores_snapshot AFTER INSERT \
           ON game_world_highscore_snapshots FOR EACH ROW \
           EXECUTE FUNCTION game_public_highscores_snapshot_trigger(); \
         GRANT SELECT, INSERT ON game_world_highscore_snapshots TO oteryn_game_runtime;",
    )
    .await?;
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(snapshot(1, 0x90, 1)))
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    assert_eq!(highscores(&pool).await?, vec![]);
    attempt(&pool, &snapshot(1, 0x90, 1)).await?;
    attempt(
        &pool,
        &format!(
            "SET LOCAL ROLE oteryn_game_runtime; {}",
            snapshot(2, 0x90, 1)
        ),
    )
    .await?;
    attempt(&pool, &snapshot(3, 0x91, 11)).await?;
    assert_eq!(
        highscores(&pool).await?,
        vec![(0x90, 1, 1, 1), (0x90, 1, 1, 2), (0x91, 11, 1, 1)]
    );
    // A category outside 1..=11 refuses the job's snapshot with it.
    let refused = attempt(&pool, &snapshot(4, 0x90, 12)).await;
    assert_eq!(sqlstate(&refused).as_deref(), Some("23514"), "{refused:?}");
    assert_eq!(highscores(&pool).await?.len(), 3);
    // With a source, a highscores resync queues every pair at a new revision.
    assert_eq!(resync(&pool, "highscores", false).await?, 1);
    assert_eq!(
        highscores(&pool).await?,
        vec![
            (0x90, 1, 1, 1),
            (0x90, 1, 1, 2),
            (0x90, 1, 1, 3),
            (0x91, 11, 1, 1),
            (0x91, 11, 1, 2)
        ]
    );

    // The runtime keeps its own world_online snapshots; a resync marks them unacknowledged.
    let digest = format!("'\\x{}'::bytea", "ab".repeat(32));
    attempt(
        &pool,
        &format!(
            "SET LOCAL ROLE oteryn_game_runtime; \
             INSERT INTO game_public_world_online VALUES ({},1,1,{digest},5,true);",
            uuid(0x90)
        ),
    )
    .await?;
    assert_eq!(resync(&pool, "world_online", false).await?, 1);
    assert_eq!(acked(&pool).await?, vec![false]);
    assert_eq!(profiles(&pool).await?.1, vec![(0x11, 2), (0x12, 1)]);
    attempt(
        &pool,
        "SET LOCAL ROLE oteryn_game_runtime; UPDATE game_public_world_online SET acked = true",
    )
    .await?;

    // A profile resync queues every Character at a new revision, in the current epoch.
    assert_eq!(resync(&pool, "character_profile", false).await?, 1);
    assert_eq!(profiles(&pool).await?.1, vec![(0x11, 3), (0x12, 2)]);
    assert_eq!(acked(&pool).await?, vec![true]);

    // An epoch raise (only with 'all') exceeds the current Unix millisecond and queues
    // every subject at a new revision of the new epoch.
    let raised = resync(&pool, "all", true).await?;
    let now_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(&pool)
            .await?;
    assert!(
        raised > 1_000_000_000_000 && raised <= now_ms && raised >= now_ms - 60_000,
        "{raised} {now_ms}"
    );
    assert_eq!(epoch(&pool).await?, raised);
    let (outbox, revisions) = profiles(&pool).await?;
    assert_eq!(revisions, vec![(0x11, 4), (0x12, 3)]);
    assert!(outbox.contains(&(0x11, raised, 4, None)) && outbox.contains(&(0x12, raised, 3, None)));
    let queued = highscores(&pool).await?;
    assert!(queued.contains(&(0x90, 1, raised, 4)) && queued.contains(&(0x91, 11, raised, 3)));
    assert_eq!(acked(&pool).await?, vec![false]);
    // A change after the raise carries the new epoch.
    attempt(&pool, &root(0x13, 0xa1, "Cato")).await?;
    assert!(profiles(&pool).await?.0.contains(&(0x13, raised, 1, None)));
    let again = resync(&pool, "all", true).await?;
    assert!(
        again > raised && again - raised < 60_000,
        "{raised} {again}"
    );
    // The runtime's next scan stores the World at the new epoch and the next revision.
    attempt(
        &pool,
        &runtime(&format!(
            "UPDATE game_public_world_online SET projection_epoch = {again}, \
               projection_revision = projection_revision + 1"
        )),
    )
    .await?;

    // The epoch only rises, a revision only advances by one, a stored online snapshot only
    // advances by one revision, nothing truncates, and the resync refuses an unknown family
    // or an epoch raise of one family.
    for (case, script, code) in [
        (
            "lower epoch",
            "UPDATE game_public_projection_epoch SET projection_epoch = 1",
            "23514",
        ),
        (
            "delete epoch",
            "DELETE FROM game_public_projection_epoch",
            "23514",
        ),
        (
            "rewind profile",
            "UPDATE game_public_character_profiles SET projection_revision = 1",
            "23514",
        ),
        (
            "skip profile",
            "UPDATE game_public_character_profiles \
               SET projection_revision = projection_revision + 2",
            "23514",
        ),
        (
            "delete profile",
            "DELETE FROM game_public_character_profiles",
            "23514",
        ),
        (
            "rewind highscores",
            "UPDATE game_public_highscores SET projection_revision = 1",
            "23514",
        ),
        (
            "online skip",
            "UPDATE game_public_world_online SET projection_revision = projection_revision + 2",
            "23514",
        ),
        (
            "online rewrite",
            "UPDATE game_public_world_online SET source_observed_at = source_observed_at + 1",
            "23514",
        ),
        (
            "online lower epoch",
            "UPDATE game_public_world_online \
               SET projection_epoch = 1, projection_revision = projection_revision + 1",
            "23514",
        ),
        (
            "online delete",
            "DELETE FROM game_public_world_online",
            "23514",
        ),
        (
            "truncate profiles",
            "TRUNCATE game_public_character_profiles CASCADE",
            "23514",
        ),
        (
            "truncate profile outbox",
            "TRUNCATE game_public_character_profile_outbox",
            "23514",
        ),
        (
            "truncate highscores outbox",
            "TRUNCATE game_public_highscore_outbox",
            "23514",
        ),
        (
            "truncate epoch",
            "TRUNCATE game_public_projection_epoch",
            "23514",
        ),
        (
            "truncate online",
            "TRUNCATE game_public_world_online",
            "23514",
        ),
        (
            "resync without family",
            "SELECT game_public_projection_resync(NULL, false)",
            "22004",
        ),
        (
            "resync without choice",
            "SELECT game_public_projection_resync('all', NULL)",
            "22004",
        ),
        (
            "resync unknown family",
            "SELECT game_public_projection_resync('guild', false)",
            "22023",
        ),
        (
            "raise one family",
            "SELECT game_public_projection_resync('world_online', true)",
            "22023",
        ),
    ] {
        let result = attempt(&pool, script).await;
        assert_eq!(
            sqlstate(&result).as_deref(),
            Some(code),
            "{case}: {result:?}"
        );
    }

    // Least privilege: the runtime reads the projection, clears the outboxes and keeps its
    // own online snapshots, but never writes a revision, an epoch or an outbox row, and
    // cannot run the resync or a touch.
    attempt(
        &pool,
        &runtime(
            "SELECT count(*) FROM game_public_character_profile_outbox; \
             SELECT count(*) FROM game_public_character_profiles; \
             SELECT count(*) FROM game_public_highscores; \
             SELECT count(*) FROM game_public_highscore_outbox; \
             SELECT projection_epoch FROM game_public_projection_epoch; \
             DELETE FROM game_public_character_profile_outbox WHERE false; \
             DELETE FROM game_public_highscore_outbox WHERE false; \
             UPDATE game_public_world_online SET acked = true;",
        ),
    )
    .await?;
    for script in [
        format!(
            "INSERT INTO game_public_character_profile_outbox VALUES ({},1,9,0,NULL)",
            uuid(0x11)
        ),
        format!(
            "INSERT INTO game_public_highscore_outbox VALUES ({},1,1,9,0)",
            uuid(0x90)
        ),
        "UPDATE game_public_character_profiles SET projection_revision = projection_revision + 1"
            .into(),
        "UPDATE game_public_highscores SET projection_revision = projection_revision + 1".into(),
        "UPDATE game_public_projection_epoch SET projection_epoch = projection_epoch + 1".into(),
        "DELETE FROM game_public_world_online".into(),
        "SELECT game_public_projection_resync('all', true)".into(),
        "SELECT game_public_projection_resync('world_online', false)".into(),
        format!("SELECT game_public_profile_touch({}, NULL)", uuid(0x11)),
        format!(
            "SELECT game_public_highscores_touch({}, 1::smallint)",
            uuid(0x90)
        ),
    ] {
        let result = attempt(&pool, &runtime(&script)).await;
        assert_eq!(
            sqlstate(&result).as_deref(),
            Some("42501"),
            "{script}: {result:?}"
        );
    }
    // The control role only reads.
    let control = |sql: &str| format!("SET LOCAL ROLE oteryn_game_control; {sql}");
    attempt(
        &pool,
        &control(
            "SELECT count(*) FROM game_public_character_profile_outbox; \
             SELECT count(*) FROM game_public_world_online;",
        ),
    )
    .await?;
    let result = attempt(
        &pool,
        &control("DELETE FROM game_public_character_profile_outbox WHERE false"),
    )
    .await;
    assert_eq!(sqlstate(&result).as_deref(), Some("42501"), "{result:?}");
    // The runtime role's own root insert still advances the revision through the definer
    // trigger.
    attempt(&pool, &runtime(&root(0x14, 0xa2, "Dara"))).await?;
    assert!(profiles(&pool).await?.1.contains(&(0x14, 1)));
    pool.close().await;
    Ok(())
}

/// The producer's store reads over a real bootstrap, after LCFA's checks in
/// `bootstrap_audit_flow`: the bootstrapped Character's profile, its clear, the watermark
/// facts, a `world_online` cut with the publisher's stored snapshot, and the highscores
/// reads before and after the HS-1 tables exist.
#[allow(clippy::too_many_lines)]
pub(crate) async fn bootstrap_reads(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    pool: &sqlx::PgPool,
    character_id: &str,
    world_id: &str,
) -> TestResult {
    const SOURCE: &str = "oteryn:character-authority:primary";
    let error = |e| format!("{e:?}");

    let facts = root.public_outbox_facts(authority).await.map_err(error)?;
    assert_eq!(facts.projection_epoch, 1);
    assert!(
        facts
            .profile_oldest_ms
            .is_some_and(|oldest| oldest <= facts.now_ms)
    );
    assert_eq!(facts.highscores, None, "no HIGHSCORES-0 source yet");
    assert!(
        root.next_public_highscores(authority, &[])
            .await
            .map_err(error)?
            .is_none()
    );

    // The bootstrap queued the Character's profile in its own transaction.
    let profile = root
        .next_public_profile(authority, &[])
        .await
        .map_err(error)?
        .ok_or("bootstrap queued no public profile")?;
    assert_eq!(profile.character_id, character_id);
    assert_eq!(profile.projection_epoch, 1);
    let entry = profile
        .profile
        .clone()
        .ok_or("an active Character has a profile")?;
    assert_eq!(entry.world_id, world_id);
    assert!(VOCATION_IDS.contains(&entry.vocation) && entry.level >= 1);
    assert!(!entry.guild_visible, "no setting owner: fails closed");
    assert_eq!(entry.guild, None);
    let body = encode_profile(SOURCE, &profile)?;
    // A held Character is skipped; a retry is byte-identical.
    assert!(
        root.next_public_profile(authority, std::slice::from_ref(&profile.character_id))
            .await
            .map_err(error)?
            .is_none()
    );
    let retry = root
        .next_public_profile(authority, &[])
        .await
        .map_err(error)?
        .ok_or("the unacknowledged profile is still queued")?;
    assert_eq!(encode_profile(SOURCE, &retry)?, body);
    root.clear_public_profile(
        authority,
        &profile.character_id,
        profile.projection_epoch,
        profile.projection_revision,
    )
    .await
    .map_err(error)?;
    assert!(
        root.next_public_profile(authority, &[])
            .await
            .map_err(error)?
            .is_none()
    );
    let facts = root.public_outbox_facts(authority).await.map_err(error)?;
    assert_eq!(facts.profile_oldest_ms, None);

    // world_online: the Character's World is listed (nobody online) with no stored
    // snapshot; the publisher's snapshot persists, then its acknowledgement.
    let cut = root.public_online_cut(authority).await.map_err(error)?;
    assert_eq!(cut.projection_epoch, 1);
    assert!(cut.cut_ms <= facts.now_ms + 60_000);
    let world = cut
        .worlds
        .iter()
        .find(|world| world.world_id == world_id)
        .ok_or("the Character's World is scanned")?;
    assert_eq!(world.state, None);
    let characters = world.characters.clone().ok_or("within the online bound")?;
    let state = OnlineState {
        world_id: world_id.to_owned(),
        projection_epoch: 1,
        projection_revision: 1,
        characters_digest: online_digest(&characters).ok_or("a valid list has a digest")?,
        source_observed_at: cut.cut_ms / 1000,
        acked: false,
    };
    root.persist_public_online(authority, std::slice::from_ref(&state))
        .await
        .map_err(error)?;
    root.ack_public_online(authority, std::slice::from_ref(&state))
        .await
        .map_err(error)?;
    let cut = root.public_online_cut(authority).await.map_err(error)?;
    let stored = cut
        .worlds
        .iter()
        .find(|world| world.world_id == world_id)
        .and_then(|world| world.state.clone())
        .ok_or("the stored snapshot is read back")?;
    assert_eq!(
        stored,
        OnlineState {
            acked: true,
            ..state
        }
    );

    // HS-1's tables (contract §11 interface), its trigger and its runtime grant.
    sqlx::raw_sql(
        "CREATE TABLE game_world_highscore_snapshots (snapshot_id BIGINT PRIMARY KEY, \
           world_id UUID NOT NULL, category SMALLINT NOT NULL, computed_at_ms BIGINT NOT NULL); \
         CREATE TABLE game_world_highscore_heads (world_id UUID NOT NULL, \
           category SMALLINT NOT NULL, current_snapshot_id BIGINT NOT NULL, \
           PRIMARY KEY (world_id, category)); \
         CREATE TABLE game_world_highscore_rows (snapshot_id BIGINT NOT NULL, \
           filter SMALLINT NOT NULL, position INTEGER NOT NULL, rank BIGINT NOT NULL, \
           character_id UUID NOT NULL, name TEXT NOT NULL, vocation TEXT NOT NULL, \
           level BIGINT NOT NULL, value_primary NUMERIC(20,0) NOT NULL); \
         CREATE TRIGGER game_public_highscores_snapshot AFTER INSERT \
           ON game_world_highscore_snapshots FOR EACH ROW \
           EXECUTE FUNCTION game_public_highscores_snapshot_trigger(); \
         GRANT SELECT ON game_world_highscore_snapshots, game_world_highscore_heads, \
           game_world_highscore_rows TO oteryn_game_runtime;",
    )
    .execute(pool)
    .await?;
    let facts = root.public_outbox_facts(authority).await.map_err(error)?;
    assert_eq!(
        facts.highscores,
        Some(None),
        "a source with nothing undelivered"
    );
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_world_highscore_snapshots VALUES (1, $1::uuid, 1, 1700000000123)",
    )
    .bind(world_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_world_highscore_heads VALUES ($1::uuid, 1, 1)")
        .bind(world_id)
        .execute(&mut *tx)
        .await?;
    // The overall list and its vocation's list: a promoted key and a wire id.
    sqlx::query(
        "INSERT INTO game_world_highscore_rows VALUES \
           (1, 0, 1, 1, $1::uuid, $2, 'elite_knight', 50, 1000), \
           (1, 2, 1, 1, $1::uuid, $2, '2', 50, 1000)",
    )
    .bind(character_id)
    .bind(&entry.name)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let batch = root
        .next_public_highscores(authority, &[])
        .await
        .map_err(error)?
        .ok_or("the snapshot job queued its revision")?;
    assert_eq!(
        (
            batch.world_id.as_str(),
            batch.category,
            batch.projection_epoch,
            batch.projection_revision,
            batch.computed_at
        ),
        (world_id, 1, 1, 1, 1_700_000_000)
    );
    assert_eq!(batch.lists[0].len(), 1);
    assert_eq!(batch.lists[0][0].character_id, character_id);
    assert_eq!(
        (batch.lists[0][0].vocation, batch.lists[2][0].vocation),
        (Some(2), Some(2))
    );
    for filter in 0..FILTERS {
        let filter = u8::try_from(filter)?;
        encode_highscores(SOURCE, &batch.snapshot(filter))?;
    }
    assert!(
        root.next_public_highscores(authority, &[batch.subject()])
            .await
            .map_err(error)?
            .is_none()
    );
    let facts = root.public_outbox_facts(authority).await.map_err(error)?;
    assert!(facts.highscores.is_some_and(|oldest| oldest.is_some()));
    root.clear_public_highscores(authority, &batch.world_id, batch.category, 1, 1)
        .await
        .map_err(error)?;
    assert!(
        root.next_public_highscores(authority, &[])
            .await
            .map_err(error)?
            .is_none()
    );
    Ok(())
}
