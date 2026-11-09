use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    guildhall as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::guildhall::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let Intent::Transfer = intent;
    let request = wire::GuildhallLeadershipIntent {
        guild: crate::play::feature_0115_mail::protocol::uuid(&panel.fields[0])?,
        successor: crate::play::feature_0115_mail::protocol::uuid(&panel.fields[1])?,
        expected_revision: panel.fields[2]
            .trim()
            .parse()
            .map_err(|_| tr!("Enter a positive guild revision"))?,
    };
    let payload = wire::encode_guildhall_leadership(request)
        .map_err(|_| tr!("Enter UUIDv7 identifiers and a positive guild revision"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_GUILDHALL_V1,
        kind: wire::COMMAND_TYPE_GUILDHALL_LEADERSHIP,
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
        panel.feedback = tr!("The server refused the request").into();
        return;
    }
    let (Ok(reply), Ok(request)) = (
        wire::decode_guild_intent_result(&result.payload),
        wire::decode_guildhall_leadership(&command.payload),
    ) else {
        panel.feedback = tr!("Invalid guildhall reply").into();
        return;
    };
    if reply.guild != request.guild
        || reply.code == 0
        || (reply.code == 1 && reply.revision.is_none())
    {
        panel.feedback = tr!("Invalid guildhall reply").into();
        return;
    }
    if reply.code != 1 {
        let reason = match reply.code {
            2 => tr!("not permitted"),
            11 => tr!("premium required"),
            12 => tr!("successor is junior"),
            13 => tr!("stale guild revision"),
            _ => tr!("guild rules"),
        };
        panel.feedback = lfmt!("The server refused the transfer: {}", reason);
        return;
    }
    panel.summary = lfmt!(
        "Leadership transferred; guild revision {}",
        reply.revision.unwrap_or_default()
    );
    panel.feedback = tr!("Leadership transfer committed").into();
}
