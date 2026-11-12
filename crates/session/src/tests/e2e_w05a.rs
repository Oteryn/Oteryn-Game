//! W05a replay: the server-save countdown snapshot admitted into a session and the countdown
//! delta applied through `Session::apply_state_delta` (domain 40, `server_save_countdown.rs`),
//! which advances the selected-domain revision and yields `SessionEvent::ServerSaveCountdown`.
use super::*;
use oteryn_protocol_oteryn::server_save_countdown::{
    DELTA_TYPE_SERVER_SAVE_COUNTDOWN_V1, SERVER_SAVE_COUNTDOWN_SNAPSHOT,
    STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, ServerSaveCountdown, encode_server_save_countdown,
};
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};
use tokio::io::DuplexStream;

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

struct Captured {
    snapshot: Vec<u8>,
    countdown: Vec<u8>,
    expected: String,
}

fn load() -> Result<Captured, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("w05a-expected.txt"))?;
        let countdown = if value(&expected, "countdown")? == "present" {
            std::fs::read(root.join("w05a-countdown.bin"))?
        } else {
            // The planned reset was outside its five-minute window during the poll.
            encode_server_save_countdown(ServerSaveCountdown {
                ending_epoch: 20_388,
                minutes_remaining: 5,
            })
            .map_err(|error| format!("{error:?}"))?
        };
        return Ok(Captured {
            snapshot: std::fs::read(root.join("w05a-snapshot.bin"))?,
            countdown,
            expected,
        });
    }
    Ok(Captured {
        snapshot: SERVER_SAVE_COUNTDOWN_SNAPSHOT.to_vec(),
        countdown: encode_server_save_countdown(ServerSaveCountdown {
            ending_epoch: 20_388,
            minutes_remaining: 5,
        })
        .map_err(|error| format!("{error:?}"))?,
        expected: "snapshot_revision=0\ncountdown=present\nbase_revision=0\nnew_revision=1\nending_epoch=20388\nminutes_remaining=5\n".into(),
    })
}

async fn admitted(snapshot: Vec<u8>) -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut server) = tokio::io::duplex(1 << 16);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            &[75],
            &[75],
            &[],
            (1, spatial(0)),
            &[(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, 0, 1, snapshot)],
        )
        .await?;
        Ok::<_, BoxError>(server)
    });
    let session = Session::admit(client, admission_with(&[75])?).await?;
    Ok((session, peer.await??))
}

fn delta(
    session: &mut Session<DuplexStream>,
    sequence: u64,
    base: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let frame = encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_SERVER_SAVE_COUNTDOWN,
        base,
        base + 1,
        DELTA_TYPE_SERVER_SAVE_COUNTDOWN_V1,
        payload,
    )?;
    session.apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())
}

#[test]
fn e2e_w05a_countdown_delta_replay() -> Result<(), BoxError> {
    let captured = load()?;
    let expected = &captured.expected;
    block_on(async {
        let (mut session, _server) = admitted(captured.snapshot.clone()).await?;
        let snapshot = session
            .gated_snapshot(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
            .ok_or("countdown snapshot not retained at admission")?;
        assert_eq!(snapshot.payload, captured.snapshot);
        assert_eq!(
            snapshot.revision.to_string(),
            value(expected, "snapshot_revision")?
        );
        assert_eq!(snapshot.revision, 0);

        let event = delta(&mut session, 41, 0, &captured.countdown)?;
        let SessionEvent::ServerSaveCountdown(applied) = event else {
            return Err("countdown delta did not produce a countdown event".into());
        };
        assert_eq!((applied.base_revision, applied.new_revision), (0, 1));
        assert!((1..=5).contains(&applied.value.minutes_remaining));
        assert_ne!(applied.value.ending_epoch, 0);
        if value(expected, "countdown")? == "present" {
            assert_eq!(
                applied.value.ending_epoch.to_string(),
                value(expected, "ending_epoch")?
            );
            assert_eq!(
                applied.value.minutes_remaining.to_string(),
                value(expected, "minutes_remaining")?
            );
        }
        assert_eq!(
            session.gated_revisions,
            vec![(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, 1)]
        );

        // A zero-minute notice and a delta on a stale base revision are refused whole: the
        // applied revision stays 1 and no event is produced.
        let zero_minutes = [0x08, 0x08, 0x10, 0x00];
        assert!(delta(&mut session, 42, 1, &zero_minutes).is_err());
        assert!(delta(&mut session, 43, 0, &captured.countdown).is_err());
        assert_eq!(
            session.gated_revisions,
            vec![(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, 1)]
        );
        Ok(())
    })?
}
