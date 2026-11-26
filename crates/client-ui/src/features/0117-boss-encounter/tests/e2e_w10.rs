//! W10 session-bound Hazard replay through `protocol::{request, apply_result}` and
//! `protocol/auxiliary.rs` (Hazard projection).
//!
//! Under `E2E_ALL_CAPTURE_DIR`, the keys are `initial`, `query`, `select`, `locked`,
//! `unchanged` and `relog`, in that order. Each key has four files:
//!
//! - `w10-<key>-request.bin` is the raw Hazard request (empty query or one level byte).
//! - `w10-<key>-status.bin` is exactly four bytes, the protocol-oteryn CommandStatus
//!   as a big-endian u32 (Accepted=1, Rejected=2, DuplicateReplay=3, DuplicateOutcomeExpired=4).
//! - `w10-<key>.bin` is the raw reply (unlocked byte then selected byte, or empty on refusal).
//! - `w10-<key>.status.json` is a JSON object with the writer's `session_id` string,
//!   positive u64 `command_id` and `server_sequence`, and numerical `status` matching
//!   the binary status. The first five probes share a session and relog uses a fresh one.
//!
//! `w10-expected.txt` is UTF-8 with four LF-terminated decimal key=value lines in this
//! order: `query_unlocked=2`, `query_selected=1`, `select_unlocked=2`, `select_selected=2`.

use super::*;
use oteryn_client_runtime::{
    boss_encounter::hazard,
    raw_feature::{CommandStatus, GatedCommandResult},
};
use std::{fs, path::Path};

type Error = Box<dyn std::error::Error>;

const INITIAL: &str = "00000000-0000-0000-0000-000000000001";
const RELOG: &str = "00000000-0000-0000-0000-000000000002";
const EXPECTED: &str = "query_unlocked=2\nquery_selected=1\nselect_unlocked=2\nselect_selected=2\n";

struct Probe {
    request: Vec<u8>,
    session_id: String,
    reply: GatedCommandResult,
}

impl Probe {
    fn load(root: &Path, key: &str) -> Result<Self, Error> {
        let prefix = format!("w10-{key}");
        let status = fs::read(root.join(format!("{prefix}-status.bin")))?;
        let status = u32::from_be_bytes(
            status
                .try_into()
                .map_err(|_| format!("{prefix}: expected four status bytes"))?,
        );
        let binding: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(format!("{prefix}.status.json")))?)?;
        if binding["status"].as_u64() != Some(u64::from(status)) {
            return Err(format!("{prefix}: mismatched capture statuses").into());
        }
        let status = match status {
            1 => CommandStatus::Accepted,
            2 => CommandStatus::Rejected,
            3 => CommandStatus::DuplicateReplay,
            4 => CommandStatus::DuplicateOutcomeExpired,
            _ => return Err(format!("{prefix}: unknown command status {status}").into()),
        };
        let positive = |field: &str| {
            binding[field]
                .as_u64()
                .filter(|value| *value > 0)
                .ok_or_else(|| format!("{prefix}: missing or invalid {field}"))
        };
        Ok(Self {
            request: fs::read(root.join(format!("{prefix}-request.bin")))?,
            session_id: binding["session_id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| format!("{prefix}: missing session_id"))?
                .into(),
            reply: GatedCommandResult {
                command_id: positive("command_id")?,
                server_sequence: positive("server_sequence")?,
                status,
                payload: fs::read(root.join(format!("{prefix}.bin")))?,
            },
        })
    }
}

fn replay(root: Option<&Path>) -> Result<(), Error> {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let mut session = String::new();
    let mut observed = String::new();
    for (index, (key, level, hazard_state)) in [
        ("initial", None, (1, 1)),
        ("query", None, (2, 1)),
        ("select", Some(2), (2, 2)),
        ("locked", Some(3), (2, 2)),
        ("unchanged", None, (2, 2)),
        ("relog", None, (2, 2)),
    ]
    .into_iter()
    .enumerate()
    {
        let refused = key == "locked";
        let status = if refused {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        let (unlocked, selected) = hazard_state;
        let expected_payload: &[u8] = if refused { &[] } else { &[unlocked, selected] };
        let probe = match root {
            Some(root) => Probe::load(root, key)?,
            None => fixture(index, level, status, expected_payload.to_vec()),
        };
        match key {
            "initial" => session.clone_from(&probe.session_id),
            "relog" => {
                assert_ne!(probe.session_id, session, "{key}");
                panel = BossEncounterPanel {
                    selected: true,
                    ..BossEncounterPanel::default()
                };
            }
            _ => assert_eq!(probe.session_id, session, "{key}"),
        }
        let selection = hazard::decode_request(&probe.request)?;
        assert_eq!(selection, level.map(u16::from), "{key}");
        let intent = match selection {
            Some(level) => {
                panel.fields[2] = level.to_string();
                Intent::HazardSelect
            }
            None => Intent::HazardQuery,
        };
        let confirmed = panel.summary.clone();
        let command = if refused {
            // The UI fences a known locked level. Simulate sending it without bounds,
            // then restore the confirmed state so the refusal must preserve it.
            assert!(request(&mut panel, intent).is_none());
            let bounds = panel.hazard.take();
            let command = request(&mut panel, intent);
            panel.hazard = bounds;
            command
        } else {
            request(&mut panel, intent)
        }
        .ok_or_else(|| format!("{key}: no Hazard request"))?;
        assert_eq!(command.capability, 117, "{key}");
        assert_eq!(command.kind, hazard::COMMAND_TYPE_HAZARD, "{key}");
        assert_eq!(command.payload, probe.request, "{key}");
        assert_eq!(probe.reply.status, status, "{key}");
        assert_eq!(probe.reply.payload, expected_payload, "{key}");
        assert_eq!(apply_result(&mut panel, &probe.reply), !refused, "{key}");
        assert_eq!(panel.hazard, Some(hazard_state), "{key}");
        assert!(!panel.pending, "{key}");
        if refused {
            assert_eq!(
                panel.feedback,
                Some("The server refused the boss status query")
            );
            assert_eq!(panel.summary, confirmed);
        } else {
            assert_eq!(panel.feedback, None, "{key}");
            assert_eq!(
                panel.summary,
                format!("Hazard: {selected} | Unlocked: {unlocked}"),
                "{key}"
            );
        }
        if matches!(key, "query" | "select") {
            use std::fmt::Write;
            let (unlocked, selected) = panel.hazard.ok_or("missing confirmed Hazard state")?;
            writeln!(
                observed,
                "{key}_unlocked={unlocked}\n{key}_selected={selected}"
            )?;
        }
    }
    let expected = match root {
        Some(root) => fs::read_to_string(root.join("w10-expected.txt"))?,
        None => EXPECTED.into(),
    };
    assert_eq!(observed, expected);
    Ok(())
}

#[test]
fn e2e_w10_rejects_hazard_reply_with_selected_above_unlocked() -> Result<(), Error> {
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    request(&mut panel, Intent::HazardQuery).ok_or("hazard query request")?;
    let valid = fixture(5, None, CommandStatus::Accepted, vec![2, 2]);
    assert!(apply_result(&mut panel, &valid.reply));
    let confirmed = panel.summary.clone();
    request(&mut panel, Intent::HazardQuery).ok_or("hazard query request")?;
    let invalid = fixture(6, None, CommandStatus::Accepted, vec![1, 2]);
    assert!(!apply_result(&mut panel, &invalid.reply));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss data")
    );
    assert_eq!(panel.summary, confirmed);
    assert_eq!(panel.hazard, Some((2, 2)));
    Ok(())
}

fn fixture(index: usize, level: Option<u8>, status: CommandStatus, payload: Vec<u8>) -> Probe {
    Probe {
        request: level.into_iter().collect(),
        session_id: if index < 5 { INITIAL } else { RELOG }.into(),
        reply: GatedCommandResult {
            command_id: 17 + index as u64,
            server_sequence: 304 + index as u64,
            status,
            payload,
        },
    }
}

#[test]
fn e2e_w10_replays_hazard_state_selection_and_locked_refusal() -> Result<(), Error> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    replay(root.as_deref().map(Path::new))
}

#[test]
fn e2e_w10_replays_session_bound_capture() -> Result<(), Error> {
    struct TempDir(std::path::PathBuf);
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!("w10-{}-{nonce}", std::process::id()));
    fs::create_dir(&root)?;
    let root = TempDir(root);
    for (index, (key, request, status, payload)) in [
        ("initial", &[][..], 1_u32, &[1, 1][..]),
        ("query", &[], 1, &[2, 1]),
        ("select", &[2], 1, &[2, 2]),
        ("locked", &[3], 2, &[]),
        ("unchanged", &[], 1, &[2, 2]),
        ("relog", &[], 1, &[2, 2]),
    ]
    .into_iter()
    .enumerate()
    {
        let prefix = format!("w10-{key}");
        let command_id = 0x101 + index as u64;
        let server_sequence = 0x10001 + 2 * index as u64;
        let session_id = if key == "relog" { RELOG } else { INITIAL };
        fs::write(root.0.join(format!("{prefix}-request.bin")), request)?;
        fs::write(
            root.0.join(format!("{prefix}-status.bin")),
            status.to_be_bytes(),
        )?;
        fs::write(root.0.join(format!("{prefix}.bin")), payload)?;
        fs::write(
            root.0.join(format!("{prefix}.status.json")),
            serde_json::to_vec(&serde_json::json!({
                "session_id": session_id,
                "command_id": command_id,
                "server_sequence": server_sequence,
                "status": status,
            }))?,
        )?;
        let probe = Probe::load(&root.0, key)?;
        assert_eq!(probe.reply.command_id, command_id);
        assert_eq!(probe.reply.server_sequence, server_sequence);
    }
    fs::write(root.0.join("w10-expected.txt"), EXPECTED)?;
    replay(Some(&root.0))
}
