//! NBREL3: the real durable guards refuse contention without ending a command stream.
use super::*;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FreshAdmissionAttempt, FreshAdmissionAuthority, LogoutReply,
};
use crate::gameplay_transport::world_spatial::StepDirection;
use crate::gameplay_transport::{StepOutcome, WorldSpatialObservation};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    world_spatial::{StepDisposition, decode_step_result},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct GuardProbe<'a, A> {
    owner: &'a A,
    admitted: AdmittedSession,
    steps: Cell<usize>,
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
    deferred: Option<&'a tokio::sync::Notify>,
}

impl<A> GuardProbe<'_, A> {
    fn report_guard(
        &self,
        result: Result<(), crate::durability::DurabilityError>,
    ) -> Result<(), crate::durability::DurabilityError> {
        if matches!(result, Err(crate::durability::DurabilityError::RootUnavailable))
            && let Some(deferred) = self.deferred
        {
            deferred.notify_one();
        }
        result
    }
}

impl<A: FreshAdmissionAuthority> FreshAdmissionAuthority for GuardProbe<'_, A> {
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }
    async fn house_transfer_input(
        &self,
        _: AdmittedSession,
    ) -> Result<(), crate::durability::DurabilityError> {
        self.report_guard(self.owner.house_transfer_input(self.admitted).await)
    }
    async fn activity_transfer_input(
        &self,
        _: AdmittedSession,
    ) -> Result<(), crate::durability::DurabilityError> {
        self.report_guard(self.owner.activity_transfer_input(self.admitted).await)
    }
    async fn logout_command(&self, _: AdmittedSession, command: u64) -> LogoutReply {
        self.owner.logout_command(self.admitted, command).await
    }
    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [1; 32],
            actor_position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        })
    }
    async fn step(&self, _: ExactActorRef, _: StepDirection) -> StepOutcome {
        self.steps.set(self.steps.get() + 1);
        StepOutcome {
            disposition: crate::gameplay_transport::world_spatial::StepDisposition::Blocked,
            moved_to: None,
        }
    }
}

fn command(id: u64) -> TestResult<Vec<u8>> {
    Ok(oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: id,
            command_type: 1,
            payload: &oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                StepDirection::East,
            ),
        },
    )?)
}

async fn result(client: &mut tokio::io::DuplexStream) -> TestResult<(CommandStatus, Vec<u8>)> {
    loop {
        let length = client.read_u32().await?;
        let mut frame = vec![0; length as usize];
        client.read_exact(&mut frame).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            return Ok((result.status, result.payload.to_vec()));
        }
    }
}

#[test]
fn house_transfer_guard_busy_budget_preserves_connection_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("house-guard-deferred", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "house_guard_deferred", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(admitted.game_session_id)
                    .await?,
            );
            for capability in [
                86,
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            ] {
                admitted.continuity.selected_capabilities =
                    crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                        &[crate::gameplay_transport::capabilities::OfferedCapability {
                            id: capability,
                            requires: &[],
                        }],
                        &[capability],
                    )
                    .ok_or("guard capability")?;
                assert!(
                    admitted
                        .continuity
                        .selected_capabilities
                        .contains(capability)
                );
                let probe = GuardProbe {
                    owner: &owner,
                    admitted,
                    steps: Cell::new(0),
                    commands: Default::default(),
                    deferred: None,
                };
                let mut transport = admitted;
                // Only durable guard calls are delegated. Snapshot/step fixtures keep
                // this test at the guard seam, through the production command dispatcher.
                transport.continuity = SessionContinuity::FRESH;
                transport.first_entry = FirstEntryOutcome::Positioned;
                let (mut server, mut client) = tokio::io::duplex(1 << 20);
                let held = h.root.try_issue_semantic_pass()?;
                let serving = crate::gameplay_transport::connection::serve_admitted(
                    &mut server,
                    transport,
                    &probe,
                    crate::gameplay_transport::connection::IDLE_LIVENESS,
                );
                let client_work = async {
                    let first = command(1)?;
                    for _ in 0..2 {
                        client.write_u32(first.len() as u32).await?;
                        client.write_all(&first).await?;
                    }
                    let refusal = result(&mut client).await?;
                    assert_eq!(refusal.0, CommandStatus::Rejected);
                    assert_eq!(decode_step_result(&refusal.1), Ok(StepDisposition::Rejected));
                    assert_eq!(refusal.1, [0x08, 0x03]);
                    assert_eq!(
                        result(&mut client).await?,
                        (CommandStatus::DuplicateReplay, refusal.1)
                    );
                    assert_eq!(probe.steps.get(), 0);
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    let next = command(2)?;
                    client.write_u32(next.len() as u32).await?;
                    client.write_all(&next).await?;
                    let next_result = result(&mut client).await?;
                    assert_eq!(next_result.0, CommandStatus::Accepted);
                    assert_eq!(decode_step_result(&next_result.1), Ok(StepDisposition::Blocked));
                    assert_eq!(probe.steps.get(), 1);
                    client.shutdown().await?;
                    Ok::<_, Box<dyn std::error::Error>>(())
                };
                let (_, client_result) = tokio::time::timeout(Duration::from_secs(15), async {
                    tokio::join!(serving, client_work)
                })
                .await?;
                client_result?;
                eprintln!(
                    "NBREL3_PG_EXECUTED guard_capability={capability} busy_budget_exhausted replay_retained next_command_executed"
                );
            }
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn logout_retries_busy_house_input_guard_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    logout_after_guard_busy(oteryn_protocol_oteryn::house_transfer::CAPABILITY_HOUSE_TRANSFER_V1)
}

#[test]
fn logout_retries_busy_activity_input_guard_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    logout_after_guard_busy(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
}

fn logout_after_guard_busy(capability: u32) -> TestResult {
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use oteryn_protocol_oteryn::logout::{CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result};

    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "logout_guard_busy", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        track(&owner, &deps)?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(store.current_session(admitted.game_session_id).await?);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability { id: capability, requires: &[] }], &[capability],
        ).ok_or("guard capability")?;
        let deferred = tokio::sync::Notify::new();
        let probe = GuardProbe { owner: &owner, admitted, steps: Cell::new(0), commands: Default::default(), deferred: Some(&deferred) };
        let mut transport = admitted;
        transport.continuity = SessionContinuity::FRESH;
        transport.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability { id: CAPABILITY_LOGOUT_V1, requires: &[] }], &[CAPABILITY_LOGOUT_V1],
        ).ok_or("logout capability")?;
        transport.first_entry = FirstEntryOutcome::Positioned;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let held = h.root.try_issue_semantic_pass()?;
        let serving = crate::gameplay_transport::connection::serve_admitted(
            &mut server, transport, &probe, crate::gameplay_transport::connection::IDLE_LIVENESS,
        );
        let client_work = async {
            let logout = oteryn_protocol_oteryn::encode_client_command(
                1, &oteryn_protocol_oteryn::ClientCommandValue { command_id: 1, command_type: COMMAND_TYPE_LOGOUT_INTENT, payload: &[] },
            )?;
            client.write_u32(logout.len() as u32).await?;
            client.write_all(&logout).await?;
            deferred.notified().await;
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            let (status, payload) = result(&mut client).await?;
            assert_eq!(status, CommandStatus::Accepted, "a busy ingress guard must not reject logout");
            assert_eq!(decode_logout_result(&payload).map_err(debug)?, LogoutResult::Accepted);
            TestResult::Ok(())
        };
        let (end, client_result) = tokio::time::timeout(Duration::from_secs(15), async { tokio::join!(serving, client_work) }).await?;
        client_result?;
        assert!(matches!(end, crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenLoggedOut(_)));
        assert_eq!(store.current_session(admitted.game_session_id).await?.session_state(), crate::foundation::GameSessionState::Terminal);
        drop(owner);
        drop(seal);
        h.cleanup().await
    })
}
