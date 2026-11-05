use oteryn_client_runtime::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViews, ItemEntry,
};
use oteryn_client_ui::container_tree::ContainerPanel;

fn view(
    id: u8,
    container: u64,
    parent: Option<u64>,
    entry: u64,
) -> Result<ContainerView, Box<dyn std::error::Error>> {
    Ok(ContainerView {
        view_id: id,
        container_handle: container.try_into()?,
        parent_handle: parent.map(TryInto::try_into).transpose()?,
        capacity: 20,
        entries: vec![ItemEntry {
            handle: entry.try_into()?,
            item_definition_ref: 2854.try_into()?,
            count: 2.try_into()?,
            sub_type: 0,
        }],
        browse: None,
    })
}

#[test]
fn nested_navigation_uses_live_handles_and_server_view_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = ContainerPanel::default();
    panel.apply(Some(ContainerViews {
        views: vec![view(3, 10, None, 11)?],
    }));
    assert_eq!(panel.active, Some(3));
    assert_eq!(panel.rows()[0].handle.get(), 11);
    assert!(!panel.select(99.try_into()?));
    assert!(panel.select(11.try_into()?));
    assert!(panel.activate(3));
    assert_eq!(
        panel.selected_item().map(|item| item.handle.get()),
        Some(11)
    );
    assert_eq!(
        panel.open_selected(),
        Some(ContainerViewIntent::Open {
            handle: 11.try_into()?,
            replace_view: Some(3)
        })
    );
    assert!(panel.open_selected().is_none());
    panel.finish(Some("Opened"));
    panel.apply(Some(ContainerViews {
        views: vec![view(3, 11, Some(10), 12)?],
    }));
    assert!(panel.selected_item().is_none());
    assert_eq!(panel.up(), Some(ContainerViewIntent::Up { view_id: 3 }));
    panel.finish(None);
    assert_eq!(
        panel.close(),
        Some(ContainerViewIntent::Close { view_id: 3 })
    );
    panel.apply(Some(ContainerViews::default()));
    assert!(panel.rows().is_empty());
    assert_eq!(panel.active, None);
    panel.end();
    assert!(panel.open(10.try_into()?, None).is_none());
    Ok(())
}

#[test]
fn refusal_and_unavailable_state_do_not_invent_contents() -> Result<(), Box<dyn std::error::Error>>
{
    let mut panel = ContainerPanel::default();
    assert!(panel.open(1.try_into()?, None).is_none());
    panel.apply(Some(ContainerViews {
        views: vec![view(0, 1, None, 2)?],
    }));
    assert!(panel.up().is_none());
    assert!(panel.open(2.try_into()?, Some(0)).is_some());
    panel.finish(Some("Not a container"));
    assert_eq!(panel.rows()[0].handle.get(), 2);
    assert_eq!(panel.feedback, Some("Not a container"));
    panel.apply(None);
    assert!(panel.rows().is_empty());
    Ok(())
}

#[test]
fn a_new_ground_view_becomes_active_even_when_result_precedes_delta()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = ContainerPanel::default();
    let backpack = view(3, 10, None, 11)?;
    panel.apply(Some(ContainerViews {
        views: vec![backpack.clone()],
    }));
    assert!(panel.open(42.try_into()?, None).is_some());
    oteryn_client_ui::container_tree::protocol::apply_result(
        &mut panel,
        Some(oteryn_client_runtime::container_tree::ContainerViewOutcome::Opened),
    );
    panel.apply(Some(ContainerViews {
        views: vec![backpack, view(7, 42, None, 43)?],
    }));
    assert_eq!(panel.active, Some(7));
    assert_eq!(panel.rows()[0].handle.get(), 43);
    Ok(())
}

#[test]
fn ground_open_chord_requires_control_shift_and_a_fresh_press()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(14)?;
    let key = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let mods = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    assert!(oteryn_client_ui::container_tree::input::open_key(&[key(
        ButtonState::Pressed,
        mods,
        false
    )]));
    for event in [
        key(ButtonState::Pressed, mods, true),
        key(ButtonState::Released, mods, false),
        key(
            ButtonState::Pressed,
            Modifiers::one(Modifier::Control),
            false,
        ),
        key(ButtonState::Pressed, mods.with(Modifier::Alt), false),
    ] {
        assert!(!oteryn_client_ui::container_tree::input::open_key(&[event]));
    }
    Ok(())
}

#[test]
fn refused_open_keeps_the_previous_active_view() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::container_tree::ContainerViewOutcome;
    for outcome in [ContainerViewOutcome::Stale, ContainerViewOutcome::TooFar] {
        let mut panel = ContainerPanel::default();
        let views = ContainerViews {
            views: vec![view(3, 10, None, 11)?, view(7, 42, None, 43)?],
        };
        panel.apply(Some(views.clone()));
        assert!(panel.open(42.try_into()?, None).is_some());
        assert_eq!(panel.active, Some(3));
        panel.apply(Some(views));
        assert_eq!(panel.active, Some(3));
        oteryn_client_ui::container_tree::protocol::apply_result(&mut panel, Some(outcome));
        assert_eq!(panel.active, Some(3));
    }
    Ok(())
}

#[test]
fn open_focus_waits_for_acceptance_when_delta_arrives_first()
-> Result<(), Box<dyn std::error::Error>> {
    for existing in [vec![], vec![view(3, 10, None, 11)?]] {
        let previous = existing.first().map(|v| v.view_id);
        let mut panel = ContainerPanel::default();
        panel.apply(Some(ContainerViews {
            views: existing.clone(),
        }));
        assert!(panel.open(42.try_into()?, None).is_some());
        let mut updated = existing;
        updated.push(view(7, 42, None, 43)?);
        panel.apply(Some(ContainerViews { views: updated }));
        assert_eq!(panel.active, previous);
        oteryn_client_ui::container_tree::protocol::apply_result(
            &mut panel,
            Some(oteryn_client_runtime::container_tree::ContainerViewOutcome::Opened),
        );
        assert_eq!(panel.active, Some(7));
    }
    Ok(())
}

#[test]
fn up_keeps_the_originating_view_when_parent_is_already_open()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = ContainerPanel::default();
    let parent = view(1, 10, None, 11)?;
    let child = view(3, 11, Some(10), 12)?;
    panel.apply(Some(ContainerViews {
        views: vec![parent.clone(), child.clone()],
    }));
    assert!(panel.activate(3));
    assert_eq!(panel.up(), Some(ContainerViewIntent::Up { view_id: 3 }));
    oteryn_client_ui::container_tree::protocol::apply_result(
        &mut panel,
        Some(oteryn_client_runtime::container_tree::ContainerViewOutcome::Opened),
    );
    panel.apply(Some(ContainerViews {
        views: vec![parent, child],
    }));
    assert_eq!(panel.active, Some(3));
    panel.apply(Some(ContainerViews {
        views: vec![view(3, 10, None, 11)?],
    }));
    assert_eq!(panel.active, Some(3));
    Ok(())
}
