//! I07 server-payload replay through the owning client projection.
//!
//! E2E_ALL_CAPTURE_DIR contains four probes keyed 1 (skip filter add), 2 (accept filter
//! add), 3 (Gold Pouch assignment to the character's own pouch), and 4 (loot all). For each
//! key, `i07-<key>-request.bin` holds the exact raw quick-loot request payload sent to the
//! server and `i07-<key>.json` the reply envelope: `game_session_id` (hyphenated UUID),
//! positive `command_id` and `server_sequence`, numeric `status` (Accepted=1, Rejected=2,
//! DuplicateReplay=3, DuplicateOutcomeExpired=4) and `payload` (byte array, may be empty).
//! Every probe is Accepted, shares one session and has strictly increasing command IDs and
//! server sequences.
//! `i07-expected.txt` holds three LF-terminated UTF-8 lines: `session=<uuid>`,
//! `pouch=<32 lowercase hex digits>` (the Gold Pouch item instance the server accepted) and
//! `skip=<id> accept=<id> confirmed=3 destinations=1 results=<count>`.
//! A replay of a capture prints `I07_NATIVE_REPLAY session=<uuid>` after every assertion
//! passes; without a capture directory the test runs on a synthetic fixture and prints nothing.
//!
//! The payload consumer is `apply_result` in
//! `crates/client-ui/src/features/0098-quick-loot/protocol.rs`; it decodes each
//! `QuickLootResult` and updates `Panel` from
//! `crates/client-ui/src/features/0098-quick-loot/state.rs`.
use super::*;
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use std::path::Path;

type Error = Box<dyn std::error::Error>;

const FIXTURE_SESSION: &str = "00000000-0000-7000-8000-000000000007";
const FIXTURE_POUCH: [u8; 16] = [0x5a; 16];

struct Probe {
    request: Vec<u8>,
    session: String,
    response: ServerMechanicResponse,
}

struct Capture {
    probes: Vec<Probe>,
    session: String,
    pouch: [u8; 16],
    summary: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A hyphenated UUIDv7 with the RFC variant, as the production `GameSessionId` decoder requires.
fn is_uuid(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(part, width)| part.len() == width && part.bytes().all(|b| b.is_ascii_hexdigit()))
        && parts[2].starts_with('7')
        && parts[3].starts_with(['8', '9', 'a', 'b', 'A', 'B'])
}

/// Envelope keys the capture writer emits, sorted.
const ENVELOPE_KEYS: [&str; 5] = [
    "command_id",
    "game_session_id",
    "payload",
    "server_sequence",
    "status",
];

/// Names of the top-level object's keys in `text`, in order and with repeats:
/// `serde_json::Value` keeps only the last of repeated keys, so a repeated `status` or session
/// would otherwise mask the first. Call only on text that already parsed as JSON.
fn top_level_keys(text: &str) -> Result<Vec<String>, Error> {
    let mut keys = Vec::new();
    let mut depth = 0usize;
    let mut chars = text.char_indices();
    while let Some((start, c)) = chars.next() {
        match c {
            '{' | '[' => depth += 1,
            '}' | ']' => depth = depth.saturating_sub(1),
            '"' => {
                let mut end = start;
                let mut escaped = false;
                for (index, c) in chars.by_ref() {
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        end = index;
                        break;
                    }
                }
                if depth == 1 && text[end + 1..].trim_start().starts_with(':') {
                    keys.push(serde_json::from_str(&text[start..=end])?);
                }
            }
            _ => {}
        }
    }
    Ok(keys)
}

/// Parses one reply envelope and fails closed unless it has exactly the writer's keys, once each.
fn envelope_value(raw: &[u8], name: &str) -> Result<serde_json::Value, Error> {
    let value: serde_json::Value = serde_json::from_slice(raw)?;
    let mut keys = top_level_keys(std::str::from_utf8(raw)?)?;
    keys.sort();
    if keys != ENVELOPE_KEYS {
        return Err(
            format!("{name}: envelope keys {keys:?} are not exactly {ENVELOPE_KEYS:?}").into(),
        );
    }
    Ok(value)
}

fn status(code: u64) -> Option<CommandStatus> {
    [
        CommandStatus::Accepted,
        CommandStatus::Rejected,
        CommandStatus::DuplicateReplay,
        CommandStatus::DuplicateOutcomeExpired,
    ]
    .into_iter()
    .find(|status| *status as u64 == code)
}

fn fixture(skip: u32, accept: u32) -> Result<Capture, Error> {
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
        recipient_id: Some(FIXTURE_POUCH),
        fallback_to_main: true,
        ..loot_all
    };
    let payload = encode_quick_loot_result(&QuickLootResult { items: Vec::new() })
        .map_err(|error| format!("{error:?}"))?;
    let probes = [
        add(QuickLootFilterMode::Skip, skip),
        add(QuickLootFilterMode::Accept, accept),
        pouch,
        loot_all,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, request)| {
        Ok(Probe {
            request: encode_quick_loot_request(&request).map_err(|error| format!("{error:?}"))?,
            session: FIXTURE_SESSION.into(),
            response: ServerMechanicResponse {
                command_id: 0x31 + index as u64,
                status: CommandStatus::Accepted,
                server_sequence: 0x700 + 3 * index as u64,
                payload: payload.clone(),
            },
        })
    })
    .collect::<Result<Vec<_>, Error>>()?;
    Ok(Capture {
        probes,
        session: FIXTURE_SESSION.into(),
        pouch: FIXTURE_POUCH,
        summary: format!("skip={skip} accept={accept} confirmed=3 destinations=1 results=0"),
    })
}

fn read_probe(path: &Path, key: usize) -> Result<Probe, Error> {
    let request = std::fs::read(path.join(format!("i07-{key}-request.bin")))?;
    let value = envelope_value(
        &std::fs::read(path.join(format!("i07-{key}.json")))?,
        &format!("I07 probe {key}"),
    )?;
    let positive = |field: &str| -> Result<u64, Error> {
        value[field]
            .as_u64()
            .filter(|number| *number > 0)
            .ok_or_else(|| format!("I07 probe {key}: {field} is not a positive integer").into())
    };
    let session = value["game_session_id"]
        .as_str()
        .filter(|id| is_uuid(id))
        .ok_or_else(|| format!("I07 probe {key}: game_session_id is not a UUID"))?
        .to_owned();
    let payload = value["payload"]
        .as_array()
        .ok_or_else(|| format!("I07 probe {key}: payload is not a byte array"))?
        .iter()
        .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("I07 probe {key}: payload is not a byte array"))?;
    Ok(Probe {
        request,
        session,
        response: ServerMechanicResponse {
            command_id: positive("command_id")?,
            status: value["status"]
                .as_u64()
                .and_then(status)
                .ok_or_else(|| format!("I07 probe {key}: unknown reply status"))?,
            server_sequence: positive("server_sequence")?,
            payload,
        },
    })
}

/// Loads a capture and fails closed unless every probe is an Accepted reply of the session
/// named by `i07-expected.txt`, in strictly increasing command and sequence order.
fn read_capture(path: &Path) -> Result<Capture, Error> {
    let expected = std::fs::read_to_string(path.join("i07-expected.txt"))?;
    let mut lines = expected.split_terminator('\n');
    let session = lines
        .next()
        .and_then(|line| line.strip_prefix("session="))
        .filter(|id| is_uuid(id))
        .ok_or("i07-expected.txt: first line is not session=<uuid>")?
        .to_owned();
    let pouch_hex = lines
        .next()
        .and_then(|line| line.strip_prefix("pouch="))
        .filter(|id| id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        .ok_or("i07-expected.txt: second line is not pouch=<32 hex>")?;
    let mut pouch = [0; 16];
    for (index, byte) in pouch.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&pouch_hex[2 * index..2 * index + 2], 16)?;
    }
    if pouch == [0; 16] {
        return Err("i07-expected.txt: pouch is the nil identity".into());
    }
    let summary = lines
        .next()
        .ok_or("i07-expected.txt: summary line missing")?
        .to_owned();
    if lines.next().is_some() || !expected.ends_with('\n') {
        return Err("i07-expected.txt: expected exactly three LF-terminated lines".into());
    }
    let probes = (1..=4)
        .map(|key| read_probe(path, key))
        .collect::<Result<Vec<_>, _>>()?;
    let mut previous = (0, 0);
    for (key, probe) in (1..=4).zip(&probes) {
        if probe.session != session {
            return Err(format!(
                "I07 probe {key}: session {} is not {session}",
                probe.session
            )
            .into());
        }
        if probe.response.status != CommandStatus::Accepted {
            return Err(format!(
                "I07 probe {key}: status {:?} is not Accepted",
                probe.response.status
            )
            .into());
        }
        let current = (probe.response.command_id, probe.response.server_sequence);
        if current.0 <= previous.0 || current.1 <= previous.1 {
            return Err(format!(
                "I07 probe {key}: {current:?} does not increase after {previous:?}"
            )
            .into());
        }
        previous = current;
    }
    Ok(Capture {
        probes,
        session,
        pouch,
        summary,
    })
}

#[test]
fn replays_i07_quick_loot_results_and_rejects_invalid_payload() -> Result<(), Error> {
    match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        Some(path) => {
            let capture = read_capture(Path::new(&path))?;
            replay(&capture)?;
            println!("I07_NATIVE_REPLAY session={}", capture.session);
            Ok(())
        }
        None => replay(&fixture(3031, 3350)?),
    }
}

fn replay(capture: &Capture) -> Result<(), Error> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let mut expected_confirmed = Vec::new();
    for (key, probe) in (1..=4).zip(&capture.probes) {
        let wire = decode_quick_loot_request(&probe.request)
            .map_err(|error| format!("I07 probe {key}: invalid request: {error:?}"))?;
        let action = match (key, wire.action, wire.filter_mode) {
            (1, QuickLootAction::FilterAdd, Some(mode @ QuickLootFilterMode::Skip))
            | (2, QuickLootAction::FilterAdd, Some(mode @ QuickLootFilterMode::Accept)) => {
                let definition = wire
                    .filter_definition_ref
                    .ok_or("I07 filter definition absent")?;
                Action::Add { mode, definition }
            }
            (3, QuickLootAction::Assign, None) => {
                assert_eq!(wire.category, Some(QuickLootCategory::Gold));
                assert_eq!(wire.recipient, Some(QuickLootRecipient::GoldPouch));
                assert_eq!(wire.recipient_id, Some(capture.pouch), "I07 pouch identity");
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
        if key != 4 {
            expected_confirmed.push(action.clone());
        }
        let command = request(&mut panel, action)
            .ok_or_else(|| format!("I07 probe {key}: client did not queue request"))?;
        assert_eq!(command.payload, probe.request, "I07 probe {key}");
        assert_eq!(
            probe.response.status,
            CommandStatus::Accepted,
            "I07 probe {key}"
        );
        let looted = apply_result(&mut panel, &probe.response);
        assert_eq!(panel.pending, None, "I07 probe {key}");
        assert_eq!(panel.feedback, "Confirmed by the server", "I07 probe {key}");
        assert_eq!(looted, key == 4, "I07 probe {key}");
    }
    assert_eq!(panel.confirmed, expected_confirmed);
    assert_eq!(panel.skipped.len(), 1);
    assert_eq!(panel.accepted.len(), 1);
    assert_eq!(panel.destinations, expected_confirmed[2..]);
    let result_count = panel.result.as_ref().map_or(0, |result| result.items.len());
    let visible = format!(
        "skip={} accept={} confirmed={} destinations={} results={}",
        panel.skipped.iter().next().copied().unwrap_or_default(),
        panel.accepted.iter().next().copied().unwrap_or_default(),
        panel.confirmed.len(),
        panel.destinations.len(),
        result_count,
    );
    assert_eq!(visible, capture.summary);

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
    assert_eq!(panel.destinations, stable.destinations);
    assert_eq!(
        panel.feedback,
        "Loot request refused or invalid reply; try again"
    );
    assert_eq!(before.result, panel.result);
    Ok(())
}

struct TempCapture(std::path::PathBuf);

impl TempCapture {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempCapture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_capture(capture: &Capture) -> Result<TempCapture, Error> {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "oteryn-i07-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&path)?;
    let root = TempCapture(path);
    for (key, probe) in (1..=4).zip(&capture.probes) {
        std::fs::write(
            root.path().join(format!("i07-{key}-request.bin")),
            &probe.request,
        )?;
        std::fs::write(
            root.path().join(format!("i07-{key}.json")),
            serde_json::to_vec(&serde_json::json!({
                "game_session_id": probe.session,
                "command_id": probe.response.command_id,
                "server_sequence": probe.response.server_sequence,
                "status": probe.response.status as u64,
                "payload": probe.response.payload,
            }))?,
        )?;
    }
    std::fs::write(
        root.path().join("i07-expected.txt"),
        format!(
            "session={}\npouch={}\n{}\n",
            capture.session,
            hex(&capture.pouch),
            capture.summary
        ),
    )?;
    Ok(root)
}

#[test]
fn replays_i07_capture_with_same_run_filter_definitions() -> Result<(), Error> {
    let root = write_capture(&fixture(4097, 4098)?)?;
    let capture = read_capture(root.path())?;
    assert_eq!(capture.session, FIXTURE_SESSION);
    assert_eq!(capture.pouch, FIXTURE_POUCH);
    replay(&capture)
}

#[test]
fn i07_capture_without_current_run_provenance_fails_closed() -> Result<(), Error> {
    type Mutation = fn(&mut Capture);
    let cases: [(&str, Mutation); 7] = [
        ("refused pouch", |c| {
            c.probes[2].response.status = CommandStatus::Rejected
        }),
        ("foreign session", |c| {
            c.probes[3].session = "00000000-0000-7000-8000-000000000008".into()
        }),
        ("non-UUID session", |c| c.session = "run-7".into()),
        ("UUIDv4 session", |c| {
            c.session = "00000000-0000-4000-8000-000000000007".into();
            for probe in &mut c.probes {
                probe.session = c.session.clone();
            }
        }),
        ("repeated command_id", |c| {
            c.probes[1].response.command_id = c.probes[0].response.command_id
        }),
        ("repeated server_sequence", |c| {
            c.probes[3].response.server_sequence = c.probes[2].response.server_sequence;
        }),
        ("nil pouch", |c| c.pouch = [0; 16]),
    ];
    for (case, mutate) in cases {
        let mut capture = fixture(4097, 4098)?;
        mutate(&mut capture);
        let root = write_capture(&capture)?;
        assert!(read_capture(root.path()).is_err(), "{case} was accepted");
    }
    let root = write_capture(&fixture(4097, 4098)?)?;
    let path = root.path().join("i07-3.json");
    let valid = std::fs::read_to_string(&path)?;
    for (case, edited) in [
        ("repeated status", valid.replacen('{', r#"{"status":2,"#, 1)),
        (
            "repeated session",
            valid.replacen('{', r#"{"game_session_id":"x","#, 1),
        ),
        ("unknown key", valid.replacen('{', r#"{"note":1,"#, 1)),
    ] {
        std::fs::write(&path, edited)?;
        assert!(read_capture(root.path()).is_err(), "{case} was accepted");
    }
    std::fs::write(&path, valid)?;
    read_capture(root.path())?;
    std::fs::remove_file(&path)?;
    assert!(
        read_capture(root.path()).is_err(),
        "missing envelope was accepted"
    );
    Ok(())
}

#[test]
fn i07_replay_rejects_an_assignment_to_another_pouch() -> Result<(), Error> {
    let mut capture = fixture(4097, 4098)?;
    capture.pouch = [0x5b; 16];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| replay(&capture)));
    assert!(!matches!(result, Ok(Ok(()))), "another pouch was accepted");
    Ok(())
}

#[test]
fn i07_refused_pouch_assignment_keeps_destinations() -> Result<(), Error> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let action = Action::Destination {
        category: Some(QuickLootCategory::Gold),
        recipient: QuickLootRecipient::GoldPouch,
        id: Some(FIXTURE_POUCH),
        fallback: true,
    };
    request(&mut panel, action).ok_or("pouch request")?;
    assert!(!apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Rejected,
            server_sequence: 1,
            payload: Vec::new(),
        },
    ));
    assert!(panel.destinations.is_empty());
    assert!(panel.confirmed.is_empty());
    assert_eq!(
        panel.feedback,
        "Loot request refused or invalid reply; try again"
    );
    Ok(())
}
