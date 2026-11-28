//! W17 client replay: the Svargrond catalogue, the arena status, the `WrongState` Advance refusal
//! and the Tibiadrome status/enter replies the server sent enter the native arena panel through
//! `request` and `apply_result` (`protocol.rs`), which decode them with the production arena
//! codecs and store them in `ArenaPanel`. A refused or malformed reply leaves the panel unchanged.
use super::*;
use oteryn_client_runtime::arena::{
    self, ArenaAction, ArenaChallenge, ArenaDisposition, ArenaRank, ArenaResult, ArenaState,
    DromeResult,
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type BoxError = Box<dyn std::error::Error>;

struct Capture {
    catalogue: GatedCommandResult,
    status: GatedCommandResult,
    advance: GatedCommandResult,
    drome_status: GatedCommandResult,
    drome_enter: GatedCommandResult,
    expected: String,
}

fn capture() -> Result<Capture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected_session = std::env::var("OTERYN_W17_CAPTURE_SESSION")?.replace('-', "");
        let mut previous = None;
        let mut read = |step| -> Result<GatedCommandResult, BoxError> {
            let bytes = std::fs::read(root.join(format!("w17-{step}.bin")))?;
            let (header, payload) = bytes
                .split_first_chunk::<36>()
                .ok_or("truncated W17 result")?;
            let session: [u8; 16] = header[..16].try_into()?;
            let command_id = u64::from_be_bytes(header[16..24].try_into()?);
            let server_sequence = u64::from_be_bytes(header[24..32].try_into()?);
            let status = u32::from_be_bytes(header[32..].try_into()?);
            let session_hex = u128::from_be_bytes(session);
            if format!("{session_hex:032x}") != expected_session
                || command_id == 0
                || server_sequence == 0
                || status != CommandStatus::Accepted as u32
                || previous.is_some_and(|(id, command, sequence)| {
                    session != id || command_id <= command || server_sequence <= sequence
                })
            {
                return Err(format!("invalid W17 result identity: {step}").into());
            }
            previous = Some((session, command_id, server_sequence));
            Ok(GatedCommandResult {
                command_id,
                status: CommandStatus::Accepted,
                server_sequence,
                payload: payload.to_vec(),
            })
        };
        let catalogue = read("catalogue")?;
        let status = read("status")?;
        let advance = read("advance")?;
        let capture = Capture {
            catalogue,
            status: status.clone(),
            advance,
            drome_status: {
                let arena_after = read("status-after-refusal")?;
                assert_eq!(arena_after.payload, status.payload);
                read("drome-status")?
            },
            drome_enter: read("drome-enter")?,
            expected: std::fs::read_to_string(root.join("w17-expected.txt"))?,
        };
        let after = read("drome-after-refusal")?;
        assert_eq!(after.payload, capture.drome_status.payload);
        return Ok(capture);
    }
    let state = ArenaState::default();
    let drome = DromeResult {
        disposition: 1,
        level: 50,
        start: 50,
        rotation_ends: 1_900_000_000,
        ..DromeResult::default()
    };
    let result = |disposition| {
        arena::encode_match_result(&ArenaResult { disposition, state })
            .map_err(|error| format!("{error:?}"))
    };
    Ok(Capture {
        catalogue: accepted(&arena::encode_catalogue().map_err(|error| format!("{error:?}"))?),
        status: accepted(&result(ArenaDisposition::Applied)?),
        advance: accepted(&result(ArenaDisposition::WrongState)?),
        drome_status: accepted(&drome.encode()),
        drome_enter: accepted(&DromeResult { disposition: 3, ..drome }.encode()),
        expected: "challenge=Greenhorn|1000|10|600\nchallenge=Scrapper|5000|10|600\nchallenge=Warlord|10000|10|600\n\
                   status=Applied|0|0|0|0|0|0,0,0\nadvance=WrongState|0|0|0|0|0|0,0,0\n\
                   drome_status=1|50|50|0|0|0|0|0|0|0|1900000000\ndrome_enter=3|50|50|0|0|0|0|0|0|0|1900000000\n"
            .into(),
    })
}

fn lines<'a>(expected: &'a str, key: &str) -> Vec<&'a str> {
    expected
        .lines()
        .filter_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .collect()
}

fn one<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    lines(expected, key)
        .first()
        .copied()
        .ok_or_else(|| format!("w17 expected lacks {key}").into())
}

fn want_result(text: &str) -> Result<ArenaResult, BoxError> {
    let fields: Vec<&str> = text.split('|').collect();
    let [disposition, completed, active, pit, phase, remaining, masks] = fields.as_slice() else {
        return Err("w17 state row malformed".into());
    };
    let masks: Vec<u8> = masks.split(',').map(str::parse).collect::<Result<_, _>>()?;
    let [m0, m1, m2] = masks.as_slice() else {
        return Err("w17 masks malformed".into());
    };
    Ok(ArenaResult {
        disposition: match *disposition {
            "Applied" => ArenaDisposition::Applied,
            "WrongState" => ArenaDisposition::WrongState,
            other => return Err(format!("w17 unexpected disposition {other}").into()),
        },
        state: ArenaState {
            completed_rank: completed.parse()?,
            active_rank: active.parse()?,
            pit: pit.parse()?,
            phase: phase.parse()?,
            remaining_seconds: remaining.parse()?,
            claimed_masks: [*m0, *m1, *m2],
        },
    })
}

fn want_drome(text: &str) -> Result<DromeResult, BoxError> {
    let values: Vec<u32> = text.split('|').map(str::parse).collect::<Result<_, _>>()?;
    let [
        disposition,
        level,
        start,
        points,
        phase,
        wave,
        remaining,
        cooldown,
        rank,
        potions,
        rotation_ends,
    ] = values.as_slice()
    else {
        return Err("w17 drome row malformed".into());
    };
    Ok(DromeResult {
        disposition: *disposition,
        level: *level,
        start: *start,
        points: *points,
        phase: *phase,
        wave: *wave,
        remaining: *remaining,
        cooldown: *cooldown,
        rank: *rank,
        potions: *potions,
        rotation_ends: *rotation_ends,
    })
}

fn accepted(payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

fn request_action(panel: &mut ArenaPanel, action: ArenaAction) -> Result<(), BoxError> {
    let command = request(panel, Intent::Action(action)).ok_or("W17 action request refused")?;
    assert_eq!((command.capability, command.kind), (170, 436));
    assert_eq!(
        arena::decode_match_request(&command.payload).ok(),
        Some(action)
    );
    assert_eq!(
        command.payload,
        arena::encode_match_request(action).map_err(|error| format!("{error:?}"))?
    );
    Ok(())
}

fn global_catalogue(rows: &[ArenaChallenge]) -> Result<(), BoxError> {
    let expected = [
        ArenaRank::Greenhorn,
        ArenaRank::Scrapper,
        ArenaRank::Warlord,
    ]
    .into_iter()
    .zip([1000, 5000, 10_000])
    .map(|(rank, fee)| ArenaChallenge {
        rank,
        fee,
        rooms: 10,
        room_seconds: 360,
    })
    .collect::<Vec<_>>();
    if rows != expected {
        return Err(
            format!("Global arena catalogue expected={expected:?} observed={rows:?}").into(),
        );
    }
    Ok(())
}

#[test]
fn global_catalogue_rejects_ten_minute_timer() -> Result<(), BoxError> {
    let bytes = arena::encode_catalogue().map_err(|error| format!("{error:?}"))?;
    let mut rows = arena::decode_catalogue(&bytes).map_err(|error| format!("{error:?}"))?;
    for row in &mut rows {
        row.room_seconds = 600;
    }
    assert!(global_catalogue(&rows).is_err());
    for row in &mut rows {
        row.room_seconds = 360;
    }
    global_catalogue(&rows)
}

#[test]
fn e2e_w17_arena_replies_replay_into_the_arena_panel() -> Result<(), BoxError> {
    let capture = capture()?;
    let expected = capture.expected.as_str();
    let mut panel = ArenaPanel {
        selected: true,
        ..ArenaPanel::default()
    };

    let command = request(&mut panel, Intent::Catalogue).ok_or("catalogue request refused")?;
    assert_eq!((command.capability, command.kind), (170, 240));
    assert!(command.payload.is_empty());
    assert!(apply_result(&mut panel, &capture.catalogue));
    let want_challenges = lines(expected, "challenge")
        .into_iter()
        .map(|row| {
            let fields: Vec<&str> = row.split('|').collect();
            let [rank, fee, rooms, seconds] = fields.as_slice() else {
                return Err(BoxError::from("w17 challenge row malformed"));
            };
            Ok(ArenaChallenge {
                rank: match *rank {
                    "Greenhorn" => ArenaRank::Greenhorn,
                    "Scrapper" => ArenaRank::Scrapper,
                    "Warlord" => ArenaRank::Warlord,
                    other => return Err(format!("w17 unknown rank {other}").into()),
                },
                fee: fee.parse()?,
                rooms: rooms.parse()?,
                room_seconds: seconds.parse()?,
            })
        })
        .collect::<Result<Vec<_>, BoxError>>()?;
    assert_eq!(panel.challenges, want_challenges);

    request_action(&mut panel, ArenaAction::Status)?;
    assert!(apply_result(&mut panel, &capture.status));
    let want_status = want_result(one(expected, "status")?)?;
    assert_eq!(want_status.state, ArenaState::default());
    assert_eq!(panel.result, Some(want_status));

    request_action(&mut panel, ArenaAction::Advance)?;
    assert!(apply_result(&mut panel, &capture.advance));
    let want_refusal = want_result(one(expected, "advance")?)?;
    assert_eq!(want_refusal.disposition, ArenaDisposition::WrongState);
    assert_eq!(panel.result, Some(want_refusal));
    assert_eq!(want_refusal.state, want_status.state);
    assert!(panel_text(&panel).contains("Action unavailable in the current state\r\n"));

    request_action(&mut panel, ArenaAction::Drome(arena::DromeAction::Status))?;
    assert!(apply_result(&mut panel, &capture.drome_status));
    assert_eq!(
        panel.drome,
        Some(want_drome(one(expected, "drome_status")?)?)
    );
    let hazards = panel.hazards.clone();
    assert!(hazards.entries.is_empty());
    let hazard_clock = panel.hazards_received_at;
    request_action(&mut panel, ArenaAction::Drome(arena::DromeAction::Enter))?;
    assert!(apply_result(&mut panel, &capture.drome_enter));
    let want_enter = want_drome(one(expected, "drome_enter")?)?;
    assert_eq!(panel.drome, Some(want_enter));
    assert_eq!(want_enter.disposition, 3);

    assert_eq!(panel.hazards, hazards);
    assert_eq!(panel.hazards_received_at, hazard_clock);
    // A refused command result and a malformed payload leave the stored replies unchanged.
    let kept = panel.clone();
    let kept_hazards = (kept.hazards.clone(), kept.hazards_received_at);
    request_action(&mut panel, ArenaAction::Status)?;
    let would_mutate = arena::encode_match_result(&ArenaResult {
        disposition: ArenaDisposition::Applied,
        state: ArenaState {
            completed_rank: 2,
            ..ArenaState::default()
        },
    })
    .map_err(|error| format!("{error:?}"))?;
    let refused = GatedCommandResult {
        status: CommandStatus::Rejected,
        ..accepted(&would_mutate)
    };
    assert!(!apply_result(&mut panel, &refused));
    assert_eq!(panel.feedback, Some("The server refused the arena request"));
    assert_eq!(
        (&panel.result, &panel.drome, &panel.challenges),
        (&kept.result, &kept.drome, &kept.challenges)
    );
    assert_eq!(
        (panel.hazards.clone(), panel.hazards_received_at),
        kept_hazards
    );
    request_action(&mut panel, ArenaAction::Status)?;
    assert!(!apply_result(&mut panel, &accepted(&[8, 11])));
    assert_eq!(
        panel.feedback,
        Some("Invalid arena reply; refresh to retry")
    );
    assert_eq!(
        (&panel.result, &panel.drome, &panel.challenges),
        (&kept.result, &kept.drome, &kept.challenges)
    );
    assert_eq!(
        (panel.hazards.clone(), panel.hazards_received_at),
        kept_hazards
    );
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_some() {
        println!(
            "W17_REPLAY_CHECK outgoing_actions=exact hazards=unchanged rejected_and_malformed=unchanged"
        );
        global_catalogue(&panel.challenges)?;
    }
    Ok(())
}
