#![allow(clippy::expect_used)]
// D1112 installed-storage cases. Every selected case creates its own PostgreSQL 18.6
// database through the actual embedded migration ledger; no fixture DDL is executed.
// Mounted once by the CI-run character_authority_postgres target, which provides
// the path-loaded crate root and the existing Bestiary harness.

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, id, runtime,
};
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceRejection, Sex, UnlockFact, validate_selection,
};
use crate::domain::{AccountId, CharacterId};
use crate::durability::character_appearance::stored as character_appearance;
use character_appearance::{
    AccountAppearanceUnlock, read_account_appearance_unlocks, read_selected_appearance,
    verify_character_appearance_integrity,
};
use sqlx::Connection;

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    // Same gate as every other case in the shared PostgreSQL targets: the routed lane sets the
    // admin URL; ordinary workspace runs without the service skip as PRE-ROUTING.
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create_appearance(admin, tag).await?;
        let outcome = body(&h).await;
        h.cleanup().await?;
        outcome
    })
}

// Test-admin storage fixtures, not a grant producer or a replacement gameplay writer.
// All root FKs, immutable receipts and the complete global revision guard remain installed.
async fn advance_revision(connection: &mut sqlx::PgConnection, next: u64) -> TestResult {
    for query in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
    ] {
        assert_eq!(
            sqlx::query(query)
                .bind(id(CHARACTER).as_slice())
                .bind(next.to_string())
                .bind((next - 1).to_string())
                .execute(&mut *connection)
                .await?
                .rows_affected(),
            1
        );
    }
    Ok(())
}

async fn insert_initial(
    connection: &mut sqlx::PgConnection,
    outfit: &str,
    mount: Option<&str>,
) -> TestResult {
    let mut tx = connection.begin().await?;
    advance_revision(&mut tx, 2).await?;
    sqlx::query("INSERT INTO game_character_appearance_receipts(appearance_occurrence_id,command_binding,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,outfit_after,colours_after,addons_after,mount_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,decode('01' || repeat('00',32),'hex'),character_id,1,2,level,level,total_experience,total_experience,$2,ARRAY[0,1,2,132],1,$3,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,1 FROM game_character_progression_state WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(70).as_slice()).bind(outfit).bind(mount).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_appearance SELECT character_id,outfit_after,colours_after,addons_after,mount_after,content_revision,committed_character_revision,appearance_occurrence_id FROM game_character_appearance_receipts WHERE appearance_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(70).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn seed_unlocks(connection: &mut sqlx::PgConnection) -> TestResult {
    for (seed, key, revision) in [
        (60, "earned", "content-1"),
        (61, "fixture-addon-one", "content-1"),
        (62, "horse", "incompatible"),
    ] {
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid,'quest',$1)")
            .bind(id(seed).as_slice()).bind(id(40).as_slice()).bind(key).bind(revision).bind(id(CHARACTER).as_slice()).execute(&mut *connection).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
            .bind(id(40).as_slice()).bind(key).bind(id(seed).as_slice()).execute(&mut *connection).await?;
    }
    Ok(())
}

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
        ("ARRAY[0,0,0,0]", "ARRAY[0,0,0]", "exactly four colours"),
        (
            "content_revision,simulation_revision",
            "repeat('é',65),simulation_revision",
            "UTF-8 provenance byte bound",
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
        advance_revision(&mut tx, 3).await?;
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
        let root_state: String = sqlx::query_scalar("SELECT r.character_revision::text || ':' || s.character_revision::text FROM game_character_roots r JOIN game_character_progression_state s USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&mut *connection).await?;
        assert_eq!(root_state, "2:2", "rollback changed root/state: {label}");
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
    commit_successor(connection).await
}

async fn commit_successor(connection: &mut sqlx::PgConnection) -> TestResult {
    let mut tx = connection.begin().await?;
    advance_revision(&mut tx, 3).await?;
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

#[test]
fn installed_appearance_roundtrip_preserves_unknown_keys_and_provenance() -> TestResult {
    run("app1_storage_roundtrip", async |h| {
        let character = CharacterId::from_bytes(id(CHARACTER))?;
        let mut connection = sqlx::PgConnection::connect(&h.database.url).await?;
        assert_eq!(
            read_selected_appearance(&mut connection, character).await?,
            None
        );
        insert_initial(&mut connection, "unknown/outfit", Some("unknown/mount")).await?;
        let selected = read_selected_appearance(&mut connection, character)
            .await?
            .ok_or("missing selection")?;
        assert_eq!(selected.selection.outfit_key, "unknown/outfit");
        assert_eq!(
            selected.selection.mount_key.as_deref(),
            Some("unknown/mount")
        );
        assert_eq!(selected.selection.colours, [0, 1, 2, 132]);
        assert_eq!(selected.selection.addons, 1);
        assert_eq!(selected.content_revision, "content-1");
        assert_eq!(selected.committed_character_revision.get(), 2);
        assert_eq!(
            read_selected_appearance(&mut connection, CharacterId::from_bytes(id(42))?).await?,
            None
        );
        connection.close().await?;
        let mut reloaded = sqlx::PgConnection::connect(&h.database.url).await?;
        assert_eq!(
            read_selected_appearance(&mut reloaded, character).await?,
            Some(selected.clone())
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            h.root
                .read_character_appearance(&authority, character)
                .await
                .map_err(debug)?,
            Some(selected)
        );
        Ok(())
    })
}

#[test]
fn installed_appearance_receipt_constraints_and_transaction_rollback() -> TestResult {
    run("app1_storage_constraints", async |h| {
        let mut connection = sqlx::PgConnection::connect(&h.database.url).await?;
        insert_initial(&mut connection, "earned", Some("horse")).await?;
        receipt_constraints(&mut connection).await?;
        verify_character_appearance_integrity(&mut connection).await?;
        assert_eq!(h.root_revision().await?, "3");
        let seal = h.recovery.seal_current().map_err(debug)?;
        h.root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn installed_appearance_unlocks_preserve_first_provenance_and_account_isolation() -> TestResult {
    run("app1_storage_unlocks", async |h| {
        let mut connection = sqlx::PgConnection::connect(&h.database.url).await?;
        let account = AccountId::from_bytes(id(40))?;
        let other = AccountId::from_bytes(id(39))?;
        seed_unlocks(&mut connection).await?;
        let facts = read_account_appearance_unlocks(
            &mut connection,
            account,
            &["earned".into(), "fixture-addon-one".into(), "horse".into()],
        )
        .await?;
        assert_eq!(facts.len(), 3);
        assert_eq!(
            facts[0].granting_character_id,
            CharacterId::from_bytes(id(CHARACTER))?
        );
        assert_eq!(facts[0].source_kind, "quest");
        assert!(!facts[0].source_event_id.is_empty());
        assert!(
            read_account_appearance_unlocks(
                &mut connection,
                other,
                &["earned".into(), "fixture-addon-one".into(), "horse".into()]
            )
            .await?
            .is_empty()
        );
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'earned','replacement',encode($3,'hex')::uuid,'npc',$1)")
            .bind(id(63).as_slice()).bind(account.as_bytes().as_slice()).bind(id(42).as_slice()).execute(&mut connection).await?;
        let duplicate=sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'earned',encode($2,'hex')::uuid) ON CONFLICT(account_id,unlock_key) DO NOTHING")
            .bind(account.as_bytes().as_slice()).bind(id(63).as_slice()).execute(&mut connection).await?;
        assert_eq!(duplicate.rows_affected(), 0);
        assert_eq!(
            read_account_appearance_unlocks(
                &mut connection,
                account,
                &["earned".into(), "fixture-addon-one".into(), "horse".into()]
            )
            .await?,
            facts
        );
        let collision=sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'earned','substitution',encode($3,'hex')::uuid,'quest',$4)")
            .bind(id(64).as_slice()).bind(other.as_bytes().as_slice()).bind(id(42).as_slice()).bind(id(60).as_slice()).execute(&mut connection).await;
        assert_eq!(
            collision
                .err()
                .and_then(|e| e
                    .as_database_error()
                    .and_then(|e| e.code())
                    .map(|s| s.into_owned()))
                .as_deref(),
            Some("23505")
        );
        for query in [
            "UPDATE game_account_appearance_unlocks SET unlock_key='changed'",
            "DELETE FROM game_account_appearance_unlocks",
            "TRUNCATE game_account_appearance_unlocks",
            "UPDATE game_account_appearance_unlock_receipts SET content_revision='changed'",
            "DELETE FROM game_account_appearance_unlock_receipts",
            "TRUNCATE game_account_appearance_unlock_receipts CASCADE",
        ] {
            let error = sqlx::query(query)
                .execute(&mut connection)
                .await
                .err()
                .ok_or("immutable account data changed")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
        }
        connection.close().await?;
        let mut reloaded = sqlx::PgConnection::connect(&h.database.url).await?;
        assert_eq!(
            read_account_appearance_unlocks(
                &mut reloaded,
                account,
                &["earned".into(), "fixture-addon-one".into(), "horse".into()]
            )
            .await?,
            facts
        );
        Ok(())
    })
}

#[test]
fn installed_appearance_validator_and_recovery_reject_corruption() -> TestResult {
    run("app1_storage_recovery", async |h| {
        let mut connection = sqlx::PgConnection::connect(&h.database.url).await?;
        let character = CharacterId::from_bytes(id(CHARACTER))?;
        insert_initial(&mut connection, "earned", Some("horse")).await?;
        seed_unlocks(&mut connection).await?;
        let facts = read_account_appearance_unlocks(
            &mut connection,
            AccountId::from_bytes(id(40))?,
            &["earned".into(), "fixture-addon-one".into(), "horse".into()],
        )
        .await?;
        let original = read_selected_appearance(&mut connection, character)
            .await?
            .ok_or("missing selection")?;
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
        let missing: Vec<_> = facts
            .iter()
            .filter(|fact| fact.unlock_key != "fixture-addon-one")
            .cloned()
            .collect();
        assert_eq!(
            validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&missing)),
            Err(AppearanceRejection::AddonNotAuthorized { addon: 1 })
        );
        valid.outfit_key = "store".into();
        assert_eq!(
            validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&facts)),
            Err(AppearanceRejection::OutfitNotAuthorized)
        );
        valid.outfit_key = "earned".into();
        valid.colours[0] = 133;
        assert_eq!(
            validate_selection(&valid, Sex::Male, &Content, &FixtureEarned(&facts)),
            Err(AppearanceRejection::ColourOutOfPalette { slot: 0 })
        );
        commit_successor(&mut connection).await?;
        verify_character_appearance_integrity(&mut connection).await?;
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
            verify_character_appearance_integrity(&mut connection).await?;
        }
        assert_eq!(
            read_selected_appearance(&mut connection, character)
                .await?
                .ok_or("lost selection")?
                .selection
                .outfit_key,
            "starter"
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        h.root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}

#[path = "character_appearance_postgres_cases/earned_unlocks_tests.rs"]
mod earned_unlocks_tests;
