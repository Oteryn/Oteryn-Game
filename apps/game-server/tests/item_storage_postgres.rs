//! DEPOT0 private schema/read qualification only, not native TRANSFER/exclusivity integration.
//! Uses real public native imports and the exact owned module by path; no extracted facade.
use oteryn_game_server::{domain, durability};
#[path = "../src/durability/character_depot.rs"]
mod depot_owner;
// One canonical mount of the three pure cases; the library owns no copy.
#[path = "../src/durability/character_depot_tests.rs"]
mod depot_tests;
use depot_owner::*;
use sqlx::{Connection, Executor, PgConnection};
use std::num::NonZeroU64;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn depot_proposed_storage_postgres17_6() -> TestResult {
    let Ok(admin_url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: depot PG17.6 qualification not executed");
        return Ok(());
    };
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut admin = PgConnection::connect(&admin_url).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&mut admin)
                .await?;
            assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let name = format!("depot_prep_{suffix}");
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {name}"
                ))))
                .await?;
            let url = format!(
                "{}/{}",
                admin_url.strip_suffix("/postgres").ok_or("admin URL")?,
                name
            );
            let outcome = async {
                let mut connection = PgConnection::connect(&url).await?;
                sqlx::migrate!("./migrations").run(&mut connection).await?;
                connection.execute(PROPOSED_SCHEMA).await?;
                qualify(&mut connection, &url).await
            }
            .await;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE {name} WITH (FORCE)"
                ))))
                .await?;
            outcome
        })
}

async fn refusal(connection: &mut PgConnection, sql: &str, expected: &str) -> TestResult {
    let mut tx = connection.begin().await?;
    let error = tx
        .execute(sqlx::AssertSqlSafe(sql))
        .await
        .err()
        .ok_or("expected refusal")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some(expected)
    );
    tx.rollback().await?;
    Ok(())
}

async fn qualify(connection: &mut PgConnection, url: &str) -> TestResult {
    // Native base rows are fixtures with native proof triggers off, never a production writer.
    // All proposed depot guards are on for each operation being qualified below.
    connection.execute(r#"
BEGIN;
SET LOCAL session_replication_role=replica;
INSERT INTO game_character_account_guards(account_id) VALUES
 ('00000000-0000-7000-8000-000000000001'),('00000000-0000-7000-8000-000000000002');
INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,
 profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES
 ('00000000-0000-7000-8000-000000000011','00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000021',1,1,'p1','r1','c1','s1','DepotHero'),
 ('00000000-0000-7000-8000-000000000012','00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000022',1,1,'p1','r1','c1','s1','OtherHero');
INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
 definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 SELECT ('00000000-0001-7000-8000-'||lpad(to_hex(n),12,'0'))::uuid,
 '00000000-0000-7000-8000-000000000021','Item','fixture:depot','r1',1,1,game_character_uuid_v7()
 FROM generate_series(1,15001) n;
INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
 definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES
 ('00000000-0002-7000-8000-000000000001','00000000-0000-7000-8000-000000000022','Item','fixture:other','r1',1,1,game_character_uuid_v7());
INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,box,ordinal)
 SELECT item_instance_id,world_id,'00000000-0000-7000-8000-000000000011',1,
 ('x'||right(item_instance_id::text,12))::bit(48)::bigint
 FROM game_item_instances WHERE world_id='00000000-0000-7000-8000-000000000021'
 AND item_instance_id < '00000000-0001-7000-8000-000000003a98'::uuid;
COMMIT;
"#).await?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 0x11;
    let character = domain::CharacterId::from_bytes(id)?;
    id[15] = 0x21;
    let world = domain::WorldId::from_bytes(id)?;
    let owner = DepotOwner { character, world };
    let box_id = DepotBox::new(1).ok_or("box")?;
    assert_eq!(read_summary(connection, owner).await?[0], 14999);
    let first = read_box_page(connection, owner, box_id, None).await?;
    assert_eq!(first.len(), 32);
    assert_eq!(first[0].ordinal.get(), 14999);
    let second = read_box_page(connection, owner, box_id, first.last().map(|r| r.ordinal)).await?;
    assert_eq!(second.len(), 32);
    assert_eq!(second[0].ordinal.get(), 14967);
    assert!(
        read_box_page(connection, owner, box_id, NonZeroU64::new(1))
            .await?
            .is_empty()
    );
    id[15] = 0x22;
    let other_world = domain::WorldId::from_bytes(id)?;
    assert_eq!(
        read_summary(
            connection,
            DepotOwner {
                world: other_world,
                ..owner
            }
        )
        .await?,
        [0; 17]
    );
    // Each invalid insertion changes one field against an otherwise valid new row.
    let valid = "INSERT INTO game_item_character_depot_locations VALUES ('00000000-0001-7000-8000-000000003a98','00000000-0000-7000-8000-000000000021','00000000-0000-7000-8000-000000000011',1,15000)";
    for (from, to) in [
        ("1,15000)", "0,15000)"),
        ("1,15000)", "18,15000)"),
        ("1,15000)", "1,0)"),
        ("1,15000)", "1,18446744073709551616)"),
    ] {
        refusal(connection, &valid.replace(from, to), "23514").await?;
    }
    refusal(connection, &valid.replace("1,15000)", "1,1)"), "23505").await?;
    refusal(
        connection,
        &valid.replace("000000003a98'", "000000003a97'"),
        "23505",
    )
    .await?;
    refusal(
        connection,
        &valid.replace("000000000011'", "000000000012'"),
        "23514",
    )
    .await?;
    let wrong_item = valid.replace(
        "00000000-0001-7000-8000-000000003a98",
        "00000000-0002-7000-8000-000000000001",
    );
    refusal(connection, &wrong_item, "23503").await?;
    refusal(
        connection,
        "UPDATE game_item_character_depot_locations SET box=2 WHERE ordinal=1",
        "23514",
    )
    .await?;
    refusal(
        connection,
        "TRUNCATE game_item_character_depot_locations",
        "23514",
    )
    .await?;
    // At the hard bound the deferred guard takes the native Character root lock.
    let mut first_tx = connection.begin().await?;
    first_tx.execute(valid).await?;
    first_tx.execute("SET CONSTRAINTS ALL IMMEDIATE").await?;
    let mut second_connection = PgConnection::connect(url).await?;
    let mut second_tx = second_connection.begin().await?;
    second_tx.execute("SET LOCAL lock_timeout='100ms'").await?;
    let extra = valid
        .replace("000000003a98'", "000000003a99'")
        .replace("1,15000)", "1,15001)");
    // Exercise a competing insertion, not a separate manual root lock. Its Character FK
    // may already wait for the winner's FOR UPDATE; otherwise force its deferred guard.
    let error = match second_tx.execute(sqlx::AssertSqlSafe(extra.as_str())).await {
        Err(error) => error,
        Ok(_) => second_tx
            .execute("SET CONSTRAINTS ALL IMMEDIATE")
            .await
            .err()
            .ok_or("competing insertion did not wait for the root lock")?,
    };
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("55P03")
    );
    second_tx.rollback().await?;
    // Rolling back the blocked insertion leaves the winner's uncommitted row untouched.
    assert_eq!(read_summary(&mut first_tx, owner).await?[0], 15000);
    first_tx.commit().await?;
    let mut over = connection.begin().await?;
    over.execute(sqlx::AssertSqlSafe(extra.as_str())).await?;
    let error = over.commit().await.err().ok_or("hard capacity exceeded")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert_eq!(read_summary(connection, owner).await?[0], 15000);
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id='00000000-0000-7000-8000-000000000011'").fetch_one(&mut *connection).await?;
    assert_eq!(
        revision, "1",
        "capacity guard does not advance CharacterRevision"
    );
    // Deliberately corrupt fixture beyond the hard ceiling; bounded summary refuses it.
    connection
        .execute("BEGIN; SET LOCAL session_replication_role=replica")
        .await?;
    connection
        .execute(sqlx::AssertSqlSafe(extra.as_str()))
        .await?;
    connection.execute("COMMIT").await?;
    assert!(matches!(
        read_summary(connection, owner).await,
        Err(durability::DurabilityError::InvalidStoredState)
    ));
    assert_eq!(
        read_box_page(connection, owner, box_id, None).await?.len(),
        32
    );
    eprintln!(
        "QUALIFIED PostgreSQL17.6: proposed depot schema/read/bounds/root-lock only; native TRANSFER/custody integration NOT qualified"
    );
    Ok(())
}
