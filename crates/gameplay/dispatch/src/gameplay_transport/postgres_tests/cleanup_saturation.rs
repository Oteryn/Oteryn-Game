//! F25: terminal cleanup bypasses the full ordinary session command ingress.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{FoundationProtocolError, GameSessionState};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, GraceExpiryResult};
use crate::gameplay_transport::session_ingress::{CommandAction, SessionCommandStream};

#[test]
fn f25_full_command_ingress_does_not_block_terminal_cleanup() -> TestResult {
    // Match neighboring composed-owner fixtures: debug futures need a larger stack.
    std::thread::Builder::new()
        .name("f25-cleanup-saturation".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "F25 cleanup scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "f25_cleanup", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        track(&owner, &dependencies)?;
        let session = dependencies.admitted.game_session_id;
        owner.states.command_ingress.admit(session);
        let mut stream = SessionCommandStream::open(
            Some(&owner.states.command_ingress),
            &dependencies.admitted,
        )
        .ok_or("command stream")?;
        let capacity = crate::foundation::MAX_OUTSTANDING_COMMANDS as u64;
        for command in 1..=capacity {
            assert!(matches!(
                stream.begin(command, 1, 0, false),
                Ok(CommandAction::Execute)
            ));
        }
        assert_eq!(
            stream.begin(capacity + 1, 1, 0, false).err(),
            Some((FoundationProtocolError::TooManyOutstandingCommands, capacity + 1))
        );
        assert!(matches!(
            stream.begin(capacity, 1, 0, false),
            Ok(CommandAction::Pending)
        ));
        // Seed an expired loss with the existing fixture, so cleanup is eligible
        // without waiting out the production reconnect grace. The actual release
        // uses the listener's composed authority and durable store.
        dependencies.loss(&harness, 0).await?;
        assert_eq!(
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                owner.expire_control_loss(dependencies.admitted),
            )
            .await?,
            GraceExpiryResult::Released
        );
        assert_eq!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .current_session(session)
                .await
                .map_err(debug)?
                .session_state(),
            GameSessionState::Terminal
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .player_control_facts(dependencies.actor, session)
                .is_err()
        );
        let mut resumed = dependencies.admitted;
        resumed.continuity.next_command_id = stream.next_command_id();
        drop(stream);
        // A resumed stream cannot reconstruct the command history retirement forgot.
        assert!(
            SessionCommandStream::open(Some(&owner.states.command_ingress), &resumed)
                .is_none()
        );
        // Probe the directory directly: no pending command state remains after retirement.
        // Only durable admission can authorize a session; this opens no actor/controller.
        assert!(
            SessionCommandStream::open(Some(&owner.states.command_ingress), &dependencies.admitted)
                .is_some()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
