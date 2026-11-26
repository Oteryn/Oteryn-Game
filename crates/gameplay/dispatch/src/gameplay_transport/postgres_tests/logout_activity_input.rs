//! Hold the real durability slot after town entry and through the first input guard.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::IssuedSemanticPass;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionState};
use crate::gameplay_transport::WorldSpatialObservation;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, IDLE_LIVENESS, LogoutReply, serve_admitted,
};
use crate::gameplay_transport::session_ingress::SessionCommands;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;
use oteryn_protocol_oteryn::logout::{
    CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use std::cell::RefCell;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

struct LogoutProbe<'a, A> {
    owner: &'a A,
    held: RefCell<Option<IssuedSemanticPass>>,
    commands: SessionCommands,
}

impl<A: FreshAdmissionAuthority> FreshAdmissionAuthority for LogoutProbe<'_, A> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    fn session_commands(&self) -> Option<&SessionCommands> {
        Some(&self.commands)
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.owner.observe(actor).await
    }

    async fn activity_scope_snapshot(
        &self,
        admitted: &mut AdmittedSession,
    ) -> Result<Option<Vec<u8>>, ()> {
        self.owner.activity_scope_snapshot(admitted).await
    }

    async fn activity_transfer_input(
        &self,
        admitted: AdmittedSession,
    ) -> Result<(), crate::durability::DurabilityError> {
        self.owner.activity_transfer_input(admitted).await
    }

    async fn logout_command(&self, admitted: AdmittedSession, command_id: u64) -> LogoutReply {
        // Release only when the production router reaches logout. A guard that
        // insists on taking the holder exhausts its budget before this point.
        let held = self.held.borrow_mut().take();
        let Some(held) = held else {
            return LogoutReply::Unknown;
        };
        if held.run(|_, _| Box::pin(async { Ok(()) })).await.is_err() {
            return LogoutReply::Unknown;
        }
        self.owner.logout_command(admitted, command_id).await
    }
}

#[test]
fn logout_first_attempt_ignores_idle_activity_transfer_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "logout_idle_activity_guard", true, false)
                .await?;
        let result = case(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

async fn case(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    track(&owner, &deps)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(
        store.current_session(admitted.game_session_id).await?,
    );
    assert!(admitted.item_fence.is_some());
    let selected = [CAPABILITY_LOGOUT_V1, CAPABILITY_BOSS_ENCOUNTER_V1];
    let offered = selected.map(|id| OfferedCapability { id, requires: &[] });
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &selected).ok_or("logout and activity selection")?;

    let probe = LogoutProbe {
        owner: &owner,
        held: RefCell::new(None),
        commands: SessionCommands::default(),
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        *probe.held.borrow_mut() = Some(h.root.try_issue_semantic_pass()?);
        let command = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: admitted.continuity.next_command_id,
                command_type: COMMAND_TYPE_LOGOUT_INTENT,
                payload: &[],
            },
        )?;
        write_frame(&mut client, &command).await?;
        let first = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                break (
                    result.command_id,
                    result.status,
                    decode_logout_result(result.payload),
                );
            }
        };
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(first)
    };
    let (ended, first) = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::join!(
            serve_admitted(&mut server, admitted, &probe, IDLE_LIVENESS),
            driving
        )
    })
    .await?;
    assert_eq!(
        first?,
        (
            admitted.continuity.next_command_id,
            CommandStatus::Accepted,
            Ok(LogoutResult::Accepted)
        ),
    );
    assert!(
        matches!(ended, ConnectionEnd::AdmittedThenLoggedOut(_)),
        "{ended:?}"
    );
    assert_eq!(
        store
            .current_session(admitted.game_session_id)
            .await?
            .session_state(),
        GameSessionState::Terminal,
    );
    eprintln!("POSTGRES_CASE_EXECUTED logout_first_attempt_ignores_idle_activity_transfer_holder");
    Ok(())
}
