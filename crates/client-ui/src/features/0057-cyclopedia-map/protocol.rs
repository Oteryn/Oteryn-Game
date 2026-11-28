use super::{CurrentSubarea, Panel};
use oteryn_client_runtime::{
    cyclopedia_map::decode_current_subarea,
    raw_feature::{CommandStatus, GatedCommandResult},
};

pub fn apply_current_subarea(panel: &mut Panel, result: &GatedCommandResult) {
    panel.current_subarea = match (result.status, decode_current_subarea(&result.payload)) {
        (CommandStatus::Accepted | CommandStatus::DuplicateReplay, Ok(Some((region, subarea)))) => {
            CurrentSubarea::Member { region, subarea }
        }
        (CommandStatus::Accepted | CommandStatus::DuplicateReplay, Ok(None)) => {
            CurrentSubarea::Outside
        }
        _ => CurrentSubarea::Unavailable,
    };
}

pub fn request(panel: &mut Panel, kind: u32) -> bool {
    panel.begin(kind)
}
