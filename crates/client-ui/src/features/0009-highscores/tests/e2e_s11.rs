//! S11 client replay: a real server ranking page enters the native Highscores panel through
//! `request` (the exact registered query command) and `apply_result` in `protocol.rs`, which
//! decodes the page with `decode_highscores_result`, checks category/page/snapshot against the
//! pending query and stores it in `HighscoresPanel::reply`.
use super::*;
use oteryn_client_runtime::highscores::wire::{self, HighscoreRow};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
use oteryn_protocol_oteryn::{
    FoundationProtocolError, MessageType, decode_command_result, decode_wire_envelope,
};
use std::path::Path;

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

fn live_reply(
    root: &Path,
    label: &str,
    expected_session: &str,
    command: &ServerMechanicCommand,
    expected_error: u32,
) -> Result<(String, ServerMechanicResponse), BoxError> {
    let capture: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(format!("s11-{label}-wire.json")))?)?;
    let replay: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(format!("s11-{label}.json")))?)?;
    assert_eq!(
        capture["run"].as_str(),
        root.file_name().and_then(|name| name.to_str())
    );
    let session = capture["session"].as_str().ok_or("missing live session")?;
    assert_eq!(session, expected_session);
    assert_eq!(session.len(), 36);
    for (index, byte) in session.bytes().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            assert_eq!(byte, b'-', "malformed admitted session ID");
        } else {
            assert!(byte.is_ascii_hexdigit(), "malformed admitted session ID");
        }
    }
    assert_eq!(session.as_bytes()[14], b'7');
    assert!(matches!(session.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    let generation = capture["generation"]
        .as_u64()
        .ok_or("missing live generation")?;
    let request: Vec<u8> = serde_json::from_value(capture["request"].clone())?;
    let request = decode_wire_envelope(&request)?.client_command(generation)?;
    assert_eq!(request.command_type, command.command_type);
    assert_eq!(request.payload, command.payload);
    let result: Vec<u8> = serde_json::from_value(capture["result"].clone())?;
    let envelope = decode_wire_envelope(&result)?;
    assert_eq!(envelope.message_type(), MessageType::CommandResult);
    assert_eq!(envelope.connection_generation(), generation);
    assert_ne!(envelope.server_sequence(), 0);
    let result = decode_command_result(envelope.payload())?;
    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.error_code, expected_error);
    assert_eq!(
        replay,
        serde_json::json!({
            "command_id": result.command_id,
            "game_session_id": expected_session,
            "payload": result.payload,
            "server_sequence": envelope.server_sequence(),
            "status": result.status as u32,
        })
    );
    Ok((
        session.to_owned(),
        ServerMechanicResponse {
            command_id: serde_json::from_value(replay["command_id"].clone())?,
            status: result.status,
            server_sequence: serde_json::from_value(replay["server_sequence"].clone())?,
            payload: serde_json::from_value(replay["payload"].clone())?,
        },
    ))
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

#[test]
#[ignore = "requires an original S11 live capture"]
fn e2e_s11_ranking_page_replays_into_the_highscores_panel() -> Result<(), BoxError> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S11 replay requires E2E_ALL_CAPTURE_DIR")?;
    let root = Path::new(&root);
    let expected = &std::fs::read_to_string(root.join("s11-expected.txt"))?;
    let mut panel = panel();
    panel.category = value(expected, "category")?.parse()?;
    panel.vocation = value(expected, "vocation")?.parse()?;
    let command = request(&mut panel, Action::Refresh).ok_or("s11 query command not produced")?;
    assert_eq!(command.capability, wire::CAPABILITY_HIGHSCORES_V1);
    assert!(panel.pending);

    let session = value(expected, "game_session_id")?;
    let (_, before) = live_reply(root, "before", session, &command, 0)?;
    assert_eq!(before.status, CommandStatus::Accepted);
    let before_page =
        wire::decode_highscores_result(&before.payload).map_err(|e| format!("{e:?}"))?;
    let (_, response) = live_reply(root, "experience", session, &command, 0)?;
    assert_eq!(before.command_id.checked_add(1), Some(response.command_id));
    assert!(before.server_sequence < response.server_sequence);
    assert_eq!(response.status, CommandStatus::Accepted);
    assert!(panel.reply.is_none());
    apply_result(&mut panel, &response);
    let page = panel.reply.clone().ok_or("s11 page not stored")?;
    assert!(!panel.pending && panel.feedback.is_none());
    assert!(before_page.snapshot_id < page.snapshot_id);
    assert!(before_page.rows != page.rows || before_page.own_row != page.own_row);
    let world = page.world.ok_or("S11 response lacks World details")?;
    let world = world
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(world, value(expected, "world")?.replace('-', ""));
    assert_eq!(page.own_page, Some(value(expected, "own_page")?.parse()?));
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
    assert!(count > 0);
    assert_eq!(page.rows.len(), count);
    let mut rendered = Vec::with_capacity(count);
    for (index, row) in page.rows.iter().enumerate() {
        let want = parse_row(value(expected, &format!("row{index}"))?)?;
        assert_eq!(*row, want);
        rendered.push(format!(
            "{} | {} | {} | {} | {}",
            want.rank, want.name, want.vocation, want.level, want.value
        ));
    }
    let text = panel_text(&panel);
    let (_, table) = text
        .split_once("Rank | Name | Vocation | Level | Value\n")
        .ok_or("missing rendered Highscores table")?;
    let (rows, own_text) = table
        .split_once("\nYour rank:")
        .ok_or("missing rendered own rank")?;
    assert_eq!(rows.lines().collect::<Vec<_>>(), rendered);
    let own = value(expected, "own")?;
    assert_eq!(
        page.own_row
            .as_ref()
            .map_or_else(|| "none".into(), row_text),
        own
    );

    let own_row = parse_row(own)?;
    assert_eq!(
        own_text.lines().next(),
        Some(
            format!(
                " {} — {} | {} | level {} | {}",
                own_row.rank, own_row.name, own_row.vocation, own_row.level, own_row.value
            )
            .as_str()
        )
    );

    // Submit an unregistered category while retaining the visible page.
    let kept = panel.reply.clone();
    panel.category = 18;
    let refusal = request(&mut panel, Action::Refresh).ok_or("s11 refusal query not produced")?;
    let (refused_session, refused) = live_reply(
        root,
        "refused",
        session,
        &refusal,
        FoundationProtocolError::NotSupported.code(),
    )?;
    assert_eq!(session, refused_session);
    assert_eq!(response.command_id.checked_add(1), Some(refused.command_id));
    assert!(response.server_sequence < refused.server_sequence);
    assert_eq!(refused.status, CommandStatus::Rejected);
    assert!(refused.payload.is_empty());
    assert_eq!(value(expected, "refused")?, "Rejected");
    apply_result(&mut panel, &refused);
    assert!(!panel.pending);
    assert_eq!(panel.reply, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused highscores: Rejected. Refresh to retry with the current snapshot.")
    );
    let loyalty_filter = Action::Filter {
        category: 13,
        vocation: 0,
    };
    assert!(request(&mut panel, loyalty_filter).is_none());
    panel.loyalty_selected = true;
    let loyalty_query =
        request(&mut panel, loyalty_filter).ok_or("s11 loyalty query not produced")?;
    let (_, loyalty) = live_reply(root, "loyalty", session, &loyalty_query, 0)?;
    assert_eq!(refused.command_id.checked_add(1), Some(loyalty.command_id));
    assert!(refused.server_sequence < loyalty.server_sequence);
    assert_eq!(loyalty.status, CommandStatus::Accepted);
    apply_result(&mut panel, &loyalty);
    let loyalty_page = panel.reply.ok_or("s11 loyalty page not stored")?;
    assert!(!panel.pending && panel.feedback.is_none());
    assert_eq!(loyalty_page.category, 13);
    assert_eq!(loyalty_page.world, page.world);
    assert_eq!(loyalty_page.page, 1);
    assert_ne!(loyalty_page.snapshot_id, 0);
    let loyalty_count: usize = value(expected, "loyalty_rows")?.parse()?;
    assert!(loyalty_count >= 2);
    let loyalty_rows = (0..loyalty_count)
        .map(|index| parse_row(value(expected, &format!("loyalty_row{index}"))?))
        .collect::<Result<Vec<_>, BoxError>>()?;
    assert_eq!(loyalty_page.rows, loyalty_rows);
    assert_eq!(
        loyalty_page.own_row,
        Some(parse_row(value(expected, "loyalty_own")?)?)
    );
    assert_eq!(
        loyalty_page.own_page,
        Some(value(expected, "loyalty_own_page")?.parse()?)
    );
    assert_eq!(
        u8::from(loyalty_page.has_more),
        value(expected, "loyalty_has_more")?.parse::<u8>()?
    );
    assert_eq!(loyalty_page.computed_at, page.computed_at);

    let relog_session = value(expected, "relog_session")?;
    assert_ne!(session, relog_session);
    let mut relog_panel = HighscoresPanel {
        selected: true,
        category: 1,
        ..Default::default()
    };
    let relog_query = request(&mut relog_panel, Action::Refresh).ok_or("missing relog refresh")?;
    let (_, relog) = live_reply(root, "relog", relog_session, &relog_query, 0)?;
    assert_eq!(relog.status, CommandStatus::Accepted);
    apply_result(&mut relog_panel, &relog);
    assert!(!relog_panel.pending && relog_panel.feedback.is_none());
    assert_eq!(relog_panel.reply, Some(page));
    assert_malformed_reply_keeps_the_visible_page()
}

#[test]
fn malformed_highscores_reply_keeps_the_visible_page() -> Result<(), BoxError> {
    assert_malformed_reply_keeps_the_visible_page()
}

fn assert_malformed_reply_keeps_the_visible_page() -> Result<(), BoxError> {
    let mut panel = panel();
    panel.category = 1;
    let kept = wire::HighscoresResult {
        world: None,
        own_page: None,
        category: 1,
        snapshot_id: 1,
        computed_at: 0,
        page: 1,
        has_more: false,
        rows: vec![row(1, "Ada", "Knight", 120, 9_000_000)],
        own_row: None,
    };
    panel.reply = Some(kept.clone());
    request(&mut panel, Action::Refresh).ok_or("highscores query not produced")?;
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![0xff, 0x01],
        },
    );
    assert_eq!(panel.reply, Some(kept));
    assert_eq!(
        panel.feedback.as_deref(),
        Some("The server returned invalid highscores")
    );
    Ok(())
}
