//! LOGDBG-C: the revision-chain integrity query names the first violating row, and the same
//! rows fail as before. Plain scratch tables stand in for the receipt families, so a broken
//! chain the migration guards refuse can be written; the query is the production one.

use super::{RECEIPT_CHAIN_TABLES, RevisionChainViolation, verify_revision_chain};
use sqlx::{Connection, Executor, PgConnection};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const CHARACTER: &str = "0190f1a2-0000-7000-8000-000000000001";
const REVISIONS: &str = "profile_revision numeric NOT NULL DEFAULT 1, \
     ruleset_revision numeric NOT NULL DEFAULT 1, content_revision numeric NOT NULL DEFAULT 1, \
     simulation_revision numeric NOT NULL DEFAULT 1, evidence_revision numeric NOT NULL DEFAULT 1, \
     declaration_revision text NOT NULL DEFAULT '1', policy_revision numeric NOT NULL DEFAULT 1, \
     reward_revision numeric NOT NULL DEFAULT 1";

/// The query on `main` before LOGDBG-C, kept to prove the same fixtures pass and fail.
fn main_integrity_sql() -> String {
    let chain = RECEIPT_CHAIN_TABLES
        .iter()
        .map(|table| {
            format!(
                "SELECT character_id, original_character_revision, committed_character_revision, \
                 level_before, level_after, experience_before, experience_after, \
                 profile_revision, ruleset_revision, content_revision, simulation_revision, \
                 evidence_revision, declaration_revision, policy_revision, reward_revision \
                 FROM {table}"
            )
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    format!(
        "WITH chain AS ({chain}) \
         SELECT 1 FROM game_character_roots r \
           LEFT JOIN game_character_progression_state s USING (character_id) \
          WHERE (r.character_revision <> 1 AND s.character_id IS NULL) \
             OR (s.character_id IS NOT NULL AND s.character_revision <> r.character_revision) \
         UNION ALL \
         SELECT 1 FROM game_character_progression_state s \
           LEFT JOIN game_character_roots r USING (character_id) \
          WHERE r.character_id IS NULL \
         UNION ALL \
         SELECT 1 FROM game_character_roots r \
           LEFT JOIN chain x USING (character_id) \
          GROUP BY r.character_id, r.character_revision \
         HAVING count(x.committed_character_revision)::numeric <> r.character_revision - 1 \
             OR count(DISTINCT x.committed_character_revision)::numeric \
                <> r.character_revision - 1 \
         UNION ALL \
         SELECT 1 FROM chain x \
           LEFT JOIN chain p \
             ON p.character_id = x.character_id \
            AND p.committed_character_revision = x.original_character_revision \
          WHERE (x.original_character_revision <> 1 AND p.committed_character_revision IS NULL) \
             OR (p.committed_character_revision IS NOT NULL AND \
                 (p.experience_after <> x.experience_before OR p.level_after <> x.level_before)) \
         UNION ALL \
         SELECT 1 FROM game_character_progression_state s \
           JOIN game_character_roots r USING (character_id) \
           LEFT JOIN chain x \
             ON x.character_id = s.character_id \
            AND x.committed_character_revision = s.character_revision \
          WHERE s.character_revision > 1 AND (x.committed_character_revision IS NULL \
             OR x.experience_after <> s.total_experience OR x.level_after <> s.level \
             OR x.profile_revision <> s.profile_revision \
             OR x.ruleset_revision <> s.ruleset_revision \
             OR x.content_revision <> s.content_revision \
             OR x.simulation_revision <> s.simulation_revision \
             OR x.evidence_revision <> s.evidence_revision \
             OR NOT (x.declaration_revision = s.declaration_revision \
                     OR (x.declaration_revision = 'character-progression-differences-v1-c6d0829911772c01b4a229a5bb0aa8f9' \
                         AND s.declaration_revision = 'character-progression-differences-v1-aa9d260834741d235ba9bd77e14888bf')) \
             OR x.policy_revision <> s.policy_revision \
             OR x.reward_revision <> s.reward_revision) \
         UNION ALL \
         SELECT 1 FROM chain x \
           LEFT JOIN game_character_progression_state s USING (character_id) \
          WHERE s.character_id IS NULL \
             OR x.committed_character_revision > s.character_revision \
             OR x.profile_revision <> s.profile_revision \
             OR x.ruleset_revision <> s.ruleset_revision \
             OR x.content_revision <> s.content_revision \
             OR x.simulation_revision <> s.simulation_revision \
             OR x.evidence_revision <> s.evidence_revision \
             OR NOT (x.declaration_revision = s.declaration_revision \
                     OR (x.declaration_revision = 'character-progression-differences-v1-c6d0829911772c01b4a229a5bb0aa8f9' \
                         AND s.declaration_revision = 'character-progression-differences-v1-aa9d260834741d235ba9bd77e14888bf')) \
             OR x.policy_revision <> s.policy_revision \
             OR x.reward_revision <> s.reward_revision \
         LIMIT 1"
    )
}

async fn create_tables(connection: &mut PgConnection) -> TestResult {
    connection
        .execute(sqlx::AssertSqlSafe(format!(
            "CREATE TABLE game_character_roots (character_id uuid PRIMARY KEY, \
               character_revision numeric NOT NULL); \
             CREATE TABLE game_character_progression_state (character_id uuid PRIMARY KEY, \
               character_revision numeric NOT NULL, total_experience numeric NOT NULL, \
               level numeric NOT NULL, {REVISIONS})"
        )))
        .await?;
    for table in RECEIPT_CHAIN_TABLES {
        connection
            .execute(sqlx::AssertSqlSafe(format!(
                "CREATE TABLE {table} (character_id uuid NOT NULL, \
                   original_character_revision numeric NOT NULL, \
                   committed_character_revision numeric NOT NULL, \
                   level_before numeric NOT NULL, level_after numeric NOT NULL, \
                   experience_before numeric NOT NULL, experience_after numeric NOT NULL, \
                   {REVISIONS})"
            )))
            .await?;
    }
    Ok(())
}

/// One case: `setup` writes the rows; returns the classification and whether `main`'s query
/// found a violation on the same rows.
async fn classify(
    connection: &mut PgConnection,
    setup: &str,
) -> Result<(Option<RevisionChainViolation>, bool), Box<dyn Error>> {
    let mut truncate = vec!["game_character_roots", "game_character_progression_state"];
    truncate.extend_from_slice(RECEIPT_CHAIN_TABLES);
    connection
        .execute(sqlx::AssertSqlSafe(format!("TRUNCATE {}", truncate.join(", "))))
        .await?;
    connection
        .execute(sqlx::AssertSqlSafe(setup.replace("$C", &format!("'{CHARACTER}'"))))
        .await?;
    let failed_on_main = sqlx::query(sqlx::AssertSqlSafe(main_integrity_sql()))
        .fetch_optional(&mut *connection)
        .await?
        .is_some();
    let mut transaction = connection.begin().await?;
    let violation = verify_revision_chain(&mut transaction).await?;
    transaction.rollback().await?;
    Ok((violation, failed_on_main))
}

fn receipt(table: &str, from: u32, level: u32, experience: (u32, u32)) -> String {
    format!(
        "INSERT INTO {table} (character_id, original_character_revision, \
           committed_character_revision, level_before, level_after, experience_before, \
           experience_after) VALUES ($C, {from}, {}, {level}, {level}, {}, {});",
        from + 1,
        experience.0,
        experience.1
    )
}

fn state(revision: u32, experience: u32) -> String {
    format!(
        "INSERT INTO game_character_roots VALUES ($C, {revision}); \
         INSERT INTO game_character_progression_state (character_id, character_revision, \
           total_experience, level) VALUES ($C, {revision}, {experience}, 8);"
    )
}

#[test]
fn the_revision_chain_check_names_the_first_violation_and_fails_the_same_rows() -> TestResult {
    let Some(admin_url) = std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let admin_url = admin_url.into_string().map_err(|_| "non-UTF-8 admin URL")?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut connection = PgConnection::connect(&admin_url).await?;
            let schema = format!("logdbg_c_{}", std::process::id());
            connection
                .execute(sqlx::AssertSqlSafe(format!(
                    "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; \
                     SET search_path TO {schema}"
                )))
                .await?;
            let result = cases(&mut connection).await;
            connection
                .execute(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
                .await?;
            result
        })
}

async fn cases(connection: &mut PgConnection) -> TestResult {
    create_tables(connection).await?;
    let xp = "game_character_xp_receipts";
    let death = "game_character_death_receipts";

    // A clean chain across two families: 1 -> 2 (XP), 2 -> 3 (death).
    let clean = format!(
        "{}{}{}",
        state(3, 30),
        receipt(xp, 1, 8, (0, 20)),
        receipt(death, 2, 8, (20, 30))
    );
    assert_eq!(classify(connection, &clean).await?, (None, false));

    // Gap: revision 3 is explained, revision 2 is not.
    let gap = format!("{}{}", state(3, 30), receipt(death, 2, 8, (20, 30)));
    let (violation, failed_on_main) = classify(connection, &gap).await?;
    assert!(failed_on_main);
    assert_eq!(violation.as_ref().map(|found| found.violation.as_str()), Some("gap"));

    // Fork: two families both commit revision 2 from revision 1.
    let fork = format!(
        "{}{}{}",
        state(2, 20),
        receipt(xp, 1, 8, (0, 20)),
        receipt(death, 1, 8, (0, 20))
    );
    let (violation, failed_on_main) = classify(connection, &fork).await?;
    assert!(failed_on_main);
    assert_eq!(
        violation,
        Some(RevisionChainViolation {
            family: "receipt_chain".to_owned(),
            violation: "fork".to_owned(),
            before: Some("2".to_owned()),
            after: Some("2".to_owned()),
        })
    );

    // Latest mismatch: the chain is whole but the typed state disagrees with its last receipt.
    let latest = format!("{}{}", state(2, 25), receipt(xp, 1, 8, (0, 20)));
    let (violation, failed_on_main) = classify(connection, &latest).await?;
    assert!(failed_on_main);
    assert_eq!(
        violation,
        Some(RevisionChainViolation {
            family: xp.to_owned(),
            violation: "latest_mismatch".to_owned(),
            before: Some("2".to_owned()),
            after: Some("2".to_owned()),
        })
    );

    // Missing typed state: an advanced root without progression state.
    let missing = format!(
        "INSERT INTO game_character_roots VALUES ($C, 2);{}",
        receipt(xp, 1, 8, (0, 20))
    );
    let (violation, failed_on_main) = classify(connection, &missing).await?;
    assert!(failed_on_main);
    assert_eq!(
        violation.map(|found| (found.family, found.violation)),
        Some((
            "game_character_progression_state".to_owned(),
            "missing_typed_state".to_owned()
        ))
    );
    eprintln!("LOGDBG_C_REVISION_CHAIN_PG_CASES executed=5");
    Ok(())
}

#[test]
fn the_revision_chain_query_runs_on_the_migrated_schema_and_passes_an_empty_one() -> TestResult {
    let Some(admin_url) = std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let admin_url = admin_url.into_string().map_err(|_| "non-UTF-8 admin URL")?;
    let prefix = admin_url
        .strip_suffix("/postgres")
        .ok_or("invalid PostgreSQL test-admin URL")?
        .to_owned();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let name = format!("logdbg_c_migrated_{}", std::process::id());
            let mut admin = PgConnection::connect(&admin_url).await?;
            admin
                .execute(sqlx::AssertSqlSafe(format!("DROP DATABASE IF EXISTS {name}")))
                .await?;
            admin
                .execute(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .await?;
            let result = async {
                let url = format!("{prefix}/{name}");
                crate::durability::MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let mut connection = PgConnection::connect(&url).await?;
                let mut transaction = connection.begin().await?;
                let violation = verify_revision_chain(&mut transaction).await?;
                transaction.rollback().await?;
                assert_eq!(violation, None);
                eprintln!("LOGDBG_C_REVISION_CHAIN_MIGRATED executed=1");
                Ok::<(), Box<dyn Error>>(())
            }
            .await;
            admin
                .execute(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
                )))
                .await?;
            result
        })
}
