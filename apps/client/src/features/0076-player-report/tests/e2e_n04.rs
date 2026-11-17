//! N04 replay: captured player bug-report replies applied through the production client bug path
//! (`bug::activate` -> `properties_result` -> `protocol::apply`). A client request that the server
//! accepted must be byte-identical to the one the client builds here.
use super::activate;
use crate::combat_input::PlayCommand;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_runtime::{
    player_report as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};

type Error = Box<dyn std::error::Error>;

struct Captured {
    comment: String,
    request: Vec<u8>,
    first: Vec<u8>,
    second: Vec<u8>,
    first_report: u64,
    second_code: String,
    second_report: u64,
    second_wait: u32,
}

fn reply_bytes(code: wire::PlayerReportCode, report: u64, wait_seconds: u32) -> Result<Vec<u8>, Error> {
    wire::encode_player_report_reply(&wire::PlayerReportReply { code, report, wait_seconds })
        .map_err(|error| format!("{error:?}").into())
}

fn request_bytes(comment: &str) -> Result<Vec<u8>, Error> {
    wire::encode_player_report_request(&wire::PlayerReportRequest {
        kind: wire::KIND_BUG,
        category: 3,
        reason: 0,
        action: 0,
        target: String::new(),
        comment: comment.into(),
        map_x: 0,
        map_y: 0,
        map_floor: 0,
    })
    .map_err(|error| format!("{error:?}").into())
}

fn captured() -> Result<Captured, Error> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        let comment = "N04 bug report: the depot chest closed by itself";
        return Ok(Captured {
            comment: comment.into(),
            request: request_bytes(comment)?,
            first: reply_bytes(wire::PlayerReportCode::Accepted, 41, 0)?,
            second: reply_bytes(wire::PlayerReportCode::RateLimited, 0, 30)?,
            first_report: 41,
            second_code: "RateLimited".into(),
            second_report: 0,
            second_wait: 30,
        });
    };
    let root = std::path::Path::new(&root);
    let expected = std::fs::read_to_string(root.join("n04-expected.txt"))?;
    let value = |key: &str| -> Result<&str, Error> {
        expected
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .ok_or_else(|| format!("n04-expected.txt lacks {key}").into())
    };
    Ok(Captured {
        comment: value("comment")?.into(),
        request: std::fs::read(root.join("n04-request.bin"))?,
        first: std::fs::read(root.join("n04-first.bin"))?,
        second: std::fs::read(root.join("n04-second.bin"))?,
        first_report: value("first_report")?.parse()?,
        second_code: value("second_code")?.into(),
        second_report: value("second_report")?.parse()?,
        second_wait: value("second_wait")?.parse()?,
    })
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn send_report(
    link: &crate::play::PlayLink,
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<FeatureCommand, Error> {
    let mut mailbox = lock(&link.mailbox);
    activate(&mut mailbox, &link.commands, 3);
    activate(&mut mailbox, &link.commands, 4);
    activate(&mut mailbox, &link.commands, 5);
    drop(mailbox);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("bug report was not sent as a feature command".into());
    };
    assert_eq!(
        (command.capability, command.kind),
        (wire::CAPABILITY_PLAYER_REPORT_V1, wire::COMMAND_TYPE_PLAYER_REPORT)
    );
    Ok(command)
}

#[test]
fn e2e_n04_bug_report_reply_feedback_and_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.properties.player_report;
        panel.selected = true;
        // An oversized comment never reaches the wire and leaves the panel idle.
        panel.bug.fields = [String::new(), "x".repeat(1500), String::new()];
        activate(&mut mailbox, &link.commands, 3);
        activate(&mut mailbox, &link.commands, 4);
        let panel = &mailbox.properties.player_report;
        assert_eq!(
            panel.feedback,
            "Comment must fit 1,024 UTF-8 bytes; target must fit 29 bytes"
        );
        assert!(panel.bug.reviewed.is_none() && !panel.pending && panel.summary.is_empty());
        activate(&mut mailbox, &link.commands, 5);
        assert!(!mailbox.properties.player_report.pending);
        mailbox.properties.player_report.bug.fields[1] = captured.comment.clone();
    }
    assert!(receiver.try_recv().is_err());

    let first = send_report(&link, &mut receiver)?;
    assert_eq!(first.payload, captured.request, "client request differs from the accepted one");
    let sent = wire::decode_player_report_request(&first.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (sent.kind, sent.category, sent.comment.as_str(), sent.map_x, sent.map_y, sent.map_floor),
        (wire::KIND_BUG, 3, captured.comment.as_str(), 0, 0, 0)
    );
    assert!(link.player_report_panel().pending);
    outbox.properties_result(&first, &result(CommandStatus::Accepted, &captured.first));
    let panel = link.player_report_panel();
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "Report accepted");
    assert_eq!(panel.summary, format!("Report #{} queued", captured.first_report));

    // The immediate second report shows exactly what the server answered.
    let second = send_report(&link, &mut receiver)?;
    outbox.properties_result(&second, &result(CommandStatus::Accepted, &captured.second));
    let panel = link.player_report_panel();
    assert!(!panel.pending);
    match captured.second_code.as_str() {
        "RateLimited" => {
            assert_eq!(
                panel.feedback,
                format!("Wait {} seconds before reporting again", captured.second_wait)
            );
            assert_eq!(panel.summary, format!("Report #{} queued", captured.first_report));
        }
        "Accepted" => {
            assert_eq!(panel.feedback, "Report accepted");
            assert_eq!(panel.summary, format!("Report #{} queued", captured.second_report));
        }
        other => return Err(format!("unexpected captured second code {other}").into()),
    }

    // A refused command leaves the last summary in place.
    let summary = panel.summary.clone();
    let third = send_report(&link, &mut receiver)?;
    outbox.properties_result(&third, &result(CommandStatus::Rejected, &[]));
    let panel = link.player_report_panel();
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "The server refused the report");
    assert_eq!(panel.summary, summary);
    Ok(())
}
