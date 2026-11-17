//! W12 replay: the Cyclopedia map QUERY, FIND-refusal and START replies applied by the production
//! client reducer `apply` (`protocol.rs`); requests are built by the production `command`.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Replies {
    query: Vec<u8>,
    find: Vec<u8>,
    start: Vec<u8>,
    expected: String,
}

fn replies() -> Result<Replies, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Replies {
            query: std::fs::read(root.join("w12-query.bin"))?,
            find: std::fs::read(root.join("w12-find.bin"))?,
            start: std::fs::read(root.join("w12-start.bin"))?,
            expected: std::fs::read_to_string(root.join("w12-expected.txt"))?,
        });
    }
    let encode = |code, revision, found, visible: Vec<(u16, u16, i8)>| {
        wire::encode_result(&wire::MapResult {
            code,
            revision,
            discovered: false,
            found,
            area_percent: 0,
            pool: 12,
            epoch: 3,
            improved: false,
            visible,
        })
        .map_err(|error| format!("{error:?}"))
    };
    Ok(Replies {
        query: encode(0, 4, 0, vec![])?,
        find: encode(4, 4, 0, vec![])?,
        start: encode(0, 5, 0, vec![(32_100, 32_200, -7), (32_101, 32_203, -7)])?,
        expected: "query_code=0\nquery_revision=4\nquery_discovered=false\nquery_found=0\nquery_percent=0\nquery_pool=12\nquery_epoch=3\nquery_improved=false\nquery_visible=[]\nfind_code=4\nfind_revision=4\nfind_discovered=false\nfind_found=0\nfind_percent=0\nfind_pool=12\nfind_epoch=3\nfind_improved=false\nfind_visible=[]\nstart_code=0\nstart_revision=5\nstart_discovered=false\nstart_found=0\nstart_percent=0\nstart_pool=12\nstart_epoch=3\nstart_improved=false\nstart_visible=[(32100, 32200, -7), (32101, 32203, -7)]\n".into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending(kind: u32) -> Panel {
    Panel {
        selected: true,
        open: true,
        pending: true,
        pending_kind: kind,
        ..Panel::default()
    }
}

fn summary(expected: &str, prefix: &str) -> Result<String, Error> {
    let field = |name: &str| value(expected, &format!("{prefix}_{name}"));
    Ok(format!(
        "Current exploration area | Revision {} | Discovered: {}\r\nFound {}/7 | Area {}% | Pool {} | Epoch {} | Improved: {}\r\nVisible points: {}",
        field("revision")?,
        field("discovered")?,
        field("found")?,
        field("percent")?,
        field("pool")?,
        field("epoch")?,
        field("improved")?,
        field("visible")?,
    ))
}

#[test]
fn e2e_w12_replays_map_query_find_refusal_and_start() -> Result<(), Error> {
    let replies = replies()?;
    let expected = replies.expected.as_str();
    let blank = Panel {
        selected: true,
        ..Panel::default()
    };
    let query = command(&blank, Intent::Query)?;
    assert_eq!((query.capability, query.kind), (57, 72));
    assert!(query.payload.is_empty());
    let find = command(&blank, Intent::Find)?;
    assert_eq!((find.capability, find.kind), (57, 73));
    assert_eq!(find.payload, [3, 1, 0, 0, 0, 0, 0, 0, 0]);
    let start = command(&blank, Intent::Start)?;
    assert_eq!(start.payload, [1, 1, 0, 0, 0, 0, 0, 0, 0]);

    // QUERY: the confirmed server state is shown verbatim.
    assert_eq!(value(expected, "query_code")?, "0");
    let mut panel = pending(72);
    apply(&mut panel, &query, &result(CommandStatus::Accepted, &replies.query));
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "Cyclopedia map updated");
    let confirmed = summary(expected, "query")?;
    assert_eq!(panel.summary, confirmed);

    // FIND before START: the server's code 4 shows the refusal and keeps the confirmed summary.
    assert_eq!(value(expected, "find_code")?, "4");
    panel.pending = true;
    panel.pending_kind = 73;
    apply(&mut panel, &find, &result(CommandStatus::Accepted, &replies.find));
    assert_eq!(panel.feedback, "No exploration in progress");
    assert_eq!(panel.summary, confirmed);

    // START: code 0 replaces the summary with the server's exploration state and visible points;
    // code 2 (a non-premium character) shows the refusal and leaves the summary unchanged.
    panel.pending = true;
    apply(&mut panel, &start, &result(CommandStatus::Accepted, &replies.start));
    match value(expected, "start_code")? {
        "0" => {
            assert_eq!(panel.feedback, "Cyclopedia map updated");
            assert_eq!(panel.summary, summary(expected, "start")?);
        }
        "2" => {
            assert_eq!(panel.feedback, "Premium required");
            assert_eq!(panel.summary, confirmed);
        }
        other => return Err(format!("unexpected START code {other}").into()),
    }

    // A rejected command and a truncated result leave the summary as it was.
    let before = panel.summary.clone();
    for (status, payload) in [
        (CommandStatus::Rejected, replies.start.as_slice()),
        (CommandStatus::Accepted, &replies.start[..replies.start.len() - 1]),
    ] {
        panel.pending = true;
        panel.pending_kind = 72;
        apply(&mut panel, &query, &result(status, payload));
        assert!(!panel.pending);
        assert_eq!(panel.summary, before);
    }
    assert_eq!(panel.feedback, "Invalid cyclopedia map reply");
    Ok(())
}
