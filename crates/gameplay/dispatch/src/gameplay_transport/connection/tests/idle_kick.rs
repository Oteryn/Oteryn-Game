//! IDLEKICK: controlled server time through the production connection/router.
use super::*;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::tutorial_notices::{TutorialNotice, decode_tutorial_notice};
use std::time::Duration;
use tokio::time::Instant;

struct IdleAuthority {
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
    now: Cell<Instant>,
    training: Cell<bool>,
    no_logout: Cell<bool>,
    fight: Cell<bool>,
    unknown: Cell<bool>,
    logouts: Cell<usize>,
    steps: Cell<usize>,
}

impl IdleAuthority {
    fn new(now: Instant) -> Self {
        Self {
            commands: Default::default(),
            now: Cell::new(now),
            training: Cell::new(false),
            no_logout: Cell::new(false),
            fight: Cell::new(false),
            unknown: Cell::new(false),
            logouts: Cell::new(0),
            steps: Cell::new(0),
        }
    }
}

impl FreshAdmissionAuthority for IdleAuthority {
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    fn gameplay_idle_now(&self) -> Instant {
        self.now.get()
    }
    async fn gameplay_idle_exemptions(&self, _: AdmittedSession) -> Option<(bool, bool)> {
        Some((self.training.get(), self.no_logout.get()))
    }
    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }
    async fn step(&self, _: ExactActorRef, _: StepDirection) -> StepOutcome {
        self.steps.set(self.steps.get() + 1);
        StepOutcome::rejected()
    }
    async fn logout(&self, _: AdmittedSession) -> LogoutReply {
        self.logouts.set(self.logouts.get() + 1);
        if self.unknown.get() {
            LogoutReply::Unknown
        } else if self.fight.get() {
            LogoutReply::Result(LogoutResult::InFight { retry_after_ms: 1 })
        } else {
            LogoutReply::Result(LogoutResult::Accepted)
        }
    }
}

struct Client {
    generation: u64,
    stream: DuplexStream,
    frames: super::super::super::tcp_tls::FrameReader,
    probes: u64,
    warnings: usize,
    sequence: u64,
}

impl Client {
    async fn receive(&mut self) -> Result<MessageType, Box<dyn Error>> {
        let frame = self.frames.next(&mut self.stream).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::LivenessProbe {
            self.probes += 1;
            self.stream
                .write_all(&framed(&ack(self.generation, self.probes)))
                .await?;
        }
        if envelope.message_type() == MessageType::StateDelta {
            let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
            assert_eq!(delta.domain_id, 23);
            let notice = decode_tutorial_notice(delta.payload).map_err(|e| format!("{e:?}"))?;
            assert_eq!(notice, TutorialNotice::StatusText("There was no variation in your behaviour for 15 minutes. You will be disconnected in one minute if there is no change in your actions until then.".into()));
            self.warnings += 1;
            assert!(envelope.server_sequence() > self.sequence);
            self.sequence = envelope.server_sequence();
            assert_eq!(delta.new_revision, self.warnings as u64);
        }
        Ok(envelope.message_type())
    }

    async fn until(&mut self, kind: MessageType) -> Result<(), Box<dyn Error>> {
        while self.receive().await? != kind {}
        Ok(())
    }

    async fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        self.until(MessageType::LivenessProbe).await
    }
}

async fn scenario(
    notices: bool,
    body: impl AsyncFnOnce(&mut Client, &IdleAuthority, Instant) -> Result<(), Box<dyn Error>>,
) -> Result<ConnectionEnd, Box<dyn Error>> {
    scenario_from(SessionContinuity::FRESH, notices, Instant::now(), body).await
}

async fn scenario_from(
    continuity: SessionContinuity,
    notices: bool,
    origin: Instant,
    body: impl AsyncFnOnce(&mut Client, &IdleAuthority, Instant) -> Result<(), Box<dyn Error>>,
) -> Result<ConnectionEnd, Box<dyn Error>> {
    let authority = IdleAuthority::new(origin);
    let base = authority.now.get();
    let mut admitted = positioned()?;
    admitted.continuity = continuity;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        PRODUCTION_OFFERED_CAPABILITIES,
        if notices { &[31] } else { &[] },
    )
    .ok_or("capabilities")?;
    let (mut server, stream) = tokio::io::duplex(1 << 16);
    let serving = async {
        let end = serve_admitted(
            &mut server,
            admitted,
            &authority,
            LivenessPolicy {
                // Gameplay time is controlled separately. Keep transport ticks apart so
                // a loaded shared runner cannot queue multiple probes before the client
                // advances the semantic clock or changes an exemption.
                interval: Duration::from_secs(1),
                missed_limit: 10,
            },
        )
        .await;
        drop(server);
        end
    };
    let client = async {
        let mut client = Client {
            generation: admitted.continuity.connection_generation,
            stream,
            frames: Default::default(),
            probes: 0,
            warnings: 0,
            sequence: 0,
        };
        client.until(MessageType::SnapshotCommit).await?;
        body(&mut client, &authority, base).await
    };
    // Keep the client and real connection driven on one thread, as the production tests do.
    let mut serving = std::pin::pin!(serving);
    let mut client = std::pin::pin!(client);
    let (mut ended, mut done) = (None, None);
    let (end, result) = std::future::poll_fn(|cx| {
        if ended.is_none()
            && let std::task::Poll::Ready(end) = serving.as_mut().poll(cx)
        {
            ended = Some(end);
        }
        if done.is_none()
            && let std::task::Poll::Ready(result) = client.as_mut().poll(cx)
        {
            done = Some(result);
        }
        if ended.is_some() && done.is_some() {
            std::task::Poll::Ready((
                ended.take().expect("connection ended"),
                done.take().expect("client ended"),
            ))
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    result?;
    Ok(end)
}

#[test]
fn idle_kick_warning_once_at_fifteen_ping_does_not_reset_removal_at_sixteen()
-> Result<(), Box<dyn Error>> {
    run(async {
        let end = scenario(true, async |client, owner, base| {
            owner.now.set(base + Duration::from_secs(899));
            client.tick().await?;
            assert_eq!(client.warnings, 0);
            owner.now.set(base + Duration::from_secs(900));
            client.tick().await?;
            assert_eq!(client.warnings, 1);
            owner.now.set(base + Duration::from_secs(959));
            client.tick().await?;
            client.tick().await?;
            assert_eq!((client.warnings, owner.logouts.get()), (1, 0));
            owner.now.set(base + Duration::from_secs(960));
            assert!(client.receive().await.is_err());
            assert_eq!((client.warnings, owner.logouts.get()), (1, 1));
            Ok(())
        })
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        Ok(())
    })
}

#[test]
fn idle_kick_activity_resets_and_rearms_warning_through_dispatch() -> Result<(), Box<dyn Error>> {
    run(async {
        let end = scenario(true, async |client, owner, base| {
            owner.now.set(base + Duration::from_secs(905)); // coarse tick crosses 15
            client.tick().await?;
            assert_eq!(client.warnings, 1);
            owner.now.set(base + Duration::from_secs(930));
            client
                .stream
                .write_all(&framed(&command(1, 1, 1, StepDirection::East)))
                .await?;
            client.until(MessageType::CommandResult).await?;
            assert_eq!(owner.steps.get(), 1);
            owner.now.set(base + Duration::from_secs(960));
            client.tick().await?;
            assert_eq!(owner.logouts.get(), 0);
            owner.now.set(base + Duration::from_secs(1831));
            client.tick().await?;
            assert_eq!(client.warnings, 2);
            owner.now.set(base + Duration::from_secs(1860));
            client
                .stream
                .write_all(&framed(&command(1, 1, 1, StepDirection::East)))
                .await?;
            client.until(MessageType::CommandResult).await?;
            assert_eq!(owner.steps.get(), 1, "retained replay is not new activity");
            owner.now.set(base + Duration::from_secs(1890));
            assert!(client.receive().await.is_err());
            Ok(())
        })
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        Ok(())
    })
}

#[test]
fn idle_kick_training_suspends_then_starts_a_fresh_window() -> Result<(), Box<dyn Error>> {
    run(async {
        let end = scenario(true, async |client, owner, base| {
            owner.now.set(base + Duration::from_secs(899));
            owner.training.set(true);
            client.tick().await?;
            owner.now.set(base + Duration::from_secs(5000));
            client.tick().await?;
            assert_eq!((client.warnings, owner.logouts.get()), (0, 0));
            owner.training.set(false);
            client.tick().await?;
            owner.now.set(base + Duration::from_secs(5899));
            client.tick().await?;
            assert_eq!(client.warnings, 0);
            owner.now.set(base + Duration::from_secs(5900));
            client.tick().await?;
            assert_eq!(client.warnings, 1);
            owner.now.set(base + Duration::from_secs(5960));
            assert!(client.receive().await.is_err());
            Ok(())
        })
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        Ok(())
    })
}

#[test]
fn idle_kick_no_logout_and_fight_refusal_preserve_the_expired_timer() -> Result<(), Box<dyn Error>>
{
    run(async {
        let end = scenario(true, async |client, owner, base| {
            owner.no_logout.set(true);
            owner.now.set(base + Duration::from_secs(1000));
            client.tick().await?;
            client.tick().await?;
            assert_eq!((client.warnings, owner.logouts.get()), (0, 0));
            owner.fight.set(true);
            owner.no_logout.set(false);
            client.tick().await?;
            assert_eq!((client.warnings, owner.logouts.get()), (1, 1));
            owner.fight.set(false);
            assert!(client.receive().await.is_err());
            assert_eq!((client.warnings, owner.logouts.get()), (1, 2));
            Ok(())
        })
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        Ok(())
    })
}

#[test]
fn idle_kick_without_notice_capability_still_logs_out_and_unknown_reconciles()
-> Result<(), Box<dyn Error>> {
    run(async {
        for unknown in [false, true] {
            let end = scenario(false, async |client, owner, base| {
                owner.unknown.set(unknown);
                owner.now.set(base + Duration::from_secs(900));
                client.tick().await?;
                assert_eq!(client.warnings, 0);
                owner.now.set(base + Duration::from_secs(960));
                assert!(client.receive().await.is_err());
                assert_eq!((client.warnings, owner.logouts.get()), (0, 1));
                Ok(())
            })
            .await?;
            assert!(if unknown {
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_))
            } else {
                matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_))
            });
        }
        Ok(())
    })
}

#[test]
fn idle_kick_reconnect_preserves_deadline_and_does_not_repeat_warning() -> Result<(), Box<dyn Error>>
{
    run(async {
        let origin = Cell::new(Instant::now());
        let end = scenario(true, async |client, owner, base| {
            origin.set(base);
            owner.now.set(base + Duration::from_secs(900));
            client.tick().await?;
            assert_eq!(client.warnings, 1);
            client.stream.shutdown().await?;
            Ok(())
        })
        .await?;
        let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
            return Err("expected disconnected session".into());
        };
        let mut continuity = admitted.continuity;
        continuity.connection_generation += 1;
        let end = scenario_from(
            continuity,
            true,
            origin.get(),
            async |client, owner, base| {
                owner.now.set(base + Duration::from_secs(959));
                client.tick().await?;
                assert_eq!((client.warnings, owner.logouts.get()), (0, 0));
                owner.now.set(base + Duration::from_secs(960));
                assert!(client.receive().await.is_err());
                assert_eq!((client.warnings, owner.logouts.get()), (0, 1));
                Ok(())
            },
        )
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        Ok(())
    })
}

#[test]
fn idle_kick_classification_excludes_queries_and_status_polling() {
    use super::super::gameplay_idle::meaningful;
    use oteryn_protocol_oteryn::ClientCommandView;
    for (kind, payload, expected) in [
        (1, &[8, 1][..], true),
        (1, &[][..], false),
        (2, &[][..], false),
        (3, &[][..], false),
        (7, &[][..], false),
        (8, &[][..], false),
        (10, &[][..], false),
        (9, &[][..], false),
        (12, &[][..], false),
        (25, &[][..], false),
        (31, &[][..], false),
        (32, &[][..], true),
        (40, &[][..], false),
        (45, &[][..], false),
        (56, &[][..], false),
        (60, &[][..], false),
        (66, &[][..], false),
        (68, &[][..], false),
        (73, &[][..], false),
        (93, &[][..], false),
        (97, &[][..], false),
        (33, &[][..], false),
        (69, &[][..], false),
        (78, &[][..], false),
        (98, &[][..], false),
        (14, &[][..], false),
        (21, &[][..], false),
        (22, &[][..], false),
        (41, &[][..], false),
        (46, &[][..], false),
        (50, &[][..], false),
        (57, &[][..], false),
        (61, &[][..], false),
        (64, &[][..], false),
        (67, &[8, 2][..], false),
        (67, &[8, 3][..], true),
        (72, &[][..], false),
        (77, &[][..], false),
        (81, &[][..], false),
        (1000, &[][..], false),
    ] {
        assert_eq!(
            meaningful(
                ClientCommandView {
                    command_id: 1,
                    command_type: kind,
                    payload
                },
                &[]
            ),
            expected,
            "command {kind}"
        );
    }
}

#[test]
fn idle_kick_queries_unknown_and_malformed_input_do_not_extend_deadline()
-> Result<(), Box<dyn Error>> {
    run(async {
        let end = scenario(true, async |client, owner, base| {
            owner.now.set(base + Duration::from_secs(900));
            client.tick().await?;
            assert_eq!(client.warnings, 1);
            owner.now.set(base + Duration::from_secs(930));
            for (id, kind) in [
                (1, 1),
                (2, 12),
                (3, 14),
                (4, 33),
                (5, 67),
                (6, 69),
                (7, 78),
                (8, 98),
                (9, 1000),
            ] {
                let frame = oteryn_protocol_oteryn::encode_client_command(
                    1,
                    &oteryn_protocol_oteryn::ClientCommandValue {
                        command_id: id,
                        command_type: kind,
                        payload: &[],
                    },
                )?;
                client.stream.write_all(&framed(&frame)).await?;
                client.until(MessageType::CommandResult).await?;
            }
            assert_eq!(owner.steps.get(), 0);
            owner.now.set(base + Duration::from_secs(960));
            assert!(client.receive().await.is_err());
            assert_eq!((client.warnings, owner.logouts.get()), (1, 1));
            Ok(())
        })
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        Ok(())
    })
}
