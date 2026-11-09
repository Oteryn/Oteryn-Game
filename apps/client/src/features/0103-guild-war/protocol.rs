use crate::play::feature_0115_mail::protocol::{accepted, uuid};
use oteryn_client_runtime::{
    guild_war as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::guild_war::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let id = uuid(&panel.fields[0])?;
    let request = match intent {
        Intent::Declare => wire::GuildWarIntent::Declare {
            opponent: id,
            frag_limit: panel.fields[1]
                .trim()
                .parse()
                .map_err(|_| tr!("Enter a valid frag limit"))?,
            duration_days: panel.fields[2]
                .trim()
                .parse()
                .map_err(|_| tr!("Enter a valid duration in days"))?,
        },
        Intent::Accept => wire::GuildWarIntent::Accept { war: id },
    };
    let payload =
        wire::encode_guild_war_intent(&request).map_err(|_| tr!("Enter a valid UUIDv7"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_GUILD_WAR_V1,
        kind: wire::COMMAND_TYPE_GUILD_WAR_INTENT,
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
        panel.feedback = tr!("The server refused the guild war request").into();
        return;
    }
    match wire::decode_guild_war_result(&result.payload) {
        Ok(reply) if reply.code == 1 => {
            let id = reply
                .war
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            panel.summary = lfmt!("War UUID: {id}", id = id);
            panel.feedback = tr!("Guild war request accepted").into();
        }
        Ok(_) => panel.feedback = tr!("The server refused the guild war request").into(),
        Err(_) => panel.feedback = tr!("Invalid guild war reply").into(),
    }
}
