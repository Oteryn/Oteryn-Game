//! Larger read-only predicates share the native Ground matcher's geometric bound.
use super::Callback;
use crate::quest::ground_stack::{GroundStackRequirement, StackItemRequirement, match_readset};

pub(super) fn admitted(callback: &Callback) -> bool {
    if callback.world_requirements.len() != 36
        || !callback.requires_world_admission
        || !callback.supports_ground_stack_guard()
    {
        return false;
    }
    let mut required = Vec::with_capacity(36);
    for requirement in &callback.world_requirements {
        let Some(stack) = &requirement.ground_stack else {
            return false;
        };
        if stack.len() != 1 || stack[0].count != 1 {
            return false;
        }
        required.push(GroundStackRequirement {
            position: requirement.position,
            items: stack
                .iter()
                .map(|item| StackItemRequirement {
                    item: item.item.clone(),
                    // Shape validation only; runtime and durable owners resolve pinned revisions.
                    revision: "shape-only".into(),
                    quantity: item.count,
                })
                .collect(),
        });
    }
    // No observations are fabricated. An admitted missing readset returns None;
    // invalid geometry, quantities or duplicate positions return an error.
    matches!(match_readset(&required, &[]), Ok(None))
}
