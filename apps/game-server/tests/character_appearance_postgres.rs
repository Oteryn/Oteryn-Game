//! Private read/schema qualification only. No production writer, event chain, grant producer,
//! authority fixture or migration is installed. Fixture INSERTs below are not gameplay writes.
pub use oteryn_game_server::domain;
#[path = "../src/durability/character_appearance.rs"]
mod character_appearance;

use character_appearance::{
    AccountAppearanceUnlock, read_account_appearance_unlocks, read_selected_appearance,
};
use domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceRejection, Sex, UnlockFact, validate_selection,
};
use domain::{AccountId, CharacterId};
use sqlx::{Connection, Executor};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

// Final-schema proposal, isolated to this database. No production names are claimed installed.
// The final CP migration must add real root/account constraints, the complete event receipt
// chain, permissions and fenced producers together; these three tables cannot authorize writes.
const PRIVATE_SCHEMA: &str = r#"
CREATE TABLE game_character_appearance (
 character_id UUID PRIMARY KEY,
 outfit_key TEXT NOT NULL,
 colours INTEGER[] NOT NULL CHECK (
   array_ndims(colours)=1 AND array_lower(colours,1)=1 AND cardinality(colours)=4
   AND array_position(colours,NULL) IS NULL
   AND 0 <= ALL(colours) AND 65535 >= ALL(colours)),
 addons SMALLINT NOT NULL CHECK(addons BETWEEN 0 AND 255),
 mount_key TEXT,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision BETWEEN 1 AND 18446744073709551615)
);
CREATE TABLE game_account_appearance_unlock_receipts (
 grant_occurrence_id UUID PRIMARY KEY,
 account_id UUID NOT NULL,
 unlock_key TEXT NOT NULL,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 granting_character_id UUID NOT NULL,
 source_kind TEXT NOT NULL CHECK(octet_length(source_kind)>0),
 source_event_id BYTEA NOT NULL CHECK(octet_length(source_event_id)>0),
 UNIQUE(source_kind,source_event_id,unlock_key),
 UNIQUE(grant_occurrence_id,account_id,unlock_key)
);
CREATE TABLE game_account_appearance_unlocks (
 account_id UUID NOT NULL,
 unlock_key TEXT NOT NULL,
 grant_occurrence_id UUID NOT NULL,
 PRIMARY KEY(account_id,unlock_key),
 FOREIGN KEY(grant_occurrence_id,account_id,unlock_key)
 REFERENCES game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key)
);
CREATE FUNCTION app_fixture_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'immutable earned appearance fact' USING ERRCODE='23514'; END;
$$;
CREATE TRIGGER app_unlock_immutable BEFORE UPDATE OR DELETE
 ON game_account_appearance_unlocks FOR EACH ROW EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_unlock_no_truncate BEFORE TRUNCATE
 ON game_account_appearance_unlocks EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_account_appearance_unlock_receipts FOR EACH ROW EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_receipt_no_truncate BEFORE TRUNCATE
 ON game_account_appearance_unlock_receipts EXECUTE FUNCTION app_fixture_immutable();
"#;

// Test-only catalogue and explicit unlock-key mapping. APP-2 owns the actual mapping and
// compatibility resolver; revision equality below is a fixture policy, not a product rule.
struct Content;
impl AppearanceContent<String> for Content {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        matches!(key.as_str(), "starter" | "earned" | "store").then_some(2)
    }
    fn outfit_is_starter(&self, key: &String, _: Sex) -> bool {
        key == "starter"
    }
    fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
        false
    }
    fn mount_exists(&self, key: &String) -> bool {
        key == "horse"
    }
}
struct FixtureEarned<'a>(&'a [AccountAppearanceUnlock]);
impl FixtureEarned<'_> {
    fn fact(&self, key: &str) -> UnlockFact {
        match self.0.iter().find(|fact| fact.unlock_key == key) {
            None => UnlockFact::Absent,
            Some(fact) if fact.content_revision == "content-1" => UnlockFact::Compatible,
            Some(_) => UnlockFact::Incompatible,
        }
    }
}
impl AccountUnlocks<String> for FixtureEarned<'_> {
    fn outfit(&self, key: &String) -> UnlockFact {
        self.fact(key)
    }
    fn addon(&self, key: &String, addon: u8) -> UnlockFact {
        if key == "earned" && addon == 1 {
            self.fact("fixture-addon-one")
        } else {
            UnlockFact::Absent
        }
    }
    fn mount(&self, key: &String) -> UnlockFact {
        self.fact(key)
    }
}

async fn qualify(connection: &mut sqlx::PgConnection) -> TestResult {
    assert!(
        read_selected_appearance(connection, CharacterId::from_bytes(id(41))?)
            .await
            .is_err()
    );
    connection.execute(sqlx::raw_sql(PRIVATE_SCHEMA)).await?;
    let character = CharacterId::from_bytes(id(41))?;
    let other_character = CharacterId::from_bytes(id(42))?;
    let account = AccountId::from_bytes(id(40))?;
    let other_account = AccountId::from_bytes(id(39))?;
    assert!(
        read_selected_appearance(connection, character)
            .await?
            .is_none()
    );
    sqlx::query("INSERT INTO game_character_appearance VALUES(encode($1,'hex')::uuid,'earned',ARRAY[0,1,2,132],1,'horse','old-world',2)")
        .bind(character.as_bytes().as_slice()).execute(&mut *connection).await?;
    // These are fixture inserts, not an earned-grant or selected-appearance writer.
    for (seed, key, revision) in [
        (60, "earned", "content-1"),
        (61, "fixture-addon-one", "content-1"),
        (62, "horse", "incompatible"),
    ] {
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid,'quest',$1)")
            .bind(id(seed).as_slice()).bind(account.as_bytes().as_slice()).bind(key).bind(revision).bind(character.as_bytes().as_slice())
            .execute(&mut *connection).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) ON CONFLICT(account_id,unlock_key) DO NOTHING")
            .bind(account.as_bytes().as_slice()).bind(key).bind(id(seed).as_slice()).execute(&mut *connection).await?;
    }
    let original = read_selected_appearance(connection, character)
        .await?
        .ok_or("missing selection")?;
    assert!(
        read_selected_appearance(connection, other_character)
            .await?
            .is_none()
    );
    assert!(
        read_account_appearance_unlocks(connection, other_account)
            .await?
            .is_empty()
    );
    let facts = read_account_appearance_unlocks(connection, account).await?;
    assert_eq!(facts.len(), 3);
    assert_eq!(facts[0].granting_character_id, character);
    assert_eq!(facts[0].source_kind, "quest");
    assert!(!facts[0].source_event_id.is_empty());
    assert_eq!(
        validate_selection(
            &original.selection,
            Sex::Male,
            &Content,
            &FixtureEarned(&facts)
        ),
        Err(AppearanceRejection::MountNotAuthorized)
    );
    let mut valid = original.selection.clone();
    valid.mount_key = None;
    assert_eq!(
        validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&facts)),
        Ok(())
    );
    let missing_addon: Vec<_> = facts
        .iter()
        .filter(|fact| fact.unlock_key != "fixture-addon-one")
        .cloned()
        .collect();
    assert_eq!(
        validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&missing_addon)),
        Err(AppearanceRejection::AddonNotAuthorized { addon: 1 })
    );
    valid.outfit_key = "store".into();
    assert_eq!(
        validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&facts)),
        Err(AppearanceRejection::OutfitNotAuthorized)
    );

    // Another granting character/revision cannot replace the first fact/provenance.
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'earned','replacement',encode($3,'hex')::uuid,'npc',$1)")
        .bind(id(63).as_slice()).bind(account.as_bytes().as_slice()).bind(other_character.as_bytes().as_slice()).execute(&mut *connection).await?;
    let replay = sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'earned',encode($2,'hex')::uuid) ON CONFLICT(account_id,unlock_key) DO NOTHING")
        .bind(account.as_bytes().as_slice()).bind(id(63).as_slice()).execute(&mut *connection).await?;
    assert_eq!(replay.rows_affected(), 0);
    assert_eq!(
        read_account_appearance_unlocks(connection, account).await?,
        facts
    );
    let conflicting_source = sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'earned','substitution',encode($3,'hex')::uuid,'quest',$4)")
        .bind(id(64).as_slice()).bind(other_account.as_bytes().as_slice()).bind(other_character.as_bytes().as_slice()).bind(id(60).as_slice())
        .execute(&mut *connection).await;
    assert!(
        conflicting_source.is_err(),
        "source-event identity must not be rebound"
    );
    for query in [
        "UPDATE game_account_appearance_unlocks SET unlock_key='changed'",
        "DELETE FROM game_account_appearance_unlocks",
        "TRUNCATE game_account_appearance_unlocks",
        "UPDATE game_account_appearance_unlock_receipts SET content_revision='changed'",
        "DELETE FROM game_account_appearance_unlock_receipts",
        "TRUNCATE game_account_appearance_unlock_receipts CASCADE",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000001','bad',ARRAY[0,0,0],0,NULL,'ok',2)",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000002','bad',ARRAY[0,0,0,NULL],0,NULL,'ok',2)",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000003','bad',ARRAY[0,0,0,0],0,NULL,repeat('é',65),2)",
    ] {
        assert!(
            sqlx::query(query).execute(&mut *connection).await.is_err(),
            "fixture schema accepted {query}"
        );
    }
    assert_eq!(
        read_selected_appearance(connection, character).await?,
        Some(original)
    );
    Ok(())
}

#[test]
fn private_appearance_schema_and_native_validation_postgres() -> TestResult {
    let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("app_private_{}_{suffix}", std::process::id());
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut admin)
            .await?;
        assert_eq!(version, "170006", "canonical PostgreSQL 17.6 required");
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut admin)
            .await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let result = async {
            let mut connection = sqlx::PgConnection::connect(&format!("{prefix}/{name}")).await?;
            qualify(&mut connection).await?;
            connection.close().await?;
            // Reload via a new connection; this is storage persistence, not an actual relog.
            let mut reloaded = sqlx::PgConnection::connect(&format!("{prefix}/{name}")).await?;
            let selected =
                read_selected_appearance(&mut reloaded, CharacterId::from_bytes(id(41))?)
                    .await?
                    .ok_or("lost selection")?;
            assert_eq!(selected.content_revision, "old-world");
            assert_eq!(selected.selection.mount_key.as_deref(), Some("horse"));
            assert_eq!(
                read_account_appearance_unlocks(&mut reloaded, AccountId::from_bytes(id(40))?)
                    .await?
                    .len(),
                3
            );
            reloaded.close().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {name} WITH (FORCE)"
        )))
        .execute(&mut admin)
        .await?;
        admin.close().await?;
        result
    })
}
