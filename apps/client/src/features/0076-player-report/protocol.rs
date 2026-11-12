use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    player_report as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::player_report::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let number = |index: usize| {
        panel.fields[index]
            .trim()
            .parse::<u8>()
            .map_err(|_| tr!("Enter a valid category, reason or report type"))
    };
    let mut request = wire::PlayerReportRequest {
        kind: wire::KIND_BUG,
        category: 0,
        reason: 0,
        action: 0,
        target: String::new(),
        comment: panel.fields[4].clone(),
        map_x: 0,
        map_y: 0,
        map_floor: 0,
    };
    match intent {
        Intent::Bug => {
            request.category = number(0)?;
            if request.category == wire::BUG_CATEGORY_MAP {
                let coordinates: Vec<_> = panel.fields[3].split(',').map(str::trim).collect();
                let [x, y, floor] = coordinates.as_slice() else {
                    return Err(tr!("Enter map position as x,y,floor").into());
                };
                request.map_x = x.parse().map_err(|_| tr!("Invalid map x coordinate"))?;
                request.map_y = y.parse().map_err(|_| tr!("Invalid map y coordinate"))?;
                request.map_floor = floor.parse().map_err(|_| tr!("Invalid map floor"))?;
                if request.map_floor > 15 {
                    return Err(tr!("Map floor must be 0-15").into());
                }
            }
        }
        Intent::Violation => {
            request.kind = wire::KIND_RULE_VIOLATION;
            request.reason = number(0)?;
            request.action = number(1)?;
            request.target = panel.fields[2].trim().to_owned();
        }
    }
    wire::validate_player_report(&request).map_err(|_| tr!("Invalid report fields"))?;
    let payload = wire::encode_player_report_request(&request)
        .map_err(|_| tr!("Comment must fit 1,024 UTF-8 bytes; target must fit 29 bytes"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_PLAYER_REPORT_V1,
        kind: wire::COMMAND_TYPE_PLAYER_REPORT,
        payload,
    })
}

pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    if !accepted(result) {
        panel.feedback = tr!("The server refused the report").into();
        return;
    }
    let Ok(reply) = wire::decode_player_report_reply(&result.payload) else {
        panel.feedback = tr!("Invalid report reply").into();
        return;
    };
    panel.feedback = match reply.code {
        wire::PlayerReportCode::Accepted => {
            panel.summary = lfmt!("Report #{} queued", reply.report);
            tr!("Report accepted").into()
        }
        wire::PlayerReportCode::Invalid => {
            tr!("The server refused the report: invalid fields").into()
        }
        wire::PlayerReportCode::SelfReport => {
            tr!("The server refused a report about yourself").into()
        }
        wire::PlayerReportCode::AlreadyPending => {
            tr!("A report about this player is already pending").into()
        }
        wire::PlayerReportCode::RateLimited => {
            lfmt!("Wait {} seconds before reporting again", reply.wait_seconds)
        }
    };
}
