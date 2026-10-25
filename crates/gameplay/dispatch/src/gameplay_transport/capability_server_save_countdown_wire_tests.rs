use super::*;
use crate::gameplay_transport::connection::LivenessPolicy;
use crate::gameplay_transport::server_save_countdown::DueServerSaveCountdown;
use oteryn_protocol_oteryn::server_save_countdown::{
    CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, DELTA_TYPE_SERVER_SAVE_COUNTDOWN_V1,
    SERVER_SAVE_COUNTDOWN_SNAPSHOT, SNAPSHOT_TYPE_SERVER_SAVE_COUNTDOWN_V1,
    STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, ServerSaveCountdown, decode_server_save_countdown,
};
use oteryn_protocol_oteryn::{decode_snapshot_chunk, decode_state_delta};

const RESET: i64 = 1_800_000_000_000;
/// Several probe ticks per serve; the unanswered probes end it as control lost.
const FAST: LivenessPolicy = LivenessPolicy {
    interval: std::time::Duration::from_millis(20),
    missed_limit: 4,
};

fn due(minutes_remaining: u8) -> DueServerSaveCountdown {
    DueServerSaveCountdown {
        reset_at_unix_ms: RESET,
        notice: ServerSaveCountdown {
            ending_epoch: 9,
            minutes_remaining,
        },
    }
}

/// Serve with the client connected but silent until the liveness cadence ends it.
async fn serve_probed(
    authority: &NegotiatingAuthority,
    admitted: AdmittedSession,
) -> Result<(AdmittedSession, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    let end = serve_admitted(&mut server, admitted, authority, FAST).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let ConnectionEnd::AdmittedThenControlLost(served) = end else {
        return Err(format!("{end:?}").into());
    };
    Ok((served, split(&output)?))
}

/// Domain 40 in the frames: snapshot revisions and the deltas `(base, new, notice)`.
#[allow(clippy::type_complexity)]
fn countdown_of(
    frames: &[Vec<u8>],
) -> Result<(Vec<u64>, Vec<(u64, u64, ServerSaveCountdown)>), Box<dyn Error>> {
    let (mut snapshots, mut deltas) = (Vec::new(), Vec::new());
    for frame in frames {
        let envelope = decode_wire_envelope(frame)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                let (_, domains) = decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_SERVER_SAVE_COUNTDOWN {
                        assert_eq!(domain.snapshot_type, SNAPSHOT_TYPE_SERVER_SAVE_COUNTDOWN_V1);
                        assert_eq!(domain.payload, SERVER_SAVE_COUNTDOWN_SNAPSHOT);
                        snapshots.push(domain.revision);
                    }
                }
            }
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload())?;
                if delta.domain_id == STATE_DOMAIN_SERVER_SAVE_COUNTDOWN {
                    assert_eq!(delta.delta_type, DELTA_TYPE_SERVER_SAVE_COUNTDOWN_V1);
                    deltas.push((
                        delta.base_revision,
                        delta.new_revision,
                        decode_server_save_countdown(delta.payload)
                            .map_err(|e| format!("{e:?}"))?,
                    ));
                }
            }
            _ => {}
        }
    }
    Ok((snapshots, deltas))
}

#[test]
fn capability_75_receives_each_countdown_minute_once_across_probes_and_reconnects()
-> Result<(), Box<dyn Error>> {
    for set in [
        PRODUCTION_OFFERED_CAPABILITIES,
        ITEM_VIEW_OFFERED_CAPABILITIES,
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    ] {
        assert!(
            set.iter()
                .any(|capability| capability.id == CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
        );
    }
    run(async {
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(&authority, &bootstrap(&[75])?).await?;
        assert_eq!(accepted_selection(&frames)?, [75]);
        let admitted = admitted.map_err(|e| format!("{e:?}"))?;
        // Nothing due: the snapshot only.
        let (served, frames) = serve_probed(&authority, admitted).await?;
        assert_eq!(countdown_of(&frames)?, (vec![0], vec![]));
        // Minute 5 is due for four probe ticks: one delta.
        authority.server_save.set(Some(due(5)));
        let (served, frames) = serve_probed(&authority, served).await?;
        let (snapshots, deltas) = countdown_of(&frames)?;
        assert_eq!(snapshots, [0]);
        assert_eq!(deltas, [(0, 1, due(5).notice)]);
        assert_eq!(served.continuity.server_save_sent, Some((RESET, 5)));
        // A reconnect in the same minute never repeats it; the next minute follows.
        let (served, frames) = serve_probed(&authority, served).await?;
        assert_eq!(countdown_of(&frames)?, (vec![1], vec![]));
        authority.server_save.set(Some(due(4)));
        let (served, frames) = serve_probed(&authority, served).await?;
        assert_eq!(countdown_of(&frames)?, (vec![1], vec![(1, 2, due(4).notice)]));
        // A clock step back to minute 5 is not a new minute.
        authority.server_save.set(Some(due(5)));
        let (_, frames) = serve_probed(&authority, served).await?;
        assert_eq!(countdown_of(&frames)?.1, []);
        // Without capability 75 neither the snapshot nor a notice is written.
        let (admitted, frames) = admit(&authority, &bootstrap(&[6])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6]);
        authority.server_save.set(Some(due(3)));
        let (_, frames) = serve_probed(&authority, admitted.map_err(|e| format!("{e:?}"))?).await?;
        assert_eq!(countdown_of(&frames)?, (vec![], vec![]));
        Ok(())
    })
}
