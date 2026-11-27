use std::{cell::Cell, future::pending, sync::Arc, time::Duration};

use oteryn_protocol_oteryn::logout::{
    CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
    encode_logout_intent,
};

use super::autowalk_fixtures::{command_reply, send};
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandStatus, GameSessionState};
use crate::gameplay_transport::COMMAND_HOLDER_WINDOW;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::composed_admission::{CONDITION_RELEASE_PROBE, HolderCheckoutProbe};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};

#[test]
fn first_logout_waits_for_checked_out_pass_and_commits_final_position_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("logout-holder-retry", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin))
    })
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "logout_holder_retry", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(&h, &authority);
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    track(&owner, &deps)?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_LOGOUT_V1])
            .ok_or("logout capability")?;

    let probe = Arc::new(HolderCheckoutProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = CONDITION_RELEASE_PROBE.scope(Cell::new(Some(Arc::clone(&probe))), async {
        Ok::<_, Box<dyn std::error::Error>>(
            serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await,
        )
    });
    let compete = async {
        probe
            .contend_until(&h.root, async {
                // Cross the old checkout bound only after the real release read
                // refused, while remaining inside the competing pass's deadline.
                let old_cutoff = COMMAND_HOLDER_WINDOW + Duration::from_millis(50);
                assert!(
                    tokio::time::timeout(old_cutoff, pending::<()>())
                        .await
                        .is_err()
                );
            })
            .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let request = async {
        let command_id = admitted.continuity.next_command_id;
        send(
            &mut client,
            command_id,
            COMMAND_TYPE_LOGOUT_INTENT,
            &encode_logout_intent(),
        )
        .await?;
        let (status, payload) = command_reply(&mut client, command_id).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            decode_logout_result(&payload).map_err(debug)?,
            LogoutResult::Accepted
        );
        let terminal: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_durability_reconnect_sessions
             WHERE session_state=3 AND control_loss_epoch IS NULL",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(terminal, 1, "the single reply follows the terminal commit");
        // The one client reply already implies a committed release and save.
        let position_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(position_count, 1);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (end, (), ()) = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::try_join!(serve, request, compete)
    })
    .await??;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)),
        "{end:?}"
    );
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(admitted.game_session_id)
        .await?;
    assert_eq!(current.session_state(), GameSessionState::Terminal);
    eprintln!(
        "POSTGRES_EXECUTED first_logout_waits_for_checked_out_pass_and_commits_final_position_postgres"
    );
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
