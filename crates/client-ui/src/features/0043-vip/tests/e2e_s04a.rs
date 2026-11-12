//! S04a replay: the VIP list replies for Add, the edited/grouped Query and the duplicate-Add
//! refusal applied through the production `request`, `apply_result` and `panel_text`. The reply
//! bytes are decoded by `vip::decode_result`, exactly as the runtime `send` does.
use super::*;
use oteryn_client_runtime::vip::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Replay {
    added: Vec<u8>,
    listed: Vec<u8>,
    duplicate: Vec<u8>,
    expected: String,
}

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn load() -> Result<Replay, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Replay {
            added: std::fs::read(root.join("s04a-added.bin"))?,
            listed: std::fs::read(root.join("s04a-listed.bin"))?,
            duplicate: std::fs::read(root.join("s04a-duplicate.bin"))?,
            expected: std::fs::read_to_string(root.join("s04a-expected.txt"))?,
        });
    }
    let view = "{\"entries\":[{\"number\":4,\"name\":\"Berenike\",\"description\":\"node-boot friend\",\"icon\":7,\"notify\":true,\"groups\":[2]}],\"groups\":[{\"number\":2,\"name\":\"Raid Team\"}]}";
    let encode = |code, revision, number, view: &str| {
        vip::encode_result(code, revision, number, 0, view).map_err(|e| format!("{e:?}"))
    };
    Ok(Replay {
        added: encode(1, 5, 4, "")?,
        listed: encode(1, 8, 0, view)?,
        duplicate: encode(3, 8, 0, "")?,
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

#[test]
fn vip_replay_add_edit_query_and_duplicate_refusal() -> Result<(), Error> {
    let replay = load()?;
    let name = value(&replay.expected, "name")?;
    let entry: u64 = value(&replay.expected, "entry")?.parse()?;
    let group: u64 = value(&replay.expected, "group")?.parse()?;
    let revision: u64 = value(&replay.expected, "revision")?.parse()?;
    let added_revision: u64 = value(&replay.expected, "added_revision")?.parse()?;
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

    assert!(request(&mut panel, Intent::Query(added_revision)).is_some());
    apply_result(&mut panel, &response(&replay.listed)?);
    assert_eq!((panel.revision, panel.feedback.clone()), (revision, None));
    let view: serde_json::Value =
        serde_json::from_str(panel.view.as_deref().ok_or("view absent")?)?;
    let item = &view["entries"][0];
    assert_eq!(item["number"].as_u64(), Some(entry));
    assert_eq!(item["name"].as_str(), Some(name));
    assert_eq!(item["description"].as_str(), Some("node-boot friend"));
    assert_eq!(item["icon"].as_u64(), Some(7));
    assert_eq!(item["notify"].as_bool(), Some(true));
    assert_eq!(item["groups"][0].as_u64(), Some(group));
    assert_eq!(view["groups"][0]["number"].as_u64(), Some(group));
    assert_eq!(view["groups"][0]["name"].as_str(), Some("Raid Team"));
    assert_eq!(Some(panel_text(&panel)), panel.view.clone());
    let listed_view = panel.view.clone();

    // The duplicate Add is refused: the revision stays and the list view is not replaced.
    assert!(request(&mut panel, Intent::Add(name.into())).is_some());
    apply_result(&mut panel, &response(&replay.duplicate)?);
    assert_eq!(value(&replay.expected, "duplicate_code")?, "3");
    assert_eq!(panel.revision, revision);
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
