//! Apply the actual E12 migration over the shipped 0405 schema and original session facts.
use super::{CurrentCharacterItemFence, Harness, HirelingLampReturn, HirelingReturnInput,
    ReconciledCharacterAuthority, TestResult, HOUSE, OTHER_HOUSE, debug, id};
use sqlx::{Connection, Executor, Row};

pub(super) struct ConvertedPlacement {
    hireling_id: [u8; 16],
    hireling: String,
    receipt: String,
}

pub(super) async fn upgrade(h: &Harness, current: CurrentCharacterItemFence)
    -> TestResult<ConvertedPlacement> {
    let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.rsplit_once('@').is_some_and(|(_, host)| {
            host.strip_prefix("127.0.0.1:")
                .and_then(|rest| rest.strip_suffix("/postgres"))
                .is_some_and(|port| port.bytes().all(|b| b.is_ascii_digit())
                    && port.parse::<u16>().is_ok())
        })
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    // These are the real admitted Character and House reconnect session from the
    // fixture. In particular no GameSessionId, lease or origin Channel is invented.
    let character: String = sqlx::query_scalar(
        "SELECT row_to_json(r)::text FROM game_character_roots r WHERE character_id=encode($1,'hex')::uuid",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let session: String = sqlx::query_scalar(
        "SELECT row_to_json(s)::text FROM game_durability_reconnect_sessions s WHERE game_session_id=encode($1,'hex')::uuid",
    ).bind(current.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let name = format!("hireling_0405_upgrade_{suffix}");
    let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
    admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))).await?;
    let url = format!("{}/{}", admin_url.strip_suffix("/postgres").ok_or("admin URL")?, name);
    let mut connection = sqlx::PgConnection::connect(&url).await?;
    let result = apply_and_check(&mut connection, character, session, current).await;
    connection.close().await?;
    admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE {name} WITH (FORCE)")))).await?;
    admin.close().await?;
    result
}

async fn apply_and_check(connection: &mut sqlx::PgConnection, character: String,
    session: String, current: CurrentCharacterItemFence) -> TestResult<ConvertedPlacement> {
    let before = sqlx::migrate::Migrator::with_migrations(
        oteryn_gameplay_durability::GAME_MIGRATOR.iter()
            .filter(|migration| migration.version < 700).cloned().collect(),
    );
    before.run(&mut *connection).await?;
    let mut tx = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    super::insert_character_root_copy(&mut tx, &character).await?;
    super::insert_reconnect_session_copy(&mut tx, &session).await?;
    for (house, x) in [(HOUSE, 32310_i32), (OTHER_HOUSE, 32311_i32)] {
        sqlx::query("INSERT INTO game_house_hirelings(world_id,house_key,owner_character_id,x,y,floor) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,32805,-7)")
            .bind(id(42).as_slice()).bind(house).bind(id(41).as_slice()).bind(x)
            .execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_house_hireling_receipts(game_session_id,command_id,active,x,y,floor) VALUES(encode($1,'hex')::uuid,55,TRUE,32310,32805,-7)")
        .bind(current.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    let upgraded = sqlx::migrate::Migrator::with_migrations(
        oteryn_gameplay_durability::GAME_MIGRATOR.iter()
            .filter(|migration| migration.version <= 700).cloned().collect(),
    );
    upgraded.run(&mut *connection).await?;
    let rows = sqlx::query("SELECT house_key,uuid_send(owner_character_id) AS owner,x,y,floor,active,legacy_grant, \
        game_character_is_uuid_v7(hireling_id) AS valid_identity,uuid_send(lamp_item_instance_id) AS lamp, \
        uuid_send(last_lamp_session) AS source_session,last_lamp_command::text AS source_command, \
        last_lamp_connection_generation::text AS connection_generation,last_lamp_lease_generation::text AS lease_generation, \
        uuid_send(last_lamp_channel) AS origin FROM game_house_hirelings ORDER BY house_key")
        .fetch_all(&mut *connection).await?;
    assert_eq!(rows.len(), 2, "every legacy placement survives conversion");
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row.try_get::<Vec<u8>, _>("owner")?, id(41));
        assert!(row.try_get::<bool, _>("active")?);
        assert!(row.try_get::<bool, _>("legacy_grant")?);
        assert!(row.try_get::<bool, _>("valid_identity")?);
        assert_eq!(row.try_get::<Option<Vec<u8>>, _>("lamp")?, None);
        assert_eq!(row.try_get::<i32, _>("x")?, 32310 + i32::try_from(index)?);
        assert_eq!(row.try_get::<i32, _>("y")?, 32805);
        assert_eq!(row.try_get::<i16, _>("floor")?, -7);
    }
    assert_eq!(rows[0].try_get::<String, _>("house_key")?, HOUSE);
    assert_eq!(rows[0].try_get::<Option<Vec<u8>>, _>("source_session")?,
        Some(current.game_session_id.as_bytes().to_vec()));
    assert_eq!(rows[0].try_get::<Option<String>, _>("source_command")?, Some("55".into()));
    assert_eq!(rows[0].try_get::<Option<String>, _>("connection_generation")?,
        Some(current.connection_generation.get().to_string()));
    assert_eq!(rows[0].try_get::<Option<String>, _>("lease_generation")?,
        Some(current.character_lease_generation.to_string()));
    assert_eq!(rows[0].try_get::<Option<Vec<u8>>, _>("origin")?, Some(id(43).to_vec()));
    assert_eq!(rows[1].try_get::<Option<Vec<u8>>, _>("source_session")?, None,
        "missing historical provenance cannot be replaced by a fabricated session");
    let quarantine: Vec<(String, String)> = sqlx::query_as(
        "SELECT house_key,reason FROM game_hireling_recovery_quarantine ORDER BY house_key",
    ).fetch_all(&mut *connection).await?;
    assert_eq!(quarantine, vec![(OTHER_HOUSE.into(),
        "missing authentic legacy creation receipt".into())],
        "Q9: report precisely the preserved malformed row; valid legacy provenance stays recoverable");
    let grants: Vec<i16> = sqlx::query_scalar("SELECT entitlement_id FROM game_hireling_entitlements WHERE character_id=encode($1,'hex')::uuid ORDER BY entitlement_id")
        .bind(id(41).as_slice()).fetch_all(&mut *connection).await?;
    assert_eq!(grants, vec![1001], "grandfather Banker once per Character, with no paid unlocks");
    let linked: (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT uuid_send(r.hireling_id),uuid_send(r.character_id) FROM game_house_hireling_receipts r \
         WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=55",
    ).bind(current.game_session_id.as_bytes().as_slice()).fetch_one(&mut *connection).await?;
    let hireling_id: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(hireling_id) FROM game_house_hirelings WHERE house_key=$1",
    ).bind(HOUSE).fetch_one(&mut *connection).await?;
    assert_eq!(linked, (hireling_id.clone(), id(41).to_vec()),
        "the authentic old receipt keeps its immutable owner/identity association");
    let hireling: String = sqlx::query_scalar(
        "SELECT row_to_json(h)::text FROM game_house_hirelings h WHERE house_key=$1",
    ).bind(HOUSE).fetch_one(&mut *connection).await?;
    let receipt: String = sqlx::query_scalar(
        "SELECT row_to_json(r)::text FROM game_house_hireling_receipts r \
         WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=55",
    ).bind(current.game_session_id.as_bytes().as_slice()).fetch_one(&mut *connection).await?;
    Ok(ConvertedPlacement { hireling_id: hireling_id.try_into().map_err(|_| "legacy Hireling identity")?,
        hireling, receipt })
}

pub(super) async fn recall_after_session_retirement(h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>, current: CurrentCharacterItemFence,
    converted: ConvertedPlacement) -> TestResult {
    // Import the exact rows produced by the real0405->0700 migration above.
    // This is a historical fixture, not a synthetic session or source receipt.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_hirelings SELECT * FROM jsonb_populate_record(NULL::game_house_hirelings,$1::text::jsonb)")
        .bind(converted.hireling).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_hireling_receipts SELECT * FROM jsonb_populate_record(NULL::game_house_hireling_receipts,$1::text::jsonb)")
        .bind(converted.receipt).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(h.root.pending_hireling_recalls(authority, &h.node, current.runtime_scope.world_id())
        .await.map_err(debug)?.is_empty());
    super::seed_released_house_and_offline_session(h, current).await?;
    let absent: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid)")
        .bind(current.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(absent);
    let pending = h.root.pending_hireling_recalls(authority, &h.node, current.runtime_scope.world_id())
        .await.map_err(debug)?;
    assert_eq!(pending.len(), 1, "a valid migrated identity remains recoverable after its old session is gone");
    let state = &pending[0];
    assert_eq!(state.hireling.id, converted.hireling_id);
    assert_eq!(state.source.command.game_session_id(), current.game_session_id);
    assert_eq!(state.source.command.command_id().get(), 55);
    let returned = HirelingLampReturn { hireling: converted.hireling_id,
        input: HirelingReturnInput::Management, source: state.source.clone(),
        mint: super::mint_request(vec![super::lamp_output(0)]) };
    h.root.recall_hireling_to_lamp(authority, &h.node, state.authority.clone(), returned.clone())
        .await.map_err(debug)?;
    super::assert_inbox_scope(h, returned.source.transaction_id, 2,
        state.authority.scope_ownership_generation).await?;
    assert!(!super::active(h, converted.hireling_id).await?);
    assert_eq!(super::inbox_count(h).await?, 1);
    h.root.recall_hireling_to_lamp(authority, &h.node, state.authority.clone(), returned)
        .await.map_err(debug)?;
    assert_eq!(super::inbox_count(h).await?, 1, "offline legacy replay must not mint a second lamp");
    assert!(h.root.pending_hireling_recalls(authority, &h.node, current.runtime_scope.world_id())
        .await.map_err(debug)?.is_empty());
    Ok(())
}
