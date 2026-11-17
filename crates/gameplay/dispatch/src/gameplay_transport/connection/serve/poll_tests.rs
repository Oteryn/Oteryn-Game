//! Exercise periodic arbitration through the real poller and a live, ACKing peer.

use super::*;
use crate::gameplay_transport::actor_spell;
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use crate::gameplay_transport::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_state_delta, encode_client_command,
    world_clock::{STATE_DOMAIN_WORLD_CLOCK, decode_world_clock},
    world_spatial::encode_step_intent,
};
use std::cell::Cell;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Default)]
struct SlowSerene {
    clock_ms: Cell<u64>,
    serene_ticks: Cell<usize>,
    steps: Cell<usize>,
    server_message: Cell<bool>,
}

impl FreshAdmissionAuthority for SlowSerene {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(observation())
    }

    async fn observe_vitals(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        Some((
            1,
            actor_spell::ActorVitals {
                health: 185,
                max_health: 185,
                mana: 90,
                max_mana: 90,
                soul: 100,
                harmony: 2,
                serene: true,
            },
        ))
    }

    async fn tick_vitals(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        self.serene_ticks.set(self.serene_ticks.get() + 1);
        // Paused time models an owner read longer than its existing one-second cadence.
        tokio::time::sleep(SERENE_EVALUATION * 2).await;
        None
    }

    async fn paced_step(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _direction: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        self.steps.set(self.steps.get() + 1);
        // Resetting Serene only after refresh completion would not cover this input delay.
        tokio::time::sleep(SERENE_EVALUATION * 2).await;
        (StepOutcome::rejected(), None)
    }

    fn world_clock_utc_ms(&self) -> Option<u64> {
        Some(self.clock_ms.get())
    }

    async fn server_messages_changed(&self, _admitted: AdmittedSession) {
        if !self.server_message.replace(false) {
            std::future::pending::<()>().await;
        }
    }
}

fn observation() -> WorldSpatialObservation {
    WorldSpatialObservation {
        content_generation: [7; 32],
        actor_position: ActorPosition {
            x: 0,
            y: 0,
            floor: 7,
        },
    }
}

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

fn admitted() -> TestResult<AdmittedSession> {
    let world_id = WorldId::decode(&uuid_v7(0x33))?;
    let channel_id = ChannelId::decode(&uuid_v7(0x44))?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22))?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[6, CAPABILITY_WORLD_LIGHT_V1],
            )
            .ok_or("select clock fixture capabilities")?,
            ..SessionContinuity::FRESH
        },
        item_fence: None,
    })
}

fn step_frame(id: u64) -> TestResult<Vec<u8>> {
    Ok(encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &encode_step_intent(StepDirection::East),
        },
    )?)
}

fn run<F: Future<Output = TestResult> + 'static>(scenario: fn() -> F) -> TestResult {
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()?
                .block_on(scenario())
        })?
        .join();
    match result {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
fn slow_serene_and_slow_input_keep_clock_and_liveness_progressing() -> TestResult {
    run(live_clock)
}

async fn live_clock() -> TestResult {
    let authority = SlowSerene::default();
    let admitted = admitted()?;
    // The snapshot carries the admission's existing clock revision without incrementing it.
    let initial_clock_revision = admitted.continuity.world_clock_revision;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let served = async {
        let end = serve_admitted_actor(&mut server, admitted, &authority, IDLE_LIVENESS).await;
        drop(server);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(end)
    };
    let peer = async {
        let mut frames = FrameReader::default();
        let (mut probes, mut results, mut clock_deltas) = (0, 0, 0_u16);
        let mut command = 1;
        while probes < 6 {
            let bytes = frames.next(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes)?;
            match envelope.message_type() {
                MessageType::SnapshotCommit => {
                    authority.clock_ms.set(150_000);
                    write_frame(&mut client, &step_frame(command)?).await?;
                }
                MessageType::LivenessProbe => {
                    let id = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())?;
                    probes += 1;
                    write_frame(
                        &mut client,
                        &oteryn_protocol_oteryn::encode_liveness_ack(1, id, 0)?,
                    )
                    .await?;
                }
                MessageType::CommandResult => {
                    results += 1;
                    command += 1;
                    write_frame(&mut client, &step_frame(command)?).await?;
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload())?;
                    if delta.domain_id == STATE_DOMAIN_WORLD_CLOCK {
                        assert_eq!(
                            delta.base_revision,
                            initial_clock_revision + u64::from(clock_deltas)
                        );
                        assert_eq!(
                            delta.new_revision,
                            initial_clock_revision + u64::from(clock_deltas) + 1
                        );
                        clock_deltas += 1;
                        let clock = decode_world_clock(delta.payload)
                            .map_err(|error| format!("clock delta: {error:?}"))?;
                        assert_eq!(clock.tibian_minute, clock_deltas * 60);
                        authority.clock_ms.set(authority.clock_ms.get() + 150_000);
                    }
                }
                _ => {}
            }
        }
        drop(client);
        assert!(results >= 3, "slow client commands must keep completing");
        assert!(
            clock_deltas >= 2,
            "ready Serene must not starve recurring Visibility clock deltas"
        );
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let (end, peer) = tokio::time::timeout(IDLE_LIVENESS.interval * 12, async {
        tokio::join!(served, peer)
    })
    .await
    .map_err(|_| "periodic fairness session deadline")?;
    peer?;
    assert!(
        matches!(end?, ConnectionEnd::AdmittedThenDisconnected(_)),
        "the ACKing peer must retain control until it closes"
    );
    assert!(authority.serene_ticks.get() >= 3);
    assert!(authority.steps.get() >= 3);
    Ok(())
}

#[test]
fn ready_periodic_refreshes_rotate_across_input_without_delaying_probe() -> TestResult {
    run(all_ready)
}

async fn all_ready() -> TestResult {
    let authority = SlowSerene::default();
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let mut admitted = admitted()?;
    let actor = admitted.runtime_actor.ok_or("fixture actor")?;
    let mut session = setup::prepare(
        &mut server,
        &mut admitted,
        &authority,
        IDLE_LIVENESS,
        actor,
        observation(),
        integrity::Integrity::default(),
    )
    .await
    .map_err(|_| "prepare connection")?;
    session.cadence = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(100),
        IDLE_LIVENESS.interval,
    );
    for timer in [
        &mut session.serene,
        &mut session.quest_log_refresh,
        &mut session.chat_refresh,
        &mut session.visibility_refresh,
        &mut session.combat_refresh,
        &mut session.npc_refresh,
    ] {
        let mut ready = tokio::time::interval(SERENE_EVALUATION);
        ready.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        *timer = Some(ready);
    }
    // Input stays ready throughout two rotations, including a slow input owner turn.
    for command in 1..=13 {
        write_frame(&mut client, &step_frame(command)?).await?;
    }
    for _ in 0..2 {
        for expected in [
            Next::Serene,
            Next::QuestLog,
            Next::Chat,
            Next::Visibility,
            Next::Combat,
            Next::Npc,
        ] {
            let selected = next(&mut server, &authority, &mut session).await;
            assert_eq!(
                std::mem::discriminant(&selected),
                std::mem::discriminant(&expected),
                "every continuously ready refresh gets one turn per rotation"
            );
            assert!(matches!(
                next(&mut server, &authority, &mut session).await,
                Next::Frame(Ok(_))
            ));
            tokio::time::advance(SERENE_EVALUATION * 2).await;
        }
    }
    // Probe and server messages keep their priority; each still grants the existing input turn.
    session.cadence = tokio::time::interval(IDLE_LIVENESS.interval);
    authority.server_message.set(true);
    assert!(matches!(
        next(&mut server, &authority, &mut session).await,
        Next::Probe
    ));
    assert!(matches!(
        next(&mut server, &authority, &mut session).await,
        Next::Frame(Ok(_))
    ));
    assert!(matches!(
        next(&mut server, &authority, &mut session).await,
        Next::ServerMessage
    ));
    assert!(matches!(
        next(&mut server, &authority, &mut session).await,
        Next::Serene
    ));
    Ok(())
}
