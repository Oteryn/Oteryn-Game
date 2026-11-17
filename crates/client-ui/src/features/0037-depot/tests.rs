use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use std::num::{NonZeroU32, NonZeroU64};
fn reply(outcome: DepotOutcome) -> DepotReply {
    DepotReply {
        outcome,
        town: Some(2),
        box_number: 17,
        page: 0,
        total: 1,
        entries: vec![oteryn_client_runtime::depot::DepotEntry {
            ordinal: 81,
            definition: 9,
            count: 3,
        }],
    }
}
#[test]
fn committed_reply_replaces_grid_and_stale_selection_cannot_move() {
    let mut p = DepotPanel {
        selected: true,
        ..Default::default()
    };
    apply_result(&mut p, Some(reply(DepotOutcome::Opened)));
    assert_eq!(
        request(&mut p, Intent::Withdraw(0)),
        Some(DepotIntent {
            town: Some(2),
            box_number: 17,
            page: 0,
            action: DepotAction::Withdraw(81)
        })
    );
    assert_eq!(p.reply.as_ref().map(|r| r.entries.len()), Some(1));
    let mut moved = reply(DepotOutcome::Moved);
    moved.entries.clear();
    moved.total = 0;
    apply_result(&mut p, Some(moved));
    assert_eq!(p.feedback, "Item moved");
    assert!(request(&mut p, Intent::Withdraw(0)).is_none());
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 18,
                page: 0
            }
        )
        .is_none()
    );
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 1,
                page: 469
            }
        )
        .is_none()
    );
}
#[test]
fn deposit_uses_live_inventory_or_container_handles() {
    let mut p = DepotPanel {
        selected: true,
        reply: Some(reply(DepotOutcome::Opened)),
        ..Default::default()
    };
    let item = ItemEntry {
        appearance_id: 0,
        handle: NonZeroU64::MIN,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    };
    assert!(request(&mut p, Intent::Deposit(item.handle)).is_none());
    p.items.views.push(item);
    assert_eq!(
        request(&mut p, Intent::Deposit(item.handle)).map(|r| r.action),
        Some(DepotAction::Deposit(item.handle))
    );
    assert!(request(&mut p, Intent::Deposit(item.handle)).is_none());
    apply_result(&mut p, None);
    assert!(p.reply.is_none());
    assert!(request(&mut p, Intent::Deposit(item.handle)).is_none());
}
#[test]
fn every_refusal_clears_pending_and_projects_text() {
    for outcome in [
        DepotOutcome::NoLocker,
        DepotOutcome::Stale,
        DepotOutcome::Full,
        DepotOutcome::NoBackpack,
        DepotOutcome::Overweight,
        DepotOutcome::Unsupported,
        DepotOutcome::Rejected,
    ] {
        let mut p = DepotPanel {
            pending: true,
            ..Default::default()
        };
        apply_result(&mut p, Some(reply(outcome)));
        assert!(!p.pending);
        assert!(!p.feedback.is_empty());
        assert!(panel_text(&p).contains(p.feedback));
    }
}
#[test]
fn open_chord_ignores_repeat_and_extra_modifiers() -> Result<(), oteryn_input_actions::InputError> {
    let key = KeyCode::new(15)?;
    let event = |repeat, modifiers| NormalizedInputEvent::Key {
        code: key,
        state: ButtonState::Pressed,
        modifiers,
        repeat,
    };
    let modifiers = Modifiers::NONE
        .with(Modifier::Control)
        .with(Modifier::Shift);
    assert!(open_key(&[event(false, modifiers)]));
    assert!(!open_key(&[event(
        false,
        Modifiers::NONE.with(Modifier::Control)
    )]));
    assert!(!open_key(&[event(false, modifiers.with(Modifier::Super))]));
    assert!(!open_key(&[event(true, modifiers)]));
    assert!(!open_key(&[event(false, modifiers.with(Modifier::Alt))]));
    Ok(())
}

#[test]
fn container_source_requires_live_handle_capability_and_one_pending_request() {
    let mut p = DepotPanel {
        selected: true,
        ..Default::default()
    };
    let item = ItemEntry {
        appearance_id: 0,
        handle: NonZeroU64::MIN,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    };
    p.items.inventory.main_backpack = Some(item);
    assert!(request_container(&mut p, item.handle).is_none());
    p.items.containers_selected = true;
    assert_eq!(request_container(&mut p, item.handle), Some(item.handle));
    assert!(request_container(&mut p, item.handle).is_none());
    apply_container(&mut p, Some(ContainerViewOutcome::Opened));
    assert!(!p.pending);
    p.items.views.push(item);
    assert_eq!(deposit_items(&p), vec![item]);
    p.reply = Some(reply(DepotOutcome::Opened));
    p.items.inventory.main_backpack = None;
    assert_eq!(
        request(&mut p, Intent::Deposit(item.handle)).map(|r| r.action),
        Some(DepotAction::Deposit(item.handle))
    );
    for outcome in [
        None,
        Some(ContainerViewOutcome::Stale),
        Some(ContainerViewOutcome::TooFar),
        Some(ContainerViewOutcome::NotAContainer),
        Some(ContainerViewOutcome::TooManyViews),
    ] {
        apply_container(&mut p, outcome);
        assert!(!p.pending);
        assert!(!p.feedback.is_empty());
    }
}

/// Tibia Global's Manual Sort Mode in a depot box: a deposit is held until the player picks the
/// box slot (a displayed entry row); the request then names that slot. Clicking the held item
/// again deposits it at the first slot, and without the mode nothing is held.
#[test]
fn manual_sort_deposit_holds_the_item_until_a_box_slot_is_chosen() {
    let mut p = DepotPanel {
        selected: true,
        ..Default::default()
    };
    let mut page = reply(DepotOutcome::Opened);
    page.page = 1;
    page.total = 33;
    apply_result(&mut p, Some(page));
    let item = ItemEntry {
        appearance_id: 0,
        handle: NonZeroU64::MIN,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    };
    p.items.inventory.main_backpack = Some(item);
    let deposit = DepotIntent {
        town: Some(2),
        box_number: 17,
        page: 1,
        action: DepotAction::Deposit(item.handle),
    };
    assert_eq!(
        request_in_mode(&mut p, Intent::Deposit(item.handle), true),
        None
    );
    assert_eq!(p.held, Some(item.handle));
    assert!(!p.pending);
    assert_eq!(
        request_in_mode(&mut p, Intent::Withdraw(0), true),
        Some((deposit, Some(32)))
    );
    assert_eq!(p.held, None);
    p.pending = false;
    assert_eq!(
        request_in_mode(&mut p, Intent::Deposit(item.handle), true),
        None
    );
    assert_eq!(
        request_in_mode(&mut p, Intent::Deposit(item.handle), true),
        Some((deposit, None))
    );
    p.pending = false;
    assert_eq!(
        request_in_mode(&mut p, Intent::Deposit(item.handle), true),
        None
    );
    let mut far = reply(DepotOutcome::Opened);
    far.page = 15;
    far.total = 513;
    far.entries.resize(32, far.entries[0].clone());
    apply_result(&mut p, Some(far));
    assert_eq!(p.placement_slot(19), Some(499));
    assert!(request_in_mode(&mut p, Intent::Withdraw(20), true).is_none());
    assert_eq!(p.held, Some(item.handle));
    assert!(!p.pending);
    let mut page = reply(DepotOutcome::Opened);
    page.page = 1;
    apply_result(&mut p, Some(page));
    // Turning the mode off drops the held item; a deposit goes out at once.
    assert_eq!(
        request_in_mode(&mut p, Intent::Deposit(item.handle), false),
        Some((deposit, None))
    );
    assert_eq!(p.held, None);
}
