//! S04a replay: the VIP list replies for Add, the edited/grouped Query and the duplicate-Add
//! refusal applied through the production `request`, `apply_result` and `panel_text`. The reply
//! bytes are decoded by `vip::decode_result`, exactly as the runtime `send` does.
use super::*;
use oteryn_client_runtime::vip::CommandStatus;
use sha2::{Digest, Sha256};

type Error = Box<dyn std::error::Error>;

struct Replay {
    added: Vec<u8>,
    listed: Vec<u8>,
    duplicate: Vec<u8>,
    expected: String,
}

fn payload_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn verify_payload(expected: &str, label: &str, bytes: &[u8]) -> Result<(), Error> {
    let digest = payload_digest(bytes);
    if value(expected, &format!("{label}_sha256"))? != digest {
        return Err(format!("S04a {label} payload does not match its live manifest").into());
    }
    Ok(())
}

fn load() -> Result<Replay, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("s04a-expected.txt"))?;
        if std::env::var_os("OTERYN_S04A_CAPTURE_REQUIRED").is_some()
            && std::env::var("OTERYN_S04A_LIVE_MANIFEST")? != expected.replace('\n', ";")
        {
            return Err("S04a manifest does not match the live capture evidence".into());
        }
        let read = |label: &str| -> Result<Vec<u8>, Error> {
            let bytes = std::fs::read(root.join(format!("s04a-{label}.bin")))?;
            verify_payload(&expected, label, &bytes)?;
            Ok(bytes)
        };
        return Ok(Replay {
            added: read("added")?,
            listed: read("listed")?,
            duplicate: read("duplicate")?,
            expected,
        });
    }
    if std::env::var_os("OTERYN_S04A_CAPTURE_REQUIRED").is_some() {
        return Err("missing live S04a capture directory".into());
    }
    let view = "{\"entries\":[{\"entry\":4,\"name\":\"Berenike\",\"description\":\"node-boot friend\",\"icon\":7,\"notify\":true,\"groups\":[2]}],\"groups\":[{\"group\":2,\"name\":\"Raid Team\"}]}";
    let encode = |code, revision, number, view: &str| {
        vip::encode_result(code, revision, number, 0, view).map_err(|e| format!("{e:?}"))
    };
    Ok(Replay {
        added: encode(1, 5, 4, "")?,
        listed: encode(1, 8, 0, view)?,
        duplicate: encode(3, 9, 0, "")?,
        expected:
            "name=Berenike\nentry=4\ngroup=2\nrevision=8\nadded_revision=5\nduplicate_code=3\n"
                .into(),
    })
}

fn response(bytes: &[u8]) -> Result<VipResponse, Error> {
    Ok(VipResponse {
        status: CommandStatus::Accepted,
        result: vip::decode_result(bytes).map_err(|e| format!("{e:?}"))?,
    })
}

fn only_item<'a>(view: &'a serde_json::Value, field: &str) -> Result<&'a serde_json::Value, Error> {
    match view[field].as_array().map(Vec::as_slice) {
        Some([item]) => Ok(item),
        _ => Err(format!("S04a replay requires exactly one item in {field}").into()),
    }
}

fn check_first_page_query(command: &VipCommand) -> Result<(), Error> {
    let decoded = vip::decode_request(command.kind, &command.payload)
        .map_err(|error| format!("VIP Query decode: {error:?}"))?;
    if decoded.operation != vip::Operation::Query || decoded.number != 0 {
        return Err("S04a replay requires a VIP Query with cursor 0".into());
    }
    Ok(())
}

fn check_group_encoder(intent: Intent, expected: vip::Request) -> Result<(), Error> {
    let mut panel = VipPanel {
        selected: true,
        ..VipPanel::default()
    };
    let command = request(&mut panel, intent).ok_or("VIP intent did not encode")?;
    let (kind, bytes) = vip::encode_request(&expected).map_err(|error| format!("{error:?}"))?;
    if (command.kind, command.payload.as_slice()) != (kind, bytes.as_slice()) {
        return Err(format!("VIP encoder mismatch for {:?}", expected.operation).into());
    }
    Ok(())
}

#[test]
fn vip_replay_add_edit_query_and_duplicate_refusal() -> Result<(), Error> {
    let replay = load()?;
    let name = value(&replay.expected, "name")?;
    let entry: u64 = value(&replay.expected, "entry")?.parse()?;
    let group: u64 = value(&replay.expected, "group")?.parse()?;
    let revision: u64 = value(&replay.expected, "revision")?.parse()?;
    let added_revision: u64 = value(&replay.expected, "added_revision")?.parse()?;
    let group_number = u8::try_from(group)?;
    for (intent, operation, number, text) in [
        (
            Intent::GroupAdd("Raid Friends".into()),
            vip::Operation::GroupAdd,
            0,
            "Raid Friends",
        ),
        (
            Intent::GroupRename(group, "Raid Team".into()),
            vip::Operation::GroupRename,
            group,
            "Raid Team",
        ),
    ] {
        check_group_encoder(
            intent,
            vip::Request {
                operation,
                number,
                text: text.into(),
                icon: 0,
                notify: false,
                groups: vec![],
            },
        )?;
    }
    check_group_encoder(
        Intent::Edit {
            number: entry,
            text: "node-boot friend".into(),
            icon: 7,
            notify: true,
            groups: vec![group_number],
        },
        vip::Request {
            operation: vip::Operation::Edit,
            number: entry,
            text: "node-boot friend".into(),
            icon: 7,
            notify: true,
            groups: vec![group_number],
        },
    )?;
    let mut panel = VipPanel {
        selected: true,
        ..VipPanel::default()
    };

    let add = request(&mut panel, Intent::Add(name.into())).ok_or("Add request refused")?;
    assert_eq!(
        vip::decode_request(add.kind, &add.payload).map_err(|e| format!("{e:?}"))?,
        vip::Request {
            operation: vip::Operation::Add,
            number: 0,
            text: name.into(),
            icon: 0,
            notify: false,
            groups: vec![],
        }
    );
    apply_result(&mut panel, &response(&replay.added)?);
    assert_eq!(
        (panel.revision, panel.feedback.clone()),
        (added_revision, None)
    );
    assert!(!panel.pending);

    let query = request(&mut panel, Intent::Query(0)).ok_or("Query request refused")?;
    check_first_page_query(&query)?;
    apply_result(&mut panel, &response(&replay.listed)?);
    assert_eq!((panel.revision, panel.feedback.clone()), (revision, None));
    let view: serde_json::Value =
        serde_json::from_str(panel.view.as_deref().ok_or("view absent")?)?;
    let item = only_item(&view, "entries")?;
    assert_eq!(item["entry"].as_u64(), Some(entry));
    assert_eq!(item["name"].as_str(), Some(name));
    assert_eq!(item["description"].as_str(), Some("node-boot friend"));
    assert_eq!(item["icon"].as_u64(), Some(7));
    assert_eq!(item["notify"].as_bool(), Some(true));
    assert_eq!(item["groups"], serde_json::json!([group]));
    let renamed = only_item(&view, "groups")?;
    assert_eq!(renamed["group"].as_u64(), Some(group));
    assert_eq!(renamed["name"].as_str(), Some("Raid Team"));
    assert_eq!(Some(panel_text(&panel)), panel.view.clone());
    let listed_view = panel.view.clone();

    // The duplicate Add advances the receipt revision and preserves the displayed list.
    assert!(request(&mut panel, Intent::Add(name.into())).is_some());
    apply_result(&mut panel, &response(&replay.duplicate)?);
    assert_eq!(value(&replay.expected, "duplicate_code")?, "3");
    assert_eq!(Some(panel.revision), revision.checked_add(1));
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Character is already listed")
    );
    assert_eq!(panel.view, listed_view);
    assert_eq!(
        panel_text(&panel),
        format!(
            "{}\r\nCharacter is already listed",
            listed_view.unwrap_or_default()
        )
    );

    // A truncated reply never reaches the panel.
    assert!(vip::decode_result(&replay.listed[..replay.listed.len() - 3]).is_err());
    Ok(())
}

#[test]
fn rejects_extra_rows_and_equivalent_payload_substitution() -> Result<(), Error> {
    let view = serde_json::json!({"entries": [1, 2], "groups": [1, 2]});
    assert!(only_item(&view, "entries").is_err());
    assert!(only_item(&view, "groups").is_err());
    let encode = |view| vip::encode_result(1, 8, 0, 0, view).map_err(|error| format!("{error:?}"));
    let original = encode("{}")?;
    let substitute = encode("{ }")?;
    let expected = format!("listed_sha256={}", payload_digest(&original));
    verify_payload(&expected, "listed", &original)?;
    assert!(verify_payload(&expected, "listed", &substitute).is_err());
    Ok(())
}

#[test]
fn rejects_wrong_edit_and_group_intents() {
    for (intent, operation, number, text, icon, notify, groups) in [
        (
            Intent::GroupAdd("Wrong".into()),
            vip::Operation::GroupAdd,
            0,
            "Raid Friends",
            0,
            false,
            vec![],
        ),
        (
            Intent::GroupRename(8, "Raid Team".into()),
            vip::Operation::GroupRename,
            4,
            "Raid Team",
            0,
            false,
            vec![],
        ),
        (
            Intent::Edit {
                number: 1,
                text: "node-boot friend".into(),
                icon: 7,
                notify: false,
                groups: vec![4],
            },
            vip::Operation::Edit,
            1,
            "node-boot friend",
            7,
            true,
            vec![4],
        ),
    ] {
        assert!(
            check_group_encoder(
                intent,
                vip::Request {
                    operation,
                    number,
                    text: text.into(),
                    icon,
                    notify,
                    groups,
                }
            )
            .is_err()
        );
    }
}

#[test]
fn rejects_add_revision_as_query_cursor() -> Result<(), Error> {
    let mut panel = VipPanel {
        selected: true,
        revision: 1,
        ..VipPanel::default()
    };
    let revision = panel.revision;
    let wrong =
        request(&mut panel, Intent::Query(revision)).ok_or("nonzero Query did not encode")?;
    assert!(check_first_page_query(&wrong).is_err());
    panel.pending = false;
    let first_page =
        request(&mut panel, Intent::Query(0)).ok_or("first-page Query did not encode")?;
    check_first_page_query(&first_page)
}
