use super::super::fixture::{Harness, debug};
use super::super::house_transfer_fixtures::RecoverySource;
use super::TestResult;
use super::support::{self, Case, count, exchange};
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AFTER_COMMAND_INPUT_GUARDS, AdmittedSession, ControlLossResult, FreshAdmissionAuthority,
    IDLE_LIVENESS, ResumeAttempt, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::daily_boost as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};
use tokio::sync::Notify;

#[test]
fn client_e2e_daily_boost() -> TestResult {
    support::scenario(Case::DailyBoost)
}

#[test]
fn client_e2e_daily_boost_first_query_after_relog_with_busy_holder() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    support::scenario_with_admin(Case::DailyBoostAfterRelog, admin)
}

pub(super) async fn exercise_relog(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    assert!(admitted.continuity.selected_capabilities.contains(
        oteryn_protocol_oteryn::house_transfer::CAPABILITY_HOUSE_TRANSFER_V1
    ));
    owner.states.command_ingress.admit(admitted.game_session_id);
    assert_eq!(
        owner.lose_control(admitted, Duration::ZERO, None).await,
        ControlLossResult::Recorded
    );
    let token = RecoverySource::token(2)?;
    let resumed = owner
        .resume(ResumeAttempt {
            game_session_id: admitted.game_session_id,
            recovery_material: token.as_bytes(),
            transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
            last_applied_server_sequence: admitted.continuity.server_sequence,
            supported_capabilities: admitted.continuity.selected_capabilities.as_slice(),
        })
        .await
        .map_err(debug)?;
    assert_eq!(resumed.game_session_id, admitted.game_session_id);
    assert!(
        resumed.continuity.connection_generation > admitted.continuity.connection_generation
    );

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let barrier = std::sync::Arc::new((Notify::new(), Notify::new()));
    let serving = async {
        AFTER_COMMAND_INPUT_GUARDS
            .scope(
                barrier.clone(),
                serve_admitted(&mut server, resumed, owner, IDLE_LIVENESS),
            )
            .await;
        drop(server);
    };
    let driving = async {
        read_message(&mut client, MessageType::SnapshotCommit).await?;
        // Exhaust any holder retry in the input guards. Return the holder for
        // the separate resume reconciliation and Daily Boost epoch read.
        let held = owner.root.try_issue_semantic_pass()?;
        let id = resumed.continuity.next_command_id;
        let frame = encode_client_command(
            resumed.continuity.connection_generation,
            &ClientCommandValue {
                command_id: id,
                command_type: wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
                payload: &[],
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &frame).await?;
        barrier.0.notified().await;
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        barrier.1.notify_one();
        let reply = read_message(&mut client, MessageType::CommandResult).await?;
        let result = decode_command_result(decode_wire_envelope(&reply).map_err(debug)?.payload())
            .map_err(debug)?;
        assert_eq!(result.command_id, id);
        assert_eq!(result.status, CommandStatus::Accepted);
        let selection = owner
            .runtime
            .lock()
            .await
            .daily_boost()
            .cloned()
            .ok_or("committed daily boost")?;
        assert_eq!(
            result.payload,
            wire::encode_view(&wire::DailyBoostView {
                reset_epoch: selection.reset_epoch,
                creature_key: selection.creature,
                boss_key: selection.boss,
            })
            .map_err(debug)?
        );
        client.shutdown().await?;
        eprintln!("POSTGRES_CASE_COMPLETED client_e2e_daily_boost_first_query_after_relog_with_busy_holder");
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        tokio::try_join!(
            async {
                serving.await;
                Ok(())
            },
            driving
        )
    })
    .await??;
    Ok(())
}

async fn read_message(client: &mut DuplexStream, kind: MessageType) -> TestResult<Vec<u8>> {
    loop {
        let frame = read_frame(client).await?;
        if decode_wire_envelope(&frame).map_err(debug)?.message_type() == kind {
            return Ok(frame);
        }
    }
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // PlayLink::daily_boost_request -> daily_boost::request sends command 177 with
    // empty payload; daily_boost::apply_result retains these exact result bytes.
    let first = exchange(
        owner,
        admitted,
        1,
        wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
        &[],
    )
    .await?;
    let choice = owner
        .runtime
        .lock()
        .await
        .daily_boost()
        .cloned()
        .ok_or("daily choice")?;
    assert_eq!(
        first,
        wire::encode_view(&wire::DailyBoostView {
            reset_epoch: 0,
            creature_key: choice.creature.clone(),
            boss_key: choice.boss.clone()
        })
        .map_err(debug)?
    );
    let stored: (String, String) =
        sqlx::query_as("SELECT creature_key,boss_key FROM game_world_daily_boosts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stored, (choice.creature, choice.boss));
    assert_eq!(
        exchange(
            owner,
            admitted,
            2,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[]
        )
        .await?,
        first
    );
    assert_eq!(
        count(h, "SELECT count(*) FROM game_world_daily_boosts").await?,
        1
    );
    Ok(())
}
