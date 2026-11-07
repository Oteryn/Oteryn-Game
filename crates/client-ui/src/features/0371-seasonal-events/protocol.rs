use crate::world_event::WorldEventPanel;
use oteryn_client_runtime::raw_feature::GatedCommandResult;

pub fn apply_result(panel: &mut WorldEventPanel, result: &GatedCommandResult) {
    crate::world_event::apply_result(panel, result);
}
