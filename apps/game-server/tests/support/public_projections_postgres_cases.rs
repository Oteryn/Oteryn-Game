// GAME-PUBLIC-PROJ-PRODUCER-1 (migration 0091): the public projections' profile and
// highscores outboxes, epoch, guards, operator resync and least privilege (SQL cases on
// their own database), and the producer's store reads over a real bootstrap
// (`bootstrap_reads`, called from `bootstrap_audit_flow` with its reconciled authority).

use super::durability::DurabilityRoot;
use super::durability::character_authority::ReconciledCharacterAuthority;
use super::durability::public_projections::{CLEAR_HANDOFFS, ONLINE, PROFILE, RESYNC_DEADLINE};
use oteryn_game_server::native_admission_source::public_projections::{
    FILTERS, MAX_ONLINE, NAME_HANDOFF_HOLD, NameHandoff, OnlineState, PROFILE_BATCH, VOCATION_IDS,
    VOCATIONS, encode_highscores, encode_profile, online_digest,
};
use sqlx::{Connection, Executor, Row};

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

/// A HIGHSCORES-0 snapshot run (HS-1's job, migration 0097): the snapshot insert fires the
/// attached trigger. The fixture pins the identity so the expected outbox rows stay exact.
fn snapshot(id: i64, world: u8, category: i16) -> String {
    format!(
        "INSERT INTO game_world_highscore_snapshots \
           (snapshot_id, world_id, category, computed_at_ms, source_cut, row_count) \
         OVERRIDING SYSTEM VALUE VALUES ({id},{},{category},0,'',0);",
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

/// The Characters the producer's profile read returns now, in its order.
async fn profile_heads(pool: &sqlx::PgPool) -> TestResult<Vec<String>> {
    let hold = i64::try_from(NAME_HANDOFF_HOLD.as_millis())?;
    let rows = sqlx::query(PROFILE)
        .bind(Vec::<String>::new())
        .bind(hold)
        .bind(i64::try_from(PROFILE_BATCH)?)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>("character_id"))
        .collect())
}

/// The names the producer's online read lists now.
async fn online_names(pool: &sqlx::PgPool) -> TestResult<Vec<String>> {
    let vocations: Vec<String> = VOCATIONS.iter().map(|(key, _)| (*key).to_owned()).collect();
    let rows = sqlx::query(ONLINE)
        .bind(i64::try_from(MAX_ONLINE + 1)?)
        .bind(vocations)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .filter(|row| row.get::<i32, _>("kind") == 1)
        .map(|row| row.get::<String, _>("name"))
        .collect())
}

/// (name key, Character, World, vacated at) of the name handoffs the producer's online read
/// sees now.
async fn online_handoffs(pool: &sqlx::PgPool) -> TestResult<Vec<(String, String, String, i64)>> {
    let vocations: Vec<String> = VOCATIONS.iter().map(|(key, _)| (*key).to_owned()).collect();
    let rows = sqlx::query(ONLINE)
        .bind(i64::try_from(MAX_ONLINE + 1)?)
        .bind(vocations)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .filter(|row| row.get::<i32, _>("kind") == 3)
        .map(|row| {
            (
                row.get("name_key"),
                row.get("character_id"),
                row.get("world_id"),
                row.get("vacated_at"),
            )
        })
        .collect())
}

/// The scan's settlement of one handoff, as the runtime role.
async fn settle(pool: &sqlx::PgPool, key: &str, character: &str, vacated_at: i64) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query(CLEAR_HANDOFFS)
        .bind(vec![key.to_owned()])
        .bind(vec![character.to_owned()])
        .bind(vec![vacated_at])
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn bare(seed: u8) -> String {
    format!("{seed:02x}020304-0506-7008-800a-0b0c0d0e0f{seed:02x}")
}

/// A game session of `character` (on World `world`) in `state`, with or without a transport
/// and a control loss, held by an eligible admission guard on that World.
fn session(
    character: u8,
    account: u8,
    world: u8,
    state: u8,
    transport: bool,
    loss: bool,
) -> String {
    format!(
        "{}{}",
        session_row(character, account, world, state, transport, loss),
        guard(character, account, world, true, character)
    )
}

/// The admission guard of `character` on World `world`, `eligible` or not, holding the game
/// session of `holder` (see `session_row`). Foreign keys stay off (`session_row`).
fn guard(character: u8, account: u8, world: u8, eligible: bool, holder: u8) -> String {
    format!(
        "INSERT INTO game_durability_admission_character_guards VALUES \
           ({},{},{},{eligible},1,{},1,'test',1,'character-current',1,0,'{{}}');",
        uuid(character),
        uuid(account),
        uuid(world),
        uuid(holder.wrapping_add(0x40)),
    )
}

/// The game session row alone. Foreign keys are off for the insert (a fresh session's
/// admission receipt is not this test's subject); every CHECK still holds.
fn session_row(
    character: u8,
    account: u8,
    world: u8,
    state: u8,
    transport: bool,
    loss: bool,
) -> String {
    let continuity = if loss {
        "1,999999,1,NULL"
    } else {
        "NULL,NULL,NULL,'\\x01'::bytea"
    };
    let transport = if transport {
        format!("'\\x{}'::bytea", format!("{character:02x}").repeat(16))
    } else {
        "NULL".into()
    };
    format!(
        "SET LOCAL session_replication_role = replica; \
         INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,\
           world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,\
           control_loss_epoch,original_grace_deadline,predecessor_generation,fresh_replay_key,\
           character_lease_generation,scope_ownership_generation,current_generation,\
           current_transport_ref,session_state) \
         VALUES ({},{},{},{world},1,{world},{},{continuity},1,1,1,{transport},{state});",
        uuid(character.wrapping_add(0x40)),
        uuid(account),
        uuid(character),
        uuid(0x80),
        world = uuid(world),
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

    // Highscores: no HIGHSCORES-0 snapshot yet, so a resync queues nothing.
    assert_eq!(resync(&pool, "highscores", false).await?, 1);
    assert_eq!(highscores(&pool).await?, vec![]);
    // The HS-1 interface (contract §11, attached by migration 0097): a snapshot insert advances
    // the (World, category) revision in the job's own transaction, whatever role the job runs as.
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(snapshot(1, 0x90, 1)))
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    assert_eq!(highscores(&pool).await?, vec![]);
    attempt(&pool, &snapshot(1, 0x90, 1)).await?;
    // HIGHSCORES0 §4: only the job's role writes; a runtime login without the role does not.
    let denied = attempt(
        &pool,
        &format!(
            "SET LOCAL ROLE oteryn_game_runtime; {}",
            snapshot(2, 0x90, 1)
        ),
    )
    .await;
    assert_eq!(sqlstate(&denied).as_deref(), Some("42501"), "{denied:?}");
    attempt(
        &pool,
        &format!(
            "SET LOCAL ROLE oteryn_game_runtime; SET LOCAL ROLE oteryn_game_highscore_job; {}",
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
        format!(
            "SELECT game_public_profile_touch({}, NULL, NULL)",
            uuid(0x11)
        ),
        format!(
            "INSERT INTO game_public_name_handoffs VALUES ('aldric',{},{},0)",
            uuid(0x11),
            uuid(90)
        ),
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
    // The resync runs as its caller (only the owner login, which holds no grant beyond it).
    let definer: bool = sqlx::query_scalar(
        "SELECT prosecdef FROM pg_proc WHERE proname = 'game_public_projection_resync'",
    )
    .fetch_one(&pool)
    .await?;
    assert!(!definer, "the resync is SECURITY INVOKER");

    handoff_cases(&pool).await?;
    online_cases(&pool).await?;
    pool.close().await;
    Ok(())
}

/// A store recovery (§5 Resync) at scale: a full resync with an epoch raise of 10,000
/// Characters and 704 highscores pairs is one transaction of set-based statements under the
/// ops command's pass deadline (`PUBPROJ-RESYNC-DEADLINE`), PostgreSQL transaction, statement
/// and lock timeouts included, and it queues every subject at the new epoch.
#[test]
fn public_projections_full_resync_fits_the_recovery_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin).await?;
            let result = large_resync(&database).await;
            database.cleanup().await?;
            result
        })
}

async fn large_resync(database: &Database) -> TestResult {
    const CHARACTERS: i64 = 10_000;
    const PAIRS: i64 = 64 * 11;
    let pool = sqlx::PgPool::connect(&database.url).await?;
    attempt(
        &pool,
        &format!(
            "INSERT INTO game_character_account_guards \
               SELECT (lpad(to_hex(a), 8, '0') || '-0000-7000-8000-0000000000aa')::uuid \
               FROM generate_series(1, 200) a; \
             INSERT INTO game_character_roots (character_id, account_id, world_id, lifecycle, \
               character_revision, profile_revision, ruleset_revision, content_revision, \
               starter_template_revision, name) \
             SELECT (lpad(to_hex(i), 8, '0') || '-0000-7000-8000-000000000000')::uuid, \
               (lpad(to_hex(1 + i % 200), 8, '0') || '-0000-7000-8000-0000000000aa')::uuid, \
               {world}, 1, 1, 'profile-1', 'ruleset-1', 'content-1', 'starter-1', \
               'Hero ' || translate(lpad(i::text, 6, '0'), '0123456789', 'abcdefghij') \
             FROM generate_series(1, {CHARACTERS}) i; \
             INSERT INTO game_world_highscore_snapshots \
               (snapshot_id, world_id, category, computed_at_ms, source_cut, row_count) \
             OVERRIDING SYSTEM VALUE \
             SELECT w * 11 + c, (lpad(to_hex(w), 8, '0') || '-0000-7000-8000-0000000000ff')::uuid, \
               c, 0, '', 0 FROM generate_series(1, 64) w, generate_series(1, 11) c;",
            world = uuid(90),
        ),
    )
    .await?;

    let root = DurabilityRoot::connect_test_runtime(&database.url).map_err(|e| format!("{e:?}"))?;
    assert!(
        root.maintain_ready_once()
            .await
            .map_err(|e| format!("{e:?}"))?
    );
    let _recovery = root.boot_pass_deadline(RESYNC_DEADLINE);
    let started = std::time::Instant::now();
    let raised = root
        .try_issue_semantic_pass()
        .map_err(|e| format!("{e:?}"))?
        .run(|holder, deadline| {
            Box::pin(async move {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                let mut tx = sqlx::Connection::begin(&mut **holder).await?;
                sqlx::query(
                    "SELECT set_config('transaction_timeout', $1, true), \
                       set_config('statement_timeout', $1, true), \
                       set_config('lock_timeout', $1, true)",
                )
                .bind(format!("{}ms", remaining.as_millis().max(1)))
                .execute(&mut *tx)
                .await?;
                let epoch: i64 =
                    sqlx::query_scalar("SELECT game_public_projection_resync('all', true)")
                        .fetch_one(&mut *tx)
                        .await?;
                tx.commit()
                    .await
                    .map_err(super::durability::DurabilityError::from_commit_error)?;
                Ok(epoch)
            })
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
    let took = started.elapsed();
    eprintln!("full resync of {CHARACTERS} Characters and {PAIRS} pairs took {took:?}");
    assert!(took < RESYNC_DEADLINE, "{took:?}");

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_public_character_profile_outbox \
                  WHERE projection_epoch = $1 AND projection_revision = 2), \
                (SELECT count(*) FROM game_public_character_profiles \
                  WHERE projection_revision = 2), \
                (SELECT count(*) FROM game_public_highscore_outbox \
                  WHERE projection_epoch = $1 AND projection_revision = 2), \
                (SELECT count(*) FROM game_public_highscores WHERE projection_revision = 2)",
    )
    .bind(raised)
    .fetch_one(&pool)
    .await?;
    assert_eq!(counts, (CHARACTERS, CHARACTERS, PAIRS, PAIRS));
    assert_eq!(epoch(&pool).await?, raised);
    pool.close().await;
    Ok(())
}

/// §7.3 name handoff. A root's name is immutable today, so the vacating rows a future rename
/// or deletion writes (`game_public_profile_root_trigger`) are inserted directly: 0x12 Bera,
/// 0x13 Cato.
async fn handoff_cases(pool: &sqlx::PgPool) -> TestResult {
    let epoch = epoch(pool).await?;
    let vacating = |character: u8, revision: u32, key: &str| {
        format!(
            "INSERT INTO game_public_character_profile_outbox VALUES ({},{epoch},{revision},\
               floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint,{key});",
            uuid(character)
        )
    };
    attempt(pool, "DELETE FROM game_public_character_profile_outbox").await?;
    assert_eq!(profile_heads(pool).await?, Vec::<String>::new());
    // Simple handoff: 0x12 vacated "Cato", which 0x13 now holds. 0x13 waits until 0x12's
    // vacating publication is delivered (cleared), then goes.
    attempt(
        pool,
        &format!(
            "{}{}",
            vacating(0x12, 1000, "'cato'"),
            vacating(0x13, 1000, "NULL")
        ),
    )
    .await?;
    assert_eq!(profile_heads(pool).await?, vec![bare(0x12)]);
    attempt(
        pool,
        &format!(
            "DELETE FROM game_public_character_profile_outbox WHERE character_id = {}",
            uuid(0x12)
        ),
    )
    .await?;
    assert_eq!(profile_heads(pool).await?, vec![bare(0x13)]);
    // A cycle (each holds the name the other vacated): both wait, but only for the bounded
    // hold; a vacating row older than `PUBPROJ-NAME-HANDOFF-HOLD` no longer holds anyone.
    attempt(
        pool,
        &format!(
            "{}{}",
            vacating(0x12, 1001, "'cato'"),
            vacating(0x13, 1001, "'bera'")
        ),
    )
    .await?;
    assert_eq!(profile_heads(pool).await?, Vec::<String>::new());
    let hold = i64::try_from(NAME_HANDOFF_HOLD.as_millis())?;
    attempt(
        pool,
        &format!(
            "UPDATE game_public_character_profile_outbox SET created_at = created_at - {}",
            hold + 1
        ),
    )
    .await?;
    let mut heads = profile_heads(pool).await?;
    heads.sort();
    assert_eq!(heads, vec![bare(0x12), bare(0x13)]);

    // The online half: Platform joins `world_online` by (World, name), so after 0x12's
    // vacating publication is delivered 0x13 still waits, until a scan acknowledged at a cut
    // that sees the handoff settles it (or the bounded hold passes).
    attempt(pool, "DELETE FROM game_public_character_profile_outbox").await?;
    attempt(
        pool,
        &format!(
            "SELECT game_public_profile_touch({}, 'cato', {}); \
             SELECT game_public_profile_touch({}, NULL, NULL);",
            uuid(0x12),
            uuid(90),
            uuid(0x13)
        ),
    )
    .await?;
    assert_eq!(profile_heads(pool).await?, vec![bare(0x12)]);
    attempt(
        pool,
        &format!(
            "DELETE FROM game_public_character_profile_outbox WHERE character_id = {}",
            uuid(0x12)
        ),
    )
    .await?;
    assert_eq!(
        profile_heads(pool).await?,
        Vec::<String>::new(),
        "the old holder's World has no acknowledged cut that sees the handoff"
    );
    let seen = online_handoffs(pool).await?;
    assert_eq!(seen.len(), 1, "{seen:?}");
    let (key, character, world, vacated_at) = seen[0].clone();
    assert_eq!(
        (key.as_str(), character.clone(), world),
        ("cato", bare(0x12), bare(90))
    );
    // Only exactly the handoff the cut saw is settled: a later vacate stays.
    settle(pool, &key, &character, vacated_at - 1).await?;
    assert_eq!(profile_heads(pool).await?, Vec::<String>::new());
    settle(pool, &key, &character, vacated_at).await?;
    assert_eq!(profile_heads(pool).await?, vec![bare(0x13)]);
    assert!(online_handoffs(pool).await?.is_empty());
    // A handoff older than the hold no longer holds anyone.
    attempt(
        pool,
        &format!(
            "DELETE FROM game_public_character_profile_outbox; \
             SELECT game_public_profile_touch({}, NULL, NULL); \
             INSERT INTO game_public_name_handoffs VALUES ('cato',{},{},\
               floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint - {});",
            uuid(0x13),
            uuid(0x12),
            uuid(90),
            hold + 1
        ),
    )
    .await?;
    assert_eq!(profile_heads(pool).await?, vec![bare(0x13)]);
    attempt(pool, "DELETE FROM game_public_name_handoffs").await?;
    // A resync re-publishes every profile and clears every recorded key.
    attempt(pool, &vacating(0x12, 1002, "'dara'")).await?;
    assert_eq!(resync(pool, "character_profile", false).await?, epoch);
    let (outbox, _) = profiles(pool).await?;
    assert!(!outbox.is_empty());
    assert!(outbox.iter().all(|row| row.3.is_none()), "{outbox:?}");
    Ok(())
}

/// §4.6: a Character is online only with a live session on its own World: state 2, a
/// current transport and no control loss, held by its eligible admission guard on that World
/// (the predicate of `spell_character_lookup`).
async fn online_cases(pool: &sqlx::PgPool) -> TestResult {
    attempt(
        pool,
        &format!(
            "{}{}{}",
            root(0x15, 0xa3, "Elo"),
            root(0x16, 0xa4, "Fen"),
            root(0x17, 0xa5, "Gil")
        ),
    )
    .await?;
    assert_eq!(online_names(pool).await?, Vec::<String>::new());
    for (character, account, world, state, transport, loss) in [
        (0x11, 0xa0, 90, 2, true, false),
        (0x12, 0xa1, 90, 1, true, true),
        (0x14, 0xa2, 90, 3, true, false),
        (0x15, 0xa3, 90, 2, false, false),
        (0x16, 0xa4, 90, 2, true, true),
        (0x17, 0xa5, 91, 2, true, false),
    ] {
        attempt(
            pool,
            &session(character, account, world, state, transport, loss),
        )
        .await?;
    }
    assert_eq!(online_names(pool).await?, vec!["Aldric".to_owned()]);
    // A live session whose admission guard is not eligible, is on another World, or holds
    // another session: not online.
    attempt(
        pool,
        &format!(
            "{}{}{}{}{}{}{}{}{}",
            root(0x1b, 0xa9, "Jon"),
            root(0x1c, 0xaa, "Kai"),
            root(0x1d, 0xab, "Lia"),
            session_row(0x1b, 0xa9, 90, 2, true, false),
            guard(0x1b, 0xa9, 90, false, 0x1b),
            session_row(0x1c, 0xaa, 90, 2, true, false),
            guard(0x1c, 0xaa, 91, true, 0x1c),
            session_row(0x1d, 0xab, 90, 2, true, false),
            guard(0x1d, 0xab, 90, true, 0x11),
        ),
    )
    .await?;
    assert_eq!(online_names(pool).await?, vec!["Aldric".to_owned()]);
    // The session ends: no longer listed.
    attempt(
        pool,
        &format!(
            "SET LOCAL session_replication_role = replica; \
             UPDATE game_durability_reconnect_sessions SET session_state = 3 \
              WHERE character_id = {}",
            uuid(0x11)
        ),
    )
    .await?;
    assert_eq!(online_names(pool).await?, Vec::<String>::new());
    unclassifiable_online_case(pool).await
}

/// §4.6: an online Character the producer cannot classify (a name outside the wire grammar,
/// an unmappable vocation) is left out, so it never withholds its World's list. The name
/// policy CHECK forbids such a name today, so it is dropped inside a transaction that is
/// rolled back; triggers are off for the inserts.
async fn unclassifiable_online_case(pool: &sqlx::PgPool) -> TestResult {
    let mut tx = pool.begin().await?;
    let setup = format!(
        "SET LOCAL session_replication_role = replica; \
         DO $$ DECLARE c TEXT; BEGIN \
           SELECT conname INTO STRICT c FROM pg_constraint \
            WHERE conrelid = 'game_character_roots'::regclass \
              AND pg_get_constraintdef(oid) LIKE '%game_character_is_name%'; \
           EXECUTE format('ALTER TABLE game_character_roots DROP CONSTRAINT %I', c); \
         END $$; \
         {}{}{}\
         INSERT INTO game_character_build_state VALUES ({},'warlock',0,0,10,0,10,0,10,0,10,0,\
           10,0,10,0,10,0,2,{}); \
         {}{}{}",
        root(0x18, 0xa6, "Bad\"Name"),
        root(0x19, 0xa7, "Hal"),
        root(0x1a, 0xa8, "Ivo"),
        uuid(0x19),
        uuid(0x19),
        session(0x18, 0xa6, 90, 2, true, false),
        session(0x19, 0xa7, 90, 2, true, false),
        session(0x1a, 0xa8, 90, 2, true, false),
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(setup))
        .execute(&mut *tx)
        .await?;
    let vocations: Vec<String> = VOCATIONS.iter().map(|(key, _)| (*key).to_owned()).collect();
    let rows = sqlx::query(ONLINE)
        .bind(i64::try_from(MAX_ONLINE + 1)?)
        .bind(vocations)
        .fetch_all(&mut *tx)
        .await?;
    let names: Vec<String> = rows
        .iter()
        .filter(|row| row.get::<i32, _>("kind") == 1)
        .map(|row| row.get::<String, _>("name"))
        .collect();
    assert_eq!(names, vec!["Ivo".to_owned()]);
    tx.rollback().await?;
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
    assert_eq!(
        facts.highscores,
        Some(None),
        "migration 0097's source with nothing undelivered"
    );
    assert!(
        root.next_public_highscores(authority, &[])
            .await
            .map_err(error)?
            .is_none()
    );

    // The bootstrap queued the Character's profile in its own transaction.
    let batch = root
        .next_public_profiles(authority, &[], PROFILE_BATCH)
        .await
        .map_err(error)?;
    assert_eq!(batch.len(), 1, "bootstrap queued one public profile");
    let profile = batch[0].clone();
    assert_eq!(profile.character_id, character_id);
    assert_eq!(profile.projection_epoch, 1);
    let entry = profile
        .profile
        .clone()
        .ok_or("an active Character has a profile")?;
    assert_eq!(entry.world_id, world_id);
    assert!(VOCATION_IDS.contains(&entry.vocation) && entry.level >= 1);
    assert!(entry.guild_visible, "defaults to public (§4.2)");
    assert_eq!(entry.guild, None);
    let body = encode_profile(SOURCE, &profile)?;
    // A held Character is skipped; a retry is byte-identical.
    assert!(
        root.next_public_profiles(
            authority,
            std::slice::from_ref(&profile.character_id),
            PROFILE_BATCH
        )
        .await
        .map_err(error)?
        .is_empty()
    );
    let retry = root
        .next_public_profiles(authority, &[], PROFILE_BATCH)
        .await
        .map_err(error)?;
    assert_eq!(retry.len(), 1, "the unacknowledged profile is still queued");
    assert_eq!(encode_profile(SOURCE, &retry[0])?, body);
    root.clear_public_profiles(
        authority,
        &[(
            profile.character_id.clone(),
            profile.projection_epoch,
            profile.projection_revision,
        )],
    )
    .await
    .map_err(error)?;
    assert!(
        root.next_public_profiles(authority, &[], PROFILE_BATCH)
            .await
            .map_err(error)?
            .is_empty()
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
    // A name handoff of this World (§7.3) is read with the cut and settled with the
    // acknowledgement.
    assert!(cut.handoffs.is_empty());
    sqlx::query(
        "INSERT INTO game_public_name_handoffs VALUES ('oldname', $1::uuid, $2::uuid, 1700000000000)",
    )
    .bind(character_id)
    .bind(world_id)
    .execute(pool)
    .await?;
    let handoffs = root
        .public_online_cut(authority)
        .await
        .map_err(error)?
        .handoffs;
    assert_eq!(
        handoffs,
        vec![NameHandoff {
            name_key: "oldname".into(),
            character_id: character_id.to_owned(),
            world_id: world_id.to_owned(),
            vacated_at_ms: 1_700_000_000_000,
        }]
    );
    root.ack_public_online(authority, std::slice::from_ref(&state), &handoffs)
        .await
        .map_err(error)?;
    let cut = root.public_online_cut(authority).await.map_err(error)?;
    assert!(cut.handoffs.is_empty(), "settled with the acknowledgement");
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

    // HS-1's tables (migration 0097, contract §11 interface): its trigger and runtime SELECT.
    let facts = root.public_outbox_facts(authority).await.map_err(error)?;
    assert_eq!(
        facts.highscores,
        Some(None),
        "a source with nothing undelivered"
    );
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_highscore_job")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_world_highscore_snapshots \
           (snapshot_id, world_id, category, computed_at_ms, source_cut, row_count) \
         OVERRIDING SYSTEM VALUE VALUES (1, $1::uuid, 1, 1700000000123, '', 2)",
    )
    .bind(world_id)
    .execute(&mut *tx)
    .await?;
    // The overall list and its vocation's list: a promoted key and a wire id. A published
    // snapshot never acquires rows, so the rows precede its head.
    sqlx::query(
        "INSERT INTO game_world_highscore_rows (snapshot_id, filter, position, rank, \
           character_id, name, vocation, level, value_primary, value_progress) VALUES \
           (1, 0, 1, 1, $1::uuid, $2, 'elite_knight', 50, 1000, 0), \
           (1, 2, 1, 1, $1::uuid, $2, '2', 50, 1000, 0)",
    )
    .bind(character_id)
    .bind(&entry.name)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_world_highscore_heads (world_id, category, current_snapshot_id) \
         VALUES ($1::uuid, 1, 1)",
    )
    .bind(world_id)
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
