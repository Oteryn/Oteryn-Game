use super::*;
use oteryn_client_runtime::world_interaction::{Action, Target, rotate_payload, world_rotate};

#[test]
fn rotate_request_keeps_target_and_separate_action() -> Result<(), Box<dyn std::error::Error>> {
    let handle = oteryn_client_runtime::inventory::ItemHandle::MIN;
    let command = request(Target::Item(handle), Some(std::num::NonZeroU32::MIN), None)
        .ok_or("rotate command")?;
    assert_eq!(
        command.action,
        Action::Rotate(Some(std::num::NonZeroU32::MIN))
    );
    let payload = rotate_payload(&command.target, Some(std::num::NonZeroU32::MIN))?;
    let (intent, observed) =
        world_rotate::decode_rotate_intent(&payload).map_err(|_| "rotate decode")?;
    assert_eq!(observed, Some(std::num::NonZeroU32::MIN));
    assert_eq!(intent.target, world_rotate::UseTarget::Item(handle));
    assert!(request(Target::Item(handle), None, None).is_none());
    assert!(
        request(
            Target::Creature(oteryn_client_runtime::world_interaction::EntityRef {
                identity: [1; 16],
                generation: 1,
            }),
            None,
            None
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn inventory_probe_binds_handle_definition_and_inventory_reset_generation()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::inventory::ItemEntry;
    use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
    let one = ItemEntry {
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let context = ([1; 32], [2; 32], 1);
    let reply = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: world_rotate::encode_rotate_affordance(true),
    };
    let mut unsupported = InventoryProbe::new(false);
    assert!(probe_request(&mut unsupported, one, context).is_none());
    let mut accepted = InventoryProbe::new(true);
    let query = probe_request(&mut accepted, one, context).ok_or("inventory probe")?;
    assert_eq!((query.capability, query.kind), (388, 670));
    let request =
        world_rotate::decode_rotate_request(&query.payload).map_err(|_| "probe payload")?;
    assert!(request.probe);
    assert_eq!(request.observed_definition, Some(one.item_definition_ref));
    assert!(!accepted.available(one, context));
    apply_probe_result(&mut accepted, &reply, Some(one), Some(context));
    assert!(accepted.available(one, context));
    assert!(probe_request(&mut accepted, one, context).is_none());
    for stale in 0..4 {
        let mut state = InventoryProbe::new(true);
        assert!(probe_request(&mut state, one, context).is_some());
        let mut current = one;
        let mut current_context = context;
        match stale {
            0 => state.inventory_changed(),
            1 => current.handle = std::num::NonZeroU64::new(2).ok_or("handle")?,
            2 => current.item_definition_ref = std::num::NonZeroU32::new(2).ok_or("definition")?,
            _ => current_context.2 += 1,
        }
        // An in-flight slot is never overwritten by another selected item.
        assert!(probe_request(&mut state, current, current_context).is_none());
        apply_probe_result(&mut state, &reply, Some(current), Some(current_context));
        assert!(!state.available(current, current_context));
        assert!(probe_request(&mut state, current, current_context).is_some());
    }
    let mut invalid = InventoryProbe::new(true);
    assert!(probe_request(&mut invalid, one, context).is_some());
    let mut bad = reply;
    bad.payload = vec![2];
    apply_probe_result(&mut invalid, &bad, Some(one), Some(context));
    assert!(!invalid.available(one, context));
    assert!(probe_request(&mut invalid, one, context).is_none());
    Ok(())
}
