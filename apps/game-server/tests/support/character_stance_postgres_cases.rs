// Shared STANCE-0 cases (migration 0017). There is no stance writer until
// STANCE-1, so every Character transaction here is the exact SQL a writer must
// issue (root successor, state successor, receipt, stance row), committed as
// one PostgreSQL transaction. The cases need no crate code, so any PostgreSQL
// wrapper target can include this file.

use sqlx::{Connection, Executor};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHARACTER: u8 = 51;
const WORLD: u8 = 52;
const CHANNEL: u8 = 53;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn uuid_text(seed: u8) -> String {
    let hex = hex(&id(seed));
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn bytea(bytes: &[u8]) -> String {
    format!("'\\x{}'::bytea", hex(bytes))
}

fn key(value: Option<&str>) -> String {
    value.map_or_else(|| "NULL".to_owned(), |value| format!("'{value}'"))
}

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String, name: &str) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("cs_{name}_{suffix}");
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

fn configured_admin() -> Option<String> {
    match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            None
        }
    }
}

fn bootstrap_binding() -> Vec<u8> {
    let mut binding = vec![2];
    binding.extend_from_slice(&id(31));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&id(30));
    binding.extend_from_slice(&id(40));
    binding.extend_from_slice(&id(WORLD));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&120_i64.to_be_bytes());
    for value in ["profile-1", "ruleset-1", "content-1", "starter-1"] {
        let length = u16::try_from(value.len()).unwrap_or(u16::MAX);
        binding.extend_from_slice(&length.to_be_bytes());
        binding.extend_from_slice(value.as_bytes());
    }
    // Contract version 2 binds the requested name last (CHAR-NAME-1).
    binding.extend_from_slice(&12_u16.to_be_bytes());
    binding.extend_from_slice(b"Fixture Hero");
    binding
}

/// One bootstrapped Character with initialized progression at revision one
/// (level 50, 1000 experience), exactly as the P03 and DEATH-0 cases seed it.
async fn seed_character(pool: &sqlx::PgPool) -> TestResult {
    let binding = bytea(&bootstrap_binding());
    let script = format!(
        "INSERT INTO game_character_interpretations VALUES \
           (1,'profile-1','ruleset-1','content-1','starter-1',1); \
         INSERT INTO game_character_account_guards VALUES ({account}); \
         INSERT INTO game_character_roots VALUES \
           ({character},{account},{world},1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Hero'); \
         INSERT INTO game_character_operation_receipts(\
           operation_id,command_binding,account_id,character_id,world_id,character_revision,\
           event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,\
           intent_source_revision,issued_at_source,expires_at_source) \
         VALUES ({operation},{binding},{account},{character},{world},1,{event},1,'test/1',\
           {transaction},{decision},1,1,120); \
         INSERT INTO game_character_bootstrap_intent_floors VALUES (1,1,{decision},{binding}); \
         INSERT INTO game_character_progression_state VALUES \
           ({character},1,50,1000,'profile-1','ruleset-1','content-1',\
            'simulation-1','evidence-1','declaration-1','policy-1','reward-1');",
        account = uuid(40),
        character = uuid(CHARACTER),
        world = uuid(WORLD),
        operation = uuid(30),
        decision = uuid(31),
        event = uuid(32),
        transaction = uuid(33),
    );
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Runs `script` as one explicit transaction and commits it, so both
/// immediate and deferred (commit-time) guard failures surface here. A failed
/// attempt rolls back completely.
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

async fn expect_committed(pool: &sqlx::PgPool, case: &str, script: &str) -> TestResult {
    attempt(pool, script)
        .await
        .map_err(|error| format!("{case}: expected commit, got {error}").into())
}

async fn expect_rejected(pool: &sqlx::PgPool, case: &str, script: &str, code: &str) -> TestResult {
    let before = snapshot(pool).await?;
    let result = attempt(pool, script).await;
    let observed = sqlstate(&result);
    if observed.as_deref() != Some(code) {
        return Err(format!("{case}: expected SQLSTATE {code}, got {result:?}").into());
    }
    if snapshot(pool).await? != before {
        return Err(format!("{case}: a rejected transaction changed durable state").into());
    }
    Ok(())
}

/// Every relation the STANCE-0 guards bind, as one comparable value.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(character_revision::text, ',') FROM game_character_roots), \
           (SELECT string_agg(concat_ws(':', character_revision, level, total_experience), ',') \
              FROM game_character_progression_state), \
           (SELECT string_agg(committed_character_revision::text, ',' ORDER BY 1) \
              FROM game_character_xp_receipts), \
           (SELECT string_agg(committed_character_revision::text, ',' ORDER BY 1) \
              FROM game_character_death_receipts), \
           (SELECT string_agg(concat_ws(':', committed_character_revision, \
                     coalesce(stance_before, '-'), coalesce(stance_after, '-')), ',' ORDER BY 1) \
              FROM game_character_stance_receipts), \
           (SELECT string_agg(concat_ws(':', coalesce(stance_key, '-'), \
                     committed_character_revision, last_stance_occurrence_id), ',') \
              FROM game_character_stance), \
           (SELECT string_agg(death_occurrence_id::text, ',') FROM game_character_pending_respawns))",
    )
    .fetch_one(pool)
    .await?)
}

/// The root and typed-state successor of one semantic Character transaction.
fn advance(original: u64, level: i64, experience: i64) -> String {
    let committed = original + 1;
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state \
            SET character_revision = {committed}, level = {level}, total_experience = {experience} \
          WHERE character_id = {character} AND character_revision = {original};",
        character = uuid(CHARACTER),
    )
}

/// An XP-award receipt row, as `commit_character_experience` writes it.
fn xp_receipt(occurrence: u8, original: u64, before: (i64, i64), after: (i64, i64)) -> String {
    format!(
        "INSERT INTO game_character_xp_receipts(\
           reward_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, level_after, \
           experience_before, experience_after, experience_awarded, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
           {level_before}, {level_after}, {experience_before}, {experience_after}, {awarded}, \
           'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1',\
           'policy-1','reward-1', 1);",
        occurrence = uuid(occurrence),
        binding = bytea(&[occurrence; 8]),
        digest = bytea(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        level_before = before.0,
        level_after = after.0,
        experience_before = before.1,
        experience_after = after.1,
        awarded = after.1 - before.1,
    )
}

/// A complete XP-award transaction.
fn award(occurrence: u8, original: u64, before: (i64, i64), after: (i64, i64)) -> String {
    format!(
        "{}{}",
        advance(original, after.0, after.1),
        xp_receipt(occurrence, original, before, after)
    )
}

/// A complete zero-blessing death transaction (0016) with its pending respawn.
fn death(occurrence: u8, original: u64, before: (i64, i64), after: (i64, i64)) -> String {
    format!(
        "{advance}INSERT INTO game_character_death_receipts(\
           death_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, \
           level_after, experience_before, experience_after, experience_lost, \
           blessings_before, blessings_after, amulet_of_loss_item_id, lost_item_ids, \
           death_world_id, death_channel_id, death_spatial_position, death_map_revision, \
           respawn_position, death_policy_revision, profile_revision, ruleset_revision, \
           content_revision, simulation_revision, evidence_revision, declaration_revision, \
           policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
           {level_before}, {level_after}, {experience_before}, {experience_after}, {lost}, \
           '{{}}'::text[], '{{}}'::text[], NULL, ARRAY[]::uuid[], {world}, {channel}, \
           {cell}, 'map-1', {respawn}, 'death-1', 'profile-1', 'ruleset-1', 'content-1', \
           'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 2); \
         INSERT INTO game_character_pending_respawns VALUES ({character}, {occurrence}, {respawn});",
        advance = advance(original, after.0, after.1),
        occurrence = uuid(occurrence),
        binding = bytea(&[occurrence; 64]),
        digest = bytea(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        level_before = before.0,
        level_after = after.0,
        experience_before = before.1,
        experience_after = after.1,
        lost = before.1 - after.1,
        world = uuid(WORLD),
        channel = uuid(CHANNEL),
        cell = bytea(&[1, 2, 3, 7]),
        respawn = bytea(b"temple:thais"),
    )
}

fn consume_pending() -> String {
    format!(
        "DELETE FROM game_character_pending_respawns WHERE character_id = {};",
        uuid(CHARACTER)
    )
}

#[derive(Clone)]
struct Toggle {
    occurrence: u8,
    original: u64,
    level: (i64, i64),
    experience: (i64, i64),
    stance_before: Option<&'static str>,
    stance_after: Option<&'static str>,
    policy_revision: &'static str,
    command_binding: Vec<u8>,
    policy_digest: Vec<u8>,
}

impl Toggle {
    /// A stance toggle at `original` for a Character at `(level, experience)`.
    fn new(
        occurrence: u8,
        original: u64,
        at: (i64, i64),
        before: Option<&'static str>,
        after: Option<&'static str>,
    ) -> Self {
        Self {
            occurrence,
            original,
            level: (at.0, at.0),
            experience: (at.1, at.1),
            stance_before: before,
            stance_after: after,
            policy_revision: "policy-1",
            command_binding: vec![occurrence; 96],
            policy_digest: vec![occurrence; 32],
        }
    }

    fn receipt(&self) -> String {
        format!(
            "INSERT INTO game_character_stance_receipts(\
               stance_occurrence_id, command_binding, policy_digest, character_id, \
               original_character_revision, committed_character_revision, level_before, \
               level_after, experience_before, experience_after, stance_before, stance_after, \
               profile_revision, ruleset_revision, content_revision, simulation_revision, \
               evidence_revision, declaration_revision, policy_revision, reward_revision, \
               committed_at) \
             VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
               {level_before}, {level_after}, {experience_before}, {experience_after}, \
               {stance_before}, {stance_after}, 'profile-1', 'ruleset-1', 'content-1', \
               'simulation-1', 'evidence-1', 'declaration-1', '{policy}', 'reward-1', 3);",
            occurrence = uuid(self.occurrence),
            binding = bytea(&self.command_binding),
            digest = bytea(&self.policy_digest),
            character = uuid(CHARACTER),
            original = self.original,
            committed = self.original + 1,
            level_before = self.level.0,
            level_after = self.level.1,
            experience_before = self.experience.0,
            experience_after = self.experience.1,
            stance_before = key(self.stance_before),
            stance_after = key(self.stance_after),
            policy = self.policy_revision,
        )
    }

    /// The slot row write: insert for the first toggle, update afterwards.
    fn slot(&self) -> String {
        slot_write(self.stance_after, self.original + 1, self.occurrence)
    }

    /// The complete toggle transaction: successor, receipt, slot row.
    fn commit(&self) -> String {
        format!(
            "{}{}{}",
            advance(self.original, self.level.1, self.experience.1),
            self.receipt(),
            self.slot()
        )
    }
}

fn slot_write(stance: Option<&str>, revision: u64, occurrence: u8) -> String {
    format!(
        "INSERT INTO game_character_stance VALUES ({character}, {stance}, {revision}, {occurrence}) \
         ON CONFLICT (character_id) DO UPDATE SET stance_key = EXCLUDED.stance_key, \
           committed_character_revision = EXCLUDED.committed_character_revision, \
           last_stance_occurrence_id = EXCLUDED.last_stance_occurrence_id;",
        character = uuid(CHARACTER),
        stance = key(stance),
        occurrence = uuid(occurrence),
    )
}

fn run<F>(admin: String, tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&sqlx::PgPool) -> TestResult,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin, tag).await?;
            let pool = sqlx::PgPool::connect(&database.url).await?;
            let outcome = async {
                seed_character(&pool).await?;
                body(&pool).await
            }
            .await;
            pool.close().await;
            database.cleanup().await?;
            outcome
        })
}

#[test]
fn stance_receipts_join_one_chain_with_xp_and_death_receipts() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "chain", async |pool| {
        // Before any toggle the slot is empty: no row.
        assert_eq!(
            snapshot(pool).await?,
            "1|1:50:1000".to_owned(),
            "revision one has no receipt and no stance row"
        );
        // XP -> death -> stance on -> XP -> stance switch -> death (keeps the
        // stance) -> stance off -> XP -> stance on again.
        expect_committed(pool, "xp r2", &award(60, 1, (50, 1000), (51, 1150))).await?;
        expect_committed(pool, "death r3", &death(61, 2, (51, 1150), (50, 1035))).await?;
        expect_committed(pool, "respawn r3", &consume_pending()).await?;
        let on = Toggle::new(62, 3, (50, 1035), None, Some("guard"));
        expect_committed(pool, "stance on r4", &on.commit()).await?;
        expect_committed(pool, "xp r5", &award(63, 4, (50, 1035), (50, 1040))).await?;
        let switch = Toggle::new(64, 5, (50, 1040), Some("guard"), Some("offense"));
        expect_committed(pool, "stance switch r6", &switch.commit()).await?;
        expect_committed(pool, "death r7", &death(65, 6, (50, 1040), (49, 900))).await?;
        expect_committed(pool, "respawn r7", &consume_pending()).await?;
        let stance: Option<String> =
            sqlx::query_scalar("SELECT stance_key FROM game_character_stance")
                .fetch_one(pool)
                .await?;
        assert_eq!(stance.as_deref(), Some("offense"), "death keeps the stance");
        let off = Toggle::new(66, 7, (49, 900), Some("offense"), None);
        expect_committed(pool, "stance off r8", &off.commit()).await?;
        expect_committed(pool, "xp r9", &award(67, 8, (49, 900), (49, 950))).await?;
        let again = Toggle::new(68, 9, (49, 950), None, Some("guard"));
        expect_committed(pool, "stance on r10", &again.commit()).await?;

        let chain: Vec<(String, String, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT kind, committed_character_revision::text, level_before, level_after, \
                    experience_before, experience_after FROM ( \
               SELECT 'xp' AS kind, committed_character_revision, level_before, level_after, \
                      experience_before, experience_after FROM game_character_xp_receipts \
               UNION ALL \
               SELECT 'death', committed_character_revision, level_before, level_after, \
                      experience_before, experience_after FROM game_character_death_receipts \
               UNION ALL \
               SELECT 'stance', committed_character_revision, level_before, level_after, \
                      experience_before, experience_after FROM game_character_stance_receipts) c \
             ORDER BY c.committed_character_revision",
        )
        .fetch_all(pool)
        .await?;
        let expected = [
            ("xp", "2", 50, 51, 1000, 1150),
            ("death", "3", 51, 50, 1150, 1035),
            ("stance", "4", 50, 50, 1035, 1035),
            ("xp", "5", 50, 50, 1035, 1040),
            ("stance", "6", 50, 50, 1040, 1040),
            ("death", "7", 50, 49, 1040, 900),
            ("stance", "8", 49, 49, 900, 900),
            ("xp", "9", 49, 49, 900, 950),
            ("stance", "10", 49, 49, 950, 950),
        ];
        assert_eq!(chain.len(), expected.len());
        for (row, want) in chain.iter().zip(expected) {
            assert_eq!(
                (row.0.as_str(), row.1.as_str(), row.2, row.3, row.4, row.5),
                want
            );
        }
        assert_eq!(
            snapshot(pool).await?,
            format!(
                "10|10:49:950|2,5,9|3,7|4:-:guard,6:guard:offense,8:offense:-,10:-:guard|guard:10:{}",
                uuid_text(68)
            )
        );
        Ok(())
    })
}

#[test]
fn stance_row_is_the_projection_of_the_latest_stance_receipt() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "row", async |pool| {
        // Row-only writes, at revision one and after an XP commit.
        expect_rejected(
            pool,
            "row-only insert at revision one",
            &slot_write(Some("guard"), 2, 70),
            "23514",
        )
        .await?;
        expect_committed(pool, "xp r2", &award(71, 1, (50, 1000), (50, 1010))).await?;
        expect_rejected(
            pool,
            "row-only insert without a stance receipt",
            &slot_write(Some("guard"), 2, 71),
            "23514",
        )
        .await?;
        // An XP commit may not create the row either.
        let xp_with_row = format!(
            "{}{}",
            award(72, 2, (50, 1010), (50, 1020)),
            slot_write(Some("guard"), 3, 72)
        );
        expect_rejected(pool, "xp commit creating the row", &xp_with_row, "23514").await?;

        let on = Toggle::new(73, 2, (50, 1010), None, Some("guard"));
        // A receipt without its row, and rows that differ from the receipt.
        let no_row = format!("{}{}", advance(2, 50, 1010), on.receipt());
        expect_rejected(pool, "stance receipt without row", &no_row, "23514").await?;
        for (case, row) in [
            ("row key != receipt", slot_write(Some("offense"), 3, 73)),
            ("row empty != receipt", slot_write(None, 3, 73)),
            ("row revision != receipt", slot_write(Some("guard"), 2, 73)),
            (
                "row occurrence != receipt",
                slot_write(Some("guard"), 3, 71),
            ),
        ] {
            let script = format!("{}{}{row}", advance(2, 50, 1010), on.receipt());
            expect_rejected(pool, case, &script, "23514").await?;
        }
        expect_committed(pool, "stance on r3", &on.commit()).await?;

        // Row-only rewrites of the committed row.
        expect_rejected(
            pool,
            "row-only key update",
            "UPDATE game_character_stance SET stance_key = 'offense'",
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "row-only clear",
            "UPDATE game_character_stance SET stance_key = NULL",
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "row delete",
            "DELETE FROM game_character_stance",
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "row reassigned to another Character",
            &format!(
                "UPDATE game_character_stance SET character_id = {}",
                uuid(99)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "row truncate",
            "TRUNCATE game_character_stance",
            "23514",
        )
        .await?;

        // XP and death commits that change the row fail (keep_on_death).
        let xp_changing_row = format!(
            "{}{}",
            award(74, 3, (50, 1010), (50, 1020)),
            slot_write(None, 4, 74)
        );
        expect_rejected(
            pool,
            "xp commit changing the row",
            &xp_changing_row,
            "23514",
        )
        .await?;
        let death_clearing_row = format!(
            "{}{}",
            death(75, 3, (50, 1010), (50, 900)),
            slot_write(None, 3, 73)
        );
        expect_rejected(
            pool,
            "death commit clearing the row",
            &death_clearing_row,
            "23514",
        )
        .await?;
        // A stance receipt of an earlier toggle cannot re-drive the row: the
        // latest receipt decides.
        let stale_row = format!(
            "{}{}",
            Toggle::new(76, 3, (50, 1010), Some("guard"), Some("offense")).commit(),
            slot_write(Some("guard"), 3, 73)
        );
        expect_rejected(pool, "row left at the older receipt", &stale_row, "23514").await?;

        expect_committed(
            pool,
            "death r4 keeps the row",
            &death(77, 3, (50, 1010), (50, 900)),
        )
        .await?;
        assert_eq!(
            snapshot(pool).await?,
            format!(
                "4|4:50:900|2|4|3:-:guard|guard:3:{}|{}",
                uuid_text(73),
                uuid_text(77)
            )
        );
        Ok(())
    })
}

#[test]
fn stance_receipts_chain_and_advance_exactly_one_revision() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "rules", async |pool| {
        // The first transition starts from an empty slot.
        let from_ghost = Toggle::new(80, 1, (50, 1000), Some("offense"), Some("guard"));
        expect_rejected(
            pool,
            "first stance_before not NULL",
            &from_ghost.commit(),
            "23514",
        )
        .await?;
        // A no-op is not a receipt (CHECK), empty or not.
        let empty_noop = Toggle::new(81, 1, (50, 1000), None, None);
        expect_rejected(pool, "no-op empty toggle", &empty_noop.commit(), "23514").await?;
        // A stance receipt never changes experience or level (CHECK).
        let mut gains = Toggle::new(82, 1, (50, 1000), None, Some("guard"));
        gains.experience = (1000, 1001);
        expect_rejected(
            pool,
            "stance receipt gaining experience",
            &gains.commit(),
            "23514",
        )
        .await?;
        let mut levels = Toggle::new(83, 1, (50, 1000), None, Some("guard"));
        levels.level = (50, 49);
        expect_rejected(
            pool,
            "stance receipt lowering level",
            &levels.commit(),
            "23514",
        )
        .await?;
        // ... nor explains a state successor that does.
        let toggle = Toggle::new(84, 1, (50, 1000), None, Some("guard"));
        let moved = format!(
            "{}{}{}",
            advance(1, 50, 990),
            toggle.receipt(),
            toggle.slot()
        );
        expect_rejected(pool, "state lost experience on a toggle", &moved, "23514").await?;
        // ... nor claims made-up before-values (DEATH-0 transition guard).
        let forged = Toggle::new(85, 1, (50, 990), None, Some("guard"));
        let forged = format!(
            "{}{}{}",
            advance(1, 50, 990),
            forged.receipt(),
            forged.slot()
        );
        expect_rejected(pool, "loss explained by a forged toggle", &forged, "23514").await?;
        // An equal successor with no receipt, and a receipt with no successor.
        expect_rejected(
            pool,
            "equal successor, no receipt",
            &advance(1, 50, 1000),
            "23514",
        )
        .await?;
        // A bare receipt: only its own deferred trigger sees it.
        expect_rejected(pool, "bare stance receipt", &toggle.receipt(), "23514").await?;
        let no_successor = format!("{}{}", toggle.receipt(), toggle.slot());
        expect_rejected(pool, "receipt without successor", &no_successor, "23514").await?;
        let root_only = format!(
            "UPDATE game_character_roots SET character_revision = 2 WHERE character_id = {}; {}{}",
            uuid(CHARACTER),
            toggle.receipt(),
            toggle.slot()
        );
        expect_rejected(pool, "root-only successor", &root_only, "23514").await?;
        // Two receipts of different kinds for one revision.
        let with_xp = format!(
            "{}{}",
            toggle.commit(),
            xp_receipt(86, 1, (50, 1000), (50, 1001))
        );
        expect_rejected(pool, "stance and xp for one revision", &with_xp, "23514").await?;
        // A receipt ahead of the root revision.
        let ahead = format!(
            "{}{}",
            toggle.commit(),
            Toggle::new(87, 2, (50, 1000), Some("guard"), Some("offense")).receipt()
        );
        expect_rejected(pool, "stance receipt ahead of the root", &ahead, "23514").await?;
        // Receipt revision fields must match the state.
        let mut policy = toggle.clone();
        policy.policy_revision = "policy-2";
        expect_rejected(
            pool,
            "receipt revision field != state",
            &policy.commit(),
            "23514",
        )
        .await?;
        // An XP receipt still needs a strict increase (0009 CHECK), so it can
        // never carry an equal successor.
        let flat_xp = format!(
            "{}{}",
            advance(1, 50, 1000),
            xp_receipt(88, 1, (50, 1000), (50, 1000))
        );
        expect_rejected(pool, "xp receipt with equal experience", &flat_xp, "23514").await?;

        expect_committed(pool, "stance on r2", &toggle.commit()).await?;
        // The stance chain: stance_before is the previous stance_after.
        let wrong_before = Toggle::new(89, 2, (50, 1000), Some("offense"), Some("defense"));
        expect_rejected(
            pool,
            "stance_before != previous after",
            &wrong_before.commit(),
            "23514",
        )
        .await?;
        let from_empty = Toggle::new(90, 2, (50, 1000), None, Some("offense"));
        expect_rejected(
            pool,
            "stance_before NULL after guard",
            &from_empty.commit(),
            "23514",
        )
        .await?;
        let noop = Toggle::new(91, 2, (50, 1000), Some("guard"), Some("guard"));
        expect_rejected(pool, "no-op toggle", &noop.commit(), "23514").await?;
        // Cross-kind level/experience chain binds a stance receipt too.
        expect_committed(pool, "xp r3", &award(92, 2, (50, 1000), (50, 1100))).await?;
        let stale = Toggle::new(93, 3, (50, 1000), Some("guard"), Some("offense"));
        let stale = format!(
            "{}{}{}",
            advance(3, 50, 1100),
            stale.receipt(),
            stale.slot()
        );
        expect_rejected(pool, "stance before != xp after", &stale, "23514").await?;
        // A stance toggle may not be replayed as a second revision with the
        // same occurrence (primary key).
        let replay = Toggle::new(84, 3, (50, 1100), Some("guard"), Some("offense"));
        expect_rejected(pool, "occurrence reused", &replay.commit(), "23505").await?;
        let switch = Toggle::new(94, 3, (50, 1100), Some("guard"), Some("offense"));
        expect_committed(pool, "stance switch r4", &switch.commit()).await?;
        Ok(())
    })
}

#[test]
fn stance_receipts_are_bounded_immutable_and_untruncatable() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "immutable", async |pool| {
        let toggle = Toggle::new(100, 1, (50, 1000), None, Some("guard"));
        let mut empty_binding = toggle.clone();
        empty_binding.command_binding = Vec::new();
        let mut long_binding = toggle.clone();
        long_binding.command_binding = vec![1; 1025];
        let mut short_digest = toggle.clone();
        short_digest.policy_digest = vec![1; 31];
        let long_key: &'static str = Box::leak("k".repeat(129).into_boxed_str());
        for (case, receipt) in [
            ("empty command binding", empty_binding),
            ("oversized command binding", long_binding),
            ("short policy digest", short_digest),
            (
                "stance key with a space",
                Toggle::new(100, 1, (50, 1000), None, Some("bad key")),
            ),
            (
                "empty stance key",
                Toggle::new(100, 1, (50, 1000), None, Some("")),
            ),
            (
                "129-byte stance key",
                Toggle::new(100, 1, (50, 1000), None, Some(long_key)),
            ),
        ] {
            expect_rejected(pool, case, &receipt.commit(), "23514").await?;
        }
        // Clear the occurrence's UUID version nibble (7 -> 4).
        let not_v7 = toggle.commit().replace(
            &hex(&id(100)),
            &hex(&[
                100, 2, 3, 4, 5, 6, 0x40, 8, 0x80, 10, 11, 12, 13, 14, 15, 100,
            ]),
        );
        expect_rejected(pool, "occurrence not UUIDv7", &not_v7, "23514").await?;
        let max_key: &'static str = Box::leak("k".repeat(128).into_boxed_str());
        let mut bounded = Toggle::new(100, 1, (50, 1000), None, Some(max_key));
        bounded.command_binding = vec![1; 1024];
        expect_committed(pool, "1024-byte binding, 128-byte key", &bounded.commit()).await?;

        for (case, script) in [
            (
                "stance receipt update",
                "UPDATE game_character_stance_receipts SET committed_at = committed_at + 1",
            ),
            (
                "stance receipt delete",
                "DELETE FROM game_character_stance_receipts",
            ),
            (
                "stance receipt truncate",
                "TRUNCATE game_character_stance_receipts CASCADE",
            ),
            ("stance row truncate", "TRUNCATE game_character_stance"),
        ] {
            expect_rejected(pool, case, script, "23514").await?;
        }
        Ok(())
    })
}

#[test]
fn stance_runtime_grants() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "grants", async |pool| {
        let granted: Vec<String> = sqlx::query_scalar(
            "SELECT r.role || '/' || t.relation || ':' || p.privilege \
               FROM (VALUES ('oteryn_game_runtime'), ('oteryn_game_control')) r(role), \
                    (VALUES ('game_character_stance_receipts'), ('game_character_stance')) \
                      t(relation), \
                    (VALUES ('SELECT'), ('INSERT'), ('UPDATE'), ('DELETE'), ('TRUNCATE')) \
                      p(privilege) \
              WHERE has_table_privilege(r.role, t.relation, p.privilege) \
              ORDER BY r.role || '/' || t.relation || ':' || p.privilege COLLATE \"C\"",
        )
        .fetch_all(pool)
        .await?;
        assert_eq!(
            granted,
            [
                "oteryn_game_control/game_character_stance:SELECT",
                "oteryn_game_control/game_character_stance_receipts:SELECT",
                "oteryn_game_runtime/game_character_stance:INSERT",
                "oteryn_game_runtime/game_character_stance:SELECT",
                "oteryn_game_runtime/game_character_stance:UPDATE",
                "oteryn_game_runtime/game_character_stance_receipts:INSERT",
                "oteryn_game_runtime/game_character_stance_receipts:SELECT",
            ]
        );
        let public: bool = sqlx::query_scalar(
            "SELECT has_function_privilege('public', \
               'game_character_stance_row_guard()', 'EXECUTE')",
        )
        .fetch_one(pool)
        .await?;
        assert!(!public, "PUBLIC cannot execute the stance row guard");
        Ok(())
    })
}
