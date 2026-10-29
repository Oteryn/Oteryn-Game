use super::super::super::actor_spell::tests::runtime_with_capacity;
use super::super::super::chat_intent::{ChatRuntime, chat_in_channel};
use super::*;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatLine, ChatRoomSet, ChatSpeechMode,
    decode_chat_intent_result, decode_chat_line, encode_chat_intent,
};
use oteryn_protocol_oteryn::{decode_command_result, decode_state_delta};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::task::Poll;
use std::time::Duration;

const NOW: SemanticTimeMicros = SemanticTimeMicros::from_micros(1_000_000);

struct ChatAuthority {
    runtime: RefCell<crate::foundation::ChannelRuntimeV1>,
    chat: ChatRuntime,
    names: Vec<(GameSessionId, &'static str)>,
}

impl FreshAdmissionAuthority for ChatAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn resume(
        &self,
        _attempt: ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        })
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        StepOutcome {
            disposition: StepDisposition::Blocked,
            moved_to: None,
        }
    }

    async fn chat_admit(&self, _actor: ExactActorRef, session: GameSessionId) -> ChatRoomSet {
        if let Some((_, name)) = self.names.iter().find(|(known, _)| *known == session) {
            self.chat.admit(session, name, NOW);
        }
        ChatRoomSet::default()
    }

    async fn chat_intent(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        _command_id: u64,
        intent: ChatIntent,
    ) -> oteryn_protocol_oteryn::chat::ChatIntentResult {
        if let Some(done) = self.chat.pre_gate(session, &intent, NOW) {
            return done;
        }
        chat_in_channel(
            &self.chat,
            &self.runtime.borrow(),
            &Default::default(),
            actor,
            session,
            &intent,
            NOW,
        )
    }

    async fn chat_next(&self, session: GameSessionId) -> Option<ChatLine> {
        self.chat.next(session)
    }

    async fn chat_delivered(&self, session: GameSessionId) {
        self.chat.delivered(session);
    }

    async fn chat_rooms_changed(&self, session: GameSessionId) -> Option<ChatRoomSet> {
        self.chat.take_rooms_change(session)
    }
}

fn admitted(
    actor: ExactActorRef,
    session: GameSessionId,
) -> Result<AdmittedSession, Box<dyn Error>> {
    let mut admitted = super::session(SessionContinuity {
        selected_capabilities: super::selection(&[7]),
        ..SessionContinuity::FRESH
    })?;
    admitted.game_session_id = session;
    admitted.runtime_actor = Some(actor);
    Ok(admitted)
}

/// Serves `admitted` over `frames`, then keeps the client open for `linger`.
async fn serve_lingering(
    authority: &ChatAuthority,
    admitted: AdmittedSession,
    frames: &[Vec<u8>],
    linger: Duration,
) -> Result<(AdmittedSession, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    for frame in frames {
        let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(frame);
        client.write_all(&framed).await?;
    }
    let closing = async {
        tokio::time::sleep(linger).await;
        client.shutdown().await?;
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        Ok::<_, std::io::Error>(output)
    };
    let serving = async move {
        let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let (mut serving, mut closing) = (std::pin::pin!(serving), std::pin::pin!(closing));
    let (mut end, mut output) = (None, None);
    std::future::poll_fn(|context| {
        if end.is_none()
            && let Poll::Ready(value) = serving.as_mut().poll(context)
        {
            end = Some(value);
        }
        if output.is_none()
            && let Poll::Ready(value) = closing.as_mut().poll(context)
        {
            output = Some(value);
        }
        if end.is_some() && output.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end.ok_or("end")? else {
        return Err("the connection did not end as a disconnect".into());
    };
    Ok((admitted, split(&output.ok_or("output")??)?))
}

fn lines(frames: &[Vec<u8>]) -> Result<Vec<ChatLine>, Box<dyn Error>> {
    let mut lines = Vec::new();
    for frame in frames {
        let envelope = decode_wire_envelope(frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 12 && delta.delta_type == 1 {
                assert_eq!(delta.new_revision, delta.base_revision + 1);
                lines.push(decode_chat_line(delta.payload).map_err(|e| format!("{e:?}"))?);
            }
        }
    }
    Ok(lines)
}

#[test]
fn a_say_reaches_the_session_in_range_and_not_the_one_out_of_range()
-> Result<(), Box<dyn Error>> {
    run(async {
        let (mut runtime, speaker, speaker_session) = runtime_with_capacity(0x71, 3);
        let mut others = Vec::new();
        for tag in [0x72, 0x73] {
            let session = GameSessionId::decode(&[
                0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
            ])?;
            let reservation = runtime
                .reserve_fresh_session(session)
                .map_err(|e| format!("{e:?}"))?;
            let actor = runtime
                .commit_fresh_session(reservation)
                .map_err(|e| format!("{e:?}"))?;
            others.push((actor, session));
        }
        for actor in [speaker, others[0].0, others[1].0] {
            runtime
                .initialize_first_entry_position(actor)
                .map_err(|e| format!("{e:?}"))?;
        }
        for _ in 0..40 {
            let at = runtime
                .read_actor_position(others[1].0)
                .map_err(|e| format!("{e:?}"))?;
            let mut next = at.position();
            next.x += 1;
            runtime
                .borrow_movement_position()
                .commit_cardinal(at, next)
                .map_err(|e| format!("{e:?}"))?;
        }
        let (near, far) = (others[0], others[1]);
        let authority = ChatAuthority {
            runtime: RefCell::new(runtime),
            chat: ChatRuntime::default(),
            names: vec![(speaker_session, "Aela"), (near.1, "Bryn"), (far.1, "Cade")],
        };
        let say = encode_chat_intent(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: "hello there".to_owned(),
        })
        .map_err(|e| format!("{e:?}"))?;
        let linger = Duration::from_millis(700);
        let hearing = serve_lingering(&authority, admitted(near.0, near.1)?, &[], linger);
        let missing = serve_lingering(&authority, admitted(far.0, far.1)?, &[], linger);
        let speaking = async {
            tokio::time::sleep(Duration::from_millis(150)).await;
            serve_lingering(
                &authority,
                admitted(speaker, speaker_session)?,
                &[command(1, 13, &say)],
                Duration::ZERO,
            )
            .await
        };
        let (hearing, missing, speaking) = join3(hearing, missing, speaking).await;
        let (near_end, near_frames) = hearing?;
        let (far_end, far_frames) = missing?;
        let (_, speaker_frames) = speaking?;

        let mut results = Vec::new();
        for frame in &speaker_frames {
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                assert_eq!(result.status, CommandStatus::Accepted);
                results.push(
                    decode_chat_intent_result(result.payload).map_err(|e| format!("{e:?}"))?,
                );
            }
        }
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].disposition, ChatDisposition::Ok);

        let heard = lines(&near_frames)?;
        assert_eq!(heard.len(), 1, "{heard:?}");
        assert!(matches!(
            &heard[0],
            ChatLine::Local { speaker_name, text, .. }
                if speaker_name == "Aela" && text == "hello there"
        ));
        // The snapshot (revision 1) and the line (revision 2); the far session has the
        // snapshot only.
        assert_eq!(near_end.continuity.chat_revision, 2);
        assert!(lines(&far_frames)?.is_empty());
        assert_eq!(far_end.continuity.chat_revision, 1);
        Ok(())
    })
}

#[test]
fn an_opened_room_follows_its_result_as_one_rooms_delta() -> Result<(), Box<dyn Error>> {
    use crate::chat::relay::{RelayKey, RelayKeyRing};
    use oteryn_protocol_oteryn::chat::{ChatRoom, DELTA_TYPE_CHAT_ROOMS_V1, decode_chat_rooms};
    use std::str::FromStr as _;
    run(async {
        let (runtime, actor, session) = runtime_with_capacity(0x71, 3);
        let authority = ChatAuthority {
            runtime: RefCell::new(runtime),
            chat: ChatRuntime::default(),
            names: vec![(session, "Aela")],
        };
        // A configured relay that never connects: opening a room needs no listener.
        let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [1; 32])])
            .map_err(|e| format!("{e:?}"))?;
        let options =
            sqlx::postgres::PgConnectOptions::from_str("postgresql://nobody@127.0.0.1:1/none")?;
        let world = [0x01, 0x90, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
        let relay = super::super::super::chat_relay::ChatRelay::new(ring, world, options)
            .map_err(|e| format!("{e:?}"))?;
        authority.chat.install_relay(std::sync::Arc::new(relay));
        let open = encode_chat_intent(&ChatIntent::OpenRoom(ChatRoom::Help))
            .map_err(|e| format!("{e:?}"))?;
        let (end, frames) = serve_lingering(
            &authority,
            admitted(actor, session)?,
            &[command(1, 13, &open), command(2, 13, &open)],
            Duration::from_millis(300),
        )
        .await?;
        let mut rooms = Vec::new();
        let mut results = Vec::new();
        for frame in &frames {
            let envelope = decode_wire_envelope(frame)?;
            match envelope.message_type() {
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload())?;
                    if delta.domain_id == 12 && delta.delta_type == DELTA_TYPE_CHAT_ROOMS_V1 {
                        rooms.push(
                            decode_chat_rooms(delta.payload).map_err(|e| format!("{e:?}"))?,
                        );
                    }
                }
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload())?;
                    results.push(
                        decode_chat_intent_result(result.payload)
                            .map_err(|e| format!("{e:?}"))?
                            .disposition,
                    );
                }
                _ => {}
            }
        }
        assert_eq!(results, vec![ChatDisposition::Ok, ChatDisposition::Ok]);
        // The second open changes nothing, so it sends no delta.
        assert_eq!(rooms.len(), 1, "{rooms:?}");
        assert!(rooms[0].contains(ChatRoom::Help));
        assert_eq!(end.continuity.chat_revision, 2);
        Ok(())
    })
}

async fn join3<A: Future, B: Future, C: Future>(
    a: A,
    b: B,
    c: C,
) -> (A::Output, B::Output, C::Output) {
    let (mut a, mut b, mut c) = (std::pin::pin!(a), std::pin::pin!(b), std::pin::pin!(c));
    let (mut x, mut y, mut z) = (None, None, None);
    std::future::poll_fn(|context| {
        if x.is_none()
            && let Poll::Ready(value) = a.as_mut().poll(context)
        {
            x = Some(value);
        }
        if y.is_none()
            && let Poll::Ready(value) = b.as_mut().poll(context)
        {
            y = Some(value);
        }
        if z.is_none()
            && let Poll::Ready(value) = c.as_mut().poll(context)
        {
            z = Some(value);
        }
        if x.is_some() && y.is_some() && z.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    (x.expect("first"), y.expect("second"), z.expect("third"))
}
