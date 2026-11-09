use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    container_sort as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::container_sort::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, _: Intent) -> Result<FeatureCommand, String> {
    let mut bytes = [0; 4];
    for (byte, field) in bytes.iter_mut().zip(&panel.fields) {
        *byte = field
            .trim()
            .parse()
            .map_err(|_| tr!("Enter numeric sort options"))?;
    }
    let request =
        wire::Request::decode(&bytes).ok_or(tr!("Choose view 0-15, key 1-4 and flags 0-1"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_CONTAINER_SORT_V1,
        kind: wire::COMMAND_TYPE_CONTAINER_SORT,
        payload: request.encode().to_vec(),
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
    panel.feedback = if !accepted(result) {
        tr!("The server refused the container sort")
    } else {
        match result.payload.as_slice() {
            [1] => {
                panel.summary =
                    tr!("Container sorted; the inventory reflects server item updates").into();
                tr!("Sort accepted")
            }
            [2] => tr!("The server refused the container sort"),
            _ => tr!("Invalid container sort reply"),
        }
    }
    .into();
}
