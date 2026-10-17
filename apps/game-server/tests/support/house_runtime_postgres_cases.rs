//! Closed-boundary cases use the target's actual native Foundation/House types
//! and the real PG17.6 schema. The canonical aggregate already mounts native
//! Foundation/Durability; its House module joins that same type universe. This
//! is not loader/placement/restart/RL02 qualification.

use oteryn_game_server::{foundation, house_scope};
use sqlx::{Connection, Executor, PgConnection};

#[path = "../../src/gameplay_transport/house_runtime.rs"]
mod house_runtime;
use house_runtime::{activate_house_for_login, exit_house_scope, prepare_house_entry};
// The focused thin wrapper and canonical aggregate share these four pure cases.
// Cargo excludes the focused wrapper from default runs to avoid duplicate cases.
#[path = "../../src/gameplay_transport/house_runtime_tests.rs"]
mod house_runtime_tests;

use foundation::{ChannelId, CharacterId, GameSessionId, RuntimeScopeRefV1, WorldId};
use house_scope::{
    HouseEntryRefusal, HouseEntryRequest, HouseHandoffDirection, HouseHandoffError, HouseId,
    HousePrepareOutcome,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

/// Whole native relations detect source changes, a destination session, a
/// reservation or accidental unrelated-scope cleanup. This snapshot is DATA,
/// never a claim that a runtime actor/assignment/admission has been installed.
async fn snapshot(connection: &mut PgConnection) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object( \
         'sessions',(SELECT COALESCE(jsonb_agg(to_jsonb(s) ORDER BY s.game_session_id),'[]'::jsonb) FROM game_durability_reconnect_sessions s), \
         'handoffs',(SELECT COALESCE(jsonb_agg(to_jsonb(h) ORDER BY h.handoff_id),'[]'::jsonb) FROM game_house_scope_handoffs h), \
         'assignments',(SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY a.scope_key),'[]'::jsonb) FROM game_runtime_scope_assignments a))",
    )
    .fetch_one(connection)
    .await?)
}

async fn closed_boundary_case(connection: &mut PgConnection) -> TestResult {
    // Schema-valid Channel session fixture, with every real trigger enabled.
    // No receipt/guard codec, current authority, physical actor or successful
    // handoff is fabricated: the runtime must refuse before any of those ports.
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions( \
         game_session_id,account_id,character_id,world_id,runtime_scope_kind, \
         runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch, \
         original_grace_deadline,predecessor_generation,character_lease_generation, \
         scope_ownership_generation,current_generation,current_transport_ref,session_state) \
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid, \
         encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid, \
         1,999999,1,1,1,1,$6,1)",
    )
    .bind(id(50).as_slice())
    .bind(id(40).as_slice())
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .bind([9_u8; 16].as_slice())
    .execute(&mut *connection)
    .await?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let request = HouseEntryRequest {
        direction: HouseHandoffDirection::Entry,
        handoff_id: id(60),
        character_id: CharacterId::decode(&id(41)).map_err(debug)?,
        source_game_session_id: GameSessionId::decode(&id(50)).map_err(debug)?,
        source_connection_generation: 1,
        source_character_lease_generation: 1,
        source_scope: RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(43)).map_err(debug)?),
        source_scope_ownership_generation: 1,
        house: HouseId::new(world, "oteryn:content.house.thais_1").ok_or("house key")?,
        house_scope_ownership_generation: 1,
        acl_revision: 1,
        guild_revisions: None,
        // Qualification must not be reached through the closed gate.
        reserved_tile: Vec::new(),
        prepared_at: 100,
    };
    let before = snapshot(connection).await?;
    assert_eq!(before["sessions"].as_array().map(Vec::len), Some(1));
    assert_eq!(before["handoffs"], serde_json::json!([]));
    assert_eq!(
        prepare_house_entry(&request),
        HousePrepareOutcome::Refused(HouseEntryRefusal::HouseClosed)
    );
    assert_eq!(snapshot(connection).await?, before);
    assert_eq!(
        activate_house_for_login(&request.house),
        Err(HouseEntryRefusal::HouseClosed)
    );
    assert_eq!(snapshot(connection).await?, before);
    assert!(matches!(
        exit_house_scope(),
        Err(HouseHandoffError::ExitNotAdmitted)
    ));
    assert_eq!(snapshot(connection).await?, before);
    Ok(())
}

#[test]
fn house_runtime_closed_boundary_preserves_native_source_and_journal() -> TestResult {
    if !crate::HOUSE_RUNTIME_POSTGRES_REQUIRED
        && std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none()
    {
        eprintln!(
            "PRE-ROUTING / NONCANONICAL: House runtime PostgreSQL case not executed; \
             OTERYN_TEST_POSTGRES_ADMIN_URL is absent"
        );
        return Ok(());
    }
    qualify_closed_house_runtime_boundaries()
}

// Preserve strict URL validation, the existing case and every SQL assertion.
// Hold the actual admin connection outside the unwind boundary so cleanup runs
// after either a returned case error or an assertion panic. This successor is
// separate from the D1421 body-preserving organization patch.
fn qualify_closed_house_runtime_boundaries() -> TestResult {
    let Ok(admin_url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Err(
            "PG17.6 case was not executed: OTERYN_TEST_POSTGRES_ADMIN_URL is missing or not valid Unicode"
                .into(),
        );
    };
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("house_runtime_closed_{suffix}");
    let mut admin = runtime.block_on(async {
        let mut admin = PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        Ok::<PgConnection, Box<dyn std::error::Error>>(admin)
    })?;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> TestResult {
        runtime.block_on(async {
            let prefix = admin_url.strip_suffix("/postgres").ok_or("database URL")?;
            let mut connection = PgConnection::connect(&format!("{prefix}/{name}")).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&mut connection)
                .await?;
            assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
            sqlx::migrate!("./migrations").run(&mut connection).await?;
            closed_boundary_case(&mut connection).await?;
            connection.close().await?;
            Ok(())
        })
    }));
    let cleanup: TestResult = runtime.block_on(async {
        let dropped = admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {name} WITH (FORCE)"
            ))))
            .await;
        // Closing is attempted even if DROP returned an error.
        let closed = admin.close().await;
        dropped?;
        closed?;
        Ok(())
    });
    match outcome {
        Ok(Ok(())) => {
            cleanup?;
            println!(
                "QUALIFIED PostgreSQL17.6: closed House runtime boundaries preserve native source session and handoff journal"
            );
            Ok(())
        }
        Ok(Err(error)) => {
            if let Err(cleanup_error) = cleanup {
                eprintln!(
                    "House runtime cleanup failed after a case error for {name}: {cleanup_error}"
                );
            }
            Err(error)
        }
        Err(panic) => {
            if let Err(cleanup_error) = cleanup {
                eprintln!(
                    "House runtime cleanup failed after a case panic for {name}: {cleanup_error}"
                );
            }
            std::panic::resume_unwind(panic)
        }
    }
}
