use std::{cell::Cell, future::pending, sync::Arc, time::Duration};

use oteryn_protocol_oteryn::logout::{
    CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
    encode_logout_intent,
};

use super::autowalk_fixtures::{command_reply, send};
use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::fixture::{Harness, debug, fence};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandStatus, GameSessionState};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::composed_admission::{
    CONDITION_RELEASE_PROBE, HolderCheckoutProbe, OFFLINE_TRAINING_RELEASE_PROBE,
};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::{COMMAND_HOLDER_WINDOW, RECONCILE_BACKOFF, item_fence_of};

#[test]
fn first_logout_waits_for_checked_out_pass_and_commits_final_position_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_scenario(admin, false)
}

#[test]
fn first_logout_waits_for_offline_training_after_an_earlier_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_scenario(admin, true)
}

fn run_scenario(admin: String, offline_training: bool) -> TestResult {
    scenario_thread::run("logout-holder-retry", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin, offline_training))
    })
}

async fn scenario(admin: String, offline_training: bool) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "logout_holder_retry", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (deps, activation) = if offline_training {
        let world = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_parts().0,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        (deps, Some(controller))
    } else {
        (Dependencies::new()?, None)
    };
    let mut owner = deps.owner(&h, &authority);
    if let Some(activation) = &activation {
        owner.active_generation = Some(activation.active().ok_or("active training content")?);
        let mut rates = oteryn_gameplay_progression::loop_training::TrainingRates::from_json(
            include_bytes!("../../../../../../rulesets/training/crystal-201189f-v1.json"),
        )
        .map_err(debug)?;
        rates.enabled = true;
        Arc::get_mut(&mut owner.states)
            .ok_or("isolated training fixture state")?
            .training
            .config = Some(Arc::new(rates));
    }
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
    if offline_training {
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(admitted.game_session_id)
            .await?;
        admitted.item_fence = Some(item_fence_of(current).ok_or("training item fence")?);
        owner.admit_loop_training(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
    }
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_LOGOUT_V1])
            .ok_or("logout capability")?;

    let conditions = Arc::new(HolderCheckoutProbe::default());
    let training = offline_training.then(|| Arc::new(HolderCheckoutProbe::default()));
    let first_deadline = Cell::new(None);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = CONDITION_RELEASE_PROBE.scope(
        Cell::new(Some(Arc::clone(&conditions))),
        OFFLINE_TRAINING_RELEASE_PROBE.scope(Cell::new(training.clone()), async {
            Ok::<_, Box<dyn std::error::Error>>(
                serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await,
            )
        }),
    );
    let compete = async {
        conditions
            .contend_until(&h.root, async {
                first_deadline.set(h.root.checked_out_pass().map(|(_, deadline)| deadline));
                // Cross the ordinary command bound while the first pass is still checked out.
                let cutoff = COMMAND_HOLDER_WINDOW + Duration::from_millis(50);
                assert!(tokio::time::timeout(cutoff, pending::<()>()).await.is_err());
            })
            .await?;
        if let Some(training) = &training {
            let old_cutoff = first_deadline.get().ok_or("first checked-out pass")?
                + RECONCILE_BACKOFF
                + Duration::from_millis(50);
            training
                .contend_until(&h.root, async {
                    // r3's first extension ended here. The training fence has now
                    // refused its own checkout, with a different pass holding it.
                    assert!(tokio::time::Instant::now() < old_cutoff);
                    assert!(
                        tokio::time::timeout_at(old_cutoff, pending::<()>())
                            .await
                            .is_err()
                    );
                })
                .await?;
        }
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
        if offline_training {
            let markers: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_training_markers
                 WHERE game_session_id = encode($1,'hex')::uuid AND cause = 'logout'",
            )
            .bind(admitted.game_session_id.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                markers, 1,
                "the first ack follows the offline-training write"
            );
        }
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
    if offline_training {
        eprintln!(
            "POSTGRES_EXECUTED first_logout_waits_for_offline_training_after_an_earlier_holder_postgres"
        );
    } else {
        eprintln!(
            "POSTGRES_EXECUTED first_logout_waits_for_checked_out_pass_and_commits_final_position_postgres"
        );
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
