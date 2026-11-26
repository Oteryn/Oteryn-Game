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
    name: &str,
    command: &ServerMechanicCommand,
    expected_error: u32,
) -> Result<(String, ServerMechanicResponse), BoxError> {
    let capture: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(name))?)?;
    assert_eq!(
        capture["run"].as_str(),
        root.file_name().and_then(|name| name.to_str())
    );
    let session = capture["session"]
        .as_str()
        .filter(|session| !session.is_empty())
        .ok_or("missing live session")?;
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
    assert_eq!(
        Some(envelope.server_sequence()),
        capture["server_sequence"].as_u64()
    );
    assert_ne!(envelope.server_sequence(), 0);
    let result = decode_command_result(envelope.payload())?;
    assert_eq!(Some(result.command_id), capture["command_id"].as_u64());
    assert_eq!(result.command_id, request.command_id);
    assert_eq!(result.error_code, expected_error);
    Ok((
        session.to_owned(),
        ServerMechanicResponse {
            command_id: result.command_id,
            status: result.status,
            server_sequence: envelope.server_sequence(),
            payload: result.payload.to_vec(),
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
fn e2e_s11_ranking_page_replays_into_the_highscores_panel() -> Result<(), BoxError> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let expected = &std::fs::read_to_string(root.join("s11-expected.txt"))?;
    let mut panel = panel();
    panel.category = value(expected, "category")?.parse()?;
    panel.vocation = value(expected, "vocation")?.parse()?;
    let command = request(&mut panel, Action::Refresh).ok_or("s11 query command not produced")?;
    assert_eq!(command.capability, wire::CAPABILITY_HIGHSCORES_V1);
    assert!(panel.pending);

    let (session, response) = live_reply(root, "s11-experience-exchange.json", &command, 0)?;
    assert_eq!(response.status, CommandStatus::Accepted);
    assert!(panel.reply.is_none());
    apply_result(&mut panel, &response);
    let page = panel.reply.clone().ok_or("s11 page not stored")?;
    assert!(!panel.pending && panel.feedback.is_none());
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
    for (index, row) in page.rows.iter().enumerate() {
        let want = parse_row(value(expected, &format!("row{index}"))?)?;
        assert_eq!(*row, want);
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

    // Submit the reserved category to the reducer while retaining the visible page.
    let kept = panel.reply.clone();
    panel.category = 15;
    let refusal = request(&mut panel, Action::Refresh).ok_or("s11 refusal query not produced")?;
    let (refused_session, refused) = live_reply(
        root,
        "s11-refused-exchange.json",
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
    assert_eq!(panel.reply, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused highscores: Rejected. Refresh to retry with the current snapshot.")
    );
    Ok(())
}

#[test]
fn malformed_highscores_reply_keeps_the_visible_page() -> Result<(), BoxError> {
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
