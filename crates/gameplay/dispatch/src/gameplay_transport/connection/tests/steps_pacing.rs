#[test]
fn admitted_steps_are_sequenced_and_a_replayed_id_expires() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        let (end, frames) = drive_admitted(
            &authority,
            &[
                command(1, 1, step, StepDirection::East),
                command(1, 2, step, StepDirection::North),
                command(1, 3, 0x7fff, StepDirection::East),
                command(1, 2, step, StepDirection::East),
            ],
        )
        .await?;
        let mut expected = baseline();
        let moved = encode_step_result(StepDisposition::Moved);
        let blocked = encode_step_result(StepDisposition::Blocked);
        expected.extend([
            encode_command_result(1, 1, 1, CommandStatus::Accepted, &moved)?,
            encode_state_delta(
                1,
                2,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                1,
                2,
                DELTA_TYPE_WORLD_SPATIAL_V1,
                &encode_world_spatial(&at(1)),
            )?,
            encode_command_result(1, 3, 2, CommandStatus::Accepted, &blocked)?,
            encode_command_result(1, 4, 3, CommandStatus::Rejected, &[])?,
            encode_command_protocol_error(FoundationProtocolError::CommandOutcomeExpired, 1, 2, 0)?,
        ]);
        assert_eq!(frames, expected);
        let ConnectionEnd::AdmittedThenClosed(
            ended,
            FoundationProtocolError::CommandOutcomeExpired,
        ) = end
        else {
            return Err(format!("unexpected end {end:?}").into());
        };
        // The ended session carries the FND-02 continuity a same-session recovery
        // resumes from: next CommandId 4, server_sequence 4, spatial revision 2.
        assert_eq!(
            ended.continuity,
            SessionContinuity {
                connection_generation: 1,
                next_command_id: 4,
                server_sequence: 4,
                spatial_revision: 2,
                player_trade_revision: 0, // This peer did not select PLAYER_TRADE_V1.
                overlay_revision: 0,
                vitals_revision: 0,
                achievement_notice_revision: None,
                selected_capabilities: SelectedCapabilities::NONE,
                item_view: ItemViewContinuity::default(),
                quest_log: QuestLogContinuity::FRESH,
                look: LookWindow::EMPTY,
                combat: CombatContinuity::FRESH,
                world_map_revision: 0,
                chat_revision: 0,
                npc: NpcContinuity::default(),
                world_clock_revision: 0,
                world_weather_revision: 0,
                tutorial_revision: 0,
                server_save_revision: 0,
                server_save_sent: None,
            }
        );
        // The unregistered type and the replayed ID never reached Movement.
        assert_eq!(
            *authority.steps.borrow(),
            [StepDirection::East, StepDirection::North]
        );
        Ok(())
    })
}

/// SPEED-1: an East step moves and paces the next step by `PACE`; every other direction is
/// blocked and leaves the clock unchanged. Each executed step is recorded with its instant;
/// it then takes `turn` before it commits, as a contended owner turn would.
struct PacedAuthority {
    steps: RefCell<Vec<(StepDirection, tokio::time::Instant)>>,
    turn: std::time::Duration,
    /// STEP-PACE-WIRE-1: the rollout switch this fixture reports.
    receipts: bool,
}

/// On the 50 ms beat, so a capability 13 session gets it as its `step_wait_ms` receipt.
const PACE: std::time::Duration = std::time::Duration::from_millis(100);

impl FreshAdmissionAuthority for PacedAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    fn step_wait_receipts(&self) -> bool {
        self.receipts
    }

    async fn paced_step(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        self.steps
            .borrow_mut()
            .push((direction, tokio::time::Instant::now()));
        tokio::time::sleep(self.turn).await;
        if direction == StepDirection::East {
            let moved = StepOutcome {
                disposition: StepDisposition::Moved,
                moved_to: Some(at(1)),
            };
            (moved, Some(PACE))
        } else {
            (
                StepOutcome {
                    disposition: StepDisposition::Blocked,
                    moved_to: None,
                },
                None,
            )
        }
    }
}

/// A positioned session that selected `capabilities` from the production offered set.
fn paced_session(capabilities: &[u32]) -> Result<AdmittedSession, Box<dyn Error>> {
    let mut admitted = positioned()?;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, capabilities)
            .ok_or("bounded")?;
    Ok(admitted)
}

/// Serve `admitted`, write `client_frames` at once, and close the client side only after
/// the server has written `wanted` frames past the snapshot: a buffered step needs the
/// connection to stay open until it is due.
async fn drive_open<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    client_frames: &[Vec<u8>],
    wanted: usize,
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, client) = tokio::io::duplex(1 << 21);
    let (mut client_read, mut client_write) = tokio::io::split(client);
    let snapshot = baseline().len();
    let served = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS);
    let client = async {
        for frame in client_frames {
            client_write.write_all(&framed(frame)).await?;
        }
        let mut reader = super::super::tcp_tls::FrameReader::default();
        let mut frames = Vec::new();
        while frames.len() < snapshot + wanted {
            frames.push(reader.next(&mut client_read).await?);
        }
        client_write.shutdown().await?;
        Ok::<_, Box<dyn Error>>(frames)
    };
    let (mut served, mut client) = (std::pin::pin!(served), std::pin::pin!(client));
    let (mut end, mut frames) = (None, None);
    std::future::poll_fn(|context| {
        if end.is_none()
            && let std::task::Poll::Ready(value) = served.as_mut().poll(context)
        {
            end = Some(value);
        }
        if frames.is_none()
            && let std::task::Poll::Ready(value) = client.as_mut().poll(context)
        {
            frames = Some(value);
        }
        if end.is_some() && frames.is_some() {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    let (Some(end), Some(frames)) = (end, frames) else {
        return Err("join incomplete".into());
    };
    Ok((end, frames?))
}

#[test]
fn a_buffered_step_runs_when_due_and_a_second_early_step_is_refused() -> Result<(), Box<dyn Error>>
{
    run(async {
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        let delta = |sequence: u64, from: u64| {
            encode_state_delta(
                1,
                sequence,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                from,
                from + 1,
                DELTA_TYPE_WORLD_SPATIAL_V1,
                &encode_world_spatial(&at(1)),
            )
        };
        // With capability 13 the refusal is TOO_EARLY; without it, REJECTED. Either way it
        // is a REJECTED command and nothing moves. STEP-PACE-WIRE-1: only with the rollout
        // switch on AND capability 13 does a MOVED carry the 100 ms step duration (field 2 =
        // 0x64) and the TOO_EARLY the time left until the clock is due (at most PACE,
        // rounded up, well above zero); every other session gets today's exact bytes.
        let receipt = |wait: u8| vec![0x08, 0x04, 0x10, wait];
        let left: Vec<Vec<u8>> = (50..=100).map(receipt).collect();
        for (capabilities, receipts, moved, refusals) in [
            (&[13][..], true, vec![0x08, 0x01, 0x10, 0x64], left),
            (&[13][..], false, vec![0x08, 0x01], vec![vec![0x08, 0x04]]),
            (&[][..], true, vec![0x08, 0x01], vec![vec![0x08, 0x03]]),
            (&[][..], false, vec![0x08, 0x01], vec![vec![0x08, 0x03]]),
        ] {
            let authority = PacedAuthority {
                steps: RefCell::new(Vec::new()),
                turn: std::time::Duration::ZERO,
                receipts,
            };
            let (end, frames) = drive_open(
                &authority,
                paced_session(capabilities)?,
                &[
                    command(1, 1, step, StepDirection::East),
                    command(1, 2, step, StepDirection::East),
                    command(1, 3, step, StepDirection::East),
                ],
                5,
            )
            .await?;
            let mut expected = baseline();
            expected.extend([
                encode_command_result(1, 1, 1, CommandStatus::Accepted, &moved)?,
                delta(2, 1)?,
                encode_command_result(1, 3, 2, CommandStatus::Accepted, &moved)?,
                delta(4, 2)?,
            ]);
            let (refusal, frames) = frames.split_last().ok_or("frames")?;
            assert_eq!(frames, expected, "{capabilities:?} {receipts}");
            let mut matched = false;
            for payload in refusals {
                matched |=
                    *refusal == encode_command_result(1, 5, 3, CommandStatus::Rejected, &payload)?;
            }
            assert!(matched, "{capabilities:?} {receipts} {refusal:?}");
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            // The first step ran at once; the buffered one only after its duration; the
            // third never reached Movement.
            let steps = authority.steps.borrow();
            assert_eq!(steps.len(), 2);
            // (The clock starts just before the first step is recorded.)
            let waited = steps[1].1.duration_since(steps[0].1);
            assert!(
                waited >= PACE - std::time::Duration::from_millis(5),
                "{waited:?}"
            );
        }
        Ok(())
    })
}

#[test]
fn a_slow_owner_turn_does_not_shorten_the_wait_for_the_next_step() -> Result<(), Box<dyn Error>> {
    run(async {
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        // The owner turn takes a whole step duration: the next step still waits a full
        // duration after the first one committed, never running back to back.
        let authority = PacedAuthority {
            steps: RefCell::new(Vec::new()),
            turn: PACE,
            receipts: true,
        };
        let (_, frames) = drive_open(
            &authority,
            paced_session(&[13])?,
            &[
                command(1, 1, step, StepDirection::East),
                command(1, 2, step, StepDirection::East),
            ],
            4,
        )
        .await?;
        assert_eq!(frames.len(), baseline().len() + 4);
        let steps = authority.steps.borrow();
        assert_eq!(steps.len(), 2);
        let committed = steps[0].1 + PACE;
        let waited = steps[1].1.duration_since(committed);
        assert!(
            waited >= PACE - std::time::Duration::from_millis(5),
            "{waited:?}"
        );
        Ok(())
    })
}

#[test]
fn a_step_after_the_duration_runs_at_once_and_a_blocked_step_does_not_pace()
-> Result<(), Box<dyn Error>> {
    run(async {
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        let authority = PacedAuthority {
            steps: RefCell::new(Vec::new()),
            turn: std::time::Duration::ZERO,
            receipts: true,
        };
        // Blocked, blocked, then a move: none waits, because only a moved step paces.
        let (_, frames) = drive_open(
            &authority,
            paced_session(&[13])?,
            &[
                command(1, 1, step, StepDirection::North),
                command(1, 2, step, StepDirection::West),
                command(1, 3, step, StepDirection::East),
            ],
            4,
        )
        .await?;
        assert_eq!(frames.len(), baseline().len() + 4);
        let steps = authority.steps.borrow();
        assert_eq!(steps.len(), 3);
        assert!(steps[2].1.duration_since(steps[0].1) < PACE);
        Ok(())
    })
}

#[test]
fn the_buffer_is_dropped_at_disconnect_and_a_resumed_connection_starts_empty()
-> Result<(), Box<dyn Error>> {
    run(async {
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        let authority = PacedAuthority {
            steps: RefCell::new(Vec::new()),
            turn: std::time::Duration::ZERO,
            receipts: true,
        };
        // The client closes while the second step waits in the buffer.
        let (end, frames) = drive_session(
            &authority,
            paced_session(&[13])?,
            &[
                command(1, 1, step, StepDirection::East),
                command(1, 2, step, StepDirection::East),
            ],
        )
        .await?;
        assert_eq!(frames.len(), baseline().len() + 2);
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(authority.steps.borrow().len(), 1);
        // The buffered step never got a result: CommandId 2 is still the next one.
        assert_eq!(ended.continuity.next_command_id, 2);
        // The resumed connection (a newer generation, same continuity) has no buffer and
        // no pacing clock: its first step runs at once, inside the old duration.
        let mut resumed = ended;
        resumed.continuity.connection_generation = 2;
        let (_, frames) = drive_open(
            &authority,
            resumed,
            &[command(2, 2, step, StepDirection::East)],
            2,
        )
        .await?;
        assert_eq!(
            frames.get(baseline().len()),
            Some(&encode_command_result(
                2,
                3,
                2,
                CommandStatus::Accepted,
                &[0x08, 0x01, 0x10, 0x64]
            )?)
        );
        let steps = authority.steps.borrow();
        assert_eq!(steps.len(), 2);
        assert!(steps[1].1.duration_since(steps[0].1) < PACE);
        Ok(())
    })
}
