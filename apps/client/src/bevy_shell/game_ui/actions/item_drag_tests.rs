use super::*;
use crate::play::{BarAction, ItemUseMode, PlayLink, SettingsState, play_channel};
use bevy::picking::{
    backend::HitData,
    events::{Pointer, PointerDragDrop},
    pointer::{Location, PointerButton, PointerId},
};

#[test]
fn inventory_drop_assigns_current_item_without_move_and_rejects_stale_source()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _) = tests::fixture()?;
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    let item = oteryn_session::ItemEntry {
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 3.try_into()?,
        sub_type: 0,
    };
    let seed = |items| {
        PlayLink::bevy_items_fixture(
            &outbox,
            Some(oteryn_session::CharacterInventory {
                main_backpack: None,
                entries: items,
                equipment: Vec::new(),
            }),
            Some(Default::default()),
            true,
            true,
        )
    };
    seed(vec![item]);
    assert!(link.bar_gesture(BarGesture::Configure {
        bar: 0,
        dock: crate::play::BarDock::Bottom,
        enabled: true,
        locked: false,
    }));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let generation = app.world().non_send::<HostOwner>().generation();
    let to = app
        .world_mut()
        .query::<(Entity, &ActionControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control, ActionControl::Slot(0)).then_some(entity))
        .ok_or("slot")?;
    let source = app
        .world_mut()
        .spawn(super::super::ItemDragSource {
            generation: generation.saturating_add(1),
            item,
        })
        .id();
    let window = app.world_mut().spawn_empty().id();
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window)
            .normalize(None)
            .ok_or("window")?,
    );
    let drop = || PointerDragDrop {
        entity: to,
        dropped: source,
        button: PointerButton::Primary,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: target.clone(),
                position: Vec2::ZERO,
            },
        ),
        hit: HitData::new(window, 0., None, None),
    };
    app.world_mut().trigger(drop());
    let mut settings = SettingsState::new(None);
    settings.sync_bar_actions(
        &app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link,
    );
    assert!(settings.preferences.bar_actions.0.is_empty());
    app.world_mut()
        .entity_mut(source)
        .insert(super::super::ItemDragSource { generation, item });
    seed(Vec::new());
    app.world_mut().trigger(drop());
    settings.sync_bar_actions(
        &app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link,
    );
    assert!(settings.preferences.bar_actions.0.is_empty());
    // A count-only server update preserves the displayed identity and remains assignable.
    seed(vec![oteryn_session::ItemEntry {
        count: 7.try_into()?,
        ..item
    }]);
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(
        link.item_drag_begin(oteryn_client_ui::inventory::movement::DragItem {
            equipped: false,
            handle: item.handle,
            count: 3,
        })
    );
    app.world_mut().trigger(drop());
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    settings.sync_bar_actions(link);
    assert_eq!(
        settings.preferences.bar_actions.0.get(&0),
        Some(&BarAction::Use {
            definition: item.item_definition_ref,
            mode: ItemUseMode::Cursor,
        })
    );
    assert!(link.item_movement().dragged.is_none());
    assert!(link.item_movement().prompt.is_none());
    assert!(!link.item_movement().pending);
    assert!(
        commands.try_recv().is_err(),
        "assignment must neither move nor use the item"
    );
    Ok(())
}
