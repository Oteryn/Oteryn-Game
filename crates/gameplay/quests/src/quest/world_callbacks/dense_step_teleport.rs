//! A solved dense Ground puzzle may use the existing post-receipt step teleport.
use super::{Callback, Trigger};

pub(super) fn admitted(callback: &Callback) -> bool {
    callback.teleport.is_some()
        && callback.trigger == Trigger::StepIn
        && callback.requires_world_admission
        && callback.world_requirements.len() == 36
    // The parser additionally requires the shared dense geometry validator;
    // supports_ground_stack_guard retains its no-value/no-mutation restrictions.
}
