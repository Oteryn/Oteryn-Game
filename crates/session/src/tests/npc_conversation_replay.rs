//! Shared client replay for NPC service deltas (S10 Rashid shop, S16a Oressa). The captured
//! `STATE_DOMAIN_NPC_CONVERSATION` and `STATE_DOMAIN_NPC_TRADE_WINDOW` delta payloads reach
//! `Session::apply_state_delta` (`domain_dispatch.rs`) through the real wire frame and surface as
//! `SessionEvent::NpcConversation` / `SessionEvent::NpcTradeWindow`, the values the native
//! dialogue and shop panels render.
use super::*;
use oteryn_protocol_oteryn::npc_service;

/// One conversation delta: the server sequence and revisions it carried, the conversation it
/// opens (identity and generation of the NPC, or `None` when closed) and the exact reply lines.
pub(super) struct Frame {
    pub(super) step: String,
    pub(super) domain: u32,
    pub(super) sequence: u64,
    pub(super) base: u64,
    pub(super) new: u64,
    pub(super) npc: Option<([u8; 16], u64)>,
    pub(super) lines: Vec<String>,
    pub(super) payload: Vec<u8>,
}

fn unhex(text: &str) -> Result<[u8; 16], BoxError> {
    let mut out = [0u8; 16];
    if text.len() != 32 {
        return Err("npc identity must be 32 hex digits".into());
    }
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(text.get(index * 2..index * 2 + 2).ok_or("hex slice")?, 16)?;
    }
    Ok(out)
}

/// Offer rows a trade window shows, after the catalogue revision.
fn window_lines(window: &npc_service::NpcTradeWindow) -> Vec<String> {
    let mut lines = vec![format!(
        "revision={}",
        window.open.map_or(0, |(_, revision)| revision)
    )];
    for offer in &window.offers {
        lines.push(format!(
            "{:?} {} x{} @{}",
            offer.side, offer.item_definition_ref, offer.count, offer.unit_price
        ));
    }
    lines
}

/// Read `<row>-expected.txt` (one tab-separated line per frame: step, domain, sequence, base,
/// new, `-` or `<identity hex>:<generation>`, visible lines joined by U+001F) and each
/// `<row>-<step>-<conversation|window>.bin` capture. A missing capture is an error.
pub(super) fn captured(row: &str, dir: &std::path::Path) -> Result<Vec<Frame>, BoxError> {
    let expected = std::fs::read_to_string(dir.join(format!("{row}-expected.txt")))?;
    let mut frames = Vec::new();
    for line in expected.lines().filter(|line| line.contains('\t')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [step, domain, sequence, base, new, npc, lines] = fields.as_slice() else {
            return Err(format!("{row} expected line has wrong field count: {line}").into());
        };
        let npc = match *npc {
            "-" => None,
            text => {
                let (identity, generation) = text.split_once(':').ok_or("npc field lacks ':'")?;
                Some((unhex(identity)?, generation.parse()?))
            }
        };
        let domain: u32 = domain.parse()?;
        let kind = if domain == npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW {
            "window"
        } else {
            "conversation"
        };
        frames.push(Frame {
            step: (*step).into(),
            domain,
            sequence: sequence.parse()?,
            base: base.parse()?,
            new: new.parse()?,
            npc,
            lines: lines
                .split('\u{1f}')
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect(),
            payload: std::fs::read(dir.join(format!("{row}-{step}-{kind}.bin")))?,
        });
    }
    if frames.is_empty() {
        return Err(format!("{row} expected file has no conversation frames").into());
    }
    Ok(frames)
}

/// Inline fixture frame built with the production conversation encoder.
pub(super) fn built(
    step: &str,
    (sequence, base): (u64, u64),
    npc: Option<([u8; 16], u64)>,
    lines: &[&str],
) -> Result<Frame, BoxError> {
    let conversation = npc_service::NpcConversation {
        open: npc.map(|(identity, generation)| {
            (
                EntityRef {
                    identity,
                    generation,
                },
                npc_service::NpcConversationState::Open,
            )
        }),
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
    };
    Ok(Frame {
        step: step.into(),
        domain: npc_service::STATE_DOMAIN_NPC_CONVERSATION,
        sequence,
        base,
        new: base + 1,
        npc,
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        payload: npc_service::encode_npc_conversation(&conversation)
            .map_err(|error| format!("{error:?}"))?,
    })
}

/// Inline trade-window frame built with the production window encoder.
pub(super) fn built_window(
    step: &str,
    (sequence, base): (u64, u64),
    npc: ([u8; 16], u64),
    window: &npc_service::NpcTradeWindow,
) -> Result<Frame, BoxError> {
    Ok(Frame {
        step: step.into(),
        domain: npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW,
        sequence,
        base,
        new: base + 1,
        npc: Some(npc),
        lines: window_lines(window),
        payload: npc_service::encode_npc_trade_window(window)
            .map_err(|error| format!("{error:?}"))?,
    })
}

pub(super) fn replay(row: &str, frames: Vec<Frame>) -> Result<(), BoxError> {
    // Captured server sequences interleave other domains; only their order is replayed.
    if frames
        .windows(2)
        .any(|pair| pair[0].sequence >= pair[1].sequence)
    {
        return Err(format!("{row}: frame sequences are not increasing").into());
    }
    let join_revision = |domain: u32| {
        frames
            .iter()
            .find(|frame| frame.domain == domain)
            .map_or(0, |frame| frame.base)
    };
    let (conversation_join, window_join) = (
        join_revision(npc_service::STATE_DOMAIN_NPC_CONVERSATION),
        join_revision(npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW),
    );
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn({
            let row = row.to_owned();
            move || {
                block_on(async move {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let joining = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            &[npc_service::CAPABILITY_NPC_SERVICE_V1],
                            &[],
                            (1, spatial(0)),
                            &[
                                (
                                    npc_service::STATE_DOMAIN_NPC_CONVERSATION,
                                    conversation_join,
                                    npc_service::SNAPSHOT_TYPE_NPC_CONVERSATION_V1,
                                    npc_service::encode_npc_conversation(&Default::default())
                                        .map_err(|error| format!("{error:?}"))?,
                                ),
                                (
                                    npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW,
                                    window_join,
                                    npc_service::SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1,
                                    npc_service::encode_npc_trade_window(&Default::default())
                                        .map_err(|error| format!("{error:?}"))?,
                                ),
                            ],
                        )
                        .await?;
                        Ok::<_, BoxError>(server)
                    });
                    let mut session =
                        Session::admit(client, admission_with(CLIENT_SUPPORTED_CAPABILITIES)?)
                            .await?;
                    let mut peer = joining.await??;
                    let mut last = (0, conversation_join);
                    for (index, frame) in frames.iter().enumerate() {
                        // The join leaves the session at sequence 40; replay is contiguous.
                        let wire = 41 + index as u64;
                        write_frame(
                            &mut peer,
                            &encode_state_delta(
                                1,
                                wire,
                                frame.domain,
                                frame.base,
                                frame.new,
                                1,
                                &frame.payload,
                            )?,
                        )
                        .await?;
                        session.service_liveness(Duration::from_millis(20)).await?;
                        let events = session.take_events();
                        let (sequence, base, new, npc, lines) = match events.as_slice() {
                            [SessionEvent::NpcConversation(delta)]
                                if frame.domain == npc_service::STATE_DOMAIN_NPC_CONVERSATION =>
                            {
                                (
                                    delta.server_sequence,
                                    delta.base_revision,
                                    delta.new_revision,
                                    delta
                                        .value
                                        .open
                                        .map(|(npc, _)| (npc.identity, npc.generation)),
                                    delta.value.lines.clone(),
                                )
                            }
                            [SessionEvent::NpcTradeWindow(delta)]
                                if frame.domain == npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW =>
                            {
                                (
                                    delta.server_sequence,
                                    delta.base_revision,
                                    delta.new_revision,
                                    delta
                                        .value
                                        .open
                                        .map(|(npc, _)| (npc.identity, npc.generation)),
                                    window_lines(&delta.value),
                                )
                            }
                            _ => {
                                return Err(format!(
                                    "{row} step {}: expected one domain {} event, saw {} events",
                                    frame.step,
                                    frame.domain,
                                    events.len()
                                )
                                .into());
                            }
                        };
                        assert_eq!(sequence, wire, "{}", frame.step);
                        assert_eq!(base, frame.base, "{}", frame.step);
                        assert_eq!(new, frame.new, "{}", frame.step);
                        assert_eq!(npc, frame.npc, "{}", frame.step);
                        assert_eq!(lines, frame.lines, "{}", frame.step);
                        if frame.domain == npc_service::STATE_DOMAIN_NPC_CONVERSATION {
                            last = (wire, frame.new);
                        }
                    }
                    // A malformed delta at the next revision is refused and publishes no event.
                    write_frame(
                        &mut peer,
                        &encode_state_delta(
                            1,
                            last.0 + 1,
                            npc_service::STATE_DOMAIN_NPC_CONVERSATION,
                            last.1,
                            last.1 + 1,
                            npc_service::DELTA_TYPE_NPC_CONVERSATION_V1,
                            &[0xff],
                        )?,
                    )
                    .await?;
                    assert!(
                        session
                            .service_liveness(Duration::from_millis(20))
                            .await
                            .is_err(),
                        "{row}: malformed conversation delta was accepted"
                    );
                    assert!(session.take_events().is_empty(), "{row}");
                    Ok::<_, BoxError>(())
                })?
            }
        })?
        .join()
        .map_err(|_| format!("{row} replay thread panicked"))?
}
