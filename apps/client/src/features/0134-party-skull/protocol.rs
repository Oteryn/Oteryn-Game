use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    party_skull as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::party_skull::{Intent, Panel};
pub(in crate::play) fn command(_: &Panel, _: Intent) -> Result<FeatureCommand, String> {
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_PARTY_SKULL_V1,
        kind: wire::COMMAND_TYPE_PARTY_SKULL_QUERY,
        payload: Vec::new(),
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
        panel.feedback = "The server refused the request".into();
        return;
    }
    let Ok(icons) = wire::decode_party_icons(&result.payload) else {
        panel.feedback = "Invalid party icons reply".into();
        return;
    };
    panel.summary = if icons.is_empty() {
        "No visible party icons".into()
    } else {
        icons
            .iter()
            .map(|icon| {
                format!(
                    "Entity {:02x?} / {} | Shield: {} | Skull: {}",
                    icon.entity.identity,
                    icon.entity.generation,
                    shield(icon.shield),
                    skull(icon.skull)
                )
            })
            .collect::<Vec<_>>()
            .join("\r\n")
    };
    panel.feedback = "Party icons updated".into();
}
fn shield(value: u8) -> &'static str {
    [
        "None",
        "Invitation blue",
        "Invitation yellow",
        "Member",
        "Leader",
        "Shared member",
        "Shared leader",
        "Inactive blinking member",
        "Inactive blinking leader",
        "Inactive member",
        "Inactive leader",
        "Outsider gray",
    ][usize::from(value)]
}
fn skull(value: u8) -> &'static str {
    ["None", "Yellow", "Green", "White", "Red", "Black", "Orange"][usize::from(value)]
}
