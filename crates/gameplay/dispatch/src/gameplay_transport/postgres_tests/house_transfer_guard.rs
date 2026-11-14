//! NBREL3: the real durable guards refuse contention without ending a command stream.
use super::*;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FreshAdmissionAttempt, FreshAdmissionAuthority,
};
use crate::gameplay_transport::world_spatial::StepDirection;
use crate::gameplay_transport::{StepOutcome, WorldSpatialObservation};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct GuardProbe<'a, A> {
    owner: &'a A,
    admitted: AdmittedSession,
    steps: Cell<usize>,
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
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
        self.owner.house_transfer_input(self.admitted).await
    }
    async fn activity_transfer_input(
        &self,
        _: AdmittedSession,
    ) -> Result<(), crate::durability::DurabilityError> {
        self.owner.activity_transfer_input(self.admitted).await
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
    super::kill_reward_e2e::scenario_thread::run("house-guard-deferred", || {
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
                    assert_eq!(refusal, (CommandStatus::Rejected, Vec::new()));
                    assert_eq!(
                        result(&mut client).await?,
                        (CommandStatus::DuplicateReplay, Vec::new())
                    );
                    assert_eq!(probe.steps.get(), 0);
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    let next = command(2)?;
                    client.write_u32(next.len() as u32).await?;
                    client.write_all(&next).await?;
                    assert_eq!(result(&mut client).await?.0, CommandStatus::Accepted);
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
