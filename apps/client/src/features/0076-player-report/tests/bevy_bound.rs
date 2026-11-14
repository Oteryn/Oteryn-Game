use super::*;
use crate::play::play_channel;
#[test]
fn player_report_rejected_when_session_replaced_or_target_changed() {
    let (link, mut commands, _events) = play_channel();
    link.player_report_open();
    lock(&link.mailbox).properties.player_report.selected = true;
    let expected = link.player_report_snapshot();
    let (other, mut other_commands, _) = play_channel();
    other.player_report_open();
    lock(&other.mailbox).properties.player_report = expected.panel.clone();
    assert!(!other.player_report_dispatch_bound(
        &expected,
        expected.panel.fields.clone(),
        PlayerReportControl::Bug
    ));
    assert!(other_commands.try_recv().is_err());
    link.player_report_edit(2, "Changed target".into());
    let before = link.player_report_panel();
    assert!(!link.player_report_dispatch_bound(
        &expected,
        expected.panel.fields.clone(),
        PlayerReportControl::Bug
    ));
    assert_eq!(link.player_report_panel(), before);
    let current = link.player_report_snapshot();
    assert!(link.player_report_dispatch_bound(
        &current,
        current.panel.fields.clone(),
        PlayerReportControl::Close
    ));
    let closed = link.player_report_panel();
    assert!(!link.player_report_dispatch_bound(
        &current,
        current.panel.fields.clone(),
        PlayerReportControl::Bug
    ));
    assert_eq!(link.player_report_panel(), closed);
    link.player_report_open();
    let current = link.player_report_snapshot();
    _events.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let ended = link.player_report_panel();
    assert!(!link.player_report_dispatch_bound(
        &current,
        current.panel.fields.clone(),
        PlayerReportControl::Bug
    ));
    assert_eq!(link.player_report_panel(), ended);
    assert!(commands.try_recv().is_err());
}
#[test]
fn player_report_actions_are_bound_and_confirmed() -> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::player_report as wire;
    let (link, mut commands, _) = play_channel();
    link.player_report_open();
    lock(&link.mailbox).properties.player_report.selected = true;
    for kind in 0..=2 {
        let current = link.player_report_snapshot();
        assert!(link.player_report_dispatch_bound(
            &current,
            current.panel.fields.clone(),
            PlayerReportControl::ReportType(kind)
        ));
        assert_eq!(link.player_report_panel().fields[1], kind.to_string());
        assert!(commands.try_recv().is_err());
    }
    for next in [false, true] {
        let current = link.player_report_snapshot();
        assert!(link.player_report_dispatch_bound(
            &current,
            current.panel.fields.clone(),
            PlayerReportControl::Reason(next)
        ));
        assert!(commands.try_recv().is_err());
    }
    let mut fields = ["0", "0", "Reported", "", "Evidence"].map(str::to_owned);
    let snapshot = link.player_report_snapshot();
    assert!(link.player_report_dispatch_bound(
        &snapshot,
        fields.clone(),
        PlayerReportControl::SendViolation
    ));
    assert!(!link.player_report_panel().pending);
    assert!(commands.try_recv().is_err());
    assert!(link.player_report_dispatch_bound(
        &link.player_report_snapshot(),
        fields.clone(),
        PlayerReportControl::ReviewViolation
    ));
    fields[4].push('!');
    assert!(link.player_report_dispatch_bound(
        &link.player_report_snapshot(),
        fields.clone(),
        PlayerReportControl::SendViolation
    ));
    assert!(commands.try_recv().is_err());
    assert!(link.player_report_dispatch_bound(
        &link.player_report_snapshot(),
        fields.clone(),
        PlayerReportControl::ReviewViolation
    ));
    let reviewed = link.player_report_snapshot();
    assert!(link.player_report_dispatch_bound(
        &reviewed,
        fields.clone(),
        PlayerReportControl::SendViolation
    ));
    let PlayCommand::Feature(sent) = commands.try_recv()? else {
        return Err("feature request".into());
    };
    assert_eq!(sent.capability, 76);
    let decoded =
        wire::decode_player_report_request(&sent.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(decoded.target, "Reported");
    assert_eq!(decoded.comment, "Evidence!");
    assert!(link.player_report_panel().summary.is_empty());
    let before = link.player_report_panel();
    assert!(!link.player_report_dispatch_bound(
        &reviewed,
        fields,
        PlayerReportControl::SendViolation
    ));
    assert_eq!(link.player_report_panel(), before);
    let pending = link.player_report_snapshot();
    assert!(!link.player_report_dispatch_bound(
        &pending,
        pending.panel.fields.clone(),
        PlayerReportControl::SendViolation
    ));
    assert_eq!(link.player_report_panel(), before);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn player_report_window_shows_server_data_only() -> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::player_report as wire;
    let (link, mut commands, events) = play_channel();
    link.player_report_open();
    events.bevy_player_report_selected();
    let fields = ["3", "", "", "", "Actual bug evidence"].map(str::to_owned);
    assert!(link.player_report_dispatch_bound(
        &link.player_report_snapshot(),
        fields,
        PlayerReportControl::Bug
    ));
    assert!(link.player_report_panel().summary.is_empty());
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature request".into());
    };
    events.properties_result(
        &command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: wire::encode_player_report_reply(&wire::PlayerReportReply {
                code: wire::PlayerReportCode::Accepted,
                report: 941,
                wait_seconds: 0,
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    let panel = link.player_report_panel();
    assert_eq!(panel.summary, "Report #941 queued");
    assert_eq!(panel.feedback, "Report accepted");
    assert!(!panel.pending);
    assert_eq!(link.player_report_snapshot().panel, panel);
    Ok(())
}
#[test]
fn player_report_window_registered() {
    let registry = include_str!("../../../bevy_shell/game_ui/mod.rs");
    let toolbar = include_str!("../../../bevy_shell/ui.rs");
    assert!(registry.contains("player_report_report::install(app)"));
    assert!(toolbar.contains("shell.player_report"));
}
