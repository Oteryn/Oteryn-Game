use std::time::Duration;

use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
use tokio::time::{Instant, timeout};

use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::GameplayScalarReadError;
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};

#[test]
fn grizzly_admission_retries_a_stalled_read_on_the_busy_publisher() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Grizzly probe admission thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "grizzly_probe_retry", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let admitted = deps.admitted;
        let session = admitted.game_session_id;
        let mut blocker = h.pool.begin().await?;
        sqlx::query("LOCK TABLE game_character_grizzly_boss_entries IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocker)
            .await?;

        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async {
            let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
            assert!(
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                "unexpected Grizzly admission end: {end:?}"
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let release_after_retry = async {
            let first = waiting_probe(&h.pool, 0).await?;
            // The first SELECT still holds the sole publisher connection.
            let competing = h.root.grizzly_boss_entry_exists_for_session(
                session,
                Instant::now() + Duration::from_millis(25),
            ).await;
            assert!(
                matches!(competing, Err(GameplayScalarReadError::PoolDeadlineExceeded { .. })),
                "expected acquisition timeout during the blocked query: {competing:?}"
            );
            waiting_probe(&h.pool, first).await?;
            blocker.rollback().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let observe_join = async move {
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                match envelope.message_type() {
                    MessageType::SnapshotCommit => return Ok::<(), Box<dyn std::error::Error>>(()),
                    MessageType::ProtocolError => {
                        return Err("Grizzly callback refused the join after contention".into());
                    }
                    _ => {}
                }
            }
        };
        timeout(Duration::from_secs(15), async {
            tokio::try_join!(serving, release_after_retry, observe_join)
        })
        .await??;
        eprintln!("POSTGRES_EXECUTED grizzly_admission_retries_a_stalled_read_on_the_busy_publisher");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn waiting_probe(pool: &sqlx::PgPool, exclude_pid: i32) -> TestResult<i32> {
    timeout(Duration::from_secs(4), async {
        loop {
            if let Some(pid) = sqlx::query_scalar(
                "SELECT pid FROM pg_locks
                  WHERE relation='game_character_grizzly_boss_entries'::regclass
                    AND database=(SELECT oid FROM pg_database WHERE datname=current_database())
                    AND NOT granted AND mode='AccessShareLock' AND pid <> $1
                  ORDER BY waitstart LIMIT 1",
            )
            .bind(exclude_pid)
            .fetch_optional(pool)
            .await?
            {
                return Ok(pid);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| format!("Grizzly admission did not queue a probe after backend {exclude_pid}"))?
}
