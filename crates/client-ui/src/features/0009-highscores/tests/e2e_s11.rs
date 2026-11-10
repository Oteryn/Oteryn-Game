//! S11 client replay: a real server ranking page enters the native Highscores panel through
//! `request` (the exact registered query command) and `apply_result` in `protocol.rs`, which
//! decodes the page with `decode_highscores_result`, checks category/page/snapshot against the
//! pending query and stores it in `HighscoresPanel::reply`.
use super::*;
use oteryn_client_runtime::highscores::wire::{self, HighscoreRow, HighscoresResult};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};

type BoxError = Box<dyn std::error::Error>;

fn row(rank: u64, name: &str, vocation: &str, level: u32, value: u64) -> HighscoreRow {
    HighscoreRow {
        rank,
        name: name.into(),
        vocation: vocation.into(),
        level,
        value,
    }
}

fn row_text(row: &HighscoreRow) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        row.rank, row.name, row.vocation, row.level, row.value
    )
}

fn parse_row(text: &str) -> Result<HighscoreRow, BoxError> {
    let parts: Vec<&str> = text.split('|').collect();
    let [rank, name, vocation, level, value] = parts.as_slice() else {
        return Err(format!("s11 row {text:?} lacks five fields").into());
    };
    Ok(row(
        rank.parse()?,
        name,
        vocation,
        level.parse()?,
        value.parse()?,
    ))
}

struct Capture {
    payload: Vec<u8>,
    expected: String,
}

fn capture() -> Result<Capture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            payload: std::fs::read(root.join("s11-page1.bin"))?,
            expected: std::fs::read_to_string(root.join("s11-expected.txt"))?,
        });
    }
    let page = HighscoresResult {
        own_page: None,
        world: None,
        category: 1,
        snapshot_id: 77,
        computed_at: 1_700_000_000_000,
        page: 1,
        has_more: false,
        rows: vec![
            row(1, "Ada", "Knight", 120, 9_000_000),
            row(2, "Bo", "Druid", 95, 4_500_000),
        ],
        own_row: Some(row(2, "Bo", "Druid", 95, 4_500_000)),
    };
    Ok(Capture {
        payload: wire::encode_highscores_result(&page).map_err(|error| format!("{error:?}"))?,
        expected: "category=1\nvocation=0\npage=1\nsnapshot_id=77\ncomputed_at=1700000000000\nhas_more=0\n\
                   rows=2\nrow0=1|Ada|Knight|120|9000000\nrow1=2|Bo|Druid|95|4500000\n\
                   own=2|Bo|Druid|95|4500000\nrefused=Rejected\n"
            .into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("s11 expected lacks {key}").into())
}

fn panel() -> HighscoresPanel {
    HighscoresPanel {
        selected: true,
        ..HighscoresPanel::default()
    }
}

fn reply(status: CommandStatus, payload: Vec<u8>) -> ServerMechanicResponse {
    ServerMechanicResponse {
        command_id: 7,
        status,
        server_sequence: 41,
        payload,
    }
}

#[test]
fn e2e_s11_ranking_page_replays_into_the_highscores_panel() -> Result<(), BoxError> {
    let capture = capture()?;
    let expected = capture.expected.as_str();
    let mut panel = panel();
    panel.category = value(expected, "category")?.parse()?;
    panel.vocation = value(expected, "vocation")?.parse()?;
    let command = request(&mut panel, Action::Refresh).ok_or("s11 query command not produced")?;
    assert_eq!(command.capability, wire::CAPABILITY_HIGHSCORES_V1);
    assert!(panel.pending);

    apply_result(
        &mut panel,
        &reply(CommandStatus::Accepted, capture.payload.clone()),
    );
    let page = panel.reply.clone().ok_or("s11 page not stored")?;
    assert!(!panel.pending && panel.feedback.is_none());
    assert_eq!(page.category, value(expected, "category")?.parse::<u32>()?);
    assert_eq!(page.page, value(expected, "page")?.parse::<u32>()?);
    assert_eq!(
        page.snapshot_id,
        value(expected, "snapshot_id")?.parse::<u64>()?
    );
    assert_eq!(
        page.computed_at,
        value(expected, "computed_at")?.parse::<i64>()?
    );
    assert_eq!(
        u8::from(page.has_more),
        value(expected, "has_more")?.parse::<u8>()?
    );
    let count: usize = value(expected, "rows")?.parse()?;
    assert_eq!(page.rows.len(), count);
    for index in 0..count {
        let want = parse_row(value(expected, &format!("row{index}"))?)?;
        assert_eq!(page.rows[index], want);
        assert!(panel_text(&panel).contains(&format!(
            "\n{} | {} | {} | {} | {}",
            want.rank, want.name, want.vocation, want.level, want.value
        )));
    }
    let own = value(expected, "own")?;
    assert_eq!(
        page.own_row
            .as_ref()
            .map_or_else(|| "none".into(), row_text),
        own
    );

    // A refused reply and a malformed payload leave the stored page unchanged.
    let kept = panel.reply.clone();
    request(&mut panel, Action::Refresh).ok_or("s11 second query not produced")?;
    let refused = value(expected, "refused")?;
    assert_eq!(refused, "Rejected");
    apply_result(&mut panel, &reply(CommandStatus::Rejected, Vec::new()));
    assert_eq!(panel.reply, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused highscores: Rejected. Refresh to retry with the current snapshot.")
    );
    request(&mut panel, Action::Refresh).ok_or("s11 third query not produced")?;
    apply_result(
        &mut panel,
        &reply(CommandStatus::Accepted, vec![0xff, 0x01]),
    );
    assert_eq!(panel.reply, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("The server returned invalid highscores")
    );
    Ok(())
}
