//! Private schema qualification: historical reads, immutable appearance receipts and projection
//! consistency. No gameplay fence, grant producer or production migration is installed. Fixture
//! INSERTs are storage constraint tests, not gameplay writes or relog qualification.
pub use oteryn_game_server::domain;
#[path = "../src/durability/character_appearance.rs"]
mod character_appearance;

use character_appearance::{
    AccountAppearanceUnlock, read_account_appearance_unlocks, read_selected_appearance,
    verify_character_appearance_integrity,
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
// The CP slot must add root/account constraints, the shared progression guard union and recovery
// verifier, permissions and fenced producers together. This fixture grants no write authority.
const PRIVATE_SCHEMA: &str = r#"
CREATE FUNCTION app_colours_valid(value INTEGER[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT array_ndims(value)=1 AND array_lower(value,1)=1 AND cardinality(value)=4
   AND array_position(value,NULL) IS NULL
   AND 0 <= ALL(value) AND 65535 >= ALL(value)
$$;
CREATE TABLE game_character_appearance_receipts (
 appearance_occurrence_id UUID PRIMARY KEY
   CHECK(get_byte(uuid_send(appearance_occurrence_id),6)>>4=7
     AND get_byte(uuid_send(appearance_occurrence_id),8)&192=128),
 command_binding BYTEA NOT NULL CHECK(octet_length(command_binding)=33 AND get_byte(command_binding,0)=1),
 character_id UUID NOT NULL,
 original_character_revision NUMERIC(20,0) NOT NULL
   CHECK(original_character_revision BETWEEN 1 AND 18446744073709551614),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision=original_character_revision+1),
 level_before BIGINT NOT NULL CHECK(level_before BETWEEN 1 AND 4294967295),
 level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL CHECK(experience_before>=0),
 experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 outfit_before TEXT,
 colours_before INTEGER[],
 addons_before SMALLINT,
 mount_before TEXT,
 content_before TEXT,
 selection_revision_before NUMERIC(20,0),
 outfit_after TEXT NOT NULL,
 colours_after INTEGER[] NOT NULL CHECK(app_colours_valid(colours_after)),
 addons_after SMALLINT NOT NULL CHECK(addons_after BETWEEN 0 AND 255),
 mount_after TEXT,
 profile_revision TEXT NOT NULL CHECK(octet_length(profile_revision) BETWEEN 1 AND 128),
 ruleset_revision TEXT NOT NULL CHECK(octet_length(ruleset_revision) BETWEEN 1 AND 128),
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 simulation_revision TEXT NOT NULL CHECK(octet_length(simulation_revision) BETWEEN 1 AND 128),
 evidence_revision TEXT NOT NULL CHECK(octet_length(evidence_revision) BETWEEN 1 AND 128),
 declaration_revision TEXT NOT NULL CHECK(octet_length(declaration_revision) BETWEEN 1 AND 128),
 policy_revision TEXT NOT NULL CHECK(octet_length(policy_revision) BETWEEN 1 AND 128),
 reward_revision TEXT NOT NULL CHECK(octet_length(reward_revision) BETWEEN 1 AND 128),
 committed_at BIGINT NOT NULL CHECK(committed_at>=0),
 CHECK((outfit_before IS NULL AND colours_before IS NULL AND addons_before IS NULL
         AND mount_before IS NULL AND content_before IS NULL AND selection_revision_before IS NULL)
    OR (outfit_before IS NOT NULL AND colours_before IS NOT NULL AND addons_before IS NOT NULL
         AND content_before IS NOT NULL AND selection_revision_before IS NOT NULL
         AND app_colours_valid(colours_before) AND addons_before BETWEEN 0 AND 255
         AND octet_length(content_before) BETWEEN 1 AND 128
         AND selection_revision_before BETWEEN 2 AND original_character_revision)),
 CHECK((outfit_before,colours_before,addons_before,mount_before)
     IS DISTINCT FROM (outfit_after,colours_after,addons_after,mount_after)),
 UNIQUE(character_id,original_character_revision),
 UNIQUE(character_id,committed_character_revision),
 UNIQUE(appearance_occurrence_id,character_id,committed_character_revision)
);
CREATE TABLE game_character_appearance (
 character_id UUID PRIMARY KEY,
 outfit_key TEXT NOT NULL,
 colours INTEGER[] NOT NULL CHECK(app_colours_valid(colours)),
 addons SMALLINT NOT NULL CHECK(addons BETWEEN 0 AND 255),
 mount_key TEXT,
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 128),
 committed_character_revision NUMERIC(20,0) NOT NULL
   CHECK(committed_character_revision BETWEEN 2 AND 18446744073709551615),
 last_appearance_occurrence_id UUID NOT NULL,
 FOREIGN KEY(last_appearance_occurrence_id,character_id,committed_character_revision)
   REFERENCES game_character_appearance_receipts
    (appearance_occurrence_id,character_id,committed_character_revision)
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
BEGIN RAISE EXCEPTION 'immutable appearance receipt or fact' USING ERRCODE='23514'; END;
$$;
CREATE TRIGGER app_selection_receipt_immutable BEFORE UPDATE OR DELETE
 ON game_character_appearance_receipts FOR EACH ROW EXECUTE FUNCTION app_fixture_immutable();
CREATE TRIGGER app_selection_receipt_no_truncate BEFORE TRUNCATE
 ON game_character_appearance_receipts EXECUTE FUNCTION app_fixture_immutable();
CREATE FUNCTION app_selection_row_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.character_id=OLD.character_id THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'appearance projection cannot be deleted or reassigned' USING ERRCODE='23514';
END;
$$;
CREATE TRIGGER app_selection_row_guard BEFORE UPDATE OR DELETE
 ON game_character_appearance FOR EACH ROW EXECUTE FUNCTION app_selection_row_guard();
CREATE TRIGGER app_selection_no_truncate BEFORE TRUNCATE
 ON game_character_appearance EXECUTE FUNCTION app_fixture_immutable();
CREATE FUNCTION app_selection_consistency() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_character UUID;
BEGIN
 v_character:=NEW.character_id;
 IF EXISTS (
   WITH ordered AS (
     SELECT a.*, lag(outfit_after) OVER w AS previous_outfit,
       lag(colours_after) OVER w AS previous_colours,
       lag(addons_after) OVER w AS previous_addons,
       lag(mount_after) OVER w AS previous_mount,
       lag(content_revision) OVER w AS previous_content,
       lag(committed_character_revision) OVER w AS previous_revision,
       row_number() OVER (ORDER BY committed_character_revision DESC) AS latest
     FROM game_character_appearance_receipts a WHERE character_id=v_character
     WINDOW w AS (ORDER BY committed_character_revision)
   )
   SELECT 1 FROM ordered a WHERE
     (outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before)
       IS DISTINCT FROM
     (previous_outfit,previous_colours,previous_addons,previous_mount,previous_content,previous_revision)
     OR (latest=1 AND NOT EXISTS (
       SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
         AND (p.outfit_key,p.colours,p.addons,p.mount_key,p.content_revision,
              p.committed_character_revision,p.last_appearance_occurrence_id)
           IS NOT DISTINCT FROM
             (a.outfit_after,a.colours_after,a.addons_after,a.mount_after,a.content_revision,
              a.committed_character_revision,a.appearance_occurrence_id)))
   UNION ALL
   SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
     AND NOT EXISTS(SELECT 1 FROM ordered)
 ) THEN
   RAISE EXCEPTION 'appearance projection differs from receipt chain' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER app_selection_consistent
 AFTER INSERT OR UPDATE ON game_character_appearance DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION app_selection_consistency();
CREATE CONSTRAINT TRIGGER app_selection_receipt_consistent
 AFTER INSERT ON game_character_appearance_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION app_selection_consistency();
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

// A successor may skip global revisions consumed by other event families. It must still name
// the exact prior appearance selection and its provenance. The fixture tests this chain only;
// the CP slot additionally binds every global revision through the 0073 union.
const SUCCESSOR_INSERT: &str = "INSERT INTO game_character_appearance_receipts \
 SELECT encode($1,'hex')::uuid,command_binding,character_id,2,3,level_after,level_after,\
 experience_after,experience_after,outfit_after,colours_after,addons_after,mount_after,\
 content_revision,committed_character_revision,'starter',ARRAY[0,0,0,0],0,NULL,\
 profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,\
 declaration_revision,policy_revision,reward_revision,committed_at \
 FROM game_character_appearance_receipts WHERE appearance_occurrence_id=encode($2,'hex')::uuid";

async fn receipt_constraints(connection: &mut sqlx::PgConnection) -> TestResult {
    // Every failure uses an otherwise valid binding and rolls back the WHOLE transaction.
    // This is storage atomicity, not a claim that the missing native gameplay fence ran.
    for (old, replacement, label) in [
        (
            "character_id,2,3",
            "character_id,2,4",
            "revision must advance exactly once",
        ),
        (
            "level_after,level_after",
            "level_after,level_after+1",
            "level unchanged",
        ),
        (
            "experience_after,experience_after",
            "experience_after,experience_after+1",
            "experience unchanged",
        ),
        (
            "'starter',ARRAY[0,0,0,0],0,NULL",
            "outfit_after,colours_after,addons_after,mount_after",
            "no cosmetic no-op",
        ),
        (
            "content_revision,committed_character_revision,'starter'",
            "NULL,committed_character_revision,'starter'",
            "complete optional before",
        ),
        ("ARRAY[0,0,0,0]", "ARRAY[0,0,0,NULL]", "non-null colours"),
        (
            "ARRAY[0,0,0,0]",
            "ARRAY[0,0,0,65536]",
            "unsigned-16 colours",
        ),
        (
            "outfit_after,colours_after,addons_after,mount_after",
            "'substituted',colours_after,addons_after,mount_after",
            "exact prior selection",
        ),
        (
            "content_revision,committed_character_revision,'starter'",
            "'substituted',committed_character_revision,'starter'",
            "exact prior provenance",
        ),
    ] {
        assert!(SUCCESSOR_INSERT.contains(old));
        let mut tx = connection.begin().await?;
        let sql = SUCCESSOR_INSERT.replacen(old, replacement, 1);
        let result = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id(71).as_slice())
            .bind(id(70).as_slice())
            .execute(&mut *tx)
            .await;
        let error = match result {
            Err(error) => error,
            Ok(_) => {
                // A deferred check must see the proposed projection as well, so the negative
                // case cannot pass merely because the successor forgot its projection update.
                sqlx::query("UPDATE game_character_appearance SET outfit_key='starter',colours=ARRAY[0,0,0,0],addons=0,mount_key=NULL,committed_character_revision=3,last_appearance_occurrence_id=encode($1,'hex')::uuid")
                    .bind(id(71).as_slice()).execute(&mut *tx).await?;
                match sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                    .execute(&mut *tx)
                    .await
                {
                    Err(error) => error,
                    Ok(_) => return Err(format!("accepted invalid receipt: {label}").into()),
                }
            }
        };
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514"),
            "wrong refusal: {label}"
        );
        tx.rollback().await?;
        let revision: String = sqlx::query_scalar(
            "SELECT committed_character_revision::text FROM game_character_appearance",
        )
        .fetch_one(&mut *connection)
        .await?;
        assert_eq!(revision, "2", "rollback changed projection: {label}");
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
                .fetch_one(&mut *connection)
                .await?;
        assert_eq!(receipts, 1, "rollback retained a receipt: {label}");
    }
    for query in [
        "UPDATE game_character_appearance_receipts SET content_revision='changed'",
        "DELETE FROM game_character_appearance_receipts",
        "TRUNCATE game_character_appearance_receipts CASCADE",
        "DELETE FROM game_character_appearance",
        "TRUNCATE game_character_appearance",
        "UPDATE game_character_appearance SET outfit_key='receipt-free'",
    ] {
        let error = match sqlx::query(query).execute(&mut *connection).await {
            Err(error) => error,
            Ok(_) => return Err(format!("accepted immutable mutation: {query}").into()),
        };
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
    }
    // A real successor of the private chain commits, keeps the historical receipt, and reloads.
    let mut tx = connection.begin().await?;
    sqlx::query(SUCCESSOR_INSERT)
        .bind(id(71).as_slice())
        .bind(id(70).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_appearance SET outfit_key='starter',colours=ARRAY[0,0,0,0],addons=0,mount_key=NULL,committed_character_revision=3,last_appearance_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
            .fetch_one(&mut *connection)
            .await?;
    assert_eq!(receipts, 2);
    Ok(())
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
    let mut selection_tx = connection.begin().await?;
    sqlx::query("INSERT INTO game_character_appearance_receipts(appearance_occurrence_id,command_binding,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,outfit_after,colours_after,addons_after,mount_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) VALUES(encode($2,'hex')::uuid,decode('01' || repeat('00',32),'hex'),encode($1,'hex')::uuid,1,2,1,1,0,0,'earned',ARRAY[0,1,2,132],1,'horse','profile-1','rules-1','old-world','sim-1','evidence-1','decl-1','policy-1','reward-1',1)")
        .bind(character.as_bytes().as_slice()).bind(id(70).as_slice()).execute(&mut *selection_tx).await?;
    sqlx::query("INSERT INTO game_character_appearance VALUES(encode($1,'hex')::uuid,'earned',ARRAY[0,1,2,132],1,'horse','old-world',2,encode($2,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).bind(id(70).as_slice()).execute(&mut *selection_tx).await?;
    selection_tx.commit().await?;
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
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000001','bad',ARRAY[0,0,0],0,NULL,'ok',2,'00000000-0000-7000-8000-000000000070')",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000002','bad',ARRAY[0,0,0,NULL],0,NULL,'ok',2,'00000000-0000-7000-8000-000000000070')",
        "INSERT INTO game_character_appearance VALUES('00000000-0000-7000-8000-000000000003','bad',ARRAY[0,0,0,0],0,NULL,repeat('é',65),2,'00000000-0000-7000-8000-000000000070')",
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
    receipt_constraints(connection).await?;
    verify_character_appearance_integrity(connection).await?;
    // Recovery checks must reject corruption even if a privileged operator bypassed triggers.
    // Each single-field mutation is rolled back before moving to the next invariant.
    for query in [
        "UPDATE game_character_appearance SET outfit_key='corrupt'",
        "UPDATE game_character_appearance_receipts SET content_before='corrupt' WHERE committed_character_revision=3",
        "UPDATE game_character_appearance_receipts SET colours_before=ARRAY[1,1,1,1] WHERE committed_character_revision=3",
        "DELETE FROM game_character_appearance",
    ] {
        let mut tx = connection.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(query).execute(&mut *tx).await?;
        assert!(matches!(
            verify_character_appearance_integrity(&mut tx).await,
            Err(character_appearance::AppearanceReadError::InvalidStoredState)
        ));
        tx.rollback().await?;
        verify_character_appearance_integrity(connection).await?;
    }
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
            assert_eq!(selected.selection.outfit_key, "starter");
            assert_eq!(selected.selection.mount_key, None);
            assert_eq!(selected.committed_character_revision.get(), 3);
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
