//! I07 server-payload replay through the owning client projection.
//!
//! E2E_ALL_CAPTURE_DIR contains four probes keyed 1 (skip filter add), 2 (accept filter
//! add), 3 (Gold Pouch assignment), and 4 (loot all). For each key, `i07-<key>-request.bin`
//! holds the exact raw quick-loot request payload sent to the server,
//! `i07-<key>-status.bin` holds the matching reply's status as one big-endian u32
//! (four bytes), and `i07-<key>.bin` holds the raw reply payload, which may be empty.
//! `i07-expected.txt` holds the UTF-8 line `skip=<id> accept=<id> confirmed=<count> results=<count>`
//! followed by a newline.
//!
//! The payload consumer is `apply_result` in
//! `crates/client-ui/src/features/0098-quick-loot/protocol.rs`; it decodes each
//! `QuickLootResult` and updates `Panel` from
//! `crates/client-ui/src/features/0098-quick-loot/state.rs`.
use super::*;
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use std::path::{Path, PathBuf};

struct Probe {
    request: Vec<u8>,
    status: CommandStatus,
    reply: Vec<u8>,
}

fn fixture(skip: u32, accept: u32) -> Result<Vec<Probe>, Box<dyn std::error::Error>> {
    let loot_all = QuickLootRequest {
        action: QuickLootAction::LootAll,
        corpse: None,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    };
    let add = |mode, definition| QuickLootRequest {
        action: QuickLootAction::FilterAdd,
        filter_mode: Some(mode),
        filter_definition_ref: Some(definition),
        ..loot_all
    };
    let pouch = QuickLootRequest {
        action: QuickLootAction::Assign,
        category: Some(QuickLootCategory::Gold),
        recipient: Some(QuickLootRecipient::GoldPouch),
        recipient_id: Some([7; 16]),
        fallback_to_main: true,
        ..loot_all
    };
    let payload = encode_quick_loot_result(&QuickLootResult { items: Vec::new() })
        .map_err(|error| format!("{error:?}"))?;
    [
        (
            add(QuickLootFilterMode::Skip, skip),
            CommandStatus::Accepted,
            payload.clone(),
        ),
        (
            add(QuickLootFilterMode::Accept, accept),
            CommandStatus::Accepted,
            payload.clone(),
        ),
        (pouch, CommandStatus::Rejected, Vec::new()),
        (loot_all, CommandStatus::Accepted, payload),
    ]
    .into_iter()
    .map(|(request, status, reply)| {
        Ok(Probe {
            request: encode_quick_loot_request(&request).map_err(|error| format!("{error:?}"))?,
            status,
            reply,
        })
    })
    .collect()
}

fn read_probe(path: &Path, key: usize) -> Result<Probe, Box<dyn std::error::Error>> {
    let request = std::fs::read(path.join(format!("i07-{key}-request.bin")))?;
    let bytes = std::fs::read(path.join(format!("i07-{key}-status.bin")))?;
    let code = u32::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| format!("I07 probe {key}: expected a four-byte status"))?,
    );
    let status = [
        CommandStatus::Accepted,
        CommandStatus::Rejected,
        CommandStatus::DuplicateReplay,
        CommandStatus::DuplicateOutcomeExpired,
    ]
    .into_iter()
    .find(|status| *status as u32 == code)
    .ok_or_else(|| format!("I07 probe {key}: unknown reply status {code}"))?;
    let reply = std::fs::read(path.join(format!("i07-{key}.bin")))?;
    Ok(Probe {
        request,
        status,
        reply,
    })
}

#[test]
fn replays_i07_quick_loot_results_and_rejects_invalid_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    replay(path.as_deref().map(Path::new))
}

fn replay(path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let (probes, expected) = if let Some(path) = path {
        (
            (1..=4)
                .map(|key| read_probe(path, key))
                .collect::<Result<Vec<_>, _>>()?,
            std::fs::read_to_string(path.join("i07-expected.txt"))?,
        )
    } else {
        (
            fixture(3031, 3350)?,
            "skip=3031 accept=3350 confirmed=2 results=0\n".into(),
        )
    };
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let mut expected_confirmed = Vec::new();
    for (key, probe) in (1..=4).zip(probes) {
        let wire = decode_quick_loot_request(&probe.request)
            .map_err(|error| format!("I07 probe {key}: invalid request: {error:?}"))?;
        let action = match (key, wire.action, wire.filter_mode) {
            (1, QuickLootAction::FilterAdd, Some(mode @ QuickLootFilterMode::Skip))
            | (2, QuickLootAction::FilterAdd, Some(mode @ QuickLootFilterMode::Accept)) => {
                let definition = wire
                    .filter_definition_ref
                    .ok_or("I07 filter definition absent")?;
                expected_confirmed.push(Action::Add { mode, definition });
                Action::Add { mode, definition }
            }
            (3, QuickLootAction::Assign, None) => {
                assert_eq!(wire.category, Some(QuickLootCategory::Gold));
                assert_eq!(wire.recipient, Some(QuickLootRecipient::GoldPouch));
                assert!(wire.fallback_to_main);
                Action::Destination {
                    category: wire.category,
                    recipient: QuickLootRecipient::GoldPouch,
                    id: wire.recipient_id,
                    fallback: wire.fallback_to_main,
                }
            }
            (4, QuickLootAction::LootAll, None) => Action::LootAll,
            _ => return Err(format!("I07 probe {key}: unexpected request {wire:?}").into()),
        };
        let expected_status = if wire.action == QuickLootAction::Assign {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        assert_eq!(probe.status, expected_status, "I07 probe {key}");
        let command = request(&mut panel, action)
            .ok_or_else(|| format!("I07 probe {key}: client did not queue request"))?;
        assert_eq!(command.payload, probe.request, "I07 probe {key}");
        let looted = apply_result(
            &mut panel,
            &ServerMechanicResponse {
                command_id: 1,
                status: probe.status,
                server_sequence: 1,
                payload: probe.reply,
            },
        );
        assert_eq!(
            looted,
            wire.action == QuickLootAction::LootAll,
            "I07 probe {key}"
        );
    }
    assert_eq!(panel.confirmed, expected_confirmed);
    assert_eq!(panel.skipped.len(), 1);
    assert_eq!(panel.accepted.len(), 1);
    assert!(panel.destinations.is_empty());
    let result_count = panel.result.as_ref().map_or(0, |result| result.items.len());
    let visible = format!(
        "skip={} accept={} confirmed={} results={}\n",
        panel.skipped.iter().next().copied().unwrap_or_default(),
        panel.accepted.iter().next().copied().unwrap_or_default(),
        panel.confirmed.len(),
        result_count,
    );
    assert_eq!(visible, expected);

    let before = panel.clone();
    assert!(request(&mut panel, Action::LootAll).is_some());
    let stable = panel.clone();
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: vec![0xff],
        },
    );
    assert_eq!(panel.result, stable.result);
    assert_eq!(panel.skipped, stable.skipped);
    assert_eq!(panel.accepted, stable.accepted);
    assert_eq!(panel.confirmed, stable.confirmed);
    assert_eq!(
        panel.feedback,
        "Loot request refused or invalid reply; try again"
    );
    assert_eq!(before.result, panel.result);
    Ok(())
}

struct TempCapture(PathBuf);

impl Drop for TempCapture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn replays_i07_capture_with_same_run_filter_definitions() -> Result<(), Box<dyn std::error::Error>>
{
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("oteryn-i07-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&path)?;
    let capture = TempCapture(path);

    for (key, probe) in (1..=4).zip(fixture(4097, 4098)?) {
        std::fs::write(
            capture.0.join(format!("i07-{key}-request.bin")),
            probe.request,
        )?;
        std::fs::write(
            capture.0.join(format!("i07-{key}-status.bin")),
            (probe.status as u32).to_be_bytes(),
        )?;
        std::fs::write(capture.0.join(format!("i07-{key}.bin")), probe.reply)?;
    }
    std::fs::write(
        capture.0.join("i07-expected.txt"),
        "skip=4097 accept=4098 confirmed=2 results=0\n",
    )?;
    replay(Some(&capture.0))
}
