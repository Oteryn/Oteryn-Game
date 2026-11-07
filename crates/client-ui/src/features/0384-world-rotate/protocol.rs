use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target};

pub fn request(
    target: Target,
    observed_definition: Option<std::num::NonZeroU32>,
    selection: Option<Selection>,
) -> Option<Command> {
    if !matches!(
        (&target, observed_definition),
        (Target::Item(_), Some(_)) | (Target::Object(_), None)
    ) {
        return None;
    }
    Some(Command {
        target,
        action: Action::Rotate(observed_definition),
        selection,
    })
}

use super::{InventoryProbe, MapBinding};
use oteryn_client_runtime::inventory::ItemEntry;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_runtime::world_interaction::world_rotate;

pub fn probe_request(
    state: &mut InventoryProbe,
    item: ItemEntry,
    map: MapBinding,
) -> Option<FeatureCommand> {
    if !state.begin(item, map) {
        return None;
    }
    Some(FeatureCommand {
        capability: world_rotate::CAPABILITY_WORLD_ROTATE_V1,
        kind: world_rotate::COMMAND_TYPE_ROTATE_WORLD_ITEM,
        payload: world_rotate::encode_rotate_probe(item.handle, item.item_definition_ref),
    })
}

pub fn apply_probe_result(
    state: &mut InventoryProbe,
    result: &GatedCommandResult,
    item: Option<ItemEntry>,
    map: Option<MapBinding>,
) {
    let allowed = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && world_rotate::decode_rotate_affordance(&result.payload).unwrap_or(false);
    state.finish(item, map, allowed);
}
