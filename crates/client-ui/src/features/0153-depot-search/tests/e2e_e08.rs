//! E08 replay ownership: `apply_result` in `protocol.rs` consumes the Depot Search SEARCH, DETAIL,
//! RETRIEVE and empty-RETRIEVE command replies and `panel_text` projects them.
use super::*;
use oteryn_client_runtime::cui3::depot_search::{self, Outcome, Reply, SearchResult, TierCount};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};

type Fixture = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, String);

fn response(command_id: u64, payload: Vec<u8>) -> ServerMechanicResponse {
    ServerMechanicResponse {
        command_id,
        status: CommandStatus::Accepted,
        server_sequence: command_id,
        payload,
    }
}

fn encode(reply: &Reply) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    depot_search::encode_reply(reply).map_err(|error| format!("{error:?}").into())
}

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let row = SearchResult {
        client_id: 2854,
        depot: 1,
        stash: 0,
        inbox: 0,
        tiers: vec![TierCount { tier: 0, count: 1 }],
    };
    Ok((
        encode(&Reply {
            outcome: Outcome::Found,
            retrieved: 0,
            results: vec![row.clone()],
        })?,
        encode(&Reply {
            outcome: Outcome::Detailed,
            retrieved: 0,
            results: vec![row],
        })?,
        encode(&Reply {
            outcome: Outcome::Retrieved,
            retrieved: 1,
            results: vec![],
        })?,
        encode(&Reply {
            outcome: Outcome::NotFound,
            retrieved: 0,
            results: vec![],
        })?,
        "item=2854\ntier=0\nsearch=Found|1|0|0|0x1\ndetail=Detailed\nretrieve=Retrieved|1\nempty=NotFound|0\n"
            .into(),
    ))
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .ok_or_else(|| format!("expected.txt lacks {key}").into())
}

#[test]
fn e08_real_server_payloads_replay_into_depot_search_panel()
-> Result<(), Box<dyn std::error::Error>> {
    let (search_bytes, detail_bytes, retrieve_bytes, empty_bytes, expected) =
        if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let root = std::path::Path::new(&root);
            (
                std::fs::read(root.join("e08-search.bin"))?,
                std::fs::read(root.join("e08-detail.bin"))?,
                std::fs::read(root.join("e08-retrieve.bin"))?,
                std::fs::read(root.join("e08-empty.bin"))?,
                std::fs::read_to_string(root.join("e08-expected.txt"))?,
            )
        } else {
            fixture()?
        };
    let item: u32 = value(&expected, "item")?.parse()?;
    let tier: u8 = value(&expected, "tier")?.parse()?;
    let mut panel = DepotSearchPanel {
        selected: true,
        item_id: item,
        tier,
        ..Default::default()
    };

    search(&mut panel).ok_or("search command not issued")?;
    assert!(panel.pending);
    apply_result(&mut panel, &response(1, search_bytes));
    assert!(!panel.pending);
    let search_line = value(&expected, "search")?;
    let mut parts = search_line.split('|');
    assert_eq!(parts.next(), Some("Found"));
    let (depot, stash, inbox) = (
        parts.next().ok_or("depot")?,
        parts.next().ok_or("stash")?,
        parts.next().ok_or("inbox")?,
    );
    let tier_count = parts.next().ok_or("tiers")?.replace('x', " x");
    let (tier_no, count) = tier_count.split_once(" x").ok_or("tier shape")?;
    let total = depot.parse::<u64>()? + stash.parse::<u64>()? + inbox.parse::<u64>()?;
    assert_eq!(
        panel_text(&panel),
        format!(
            "Depot Search\nItem id: {item}\nSelected tier: {tier}\nRetrieve one retrieves one unit. Retrieve all includes every tier of the selected item.\nResult: Found\nRetrieved: 0\n\nRow 0 | Item {item}: depot {depot}, stash {stash}, inbox {inbox}; total {total}; tier {tier_no} x{count}"
        )
    );
    assert_eq!(panel.feedback, None);

    detail(&mut panel).ok_or("detail command not issued")?;
    apply_result(&mut panel, &response(2, detail_bytes));
    assert_eq!(
        panel.reply.as_ref().map(|reply| reply.outcome),
        Some(Outcome::Detailed)
    );
    assert_eq!(
        panel
            .reply
            .as_ref()
            .map(|reply| reply.results[0].tiers[0].count),
        Some(count.parse()?)
    );

    let retrieved: u32 = value(&expected, "retrieve")?
        .strip_prefix("Retrieved|")
        .ok_or("retrieve shape")?
        .parse()?;
    request(&mut panel, command(vec![])).ok_or("retrieve request not issued")?;
    apply_result(&mut panel, &response(3, retrieve_bytes));
    let reply = panel.reply.clone().ok_or("no retrieve reply")?;
    assert_eq!(
        (reply.outcome, reply.retrieved),
        (Outcome::Retrieved, retrieved)
    );
    assert!(reply.results.is_empty());

    request(&mut panel, command(vec![])).ok_or("empty retrieve not issued")?;
    apply_result(&mut panel, &response(4, empty_bytes));
    let reply = panel.reply.clone().ok_or("no empty reply")?;
    assert_eq!((reply.outcome, reply.retrieved), (Outcome::NotFound, 0));

    // A refused reply and an undecodable payload leave the last reply unchanged.
    let before = panel.reply.clone();
    request(&mut panel, command(vec![])).ok_or("refusal request not issued")?;
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 5,
            status: CommandStatus::Rejected,
            server_sequence: 5,
            payload: vec![],
        },
    );
    assert_eq!(panel.reply, before);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused depot search: Rejected")
    );
    request(&mut panel, command(vec![])).ok_or("invalid request not issued")?;
    apply_result(&mut panel, &response(6, vec![0xff, 0xff]));
    assert_eq!(panel.reply, before);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("The server returned invalid depot results")
    );
    Ok(())
}
