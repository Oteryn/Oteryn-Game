#![allow(clippy::expect_used)]

use oteryn_game_server::character_recovery_fence::{
    CharacterRecoveryError, CharacterRecoveryStore,
};
use oteryn_game_server::domain::{AccountId, CharacterId, WorldId};
use prost::Message;
use sqlx::Connection;

#[path = "../src/durability/character_authority_audit.rs"]
mod audit;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

#[test]
fn registered_payload_encoding_matches_descriptor_shape() {
    let bytes = audit::encode_bootstrap(&id(1), &id(2), &id(3));
    let decoded = audit::CharacterAuthorityBootstrappedV1::decode(bytes.as_slice())
        .expect("registered payload decodes");
    assert_eq!(decoded.account_id, id(1));
    assert_eq!(decoded.character_id, id(2));
    assert_eq!(decoded.world_id, id(3));
    assert_eq!(decoded.character_revision, 1);
    assert_eq!(audit::EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED, 1);
    assert_eq!(audit::EVENT_SCHEMA_REVISION, 1);
    assert_eq!(
        audit::RETENTION_PROFILE,
        "CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1"
    );
}

#[test]
fn authority_identifiers_are_distinct_uuidv7_types() {
    assert!(AccountId::from_bytes(id(1)).is_ok());
    assert!(CharacterId::from_bytes(id(2)).is_ok());
    assert!(WorldId::from_bytes(id(3)).is_ok());
}

#[test]
fn external_recovery_register_is_strict_and_fail_closed() {
    let directory = std::env::temp_dir().join(format!(
        "oteryn-character-recovery-integration-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).expect("retained directory");
    let store =
        CharacterRecoveryStore::open(&directory, "character-primary", "game-ops").expect("store");
    assert!(matches!(
        store.seal_current(),
        Err(CharacterRecoveryError::Unavailable)
    ));
    drop(
        store
            .authorize_fresh_store(id(1), 100)
            .expect("explicit fresh authorization"),
    );
    drop(
        store
            .begin_recovery(1, id(2), 200)
            .expect("strict successor"),
    );
    drop(
        store
            .begin_recovery(1, id(2), 200)
            .expect("exact ambiguous reconciliation"),
    );
    assert!(matches!(
        store.begin_recovery(1, id(3), 200),
        Err(CharacterRecoveryError::Conflict)
    ));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn postgres_schema_enforces_atomic_immutable_first_slice() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("character_authority_postgres requires configured PostgreSQL 17.6");
        return;
    };
    let mut connection = sqlx::PgConnection::connect(&admin)
        .await
        .expect("connect PostgreSQL");
    let schema = format!("character_authority_{}", std::process::id());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&mut connection)
        .await
        .expect("schema");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&mut connection)
        .await
        .expect("path");
    sqlx::raw_sql(include_str!("../migrations/0005_character_authority.sql"))
        .execute(&mut connection)
        .await
        .expect("migration");
    let retained = std::env::temp_dir().join(format!("oteryn-character-pg-fence-{schema}"));
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained).expect("external retained directory");
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .expect("external recovery store");
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .expect("explicit fresh authorization");
    sqlx::query("INSERT INTO game_character_recovery_admissions VALUES ('character-primary', 1, encode($1,'hex')::uuid, 0, 100, 'game-ops', 100)")
        .bind(id(11).as_slice())
        .execute(&mut connection)
        .await
        .expect("fresh DB admission");
    drop(fresh);
    let transition = recovery
        .begin_recovery(1, id(12), 200)
        .expect("external strict successor");
    let db_generation: String = sqlx::query_scalar("SELECT max(recovery_generation)::text FROM game_character_recovery_admissions")
        .fetch_one(&mut connection).await.expect("DB generation");
    assert_eq!(db_generation, "1");
    assert_eq!(transition.record.recovery_generation, 2, "older DB stays distinguishable while the external retained directory remains advanced");
    sqlx::query("INSERT INTO game_character_recovery_admissions VALUES ('character-primary', 2, encode($1,'hex')::uuid, 1, 200, 'game-ops', 200)")
        .bind(id(12).as_slice())
        .execute(&mut connection)
        .await
        .expect("explicit recovery reconciliation");
    drop(transition);
    let mut tx = connection.begin().await.expect("tx");
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0001')").execute(&mut *tx).await.expect("guard");
    sqlx::query("INSERT INTO game_character_roots VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0002','01890f4c-3b2a-7cc2-8d11-9a321b7c0001','01890f4c-3b2a-7cc2-8d11-9a321b7c0003',1,1,'p','r','c','s')").execute(&mut *tx).await.expect("root");
    tx.rollback().await.expect("rollback");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_roots")
        .fetch_one(&mut connection)
        .await
        .expect("count");
    assert_eq!(count, 0, "rollback cannot leave an unaudited root");
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&mut connection)
        .await
        .expect("cleanup");
    std::fs::remove_dir_all(retained).expect("retained cleanup");
        });
}
