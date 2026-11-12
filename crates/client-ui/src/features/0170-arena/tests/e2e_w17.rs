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
    catalogue: Vec<u8>,
    status: Vec<u8>,
    advance: Vec<u8>,
    drome_status: Vec<u8>,
    drome_enter: Vec<u8>,
    expected: String,
}

fn capture() -> Result<Capture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            catalogue: std::fs::read(root.join("w17-catalogue.bin"))?,
            status: std::fs::read(root.join("w17-status.bin"))?,
            advance: std::fs::read(root.join("w17-advance.bin"))?,
            drome_status: std::fs::read(root.join("w17-drome-status.bin"))?,
            drome_enter: std::fs::read(root.join("w17-drome-enter.bin"))?,
            expected: std::fs::read_to_string(root.join("w17-expected.txt"))?,
        });
    }
    let state = ArenaState {
        completed_rank: 1,
        ..ArenaState::default()
    };
    let drome = DromeResult {
        disposition: 3,
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
        catalogue: arena::encode_catalogue().map_err(|error| format!("{error:?}"))?,
        status: result(ArenaDisposition::Applied)?,
        advance: result(ArenaDisposition::WrongState)?,
        drome_status: drome.encode(),
        drome_enter: drome.encode(),
        expected: "challenge=Greenhorn|1000|10|600\nchallenge=Scrapper|5000|10|600\nchallenge=Warlord|10000|10|600\n\
                   status=Applied|1|0|0|0|0|0,0,0\nadvance=WrongState|1|0|0|0|0|0,0,0\n\
                   drome_status=3|50|50|0|0|0|0|0|0|0|1900000000\ndrome_enter=3|50|50|0|0|0|0|0|0|0|1900000000\n"
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
    assert!(apply_result(&mut panel, &accepted(&capture.catalogue)));
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

    let status =
        request(&mut panel, Intent::Action(ArenaAction::Status)).ok_or("status request refused")?;
    assert_eq!((status.capability, status.kind), (170, 436));
    assert!(apply_result(&mut panel, &accepted(&capture.status)));
    let want_status = want_result(one(expected, "status")?)?;
    assert_eq!(panel.result, Some(want_status));

    request(&mut panel, Intent::Action(ArenaAction::Advance)).ok_or("advance request refused")?;
    assert!(apply_result(&mut panel, &accepted(&capture.advance)));
    let want_refusal = want_result(one(expected, "advance")?)?;
    assert_eq!(want_refusal.disposition, ArenaDisposition::WrongState);
    assert_eq!(panel.result, Some(want_refusal));
    assert_eq!(want_refusal.state, want_status.state);
    assert!(panel_text(&panel).contains("Action unavailable in the current state\r\n"));

    request(
        &mut panel,
        Intent::Action(ArenaAction::Drome(arena::DromeAction::Status)),
    )
    .ok_or("drome status request refused")?;
    assert!(apply_result(&mut panel, &accepted(&capture.drome_status)));
    assert_eq!(
        panel.drome,
        Some(want_drome(one(expected, "drome_status")?)?)
    );
    request(
        &mut panel,
        Intent::Action(ArenaAction::Drome(arena::DromeAction::Enter)),
    )
    .ok_or("drome enter request refused")?;
    assert!(apply_result(&mut panel, &accepted(&capture.drome_enter)));
    let want_enter = want_drome(one(expected, "drome_enter")?)?;
    assert_eq!(panel.drome, Some(want_enter));

    // A refused command result and a malformed payload leave the stored replies unchanged.
    let kept = panel.clone();
    request(&mut panel, Intent::Action(ArenaAction::Status)).ok_or("status request refused")?;
    let refused = GatedCommandResult {
        status: CommandStatus::Rejected,
        ..accepted(&[])
    };
    assert!(!apply_result(&mut panel, &refused));
    assert_eq!(panel.feedback, Some("The server refused the arena request"));
    assert_eq!(
        (&panel.result, &panel.drome, &panel.challenges),
        (&kept.result, &kept.drome, &kept.challenges)
    );
    request(&mut panel, Intent::Action(ArenaAction::Status)).ok_or("status request refused")?;
    assert!(!apply_result(&mut panel, &accepted(&[8, 11])));
    assert_eq!(
        panel.feedback,
        Some("Invalid arena reply; refresh to retry")
    );
    assert_eq!(
        (&panel.result, &panel.drome, &panel.challenges),
        (&kept.result, &kept.drome, &kept.challenges)
    );
    Ok(())
}
