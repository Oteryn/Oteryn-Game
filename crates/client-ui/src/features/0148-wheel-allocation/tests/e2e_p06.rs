//! P06 replay: server WHEEL_ALLOCATION replies (current-state query, then a stale-revision Save)
//! with their current Wheel/Atelier frames, applied through the production `request` and
//! `apply_result`; the visible text comes from `panel_text`.
use super::*;

type Error = Box<dyn std::error::Error>;
const CORE_SLOT: usize = 20;

struct Frames {
    receipt: Vec<u8>,
    wheel: Vec<u8>,
    atelier: Vec<u8>,
    resources: Option<Vec<u8>>,
}

struct Captured {
    query: Frames,
    stale: Frames,
    expected: String,
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

fn atelier_frame(revision: u64) -> Result<Vec<u8>, Error> {
    wire(atelier::encode_reply(&atelier::AtelierReply {
        refused: false,
        revision,
        gems: Vec::new(),
        grades: Vec::new(),
        quality_quotes: Vec::new(),
        grade_quotes: Vec::new(),
    }))
}

fn wheel_frame(refused: bool, revision: u64) -> Result<Vec<u8>, Error> {
    wire(wheel::encode_wheel_reply(&wheel::WheelAllocationReply {
        refused,
        revision,
        points: [0; 36],
        vessels: None,
    }))
}

fn fixture() -> Result<Captured, Error> {
    Ok(Captured {
        query: Frames {
            receipt: wheel_frame(false, 4)?,
            wheel: wheel_frame(false, 4)?,
            atelier: atelier_frame(4)?,
            resources: None,
        },
        stale: Frames {
            receipt: wheel_frame(true, 4)?,
            wheel: wheel_frame(false, 4)?,
            atelier: atelier_frame(4)?,
            resources: None,
        },
        expected:
            "revision=4\nallocated=0\ncore_slot=0\nstale_refused=true\nstale_current_revision=4\n"
                .into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    let frames = |name: &str| -> Result<Frames, Error> {
        Ok(Frames {
            receipt: std::fs::read(path.join(format!("p06-{name}-receipt.bin")))?,
            wheel: std::fs::read(path.join(format!("p06-{name}-wheel.bin")))?,
            atelier: std::fs::read(path.join(format!("p06-{name}-atelier.bin")))?,
            resources: std::fs::read(path.join(format!("p06-{name}-resources.bin"))).ok(),
        })
    };
    Ok(Captured {
        query: frames("query")?,
        stale: frames("stale")?,
        expected: std::fs::read_to_string(path.join("p06-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn response(kind: u32, frames: &Frames) -> WheelResponse {
    WheelResponse {
        kind,
        status: CommandStatus::Accepted,
        payload: frames.receipt.clone(),
        wheel: Some(frames.wheel.clone()),
        atelier: Some(frames.atelier.clone()),
        resources: frames.resources.clone(),
    }
}

#[test]
fn e2e_p06_replays_query_and_stale_revision_save() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let revision: u64 = value(e, "revision")?.parse()?;
    let allocated: u32 = value(e, "allocated")?.parse()?;
    let mut panel = WheelPanel {
        selected: true,
        ..WheelPanel::default()
    };
    let refresh = request(&mut panel, Intent::Refresh, occurrence(1)).ok_or("refresh")?;
    assert_eq!(
        (refresh.kind, refresh.payload.as_slice()),
        (161, &[8, 0][..])
    );
    assert!(apply_result(&mut panel, &response(161, &captured.query)));
    assert!(panel.ready());
    assert_eq!(
        panel.confirmed_wheel.as_ref().map(|w| w.revision),
        Some(revision)
    );
    assert_eq!(panel.points[CORE_SLOT].to_string(), value(e, "core_slot")?);
    let line = format!("Allocated points: {allocated}. Edited points: {allocated}.");
    assert!(panel_text(&panel).split("\r\n").any(|l| l == line));

    // Editing then saving sends the confirmed revision; the server answers a stale refusal.
    assert!(panel.set_points(CORE_SLOT, 25));
    let edited = format!(
        "Allocated points: {allocated}. Edited points: {} (not saved).",
        allocated
            - u32::from(
                panel
                    .confirmed_wheel
                    .as_ref()
                    .map_or(0, |w| w.points[CORE_SLOT])
            )
            + 25
    );
    assert!(panel_text(&panel).split("\r\n").any(|l| l == edited));
    let save = request(&mut panel, Intent::Save, occurrence(2)).ok_or("save")?;
    let sent = wire(wheel::decode_wheel_request(&save.payload))?;
    assert_eq!(
        (sent.action, sent.expected_revision, sent.points[CORE_SLOT]),
        (1, revision, 25)
    );
    assert!(apply_result(&mut panel, &response(161, &captured.stale)));
    assert_eq!(
        panel.feedback,
        Some("The server refused the change; check Premium, points, location and resources")
    );
    assert_eq!(value(e, "stale_refused")?, "true");
    assert_eq!(
        panel
            .confirmed_wheel
            .as_ref()
            .map(|w| w.revision.to_string())
            .as_deref(),
        Some(value(e, "stale_current_revision")?)
    );
    assert!(!panel.dirty());
    assert_eq!(panel.points[CORE_SLOT].to_string(), value(e, "core_slot")?);
    assert!(panel_text(&panel).split("\r\n").any(|l| l == line));

    // A refresh whose current Wheel frame is garbage leaves the confirmed state and blocks edits.
    let confirmed = panel.confirmed_wheel.clone();
    request(&mut panel, Intent::Refresh, occurrence(3)).ok_or("refresh")?;
    let mut broken = response(161, &captured.query);
    broken.wheel = Some(vec![0xff, 0xff]);
    assert!(!apply_result(&mut panel, &broken));
    assert_eq!(
        panel.feedback,
        Some("The current view is unavailable; refresh before editing")
    );
    assert_eq!(panel.confirmed_wheel, confirmed);
    assert!(!panel.synchronized);
    Ok(())
}
