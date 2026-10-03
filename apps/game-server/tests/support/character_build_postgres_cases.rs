// CHAR-BUILD-1a cases (migration 0030, A13 §4.1-§4.2 and §4.6 as amended by SKILLS-0
// §3.1-§3.2 and §3.6). The guard cases issue the exact SQL a writer must (root successor, state
// successor, receipt, build row and, for a prune, the stance row), committed as one PostgreSQL
// transaction on the CHARM-2 harness, so every guard branch is reached independently of the
// writer. The admission verifier (#1271 F4) runs through `open_character_authority`. The
// CHAR-BUILD-1b writer, reconcile and admission load (`durability::character_build`) run in
// `build_writer_is_fenced_replayed_and_reconciled`.

use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
    DurableBuildState, skill_tries_required,
};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::foundation::ConnectionGeneration;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn bytea(bytes: &[u8]) -> String {
    format!("'\\x{}'::bytea", hex(bytes))
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

fn key(value: Option<&str>) -> String {
    value.map_or_else(|| "NULL".to_owned(), |value| format!("'{value}'"))
}

const SKILLS: [&str; 7] = [
    "fist",
    "club",
    "sword",
    "axe",
    "distance",
    "shielding",
    "fishing",
];

/// One build state: vocation, magic level as (`magic_level`, `mana_spent`) and the seven skills
/// as (`level`, `tries`), in the column order of 0030.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Build {
    vocation: &'static str,
    families: [(i32, i64); 8],
}

impl Build {
    /// The chain seed: no row means exactly this.
    const SEED: Self = Self {
        vocation: "none",
        families: [
            (0, 0),
            (10, 0),
            (10, 0),
            (10, 0),
            (10, 0),
            (10, 0),
            (10, 0),
            (10, 0),
        ],
    };

    fn with(self, family: usize, value: (i32, i64)) -> Self {
        let mut next = self;
        next.families[family] = value;
        next
    }

    fn vocation(self, vocation: &'static str) -> Self {
        Self { vocation, ..self }
    }

    fn columns(suffix: &str) -> String {
        let mut columns = vec![
            format!("magic_level{suffix}"),
            format!("mana_spent{suffix}"),
        ];
        for skill in SKILLS {
            columns.push(format!("{skill}_level{suffix}"));
            columns.push(format!("{skill}_tries{suffix}"));
        }
        columns.join(", ")
    }

    fn values(&self) -> String {
        self.families
            .iter()
            .map(|(level, progress)| format!("{level}, {progress}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The root and typed-state successor of one semantic Character transaction.
fn advance(original: u64, level: i64, experience: i64) -> String {
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state \
            SET character_revision = {committed}, level = {level}, total_experience = {experience} \
          WHERE character_id = {character} AND character_revision = {original};",
        committed = original + 1,
        character = uuid(CHARACTER),
    )
}

/// A build receipt row for a Character at (`level`, `experience`).
struct Change {
    occurrence: u8,
    original: u64,
    at: (i64, i64),
    cause: &'static str,
    before: Build,
    after: Build,
    stance: (Option<&'static str>, Option<&'static str>),
}

impl Change {
    fn new(
        occurrence: u8,
        original: u64,
        cause: &'static str,
        before: Build,
        after: Build,
    ) -> Self {
        Self {
            occurrence,
            original,
            at: (50, 1000),
            cause,
            before,
            after,
            stance: (None, None),
        }
    }

    fn receipt(&self) -> String {
        format!(
            "INSERT INTO game_character_build_receipts(\
               build_occurrence_id, command_binding, policy_digest, character_id, \
               original_character_revision, committed_character_revision, cause, level_before, \
               level_after, experience_before, experience_after, vocation_before, {before_columns}, \
               vocation_after, {after_columns}, stance_before, stance_after, profile_revision, \
               ruleset_revision, content_revision, simulation_revision, evidence_revision, \
               declaration_revision, policy_revision, reward_revision, committed_at) \
             VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
               '{cause}', {level}, {level}, {experience}, {experience}, '{vocation_before}', \
               {before}, '{vocation_after}', {after}, {stance_before}, {stance_after}, \
               'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
               'declaration-1', 'policy-1', 'reward-1', 4);",
            before_columns = Build::columns("_before"),
            after_columns = Build::columns("_after"),
            occurrence = uuid(self.occurrence),
            binding = bytea(&[self.occurrence; 33]),
            digest = bytea(&[self.occurrence; 32]),
            character = uuid(CHARACTER),
            original = self.original,
            committed = self.original + 1,
            cause = self.cause,
            level = self.at.0,
            experience = self.at.1,
            vocation_before = self.before.vocation,
            before = self.before.values(),
            vocation_after = self.after.vocation,
            after = self.after.values(),
            stance_before = key(self.stance.0),
            stance_after = key(self.stance.1),
        )
    }

    /// The complete build transaction: successor, receipt, build row and, for a prune, the
    /// stance row.
    fn commit(&self) -> String {
        let mut script = format!(
            "{}{}{}",
            advance(self.original, self.at.0, self.at.1),
            self.receipt(),
            row_write(&self.after, self.original + 1, self.occurrence)
        );
        if self.stance.0 != self.stance.1 {
            script.push_str(&stance_row(
                self.stance.1,
                self.original + 1,
                self.occurrence,
            ));
        }
        script
    }
}

fn row_write(build: &Build, revision: u64, occurrence: u8) -> String {
    format!(
        "INSERT INTO game_character_build_state(character_id, vocation, {columns}, \
           committed_character_revision, last_build_occurrence_id) \
         VALUES ({character}, '{vocation}', {values}, {revision}, {occurrence}) \
         ON CONFLICT (character_id) DO UPDATE SET vocation = EXCLUDED.vocation, \
           ({columns}, committed_character_revision, last_build_occurrence_id) = \
           (SELECT {excluded}, EXCLUDED.committed_character_revision, \
                   EXCLUDED.last_build_occurrence_id);",
        columns = Build::columns(""),
        character = uuid(CHARACTER),
        vocation = build.vocation,
        values = build.values(),
        occurrence = uuid(occurrence),
        excluded = Build::columns("")
            .split(", ")
            .map(|column| format!("EXCLUDED.{column}"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn stance_row(stance: Option<&str>, revision: u64, occurrence: u8) -> String {
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

/// A stance toggle transaction (0017) at level 50 and 1000 experience.
fn toggle(occurrence: u8, original: u64, after: Option<&str>) -> String {
    format!(
        "{advance}INSERT INTO game_character_stance_receipts(\
           stance_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, \
           level_after, experience_before, experience_after, stance_before, stance_after, \
           profile_revision, ruleset_revision, content_revision, simulation_revision, \
           evidence_revision, declaration_revision, policy_revision, reward_revision, \
           committed_at) \
         VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
           50, 50, 1000, 1000, NULL, {after}, 'profile-1', 'ruleset-1', 'content-1', \
           'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 3);{row}",
        advance = advance(original, 50, 1000),
        occurrence = uuid(occurrence),
        binding = bytea(&[occurrence; 8]),
        digest = bytea(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        after = key(after),
        row = stance_row(after, original + 1, occurrence),
    )
}

/// An XP award transaction from (50, `before`) to (50, `after`) experience.
fn award(occurrence: u8, original: u64, before: i64, after: i64) -> String {
    format!(
        "{advance}INSERT INTO game_character_xp_receipts(\
           reward_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, level_after, \
           experience_before, experience_after, experience_awarded, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
           50, 50, {before}, {after}, {awarded}, 'profile-1','ruleset-1','content-1',\
           'simulation-1','evidence-1','declaration-1','policy-1','reward-1', 1);",
        advance = advance(original, 50, after),
        occurrence = uuid(occurrence),
        binding = bytea(&[occurrence; 8]),
        digest = bytea(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        awarded = after - before,
    )
}

/// A zero-blessing death from (50, `experience.0`) to (50, `experience.1`) with its pending
/// respawn, optionally carrying build fields (before, after). The caller writes the build row.
fn death(
    occurrence: u8,
    original: u64,
    experience: (i64, i64),
    build: Option<(Build, Build)>,
) -> String {
    let (vocation, before, after) = match build {
        Some((before, after)) => (
            format!("'{}'", before.vocation),
            before.values(),
            after.values(),
        ),
        None => {
            let nulls = vec!["NULL"; 16].join(", ");
            ("NULL".to_owned(), nulls.clone(), nulls)
        }
    };
    format!(
        "{advance}INSERT INTO game_character_death_receipts(\
           death_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, \
           level_after, experience_before, experience_after, experience_lost, \
           blessings_before, blessings_after, amulet_of_loss_item_id, lost_item_ids, \
           death_world_id, death_channel_id, death_spatial_position, death_map_revision, \
           respawn_position, death_policy_revision, profile_revision, ruleset_revision, \
           content_revision, simulation_revision, evidence_revision, declaration_revision, \
           policy_revision, reward_revision, committed_at, vocation, {before_columns}, \
           {after_columns}) \
         VALUES ({occurrence}, {binding}, {digest}, {character}, {original}, {committed}, \
           50, 50, {experience_before}, {experience_after}, {lost}, \
           '{{}}'::text[], '{{}}'::text[], NULL, ARRAY[]::uuid[], {world}, {channel}, \
           {cell}, 'map-1', {respawn}, 'death-1', 'profile-1', 'ruleset-1', 'content-1', \
           'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 2, \
           {vocation}, {before}, {after}); \
         INSERT INTO game_character_pending_respawns VALUES ({character}, {occurrence}, {respawn});",
        advance = advance(original, 50, experience.1),
        before_columns = Build::columns("_before"),
        after_columns = Build::columns("_after"),
        occurrence = uuid(occurrence),
        binding = bytea(&[occurrence; 64]),
        digest = bytea(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        experience_before = experience.0,
        experience_after = experience.1,
        lost = experience.0 - experience.1,
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

/// Runs `script` as one transaction and commits it, so immediate and deferred guard failures
/// both surface here.
async fn attempt(pool: &sqlx::PgPool, script: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

async fn expect_committed(pool: &sqlx::PgPool, case: &str, script: &str) -> TestResult {
    attempt(pool, script)
        .await
        .map_err(|error| format!("{case}: expected commit, got {error}").into())
}

/// Every relation 0030 binds, as one comparable value.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(character_revision::text, ',') FROM game_character_roots), \
           (SELECT string_agg(character_revision::text, ',') \
              FROM game_character_progression_state), \
           (SELECT string_agg(committed_character_revision::text, ',' ORDER BY 1) \
              FROM game_character_build_receipts), \
           (SELECT string_agg(committed_character_revision::text, ',' ORDER BY 1) \
              FROM game_character_death_receipts), \
           (SELECT string_agg(concat_ws(':', vocation, magic_level, mana_spent, sword_level, \
                     sword_tries, committed_character_revision, last_build_occurrence_id), ',') \
              FROM game_character_build_state), \
           (SELECT string_agg(concat_ws(':', coalesce(stance_key, '-'), \
                     committed_character_revision), ',') FROM game_character_stance))",
    )
    .fetch_one(pool)
    .await?)
}

/// The rejection must be the named guard message (or CHECK constraint) and change nothing.
async fn expect_rejected(pool: &sqlx::PgPool, case: &str, script: &str, rule: &str) -> TestResult {
    let before = snapshot(pool).await?;
    let error = attempt(pool, script)
        .await
        .err()
        .ok_or_else(|| format!("{case}: expected rejection"))?;
    let database = error
        .as_database_error()
        .ok_or_else(|| format!("{case}: not a database error: {error}"))?;
    assert_eq!(database.code().as_deref(), Some("23514"), "{case}: {error}");
    let observed = database.constraint().unwrap_or(database.message());
    assert_eq!(observed, rule, "{case}");
    assert_eq!(
        snapshot(pool).await?,
        before,
        "{case}: rejected write changed state"
    );
    Ok(())
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}

const INITIAL: &str = "initial Character progression is inconsistent";
const CHAIN: &str = "Character progression revision/receipt chain is inconsistent";
const BUILD: &str = "Character build state is inconsistent with its build receipts";
const STANCE: &str = "Character stance slot is inconsistent with its stance receipts";
const DIRECTION: &str = "game_character_build_receipts_cause_direction";

#[test]
fn build_receipts_join_the_chain_and_the_row_follows_the_latest() -> TestResult {
    run("build_chain", async |harness| {
        let pool = &harness.pool;
        // Revision one: no build row and no build receipt, whatever the values.
        assert_eq!(snapshot(pool).await?, "1|1");
        expect_rejected(
            pool,
            "row-only r1",
            &row_write(&Build::SEED, 2, 60),
            INITIAL,
        )
        .await?;
        let knight = Build::SEED.vocation("knight");
        let receipt_only = Change::new(60, 1, "vocation_choice", Build::SEED, knight).receipt();
        expect_rejected(pool, "receipt at r1", &receipt_only, INITIAL).await?;

        // r2 stance on; r3 the Dawnport choice prunes it in the same receipt (A13 §4.4).
        expect_committed(pool, "stance r2", &toggle(61, 1, Some("guard"))).await?;
        let mut choice = Change::new(62, 2, "vocation_choice", Build::SEED, knight);
        choice.stance = (Some("guard"), None);
        expect_committed(pool, "choice r3", &choice.commit()).await?;
        expect_committed(pool, "xp r4", &award(63, 3, 1000, 1100)).await?;
        // r5 training advances magic level and sword in one receipt (SKILLS-0 §3.4).
        let trained = knight.with(0, (1, 20)).with(3, (12, 7));
        let mut training = Change::new(64, 4, "training", knight, trained);
        training.at = (50, 1100);
        expect_committed(pool, "training r5", &training.commit()).await?;
        // r6 a death takes progress in its own receipt and updates the row (A13 §4.6).
        let lost = trained.with(0, (0, 90)).with(3, (11, 400));
        let with_loss = format!(
            "{}{}",
            death(65, 5, (1100, 1000), Some((trained, lost))),
            row_write(&lost, 6, 65)
        );
        expect_committed(pool, "death with loss r6", &with_loss).await?;
        expect_committed(pool, "respawn", &consume_pending()).await?;
        // r7 a death without build fields leaves the row alone.
        expect_committed(pool, "death r7", &death(66, 6, (1000, 950), None)).await?;
        expect_committed(pool, "respawn", &consume_pending()).await?;
        // r8 promotion keeps every family (SKILLS-0 §3.2).
        let elite = lost.vocation("elite_knight");
        let mut promotion = Change::new(67, 7, "promotion", lost, elite);
        promotion.at = (50, 950);
        expect_committed(pool, "promotion r8", &promotion.commit()).await?;

        assert_eq!(
            snapshot(pool).await?,
            format!(
                "8|8|3,5,8|6,7|elite_knight:0:90:11:400:8:{}|-:3",
                uuid_text(67)
            )
        );
        // The admission verifier accepts the chain.
        let seal = harness.recovery.seal_current().map_err(debug)?;
        harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn build_guard_rejects_each_inconsistent_write() -> TestResult {
    run("build_guard", async |harness| {
        let pool = &harness.pool;
        let knight = Build::SEED.vocation("knight");
        // The first build-carrying receipt starts from the seed.
        let from_knight = Change::new(60, 1, "training", knight, knight.with(0, (0, 5)));
        expect_rejected(pool, "first not from seed", &from_knight.commit(), BUILD).await?;
        let choice = Change::new(61, 1, "vocation_choice", Build::SEED, knight);
        expect_rejected(
            pool,
            "receipt without row",
            &format!("{}{}", advance(1, 50, 1000), choice.receipt()),
            BUILD,
        )
        .await?;
        expect_committed(pool, "choice r2", &choice.commit()).await?;

        // A row-only write after revision one.
        let bumped = format!(
            "UPDATE game_character_build_state SET magic_level = 1 WHERE character_id = {};",
            uuid(CHARACTER)
        );
        expect_rejected(pool, "row-only update", &bumped, BUILD).await?;
        // Another kind that changes the row.
        expect_rejected(
            pool,
            "xp changes the row",
            &format!(
                "{}{}",
                award(62, 2, 1000, 1100),
                row_write(&knight.with(0, (1, 0)), 3, 62)
            ),
            BUILD,
        )
        .await?;
        // A receipt whose `before` is not the previous `after`.
        let skipped = Change::new(
            63,
            2,
            "training",
            knight.with(0, (0, 5)),
            knight.with(0, (0, 9)),
        );
        expect_rejected(pool, "broken build chain", &skipped.commit(), BUILD).await?;
        // A death whose build `before` is not the latest `after`.
        let wrong_death = format!(
            "{}{}",
            death(
                64,
                2,
                (1000, 900),
                Some((knight.with(0, (2, 0)), knight.with(0, (1, 0))))
            ),
            row_write(&knight.with(0, (1, 0)), 3, 64)
        );
        expect_rejected(pool, "death off the chain", &wrong_death, BUILD).await?;
        // A build receipt whose level does not match the state it explains.
        let mut elsewhere = Change::new(65, 2, "training", knight, knight.with(0, (0, 5)));
        elsewhere.at = (50, 1000);
        let moved =
            elsewhere
                .commit()
                .replacen("total_experience = 1000", "total_experience = 1001", 1);
        expect_rejected(pool, "state moved under a build receipt", &moved, CHAIN).await?;
        // A prune without a stance to prune, and a prune that leaves the stance row.
        let mut prune = Change::new(66, 2, "promotion", knight, knight.vocation("elite_knight"));
        prune.stance = (Some("guard"), None);
        expect_rejected(pool, "prune without a stance", &prune.commit(), STANCE).await?;
        expect_committed(pool, "stance r3", &toggle(67, 2, Some("guard"))).await?;
        let mut stale_slot =
            Change::new(68, 3, "promotion", knight, knight.vocation("elite_knight"));
        stale_slot.stance = (Some("guard"), None);
        let script = format!(
            "{}{}{}",
            advance(3, 50, 1000),
            stale_slot.receipt(),
            row_write(&stale_slot.after, 4, 68)
        );
        expect_rejected(pool, "prune leaves the stance row", &script, STANCE).await?;
        // A vocation change without a prune keeps the stance row as it is.
        let keep = Change::new(69, 3, "promotion", knight, knight.vocation("elite_knight"));
        expect_committed(pool, "promotion r4", &keep.commit()).await?;

        // #1393 LOW: a death with correct build fields that leaves the build row unchanged.
        let elite = knight.vocation("elite_knight");
        let trained = elite.with(0, (0, 5));
        expect_committed(
            pool,
            "training r5",
            &Change::new(70, 4, "training", elite, trained).commit(),
        )
        .await?;
        let lost = trained.with(0, (0, 2));
        let without_row = death(71, 5, (1000, 900), Some((trained, lost)));
        expect_rejected(pool, "death without its row update", &without_row, BUILD).await?;
        expect_committed(
            pool,
            "death with its row r6",
            &format!("{without_row}{}", row_write(&lost, 6, 71)),
        )
        .await?;

        // Immutability, no delete, no truncate.
        for (case, script, rule) in [
            (
                "receipt update",
                "UPDATE game_character_build_receipts SET committed_at = 9",
                "Character first-slice authority history is immutable",
            ),
            (
                "row delete",
                "DELETE FROM game_character_build_state",
                "Character build state is never deleted or reassigned",
            ),
            (
                "receipt truncate",
                "TRUNCATE game_character_build_receipts CASCADE",
                "Character authority relations cannot be truncated",
            ),
            (
                "row truncate",
                "TRUNCATE game_character_build_state",
                "Character authority relations cannot be truncated",
            ),
        ] {
            expect_rejected(pool, case, script, rule).await?;
        }
        Ok(())
    })
}

#[test]
fn build_checks_reject_the_wrong_direction() -> TestResult {
    run("build_checks", async |harness| {
        let pool = &harness.pool;
        let knight = Build::SEED.vocation("knight");
        let trained = knight.with(1, (11, 0));
        for (case, change, rule) in [
            (
                "training changes vocation",
                Change::new(60, 1, "training", Build::SEED, knight.with(0, (0, 1))),
                DIRECTION,
            ),
            (
                "training without progress",
                Change::new(60, 1, "training", Build::SEED, Build::SEED),
                DIRECTION,
            ),
            (
                "training lowers a family",
                Change::new(
                    60,
                    1,
                    "training",
                    trained,
                    trained.with(1, (10, 90)).with(0, (0, 1)),
                ),
                DIRECTION,
            ),
            (
                "choice from a vocation",
                Change::new(60, 1, "vocation_choice", knight, knight.vocation("paladin")),
                DIRECTION,
            ),
            (
                "choice to none",
                Change::new(60, 1, "vocation_choice", Build::SEED, Build::SEED),
                DIRECTION,
            ),
            (
                "promotion from none",
                Change::new(60, 1, "promotion", Build::SEED, knight),
                DIRECTION,
            ),
            (
                "promotion changes a family",
                Change::new(
                    60,
                    1,
                    "promotion",
                    trained,
                    trained.vocation("elite_knight").with(1, (11, 1)),
                ),
                DIRECTION,
            ),
            (
                "promotion to the same key",
                Change::new(60, 1, "promotion", trained, trained.with(0, (0, 0))),
                DIRECTION,
            ),
            (
                "a skill below 10",
                Change::new(
                    60,
                    1,
                    "vocation_choice",
                    Build::SEED,
                    knight.with(3, (9, 0)),
                ),
                "game_character_build_receipts_sword_level_after_check",
            ),
        ] {
            expect_rejected(pool, case, &change.receipt(), rule).await?;
        }
        let mut prune = Change::new(60, 1, "training", Build::SEED, Build::SEED.with(0, (0, 1)));
        prune.stance = (Some("guard"), None);
        expect_rejected(
            pool,
            "training prunes",
            &prune.receipt(),
            "game_character_build_receipts_stance_prune",
        )
        .await?;
        let mut set = Change::new(60, 1, "vocation_choice", Build::SEED, knight);
        set.stance = (None, Some("guard"));
        expect_rejected(
            pool,
            "vocation change sets a stance",
            &set.receipt(),
            "game_character_build_receipts_stance_prune",
        )
        .await?;

        // Death build fields: all or none, and a strict loss (so never the first receipt).
        let partial = death(
            61,
            1,
            (1000, 900),
            Some((trained, trained.with(1, (10, 5)))),
        )
        .replacen("'knight', 0, 0", "NULL, 0, 0", 1);
        expect_rejected(
            pool,
            "partial death fields",
            &partial,
            "game_character_death_receipts_build_all_or_none",
        )
        .await?;
        for (case, after) in [
            ("death gains", trained.with(0, (0, 1))),
            ("death keeps everything", trained),
        ] {
            let script = death(61, 1, (1000, 900), Some((trained, after)));
            expect_rejected(
                pool,
                case,
                &script,
                "game_character_death_receipts_build_loss",
            )
            .await?;
        }
        let seed_death = death(61, 1, (1000, 900), Some((Build::SEED, Build::SEED)));
        expect_rejected(
            pool,
            "death as first build receipt",
            &seed_death,
            "game_character_death_receipts_build_loss",
        )
        .await?;
        Ok(())
    })
}

#[test]
fn build_grants_and_admission_verifier() -> TestResult {
    run("build_verifier", async |harness| {
        let pool = &harness.pool;
        let granted: Vec<String> = sqlx::query_scalar(
            "SELECT r.role || '/' || t.relation || ':' || p.privilege \
               FROM (VALUES ('oteryn_game_runtime'), ('oteryn_game_control')) r(role), \
                    (VALUES ('game_character_build_receipts'), ('game_character_build_state')) \
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
                "oteryn_game_control/game_character_build_receipts:SELECT",
                "oteryn_game_control/game_character_build_state:SELECT",
                "oteryn_game_runtime/game_character_build_receipts:INSERT",
                "oteryn_game_runtime/game_character_build_receipts:SELECT",
                "oteryn_game_runtime/game_character_build_state:INSERT",
                "oteryn_game_runtime/game_character_build_state:SELECT",
                "oteryn_game_runtime/game_character_build_state:UPDATE",
            ]
        );
        let public: bool = sqlx::query_scalar(
            "SELECT has_function_privilege('public', \
               'game_character_build_state_row_guard()', 'EXECUTE')",
        )
        .fetch_one(pool)
        .await?;
        assert!(!public, "PUBLIC cannot execute the build row guard");

        // #1271 F2: the runtime role itself commits a build transaction.
        let knight = Build::SEED.vocation("knight");
        let choice = Change::new(60, 1, "vocation_choice", Build::SEED, knight);
        expect_committed(
            pool,
            "runtime choice r2",
            &format!("SET LOCAL ROLE oteryn_game_runtime;{}", choice.commit()),
        )
        .await?;
        let trained = knight.with(0, (0, 30));
        let training = Change::new(61, 2, "training", knight, trained);
        expect_committed(pool, "training r3", &training.commit()).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        drop(
            harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?,
        );

        // r4 a death takes mana from the death receipt's build fields; r5 trains on from its
        // `after`, so both sides of the death join the build chain only through the verifier's
        // death arm (#1393 MEDIUM).
        let lost = knight.with(0, (0, 10));
        expect_committed(
            pool,
            "death with loss r4",
            &format!(
                "{}{}",
                death(62, 3, (1000, 900), Some((trained, lost))),
                row_write(&lost, 4, 62)
            ),
        )
        .await?;
        expect_committed(pool, "respawn", &consume_pending()).await?;
        let mut after_death = Change::new(63, 4, "training", lost, lost.with(0, (0, 20)));
        after_death.at = (50, 900);
        expect_committed(pool, "training r5", &after_death.commit()).await?;
        drop(
            harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?,
        );

        // #1271 F4: each named verifier check fails admission closed on planted state.
        for (case, tamper, repair) in [
            (
                "row differs from the latest receipt",
                "UPDATE game_character_build_state SET mana_spent = 21",
                "UPDATE game_character_build_state SET mana_spent = 20",
            ),
            (
                "row names another occurrence",
                "UPDATE game_character_build_state SET committed_character_revision = 4",
                "UPDATE game_character_build_state SET committed_character_revision = 5",
            ),
            (
                "build chain gap",
                "UPDATE game_character_build_receipts SET mana_spent_before = 1 \
                  WHERE committed_character_revision = 3",
                "UPDATE game_character_build_receipts SET mana_spent_before = 0 \
                  WHERE committed_character_revision = 3",
            ),
            (
                "death build before off the chain",
                "UPDATE game_character_death_receipts SET mana_spent_before = 31 \
                  WHERE committed_character_revision = 4",
                "UPDATE game_character_death_receipts SET mana_spent_before = 30 \
                  WHERE committed_character_revision = 4",
            ),
            (
                "death build after off the chain",
                "UPDATE game_character_death_receipts SET mana_spent_after = 11 \
                  WHERE committed_character_revision = 4",
                "UPDATE game_character_death_receipts SET mana_spent_after = 10 \
                  WHERE committed_character_revision = 4",
            ),
            (
                "death build vocation off the chain",
                "UPDATE game_character_death_receipts SET vocation = 'paladin' \
                  WHERE committed_character_revision = 4",
                "UPDATE game_character_death_receipts SET vocation = 'knight' \
                  WHERE committed_character_revision = 4",
            ),
            (
                "first receipt off the seed",
                "UPDATE game_character_build_receipts SET fishing_tries_before = 1 \
                  WHERE committed_character_revision = 2",
                "UPDATE game_character_build_receipts SET fishing_tries_before = 0 \
                  WHERE committed_character_revision = 2",
            ),
        ] {
            attempt(
                pool,
                &format!("SET LOCAL session_replication_role = replica;{tamper}"),
            )
            .await?;
            assert!(
                harness.root.open_character_authority(&seal).await.is_err(),
                "{case}: admission must fail closed"
            );
            attempt(
                pool,
                &format!("SET LOCAL session_replication_role = replica;{repair}"),
            )
            .await?;
            drop(
                harness
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?,
            );
        }
        Ok(())
    })
}

fn state(vocation: &str, magic: (u16, u64), sword: (u16, u64)) -> TestResult<DurableBuildState> {
    let mut skills = [(10, 0); 7];
    skills[2] = sword;
    Ok(DurableBuildState::new(vocation, magic, skills).map_err(debug)?)
}

fn change(
    tag: u8,
    cause: BuildCause,
    before: &DurableBuildState,
    after: &DurableBuildState,
    pruned_stance: Option<&str>,
) -> TestResult<BuildChangeRequest> {
    Ok(BuildChangeRequest {
        occurrence: BuildOccurrence::from_bytes(id(tag)).map_err(debug)?,
        cause,
        before: before.clone(),
        after: after.clone(),
        pruned_stance: pruned_stance.map(str::to_owned),
    })
}

/// A test formula table: skills base 50 with multiplier 2.0 without a vocation and 1.1 with one;
/// magic level 100 x L. `.0` is the table digest and `.1` its content revision.
struct Table([u8; 32], &'static str);

impl BuildFormula for Table {
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
        match (family, vocation) {
            (0, _) => Some(100 * u64::from(level)),
            (_, "none") => skill_tries_required(50, 2.0, level),
            _ => skill_tries_required(50, 1.1, level),
        }
    }

    fn digest(&self) -> [u8; 32] {
        self.0
    }

    fn content_revision(&self) -> &str {
        self.1
    }
}

const TABLE: Table = Table([1; 32], "content-1");

#[test]
fn build_writer_is_fenced_replayed_and_reconciled() -> TestResult {
    run("build_writer", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let root: &DurabilityRoot = &harness.root;
        let node = &harness.node;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let seed = DurableBuildState::default();
        assert_eq!(
            root.read_character_build_state(&authority, character)
                .await
                .map_err(debug)?,
            seed,
            "no row loads the seed"
        );
        assert!(BuildOccurrence::from_bytes([7; 16]).is_err());

        // r2 a stance the Dawnport choice prunes (A13 §4.4).
        expect_committed(pool, "stance r2", &toggle(60, 1, Some("guard"))).await?;
        let knight = state("knight", (0, 0), (10, 0))?;
        let choice = change(
            61,
            BuildCause::VocationChoice,
            &seed,
            &knight,
            Some("guard"),
        )?;
        let before = snapshot(pool).await?;
        for (fence_revision, request, case) in [
            (1, choice.clone(), "stale revision"),
            (
                2,
                change(
                    61,
                    BuildCause::VocationChoice,
                    &seed,
                    &knight,
                    Some("shield"),
                )?,
                "another stance",
            ),
            (
                2,
                change(
                    61,
                    BuildCause::VocationChoice,
                    &knight,
                    &knight.clone(),
                    None,
                )?,
                "choice from a vocation",
            ),
            (
                2,
                change(
                    61,
                    BuildCause::VocationChoice,
                    &seed,
                    &state("knight", (0, 0), (11, 0))?,
                    Some("guard"),
                )?,
                "choice that is not the writer's conversion",
            ),
        ] {
            let outcome = root
                .commit_character_build(&authority, node, fence(fence_revision)?, request, &TABLE)
                .await;
            assert!(
                matches!(
                    outcome,
                    Err(CharacterProgressionError::CharacterRevisionMismatch
                        | CharacterProgressionError::BuildStateMismatch
                        | CharacterProgressionError::InvalidInput)
                ),
                "{case}: {outcome:?}"
            );
            assert_eq!(snapshot(pool).await?, before, "{case} wrote nothing");
        }
        let first = root
            .commit_character_build(&authority, node, fence(2)?, choice.clone(), &TABLE)
            .await
            .map_err(debug)?;
        let BuildCommitOutcome::Committed(committed) = first else {
            return Err(format!("unexpected first commit: {first:?}").into());
        };
        assert_eq!(committed.committed_character_revision.get(), 3);
        assert_eq!(committed.pruned_stance.as_deref(), Some("guard"));
        assert_eq!(
            snapshot(pool).await?,
            format!("3|3|3|knight:0:0:10:0:3:{}|-:3", uuid_text(61))
        );

        // Exact replay, even at the now stale revision, returns the receipt; the binding is
        // compared first, so changed reuse conflicts; reconciliation proves what committed.
        let replay = root
            .commit_character_build(&authority, node, fence(2)?, choice.clone(), &TABLE)
            .await
            .map_err(debug)?;
        assert_eq!(
            replay,
            BuildCommitOutcome::AlreadyCommitted(committed.clone())
        );
        let conflict = root
            .commit_character_build(
                &authority,
                node,
                fence(3)?,
                choice.clone(),
                &Table([9; 32], "content-1"),
            )
            .await;
        assert!(
            matches!(
                conflict,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ),
            "{conflict:?}"
        );
        assert_eq!(
            root.reconcile_character_build(&authority, choice.occurrence)
                .await
                .map_err(debug)?,
            Some(committed)
        );
        assert_eq!(
            root.reconcile_character_build(
                &authority,
                BuildOccurrence::from_bytes(id(79)).map_err(debug)?
            )
            .await
            .map_err(debug)?,
            None
        );

        // r4 training advances magic level and sword in one receipt.
        let trained = state("knight", (1, 20), (12, 7))?;
        let training = change(62, BuildCause::Training, &knight, &trained, None)?;
        let outcome = root
            .commit_character_build(&authority, node, fence(3)?, training, &TABLE)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, BuildCommitOutcome::Committed(_)));
        let before = snapshot(pool).await?;

        // Rejected before any write: the wrong direction, a training prune, a stale copy.
        for (request, case) in [
            (
                change(63, BuildCause::Training, &trained, &knight, None)?,
                "training lowers",
            ),
            (
                change(63, BuildCause::Training, &trained, &trained, None)?,
                "training without progress",
            ),
            (
                change(
                    63,
                    BuildCause::Training,
                    &trained,
                    &state("knight", (1, 21), (12, 7))?,
                    Some("guard"),
                )?,
                "training prunes",
            ),
            (
                change(
                    63,
                    BuildCause::Training,
                    &trained,
                    &state("knight", (1, 20), (12, 60))?,
                    None,
                )?,
                "sword tries that pay for level 13",
            ),
            (
                change(
                    63,
                    BuildCause::Training,
                    &trained,
                    &state("knight", (1, 20), (1000, 0))?,
                    None,
                )?,
                "a sword level past the first unreachable one",
            ),
        ] {
            let outcome = root
                .commit_character_build(&authority, node, fence(4)?, request, &TABLE)
                .await;
            assert!(
                matches!(outcome, Err(CharacterProgressionError::InvalidInput)),
                "{case}: {outcome:?}"
            );
        }
        let stale_copy = change(64, BuildCause::Training, &knight, &trained, None)?;
        let outcome = root
            .commit_character_build(&authority, node, fence(4)?, stale_copy, &TABLE)
            .await;
        assert!(
            matches!(outcome, Err(CharacterProgressionError::BuildStateMismatch)),
            "{outcome:?}"
        );
        // A table from another content revision decides nothing.
        let next = state("knight", (1, 30), (12, 7))?;
        let request = change(64, BuildCause::Training, &trained, &next, None)?;
        let outcome = root
            .commit_character_build(
                &authority,
                node,
                fence(4)?,
                request,
                &Table([1; 32], "content-2"),
            )
            .await;
        assert!(
            matches!(
                outcome,
                Err(CharacterProgressionError::ProgressionContextMismatch)
            ),
            "{outcome:?}"
        );

        // A stale fence writes nothing: each case changes exactly one fact.
        let mut other_connection = fence(4)?;
        other_connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut other_lease = fence(4)?;
        other_lease.character_lease_generation = 2;
        let mut other_session = fence(4)?;
        other_session.game_session_id =
            crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
        for (tag, stale, case) in [
            (65, other_connection, "another connection generation"),
            (66, other_lease, "another lease generation"),
            (67, other_session, "another game session"),
        ] {
            let request = change(tag, BuildCause::Training, &trained, &next, None)?;
            let outcome = root
                .commit_character_build(&authority, node, stale, request, &TABLE)
                .await;
            assert!(
                matches!(outcome, Err(CharacterProgressionError::AuthorityRejected)),
                "{case}: {outcome:?}"
            );
        }
        assert_eq!(
            snapshot(pool).await?,
            before,
            "rejected writes wrote nothing"
        );

        // r5 a death with its respawn pending: no build change commits until it is consumed.
        expect_committed(pool, "death r5", &death(68, 4, (1000, 900), None)).await?;
        let promotion = change(
            69,
            BuildCause::Promotion,
            &trained,
            &state("elite_knight", (1, 20), (12, 7))?,
            None,
        )?;
        let pending = root
            .commit_character_build(&authority, node, fence(5)?, promotion.clone(), &TABLE)
            .await;
        assert!(
            matches!(pending, Err(CharacterProgressionError::RespawnPending)),
            "{pending:?}"
        );
        expect_committed(pool, "respawn", &consume_pending()).await?;
        // r6 promotion; the receipt takes level and experience from the state the death left.
        let outcome = root
            .commit_character_build(&authority, node, fence(5)?, promotion.clone(), &TABLE)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, BuildCommitOutcome::Committed(_)));

        // The admission verifier accepts the writer's chain, and a restarted root loads it.
        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let reopened = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .read_character_build_state(&reopened, character)
                .await
                .map_err(debug)?,
            promotion.after
        );
        let experience: (i64, i64) = sqlx::query_as(
            "SELECT experience_before, experience_after FROM game_character_build_receipts \
              WHERE committed_character_revision = 6",
        )
        .fetch_one(pool)
        .await?;
        assert_eq!(experience, (900, 900));
        Ok(())
    })
}

/// CHAR-REV-SEQ-1: a build change runs in the Character's revision slot. A write that bypassed
/// the sequencer makes the next one fail closed with nothing written (the binding includes the
/// revision, so it is not retried); the next request reloads the cursor.
#[test]
fn a_sequenced_build_change_fails_closed_after_a_bypass_writer() -> TestResult {
    run("build_sequenced", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (root, node) = (&harness.root, &harness.node);
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer
            .acquire(CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?)
            .await;
        let knight = state("knight", (0, 0), (10, 0))?;
        // The caller's stale fence revision is replaced by the slot's cursor.
        let choice = change(
            70,
            BuildCause::VocationChoice,
            &DurableBuildState::default(),
            &knight,
            None,
        )?;
        let outcome = slot
            .commit_build(root, &authority, node, fence(1)?, choice, &TABLE, None)
            .await
            .map_err(debug)?;
        let BuildCommitOutcome::Committed(committed) = outcome else {
            return Err(format!("unexpected outcome: {outcome:?}").into());
        };
        assert_eq!(committed.committed_character_revision.get(), 2);

        // r3 a stance write that bypassed the sequencer.
        expect_committed(pool, "bypass r3", &toggle(71, 2, Some("guard"))).await?;
        let before = snapshot(pool).await?;
        let trained = state("knight", (1, 20), (12, 7))?;
        let training = change(72, BuildCause::Training, &knight, &trained, None)?;
        let outcome = slot
            .commit_build(root, &authority, node, fence(2)?, training, &TABLE, None)
            .await;
        assert!(
            matches!(
                outcome,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ),
            "{outcome:?}"
        );
        assert_eq!(snapshot(pool).await?, before, "no retry committed");

        let training = change(73, BuildCause::Training, &knight, &trained, None)?;
        let outcome = slot
            .commit_build(root, &authority, node, fence(2)?, training, &TABLE, None)
            .await
            .map_err(debug)?;
        let BuildCommitOutcome::Committed(committed) = outcome else {
            return Err(format!("unexpected outcome: {outcome:?}").into());
        };
        assert_eq!(committed.original_character_revision.get(), 3);
        drop(slot);
        drop(authority);
        drop(seal);
        Ok(())
    })
}
