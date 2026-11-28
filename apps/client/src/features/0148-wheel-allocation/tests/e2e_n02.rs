//! N02 replay: a captured Wheel allocation Save feeds the named-build path of the production
//! client: save a named build, export its code, import it into a fresh panel, and a malformed code.
//! Consumers: `submit` in `presets.rs` over client-ui `PresetBook` / `WheelPanel::import_build`.
use super::submit;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::wheel_allocation::{
    CommandStatus, Intent, WheelPanel, WheelResponse, apply_result, atelier, presets::PresetBook,
    request, wheel,
};

type Error = Box<dyn std::error::Error>;
const CORE_SLOT: usize = 20;
const PRESET_OK: &str = "Preset updated. Loaded points are proposals; Save Wheel submits them.";

struct Frames {
    atelier_v2: bool,
    kind: u32,
    status: CommandStatus,
    receipt: Vec<u8>,
    wheel: Vec<u8>,
    atelier: Vec<u8>,
    resources: Option<Vec<u8>>,
    request: Option<Vec<u8>>,
    diagram: Option<oteryn_client_runtime::wheel_presentation::Reply>,
}

struct Captured {
    query: Frames,
    save: Frames,
    initial: [u16; 36],
    saved: [u16; 36],
    saved_revision: u64,
}

fn occurrence(byte: u8) -> [u8; 16] {
    let mut id = [byte; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn wire<T>(result: Result<T, impl std::fmt::Debug>) -> Result<T, Error> {
    result.map_err(|error| format!("{error:?}").into())
}

fn points(text: &str) -> Result<[u16; 36], Error> {
    let values = text
        .split(',')
        .map(str::parse)
        .collect::<Result<Vec<u16>, _>>()?;
    values
        .try_into()
        .map_err(|_| "capture needs 36 points".into())
}

fn frames(revision: u64, points: [u16; 36]) -> Result<Frames, Error> {
    let wheel_frame = wire(wheel::encode_wheel_reply(&wheel::WheelAllocationReply {
        refused: false,
        revision,
        points,
        vessels: None,
    }))?;
    Ok(Frames {
        atelier_v2: false,
        kind: 161,
        status: CommandStatus::Accepted,
        receipt: wheel_frame.clone(),
        wheel: wheel_frame,
        atelier: wire(atelier::encode_reply(&atelier::AtelierReply {
            refused: false,
            revision: 4,
            gems: Vec::new(),
            grades: Vec::new(),
            quality_quotes: Vec::new(),
            grade_quotes: Vec::new(),
        }))?,
        resources: None,
        request: None,
        diagram: None,
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        if std::env::var_os("N02_REPLAY_BINDING").is_some() {
            return Err("bound N02 replay requires live captures".into());
        }
        let (mut initial, mut saved) = ([0; 36], [0; 36]);
        initial[7] = 10;
        saved[7] = 10;
        saved[CORE_SLOT] = 25;
        return Ok(Captured {
            query: frames(4, initial)?,
            save: frames(5, saved)?,
            initial,
            saved,
            saved_revision: 5,
        });
    };
    let path = std::path::Path::new(&path);
    let read = |name: &str| -> Result<Frames, Error> {
        let metadata = std::fs::read_to_string(path.join(format!("n02-{name}-wire.txt")))?;
        let fields: Vec<_> = metadata.split_whitespace().collect();
        let [session, sequence, kind, status, atelier_v2] = fields.as_slice() else {
            return Err("invalid N02 wire metadata".into());
        };
        {
            let binding = std::env::var("N02_REPLAY_BINDING")?;
            let expected: Vec<_> = binding.split_whitespace().collect();
            let [expected_session, query, save] = expected.as_slice() else {
                return Err("invalid N02 replay binding".into());
            };
            let expected_sequence = if name == "query" { query } else { save };
            if session != expected_session
                || sequence != expected_sequence
                || query.parse::<u64>()? >= save.parse::<u64>()?
            {
                return Err(
                    "N02 capture does not match the live session and reply sequence".into(),
                );
            }
        }
        let kind = kind.parse()?;
        let status = match status.parse::<u32>()? {
            value if value == CommandStatus::Accepted as u32 => CommandStatus::Accepted,
            _ => return Err("N02 capture status was not Accepted".into()),
        };
        let atelier_v2 = match *atelier_v2 {
            "1" => true,
            _ => return Err("invalid negotiated Atelier mode".into()),
        };
        Ok(Frames {
            atelier_v2,
            kind,
            status,
            receipt: std::fs::read(path.join(format!("n02-{name}-receipt.bin")))?,
            wheel: std::fs::read(path.join(format!("n02-{name}-wheel.bin")))?,
            atelier: std::fs::read(path.join(format!("n02-{name}-atelier.bin")))?,
            resources: Some(std::fs::read(
                path.join(format!("n02-{name}-resources.bin")),
            )?),
            request: Some(std::fs::read(path.join(format!("n02-{name}-request.bin")))?),
            diagram: Some(
                oteryn_client_runtime::wheel_presentation::Reply::decode(&std::fs::read(
                    path.join(format!("n02-{name}-diagram.bin")),
                )?)
                .ok_or("invalid live Wheel presentation")?,
            ),
        })
    };
    let expected = std::fs::read_to_string(path.join("n02-expected.txt"))?;
    let value = |key: &str| -> Result<&str, Error> {
        expected
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| format!("n02-expected.txt lacks {key}").into())
    };
    Ok(Captured {
        query: read("query")?,
        save: read("save")?,
        initial: points(value("points")?)?,
        saved: points(value("saved_points")?)?,
        saved_revision: value("saved_revision")?.parse()?,
    })
}

fn response(frames: &Frames) -> WheelResponse {
    WheelResponse {
        kind: frames.kind,
        status: frames.status,
        payload: frames.receipt.clone(),
        wheel: Some(frames.wheel.clone()),
        atelier: Some(frames.atelier.clone()),
        resources: frames.resources.clone(),
    }
}

fn synchronised(panel: &mut WheelPanel, frames: &Frames) -> Result<(), Error> {
    panel.selected = true;
    panel.atelier_v2 = frames.atelier_v2;
    let refresh = request(panel, Intent::Refresh, occurrence(1)).ok_or("refresh was not sent")?;
    if let Some(payload) = &frames.request {
        assert_eq!(
            &refresh.payload, payload,
            "client Refresh differs from live request"
        );
    }
    assert!(apply_result(panel, &response(frames)));
    assert!(panel.ready());
    Ok(())
}

#[test]
fn e2e_n02_named_build_export_import_and_malformed_code() -> Result<(), Error> {
    let captured = captured()?;
    let (link, _receiver, _outbox) = play_channel();
    let mut mailbox = lock(&link.mailbox);
    synchronised(&mut mailbox.wheel, &captured.query)?;
    assert_eq!(mailbox.wheel.points, captured.initial);

    // The captured accepted Save becomes the confirmed allocation.
    assert!(
        mailbox
            .wheel
            .set_points(CORE_SLOT, captured.saved[CORE_SLOT])
    );
    let id = captured
        .save
        .request
        .as_deref()
        .map(|bytes| wire(wheel::decode_wheel_request(bytes)).map(|request| request.occurrence))
        .transpose()?
        .unwrap_or_else(|| occurrence(2));
    let save = request(&mut mailbox.wheel, Intent::Save, id).ok_or("save was not sent")?;
    assert_eq!(save.kind, wheel::COMMAND_TYPE_WHEEL_ALLOCATION);
    assert!(save.mutation);
    if let Some(payload) = &captured.save.request {
        assert_eq!(
            &save.payload, payload,
            "client Save differs from live request"
        );
    }
    let decoded = wire(wheel::decode_wheel_request(&save.payload))?;
    assert_eq!(decoded.action, 1);
    assert_eq!(decoded.occurrence, id);
    assert_eq!(decoded.expected_revision, captured.saved_revision - 1);
    assert_eq!(decoded.points, captured.saved);
    if captured.save.atelier_v2 {
        assert!(decoded.vessels.is_none());
    }
    assert!(apply_result(&mut mailbox.wheel, &response(&captured.save)));
    assert_eq!(
        mailbox.wheel.feedback,
        Some("The server confirmed the request")
    );
    assert_eq!(mailbox.wheel.points, captured.saved);
    assert_eq!(
        mailbox
            .wheel
            .confirmed_wheel
            .as_ref()
            .map(|state| state.revision),
        Some(captured.saved_revision)
    );

    // Save a named build and read it back from disk.
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("oteryn-n02.json");
    let code = format!(
        "OW1:{}",
        captured
            .saved
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(captured.query.atelier_v2, captured.save.atelier_v2);
    let named = if captured.save.atelier_v2 {
        let board = mailbox
            .wheel
            .vessel_board
            .as_ref()
            .ok_or("live v2 Vessel board absent")?;
        assert_eq!(board.slots.len(), 8);
        assert!(board.slots.iter().all(|slot| slot.gem.is_none()));
        format!("{code}#v2:{}", ["-"; 8].join(","))
    } else {
        code.clone()
    };
    mailbox.wheel.preset_fields[0] = "Core build".into();
    submit(&mut mailbox, 0, &path);
    assert_eq!(mailbox.wheel.preset_feedback, PRESET_OK);
    assert_eq!(mailbox.wheel.preset_book.0.get("Core build"), Some(&named));
    assert_eq!(PresetBook::load(&path)?.0.get("Core build"), Some(&named));

    // Export the code and the URL form.
    submit(&mut mailbox, 3, &path);
    assert_eq!(mailbox.wheel.preset_fields[1], code);
    submit(&mut mailbox, 4, &path);
    assert_eq!(
        mailbox.wheel.preset_fields[1],
        code.replacen("OW1:", "oteryn://wheel/v1/", 1)
    );
    submit(&mut mailbox, 3, &path);
    let exported = mailbox.wheel.preset_fields[1].clone();
    let tibia = captured.save.diagram.as_ref().map(|diagram| {
        assert_eq!(diagram.available, 25);
        mailbox.wheel.diagram.reply = Some(diagram.clone());
        submit(&mut mailbox, 7, &path);
        assert_eq!(mailbox.wheel.preset_feedback, PRESET_OK);
        let code = mailbox.wheel.preset_fields[1].clone();
        assert!(code.starts_with("K0"));
        code
    });
    drop(mailbox);

    // A fresh panel, synchronised to the pre-save server state, imports the exported code.
    let (fresh_link, _fresh_receiver, _fresh_outbox) = play_channel();
    let mut fresh = lock(&fresh_link.mailbox);
    synchronised(&mut fresh.wheel, &captured.query)?;
    assert_eq!(fresh.wheel.points, captured.initial);
    fresh.wheel.preset_fields[1] = exported;
    submit(&mut fresh, 5, &path);
    assert_eq!(fresh.wheel.preset_feedback, PRESET_OK);
    assert_eq!(fresh.wheel.points, captured.saved);
    assert!(fresh.wheel.dirty());

    if let Some(code) = tibia {
        assert!(fresh.wheel.set_points(CORE_SLOT, 0));
        fresh.wheel.diagram.reply = captured.query.diagram.clone();
        fresh.wheel.preset_fields[1] = code;
        submit(&mut fresh, 5, &path);
        assert_eq!(fresh.wheel.preset_feedback, PRESET_OK);
        assert_eq!(fresh.wheel.points, captured.saved);
        assert!(fresh.wheel.dirty());
    }

    // Malformed codes leave the Wheel state and preset book unchanged.
    let before = fresh.wheel.clone();
    let mut over = captured.saved;
    over[CORE_SLOT] = 51;
    let over = format!(
        "OW1:{}",
        over.iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    for (bad, message) in [
        ("OW1:1,2,3".to_string(), "Build needs 36 slots"),
        ("not a build".to_string(), "Use an Oteryn Wheel code or URL"),
        (over, "Slot points exceed capacity"),
    ] {
        fresh.wheel.preset_fields[1] = bad;
        submit(&mut fresh, 5, &path);
        assert_eq!(fresh.wheel.preset_feedback, message);
        let mut expected = before.clone();
        expected.preset_fields[1] = fresh.wheel.preset_fields[1].clone();
        expected.preset_feedback = message.into();
        assert_eq!(fresh.wheel, expected);
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn loading_build_clears_pending_reset_for_both_import_paths() -> Result<(), Error> {
    let captured = captured()?;
    let (link, _receiver, _outbox) = play_channel();
    let mut mailbox = lock(&link.mailbox);
    synchronised(&mut mailbox.wheel, &captured.query)?;
    let code = mailbox.wheel.build_code();
    mailbox.wheel.preset_fields[0] = "Saved".into();
    mailbox.wheel.preset_fields[1] = code.clone();
    mailbox.wheel.preset_book.0.insert("Saved".into(), code);
    for action in [1, 5] {
        mailbox.wheel.diagram.reset_requested = true;
        mailbox.wheel.points = [0; 36];
        submit(&mut mailbox, action, std::path::Path::new("unused"));
        assert_eq!(mailbox.wheel.preset_feedback, PRESET_OK);
        assert_eq!(mailbox.wheel.points, captured.initial);
        assert!(!mailbox.wheel.diagram.reset_requested);
    }
    mailbox.wheel.diagram.reset_requested = true;
    mailbox.wheel.preset_fields[1] = "invalid".into();
    submit(&mut mailbox, 5, std::path::Path::new("unused"));
    assert!(mailbox.wheel.diagram.reset_requested);
    Ok(())
}
