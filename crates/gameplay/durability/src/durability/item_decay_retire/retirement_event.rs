//! One shared before/after event builder; only the admitted cause selects its grammar.
use super::*;

pub(super) fn decay_message(
    step: DecayRetireStep,
    fence: &CorpseDecayFence,
    admitted: &Admitted,
) -> OneItemDecayRetireV1 {
    let state = |quantity, lifecycle| OneItemStateV1 {
        item_instance_id: step.item_instance_id.to_vec(),
        world_id: admitted.world_id.to_vec(),
        definition: Some(admitted.definition.clone()),
        quantity,
        lifecycle,
        key_number: admitted.key_number,
    };
    let entry = admitted
        .placement_ordinal
        .map(|placement_ordinal| OneItemCorpseSourceV1 {
            corpse_item_instance_id: step.corpse_item_instance_id.to_vec(),
            placement_ordinal,
            corpse_ground: Some(admitted.corpse_ground.clone()),
        });
    let mut message = OneItemDecayRetireV1 {
        world_reset: None,
        ground_container_entry: None,
        before: Some(state(admitted.quantity_before, ITEM_LIFECYCLE_LIVE)),
        after: Some(state(0, ITEM_LIFECYCLE_RETIRED)),
        ground: entry.is_none().then(|| admitted.corpse_ground.clone()),
        corpse_entry: entry,
        cause: Some(OneItemCorpseDecayV1 {
            corpse_item_instance_id: step.corpse_item_instance_id.to_vec(),
            deadline_unix_ms: u64::try_from(admitted.deadline_unix_ms).unwrap_or(0),
        }),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
    };
    world_reset_cause::bind_message(&mut message, step.cause);
    message
}
