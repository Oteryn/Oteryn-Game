// Shared DEATH-0 cases (migration 0016). There is no death writer until
// DEATH-1, so every Character transaction here is the exact SQL a writer must
// issue (root successor, state successor, receipt, blessing consumption,
// pending respawn), committed as one PostgreSQL transaction.  The cases need
// no crate code, so any PostgreSQL wrapper target can include this file.

use sqlx::{Connection, Executor};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHARACTER: u8 = 41;
const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const OTHER_WORLD: u8 = 44;

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

fn bytea(bytes: &[u8]) -> String {
    format!("'\\x{}'::bytea", hex(bytes))
}

fn text_array(values: &[&str]) -> String {
    if values.is_empty() {
        return "'{}'::text[]".to_owned();
    }
    let quoted: Vec<String> = values.iter().map(|value| format!("'{value}'")).collect();
    format!("ARRAY[{}]::text[]", quoted.join(","))
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
        let name = format!("cd_{name}_{suffix}");
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
    let mut binding = vec![1];
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
    binding
}

/// One bootstrapped Character with initialized progression at revision one
/// (level 50, 1000 experience), exactly as the P03 cases seed it.
async fn seed_character(pool: &sqlx::PgPool) -> TestResult {
    let binding = bytea(&bootstrap_binding());
    let script = format!(
        "INSERT INTO game_character_interpretations VALUES \
           (1,'profile-1','ruleset-1','content-1','starter-1',1); \
         INSERT INTO game_character_account_guards VALUES ({account}); \
         INSERT INTO game_character_roots VALUES \
           ({character},{account},{world},1,1,'profile-1','ruleset-1','content-1','starter-1'); \
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

/// Every relation DEATH-0 guards, as one comparable value.
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
           (SELECT string_agg(blessing_key, ',' ORDER BY 1) FROM game_character_blessings), \
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

#[derive(Clone)]
struct Death {
    occurrence: u8,
    character: u8,
    original: u64,
    level_before: i64,
    level_after: i64,
    experience_before: i64,
    experience_after: i64,
    experience_lost: i64,
    blessings_before: Vec<&'static str>,
    blessings_after: Vec<&'static str>,
    amulet: Option<u8>,
    lost_items: Vec<u8>,
    world: u8,
    respawn: Vec<u8>,
    policy_revision: &'static str,
    command_binding: Vec<u8>,
    policy_digest: Vec<u8>,
}

impl Death {
    fn new(occurrence: u8, original: u64, before: (i64, i64), after: (i64, i64)) -> Self {
        Self {
            occurrence,
            character: CHARACTER,
            original,
            level_before: before.0,
            level_after: after.0,
            experience_before: before.1,
            experience_after: after.1,
            experience_lost: before.1 - after.1,
            blessings_before: Vec::new(),
            blessings_after: Vec::new(),
            amulet: None,
            lost_items: Vec::new(),
            world: WORLD,
            respawn: b"temple:thais".to_vec(),
            policy_revision: "policy-1",
            command_binding: vec![occurrence; 64],
            policy_digest: vec![occurrence; 32],
        }
    }

    fn blessings(mut self, before: &[&'static str], after: &[&'static str]) -> Self {
        self.blessings_before = before.to_vec();
        self.blessings_after = after.to_vec();
        self
    }

    fn receipt(&self) -> String {
        let lost: Vec<String> = self.lost_items.iter().map(|seed| uuid(*seed)).collect();
        format!(
            "INSERT INTO game_character_death_receipts(\
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
               {blessings_before}, {blessings_after}, {amulet}, ARRAY[{lost_items}]::uuid[], \
               {world}, {channel}, {cell}, 'map-1', {respawn}, 'death-1', 'profile-1', \
               'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', 'declaration-1', \
               '{policy}', 'reward-1', 2);",
            occurrence = uuid(self.occurrence),
            binding = bytea(&self.command_binding),
            digest = bytea(&self.policy_digest),
            character = uuid(self.character),
            original = self.original,
            committed = self.original + 1,
            level_before = self.level_before,
            level_after = self.level_after,
            experience_before = self.experience_before,
            experience_after = self.experience_after,
            lost = self.experience_lost,
            blessings_before = text_array(&self.blessings_before),
            blessings_after = text_array(&self.blessings_after),
            amulet = self.amulet.map_or_else(|| "NULL".to_owned(), uuid),
            lost_items = lost.join(","),
            world = uuid(self.world),
            channel = uuid(CHANNEL),
            cell = bytea(&[1, 2, 3, 7]),
            respawn = bytea(&self.respawn),
            policy = self.policy_revision,
        )
    }

    fn consume(&self) -> String {
        self.blessings_before
            .iter()
            .filter(|key| !self.blessings_after.contains(key))
            .map(|key| {
                format!(
                    "DELETE FROM game_character_blessings \
                      WHERE character_id = {} AND blessing_key = '{key}';",
                    uuid(self.character)
                )
            })
            .collect()
    }

    fn pending(&self) -> String {
        format!(
            "INSERT INTO game_character_pending_respawns VALUES ({}, {}, {});",
            uuid(self.character),
            uuid(self.occurrence),
            bytea(&self.respawn)
        )
    }

    /// The complete death transaction: successor, receipt (before the
    /// blessings it consumes are deleted), consumption, pending respawn.
    fn commit(&self) -> String {
        format!(
            "{}{}{}{}",
            advance(self.original, self.level_after, self.experience_after),
            self.receipt(),
            self.consume(),
            self.pending()
        )
    }
}

fn hold(keys: &[&str]) -> String {
    keys.iter()
        .map(|key| {
            format!(
                "INSERT INTO game_character_blessings VALUES ({}, '{key}', 'test-grant');",
                uuid(CHARACTER)
            )
        })
        .collect()
}

fn consume_pending() -> String {
    format!(
        "DELETE FROM game_character_pending_respawns WHERE character_id = {};",
        uuid(CHARACTER)
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
fn death_receipts_join_one_chain_with_xp_receipts() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "chain", async |pool| {
        // XP (level up) -> death (level down) -> XP -> zero-loss death that
        // consumes regular blessings -> death to zero experience.
        expect_committed(pool, "xp r2", &award(60, 1, (50, 1000), (51, 1150))).await?;
        let first = Death::new(61, 2, (51, 1150), (50, 1035));
        expect_committed(pool, "death r3", &first.commit()).await?;
        let pending: String = sqlx::query_scalar(
            "SELECT encode(respawn_position, 'escape') FROM game_character_pending_respawns \
              WHERE death_occurrence_id = $1::text::uuid",
        )
        .bind(hex(&id(61)))
        .fetch_one(pool)
        .await?;
        assert_eq!(pending, "temple:thais");
        expect_committed(pool, "respawn consumes r3", &consume_pending()).await?;
        expect_committed(pool, "xp r4", &award(62, 3, (50, 1035), (50, 1040))).await?;
        expect_committed(pool, "hold blessings", &hold(&["spark", "twist", "wisdom"])).await?;
        let blessed = Death::new(63, 4, (50, 1040), (50, 1040))
            .blessings(&["spark", "twist", "wisdom"], &["twist"]);
        expect_committed(pool, "zero-loss death r5", &blessed.commit()).await?;
        expect_committed(pool, "respawn consumes r5", &consume_pending()).await?;
        let floor = Death::new(64, 5, (50, 1040), (1, 0)).blessings(&["twist"], &["twist"]);
        expect_committed(pool, "death to zero r6", &floor.commit()).await?;

        let chain: Vec<(String, String, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT kind, committed_character_revision::text, level_before, level_after, \
                    experience_before, experience_after FROM ( \
               SELECT 'xp' AS kind, committed_character_revision, level_before, level_after, \
                      experience_before, experience_after FROM game_character_xp_receipts \
               UNION ALL \
               SELECT 'death', committed_character_revision, level_before, level_after, \
                      experience_before, experience_after FROM game_character_death_receipts) c \
             ORDER BY committed_character_revision",
        )
        .fetch_all(pool)
        .await?;
        let expected = [
            ("xp", "2", 50, 51, 1000, 1150),
            ("death", "3", 51, 50, 1150, 1035),
            ("xp", "4", 50, 50, 1035, 1040),
            ("death", "5", 50, 50, 1040, 1040),
            ("death", "6", 50, 1, 1040, 0),
        ];
        assert_eq!(chain.len(), expected.len());
        for (row, want) in chain.iter().zip(expected) {
            assert_eq!(
                (row.0.as_str(), row.1.as_str(), row.2, row.3, row.4, row.5),
                want
            );
        }
        let lost: Vec<i64> = sqlx::query_scalar(
            "SELECT experience_lost FROM game_character_death_receipts \
              ORDER BY committed_character_revision",
        )
        .fetch_all(pool)
        .await?;
        assert_eq!(lost, [115, 0, 1040]);
        let consumed: Vec<String> = sqlx::query_scalar(
            "SELECT array_to_string(blessings_before, ',') || '>' \
                    || array_to_string(blessings_after, ',') \
               FROM game_character_death_receipts ORDER BY committed_character_revision",
        )
        .fetch_all(pool)
        .await?;
        assert_eq!(consumed, [">", "spark,twist,wisdom>twist", "twist>twist"]);
        assert_eq!(
            snapshot(pool).await?,
            format!("6|6:1:0|2,4|3,5,6|twist|{}", uuid_text(64))
        );
        Ok(())
    })
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

#[test]
fn a_death_cannot_masquerade_as_an_award_nor_the_reverse() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "masquerade", async |pool| {
        // An XP receipt can never record a loss.
        let lowered = format!(
            "{}{}",
            advance(1, 50, 900),
            xp_receipt(70, 1, (50, 1000), (50, 900))
        );
        expect_rejected(pool, "xp receipt recording a loss", &lowered, "23514").await?;
        // ... nor claim a made-up predecessor to look like an increase. The
        // first receipt has no predecessor receipt; the state transition binds it.
        let forged_award = format!(
            "{}{}",
            advance(1, 50, 900),
            xp_receipt(71, 1, (50, 800), (50, 900))
        );
        expect_rejected(
            pool,
            "loss explained by a forged award",
            &forged_award,
            "23514",
        )
        .await?;
        // A death receipt can never record a gain, in experience or level.
        let raised = Death::new(72, 1, (50, 1000), (50, 1100)).commit();
        expect_rejected(pool, "death receipt recording a gain", &raised, "23514").await?;
        let level_up = Death::new(73, 1, (50, 1000), (51, 1000)).commit();
        expect_rejected(pool, "death receipt raising level", &level_up, "23514").await?;
        // ... nor claim a made-up predecessor to look like a loss.
        let forged_death = Death::new(74, 1, (50, 1200), (50, 1100));
        let forged_death = format!(
            "{}{}{}",
            advance(1, 50, 1100),
            forged_death.receipt(),
            forged_death.pending()
        );
        expect_rejected(
            pool,
            "gain explained by a forged death",
            &forged_death,
            "23514",
        )
        .await?;
        // Mixed direction successors fit neither kind (state guard).
        expect_rejected(
            pool,
            "experience up, level down",
            &format!(
                "{}{}",
                advance(1, 49, 1100),
                xp_receipt(75, 1, (50, 1000), (49, 1100))
            ),
            "23514",
        )
        .await?;
        let up_level = Death::new(76, 1, (50, 1000), (51, 900));
        expect_rejected(
            pool,
            "experience down, level up",
            &up_level.commit(),
            "23514",
        )
        .await?;

        // The same after one committed receipt of each kind: the predecessor
        // chain binds `before` across kinds.
        expect_committed(pool, "xp r2", &award(77, 1, (50, 1000), (50, 1050))).await?;
        let death = Death::new(78, 2, (50, 1050), (50, 1000));
        expect_committed(pool, "death r3", &death.commit()).await?;
        expect_committed(pool, "respawn", &consume_pending()).await?;
        let across = format!(
            "{}{}",
            advance(3, 50, 1010),
            xp_receipt(79, 3, (50, 1005), (50, 1010))
        );
        expect_rejected(pool, "xp before != death after", &across, "23514").await?;
        let across_death = Death::new(80, 3, (50, 1001), (50, 990));
        expect_rejected(
            pool,
            "death before != death after",
            &across_death.commit(),
            "23514",
        )
        .await?;
        expect_committed(pool, "xp r4", &award(81, 3, (50, 1000), (50, 1010))).await?;
        Ok(())
    })
}

#[test]
fn every_revision_has_exactly_one_matching_receipt() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "exactlyone", async |pool| {
        let death = Death::new(90, 1, (50, 1000), (50, 950));
        // Missing receipt: a lowered successor with no receipt of either kind.
        expect_rejected(pool, "no receipt", &advance(1, 50, 950), "23514").await?;
        // Receipt without the revision successor.
        let no_successor = format!("{}{}", death.receipt(), death.pending());
        expect_rejected(pool, "receipt without successor", &no_successor, "23514").await?;
        // Receipt without the typed-state successor (root only).
        let root_only = format!(
            "UPDATE game_character_roots SET character_revision = 2 WHERE character_id = {}; {}{}",
            uuid(CHARACTER),
            death.receipt(),
            death.pending()
        );
        expect_rejected(pool, "root-only successor", &root_only, "23514").await?;
        // Two receipts, one of each kind, for one revision.
        let duplicate = format!(
            "{}{}",
            death.commit(),
            xp_receipt(91, 1, (50, 1000), (50, 1001))
        );
        expect_rejected(pool, "xp and death for one revision", &duplicate, "23514").await?;
        // A receipt ahead of the root revision.
        let ahead = format!(
            "{}{}",
            death.commit(),
            xp_receipt(92, 2, (50, 950), (50, 960))
        );
        expect_rejected(pool, "receipt ahead of the root", &ahead, "23514").await?;
        // Receipt does not match the committed state.
        let mismatch = format!(
            "{}{}{}",
            advance(1, 50, 940),
            death.receipt(),
            death.pending()
        );
        expect_rejected(pool, "receipt experience != state", &mismatch, "23514").await?;
        let level = Death::new(93, 1, (50, 1000), (49, 950));
        let level = format!(
            "{}{}{}",
            advance(1, 50, 950),
            level.receipt(),
            level.pending()
        );
        expect_rejected(pool, "receipt level != state", &level, "23514").await?;
        let mut policy = death.clone();
        policy.policy_revision = "policy-2";
        expect_rejected(
            pool,
            "receipt revision field != state",
            &policy.commit(),
            "23514",
        )
        .await?;
        // Every rejected shape left revision one intact; the exact death commits.
        expect_committed(pool, "death r2", &death.commit()).await?;
        Ok(())
    })
}

#[test]
fn death_outcome_binds_blessings_pending_respawn_and_cell() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "outcome", async |pool| {
        let death = Death::new(100, 1, (50, 1000), (50, 950));
        // The death's own pending respawn is required, at its receipt position.
        let no_pending = format!("{}{}", advance(1, 50, 950), death.receipt());
        expect_rejected(pool, "no pending respawn", &no_pending, "23514").await?;
        let moved = format!(
            "{}{}INSERT INTO game_character_pending_respawns VALUES ({}, {}, {});",
            advance(1, 50, 950),
            death.receipt(),
            uuid(CHARACTER),
            uuid(100),
            bytea(b"temple:carlin")
        );
        expect_rejected(pool, "pending respawn elsewhere", &moved, "23514").await?;
        // The death cell is in the Character's World.
        let mut foreign = death.clone();
        foreign.world = OTHER_WORLD;
        expect_rejected(
            pool,
            "death cell in another World",
            &foreign.commit(),
            "23514",
        )
        .await?;
        // DEATH-3 delivery gap: no amulet selection and no lost item yet.
        let mut amulet = death.clone();
        amulet.amulet = Some(101);
        expect_rejected(pool, "amulet before DEATH-3", &amulet.commit(), "23514").await?;
        let mut lost = death.clone();
        lost.lost_items = vec![102];
        expect_rejected(pool, "lost item before DEATH-3", &lost.commit(), "23514").await?;

        expect_committed(pool, "hold blessings", &hold(&["spark", "twist"])).await?;
        // blessings_before must be the held set.
        let understated = death.clone().blessings(&["spark"], &[]);
        expect_rejected(
            pool,
            "before omits a held blessing",
            &understated.commit(),
            "23514",
        )
        .await?;
        let overstated = death
            .clone()
            .blessings(&["spark", "twist", "wisdom"], &["twist"]);
        expect_rejected(
            pool,
            "before invents a blessing",
            &overstated.commit(),
            "23514",
        )
        .await?;
        // The held set after commit must be blessings_after.
        let kept = death.clone().blessings(&["spark", "twist"], &["twist"]);
        let unconsumed = format!(
            "{}{}{}",
            advance(1, 50, 950),
            kept.receipt(),
            kept.pending()
        );
        expect_rejected(pool, "regular blessing not consumed", &unconsumed, "23514").await?;
        // A blessing is only consumed by this transaction's death receipt,
        // and only when that receipt records it as consumed.
        let orphan = format!(
            "DELETE FROM game_character_blessings WHERE character_id = {};",
            uuid(CHARACTER)
        );
        expect_rejected(pool, "blessing deleted without a death", &orphan, "23514").await?;
        let kept_twist = format!(
            "{}DELETE FROM game_character_blessings WHERE blessing_key = 'twist';",
            kept.commit()
        );
        expect_rejected(
            pool,
            "blessing kept by receipt deleted",
            &kept_twist,
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "blessing rewritten",
            "UPDATE game_character_blessings SET provenance = 'other'",
            "23514",
        )
        .await?;
        // A death only consumes: after must be within before.
        let widened = death
            .clone()
            .blessings(&["spark", "twist"], &["twist", "wisdom"]);
        expect_rejected(pool, "after not within before", &widened.commit(), "23514").await?;
        expect_committed(pool, "death r2", &kept.commit()).await?;

        // A second death cannot commit while the first respawn is pending.
        let second = Death::new(103, 2, (50, 950), (50, 900)).blessings(&["twist"], &["twist"]);
        expect_rejected(
            pool,
            "death while respawn pending",
            &second.commit(),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "pending respawn rewritten",
            &format!(
                "UPDATE game_character_pending_respawns SET respawn_position = {}",
                bytea(b"temple:carlin")
            ),
            "23514",
        )
        .await?;
        expect_committed(pool, "respawn consumes r2", &consume_pending()).await?;
        // A consumed respawn is never recreated for its committed death.
        expect_rejected(pool, "respawn recreated", &kept.pending(), "23514").await?;
        expect_committed(pool, "death r3 after respawn", &second.commit()).await?;
        Ok(())
    })
}

#[test]
fn death_receipts_are_bounded_immutable_and_untruncatable() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "immutable", async |pool| {
        let death = Death::new(110, 1, (50, 1000), (50, 950));
        let mut empty_binding = death.clone();
        empty_binding.command_binding = Vec::new();
        let mut long_binding = death.clone();
        long_binding.command_binding = vec![1; 1025];
        let mut short_digest = death.clone();
        short_digest.policy_digest = vec![1; 31];
        let mut wrong_lost = death.clone();
        wrong_lost.experience_lost = 49;
        let mut negative = Death::new(110, 1, (50, 1000), (50, -1));
        negative.experience_lost = 1001;
        let mut long_respawn = death.clone();
        long_respawn.respawn = vec![b'x'; 129];
        for (case, receipt) in [
            ("empty command binding", &empty_binding),
            ("oversized command binding", &long_binding),
            ("short policy digest", &short_digest),
            ("experience_lost != before - after", &wrong_lost),
            ("negative experience", &negative),
            ("oversized respawn position", &long_respawn),
        ] {
            expect_rejected(pool, case, &receipt.commit(), "23514").await?;
        }
        // Clear the occurrence's UUID version nibble (7 -> 4).
        let not_v7 = death.receipt().replacen(
            &hex(&id(110)),
            &hex(&[
                110, 2, 3, 4, 5, 6, 0x40, 8, 0x80, 10, 11, 12, 13, 14, 15, 110,
            ]),
            1,
        );
        expect_rejected(
            pool,
            "occurrence not UUIDv7",
            &format!("{}{}", advance(1, 50, 950), not_v7),
            "23514",
        )
        .await?;
        let mut oversized_binding_ok = death.clone();
        oversized_binding_ok.command_binding = vec![1; 1024];
        expect_committed(pool, "1024-byte binding", &oversized_binding_ok.commit()).await?;

        for (case, script) in [
            (
                "death receipt update",
                "UPDATE game_character_death_receipts SET committed_at = committed_at + 1",
            ),
            (
                "death receipt delete",
                "DELETE FROM game_character_death_receipts",
            ),
            (
                "death receipt truncate",
                "TRUNCATE game_character_death_receipts CASCADE",
            ),
            ("blessings truncate", "TRUNCATE game_character_blessings"),
            (
                "pending respawns truncate",
                "TRUNCATE game_character_pending_respawns",
            ),
        ] {
            expect_rejected(pool, case, script, "23514").await?;
        }
        Ok(())
    })
}

#[test]
fn runtime_grants_and_canonical_blessing_sets() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    run(admin, "grants", async |pool| {
        let granted: Vec<String> = sqlx::query_scalar(
            "SELECT r.role || '/' || t.relation || ':' || p.privilege \
               FROM (VALUES ('oteryn_game_runtime'), ('oteryn_game_control')) r(role), \
                    (VALUES ('game_character_death_receipts'), ('game_character_blessings'), \
                            ('game_character_pending_respawns')) t(relation), \
                    (VALUES ('SELECT'), ('INSERT'), ('UPDATE'), ('DELETE'), ('TRUNCATE')) \
                      p(privilege) \
              WHERE has_table_privilege(r.role, t.relation, p.privilege) \
              ORDER BY 1",
        )
        .fetch_all(pool)
        .await?;
        // Blessing INSERT belongs to DEATH-4's receipt kind, not DEATH-0.
        assert_eq!(
            granted,
            [
                "oteryn_game_control/game_character_blessings:SELECT",
                "oteryn_game_control/game_character_death_receipts:SELECT",
                "oteryn_game_control/game_character_pending_respawns:SELECT",
                "oteryn_game_runtime/game_character_blessings:DELETE",
                "oteryn_game_runtime/game_character_blessings:SELECT",
                "oteryn_game_runtime/game_character_death_receipts:INSERT",
                "oteryn_game_runtime/game_character_death_receipts:SELECT",
                "oteryn_game_runtime/game_character_pending_respawns:DELETE",
                "oteryn_game_runtime/game_character_pending_respawns:INSERT",
                "oteryn_game_runtime/game_character_pending_respawns:SELECT",
            ]
        );
        let long_key = "k".repeat(129);
        let full: Vec<String> = (0..32).map(|index| format!("'b{index:02}'")).collect();
        let over: Vec<String> = (0..33).map(|index| format!("'b{index:02}'")).collect();
        for (set, valid) in [
            ("'{}'::text[]".to_owned(), true),
            ("ARRAY['spark','twist']".to_owned(), true),
            ("ARRAY['Twist','spark']".to_owned(), true),
            (format!("ARRAY[{}]", full.join(",")), true),
            ("ARRAY['twist','spark']".to_owned(), false),
            ("ARRAY['spark','Twist']".to_owned(), false),
            ("ARRAY['spark','spark']".to_owned(), false),
            ("ARRAY['spark',NULL]".to_owned(), false),
            ("ARRAY['bad key']".to_owned(), false),
            ("ARRAY['']".to_owned(), false),
            (format!("ARRAY['{long_key}']"), false),
            (format!("ARRAY[{}]", over.join(",")), false),
            ("ARRAY[['a'],['b']]".to_owned(), false),
            ("'[0:0]={spark}'::text[]".to_owned(), false),
        ] {
            let observed: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT coalesce(game_character_is_blessing_set({set}), false)"
            )))
            .fetch_one(pool)
            .await?;
            assert_eq!(observed, valid, "blessing set {set}");
        }
        Ok(())
    })
}
