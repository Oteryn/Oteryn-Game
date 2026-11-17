//! N02 replay: a captured Wheel allocation Save feeds the named-build path of the production
//! client: save a named build, export its code, import it into a fresh panel, and a malformed code.
//! Consumers: `submit` in `presets.rs` over client-ui `PresetBook` / `WheelPanel::import_build`.
use super::submit;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::wheel_allocation::{
    CommandStatus, Intent, WheelPanel, WheelResponse, apply_result, atelier,
    presets::PresetBook, request, wheel,
};

type Error = Box<dyn std::error::Error>;
const CORE_SLOT: usize = 20;
const PRESET_OK: &str = "Preset updated. Loaded points are proposals; Save Wheel submits them.";

struct Frames {
    receipt: Vec<u8>,
    wheel: Vec<u8>,
    atelier: Vec<u8>,
    resources: Option<Vec<u8>>,
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
    values.try_into().map_err(|_| "capture needs 36 points".into())
}

fn frames(revision: u64, points: [u16; 36]) -> Result<Frames, Error> {
    let wheel_frame = wire(wheel::encode_wheel_reply(&wheel::WheelAllocationReply {
        refused: false,
        revision,
        points,
        vessels: None,
    }))?;
    Ok(Frames {
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
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
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
        Ok(Frames {
            receipt: std::fs::read(path.join(format!("n02-{name}-receipt.bin")))?,
            wheel: std::fs::read(path.join(format!("n02-{name}-wheel.bin")))?,
            atelier: std::fs::read(path.join(format!("n02-{name}-atelier.bin")))?,
            resources: std::fs::read(path.join(format!("n02-{name}-resources.bin"))).ok(),
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
        kind: 161,
        status: CommandStatus::Accepted,
        payload: frames.receipt.clone(),
        wheel: Some(frames.wheel.clone()),
        atelier: Some(frames.atelier.clone()),
        resources: frames.resources.clone(),
    }
}

fn synchronised(panel: &mut WheelPanel, frames: &Frames) -> Result<(), Error> {
    panel.selected = true;
    request(panel, Intent::Refresh, occurrence(1)).ok_or("refresh was not sent")?;
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
    assert!(mailbox.wheel.set_points(CORE_SLOT, captured.saved[CORE_SLOT]));
    request(&mut mailbox.wheel, Intent::Save, occurrence(2)).ok_or("save was not sent")?;
    assert!(apply_result(&mut mailbox.wheel, &response(&captured.save)));
    assert_eq!(mailbox.wheel.feedback, Some("The server confirmed the request"));
    assert_eq!(mailbox.wheel.points, captured.saved);
    assert_eq!(
        mailbox.wheel.confirmed_wheel.as_ref().map(|state| state.revision),
        Some(captured.saved_revision)
    );

    // Save a named build and read it back from disk.
    let path = std::env::temp_dir().join(format!("oteryn-n02-{}.json", std::process::id()));
    let code = format!(
        "OW1:{}",
        captured.saved.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
    );
    mailbox.wheel.preset_fields[0] = "Core build".into();
    submit(&mut mailbox, 0, &path);
    assert_eq!(mailbox.wheel.preset_feedback, PRESET_OK);
    assert_eq!(mailbox.wheel.preset_book.0.get("Core build"), Some(&code));
    assert_eq!(PresetBook::load(&path)?.0.get("Core build"), Some(&code));

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

    // Malformed codes are refused and leave the panel exactly as it was.
    let before = fresh.wheel.clone();
    let mut over = captured.saved;
    over[CORE_SLOT] = 51;
    let over = format!("OW1:{}", over.iter().map(u16::to_string).collect::<Vec<_>>().join(","));
    for (bad, message) in [
        ("OW1:1,2,3".to_string(), "Build needs 36 slots"),
        ("not a build".to_string(), "Use an Oteryn Wheel code or URL"),
        (over, "Slot points exceed capacity"),
    ] {
        fresh.wheel.preset_fields[1] = bad;
        submit(&mut fresh, 5, &path);
        assert_eq!(fresh.wheel.preset_feedback, message);
        assert_eq!(fresh.wheel.points, before.points);
    }
    std::fs::remove_file(path)?;
    Ok(())
}
