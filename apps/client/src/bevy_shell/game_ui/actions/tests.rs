use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
use bevy::asset::AssetPlugin;

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, _) = play_channel();
    assert!(link.spell_bar_bind(&["71".into(), "".into(), "".into(), "".into()]));
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    app.add_systems(Update, super::super::project);
    install(&mut app);
    app.update();
    Ok((app, commands))
}

fn slot(app: &mut App, wanted: usize) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &ActionControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, ActionControl::Slot(index) if *index == wanted).then_some(entity)
        })
        .ok_or_else(|| "slot".into())
}

#[test]
fn native_slot_activation_reaches_existing_cast_owner_without_local_prediction() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let slot = slot(&mut app, 0)?;
    let labels = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .spell_slot_labels();
    assert!(commands.try_recv().is_err());
    app.world_mut().trigger(Activate { entity: slot });
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(
        crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels(),
        labels
    );
    Ok(())
}

#[test]
fn stale_modal_disabled_and_empty_slots_never_dispatch_casts() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let first = slot(&mut app, 0)?;
    app.world_mut()
        .entity_mut(first)
        .insert(Generation(u64::MAX));
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(first).insert(Generation(0));
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: false,
            locked: true,
        });
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: true,
        });
    let empty = slot(&mut app, 1)?;
    app.world_mut().trigger(Activate { entity: empty });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn visible_slots_follow_real_computed_width_and_saved_scroll_offset() -> TestResult {
    let (mut app, _) = fixture()?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Scroll { bar: 0, by: 40 });
    for (dock, extent, expected) in [
        (crate::play::BarDock::Bottom, 218., 40..44),
        (crate::play::BarDock::Bottom, 110., 40..41),
        (crate::play::BarDock::Left, 230., 40..44),
    ] {
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        link.bar_gesture(BarGesture::Configure {
            bar: 0,
            dock,
            enabled: true,
            locked: true,
        });
        let side = dock != crate::play::BarDock::Bottom;
        for (bar, mut computed) in app
            .world_mut()
            .query::<(&BarRow, &mut ComputedNode)>()
            .iter_mut(app.world_mut())
        {
            if bar.0 == 0 {
                computed.size = if side {
                    Vec2::new(34., extent)
                } else {
                    Vec2::new(extent, 34.)
                };
                computed.inverse_scale_factor = 1.;
            }
        }
        app.update();
        let visible = app
            .world_mut()
            .query::<(&ActionControl, &Node)>()
            .iter(app.world())
            .filter_map(|(control, node)| match control {
                ActionControl::Slot(index)
                    if *index < ROW_SLOTS && node.display != Display::None =>
                {
                    Some(*index)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let lock_extent = if side { LOCK_SIZE.y } else { LOCK_SIZE.x };
        let slots = visible.len() as f32;
        let used = slots * SLOT_SIZE + 2. * SCROLL_SIZE + lock_extent + (slots + 2.) * SLOT_GAP;
        assert!(
            used <= extent,
            "lock ends at {used} in a {extent}-pixel row"
        );
        assert_eq!(visible, expected.collect::<Vec<_>>());
    }
    Ok(())
}

#[test]
fn unlocking_and_selecting_slot_opens_shared_editor_without_casting() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let lock = app
        .world_mut()
        .query::<(Entity, &ActionControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control, ActionControl::Lock(0)).then_some(entity))
        .ok_or("lock")?;
    app.world_mut().trigger(Activate { entity: lock });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .bar_layout()
            .rows[0]
            .locked
    );
    let first = slot(&mut app, 0)?;
    app.insert_resource(UiScale(1.5));
    // Editing is an explicit secondary click; ordinary activation still executes.
    app.world_mut().trigger(PointerClick {
        entity: first,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 1000,
                    height: 600,
                },
                position: Vec2::new(200., 100.),
            },
        ),
        button: bevy::picking::pointer::PointerButton::Secondary,
        hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<&ChildOf, With<popup::PopupEntry>>()
        .iter(app.world())
        .next()
        .ok_or("popup entry")?
        .parent();
    let frame = app.world().get::<Node>(parent).ok_or("popup frame")?;
    assert_eq!((frame.left, frame.top), (px(200. / 1.5), px(100. / 1.5)));
    // The secondary click opens the Tibia slot menu; its Assign Spell entry opens the editor.
    let mut labels = Vec::new();
    let mut assign = None;
    for (entity, entry) in app
        .world_mut()
        .query::<(Entity, &MenuEntry)>()
        .iter(app.world())
    {
        labels.push(entry.choice);
        if entry.choice == MenuChoice::AssignSpell {
            assign = Some(entity);
        }
    }
    assert_eq!(labels.len(), 8);
    for choice in [
        MenuChoice::AssignSpell,
        MenuChoice::AssignObject,
        MenuChoice::SetText,
        MenuChoice::EditHotkey,
        MenuChoice::Clear,
        MenuChoice::Lock,
        MenuChoice::Hide,
        MenuChoice::ResetZoom,
    ] {
        assert!(labels.contains(&choice), "{choice:?}");
    }
    assert!(!app.world().resource::<editor::EditorState>().active());
    app.world_mut().trigger(Activate {
        entity: assign.ok_or("Assign Spell")?,
    });
    app.update();
    assert!(app.world().resource::<SlotMenu>().0.is_none());
    assert!(app.world().resource::<editor::EditorState>().active());
    assert!(commands.try_recv().is_err());
    // Disconnect clears the session-bound edit and its text focus.
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(!app.world().resource::<editor::EditorState>().active());
    assert!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get()
            .is_none()
    );
    Ok(())
}

#[test]
fn action_button_obeys_existing_server_observed_death_gate() -> TestResult {
    let (mut app, _) = fixture()?;
    let (link, mut commands, outbox) = play_channel();
    assert!(link.spell_bar_bind(&["71".into(), "".into(), "".into(), "".into()]));
    outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ActorVitals {
                health: 0,
                max_health: 100,
                ..Default::default()
            },
        },
    )]);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let first = slot(&mut app, 0)?;
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_drop_swaps_only_unlocked_slots_then_casts_new_binding() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDragDrop},
        pointer::{Location, PointerButton, PointerId},
    };
    let (mut app, mut commands) = fixture()?;
    let from = slot(&mut app, 0)?;
    let to = slot(&mut app, 1)?;
    // Pointer traversal must terminate at a real window, rather than revisit an empty entity.
    let window = app.world_mut().spawn(Window::default()).id();
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window)
            .normalize(None)
            .ok_or("window")?,
    );
    let drop = || PointerDragDrop {
        entity: to,
        dropped: from,
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
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("71")
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: false,
        });
    super::super::modal::install(&mut app);
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(80)))
        .id();
    app.world_mut().trigger(drop());
    let original = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .spell_slot_labels();
    assert!(original[0].contains("71"));
    assert!(
        !original[1].contains("71"),
        "background drop changed the binding behind the modal"
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.world_mut().trigger(drop());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[1]
            .contains("71")
    );
    assert!(commands.try_recv().is_err());
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: true,
        });
    app.world_mut().trigger(Activate { entity: to });
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    Ok(())
}

#[test]
fn inventory_modal_blocks_prior_action_slot_focus_enter_and_direct_activation() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
    for keyboard in [true, false] {
        let (mut app, mut commands) = fixture()?;
        app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        super::super::modal::install(&mut app);
        let button = slot(&mut app, 0)?;
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(button, FocusCause::Navigated);
        app.world_mut()
            .spawn((super::super::GameModal, Node::default(), GlobalZIndex(80)));
        if keyboard {
            let window = app.world_mut().spawn_empty().id();
            let key = KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            };
            app.world_mut()
                .trigger(FocusedInput::new(button, key, window));
            app.world_mut().flush();
        } else {
            app.world_mut().trigger(Activate { entity: button });
        }
        assert!(
            commands.try_recv().is_err(),
            "background cast escaped the local modal fence"
        );
    }
    Ok(())
}

#[test]
fn unlocked_action_slot_still_executes_ordinary_activation() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let entity = slot(&mut app, 0)?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: false,
        });
    app.world_mut().trigger(Activate { entity });
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    assert!(!app.world().resource::<editor::EditorState>().active());
    Ok(())
}

#[test]
fn side_bars_project_their_dock_and_vertical_capacity() -> TestResult {
    let (mut app, _) = fixture()?;
    for (bar, dock) in [
        (1, crate::play::BarDock::Left),
        (2, crate::play::BarDock::Right),
    ] {
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .bar_gesture(BarGesture::Configure {
                bar,
                dock,
                enabled: true,
                locked: true,
            });
    }
    app.update();
    let layout = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_layout();
    for (bar, node, parent) in app
        .world_mut()
        .query::<(&BarRow, &Node, &ChildOf)>()
        .iter(app.world())
    {
        if layout.rows[bar.0].dock != crate::play::BarDock::Bottom {
            assert_eq!(node.flex_direction, FlexDirection::Column);
            assert_ne!(
                parent.parent(),
                app.world().resource::<super::super::GameRegions>().bars
            );
            assert_eq!(node.width, px(SLOT_SIZE));
        } else {
            assert_eq!(node.flex_direction, FlexDirection::Row);
        }
    }
    Ok(())
}

#[test]
fn slot_menu_lists_tibia_entries_and_the_bar_lock_state() {
    let labels = |locked| {
        menu_items(locked)
            .into_iter()
            .map(|(item, _)| match item {
                popup::Item::Entry { label, checked, .. } => {
                    format!(
                        "{label}{}",
                        checked.map_or("", |on| if on { " [x]" } else { " [ ]" })
                    )
                }
                popup::Item::Separator => "-".to_owned(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels(true),
        [
            "Assign Spell",
            "Assign Object",
            "Set Text",
            "Edit Hotkey",
            "Clear",
            "-",
            "Lock Action Bar [x]",
            "Hide Action Bar",
            "-",
            "Reset Zoom",
        ]
    );
    assert_eq!(labels(false)[6], "Lock Action Bar [ ]");
}

#[test]
fn each_bar_has_a_twelve_by_sixteen_lock_and_no_configuration_row() -> TestResult {
    let (mut app, _) = fixture()?;
    let mut locks = app
        .world_mut()
        .query::<(&ActionControl, &Node, &Children)>();
    let mut found = Vec::new();
    for (control, node, children) in locks.iter(app.world()) {
        if let ActionControl::Lock(bar) = control {
            found.push(*bar);
            assert_eq!((node.width, node.height), (px(12.), px(16.)));
            assert!(
                children
                    .iter()
                    .any(|child| app.world().get::<art::ArtIcon>(child).is_some())
            );
        }
    }
    found.sort_unstable();
    assert_eq!(found, (0..9).collect::<Vec<_>>());
    // The removed "1 on / Locked" row left no text captions behind.
    let captions: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(
        !captions
            .iter()
            .any(|text| text == "Locked" || text.ends_with(" on"))
    );
    Ok(())
}

#[test]
fn hotkey_labels_abbreviate_modifiers_like_the_tibia_bar() {
    assert_eq!(projection::hotkey_label("F1"), "F1");
    assert_eq!(projection::hotkey_label("CTRL+F2"), "C+F2");
    assert_eq!(projection::hotkey_label("SHIFT+ALT+3"), "S+A+3");
    assert_eq!(projection::hotkey_label("SUPER+J"), "M+J");
    assert_eq!(projection::hotkey_label(""), "");
}

#[test]
fn slots_show_their_bound_hotkey_and_follow_a_150_percent_bar_zoom() -> TestResult {
    use crate::bevy_shell::gameplay_dialogs::scale::{self, Module, ModuleScale};
    let (mut app, _) = fixture()?;
    let mut zoom = ModuleScale::default();
    zoom.set(Module::ActionBars, 1.5);
    app.insert_resource(zoom)
        .add_systems(PostUpdate, scale::apply);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .rebind("local-spell-action-bar-spell-1", "CTRL+F1")?;
    app.update();
    let label = app
        .world_mut()
        .query::<(&SlotHotkey, &Text)>()
        .iter(app.world())
        .find_map(|(hotkey, text)| (hotkey.0 == 0).then(|| text.0.clone()))
        .ok_or("slot hotkey")?;
    assert_eq!(label, "C+F1");
    let first = slot(&mut app, 0)?;
    let node = app.world().get::<Node>(first).ok_or("slot node")?;
    assert_eq!((node.width, node.height), (px(51.), px(51.)));
    Ok(())
}

#[test]
fn clear_removes_assigned_fourth_bar_action() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let generation = {
        let owner = app.world().non_send::<HostOwner>();
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .bar_gesture(BarGesture::Configure {
                bar: 3,
                dock: crate::play::BarDock::Bottom,
                enabled: true,
                locked: false,
            });
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.assign_bar_action(
            150,
            crate::play::BarAction::Text {
                text: "hello".into(),
                send_automatically: true,
            }
        ));
        assert!(link.spell_slot_tooltips()[150].contains("hello"));
        owner.generation()
    };
    let entry = app
        .world_mut()
        .spawn(MenuEntry {
            slot: 3 * ROW_SLOTS,
            generation,
            choice: MenuChoice::Clear,
        })
        .observe(menu_action)
        .id();
    app.world_mut().trigger(Activate { entity: entry });
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(!link.spell_slot_tooltips()[150].contains("hello"));
    assert!(!app.world().resource::<editor::EditorState>().active());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn each_side_shows_an_enabled_bar_on_its_own_dock() -> TestResult {
    for (bar, dock) in [
        (3, crate::play::BarDock::Bottom),
        (5, crate::play::BarDock::Left),
        (7, crate::play::BarDock::Right),
    ] {
        let (mut app, _) = fixture()?;
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            let mut preferences = owner.settings.preferences.clone();
            preferences.bar_layout.rows[bar].enabled = true;
            owner.settings.apply(preferences);
            let owner = &mut *owner;
            let link = &owner.play.as_ref().ok_or("play")?.link;
            owner.settings.sync_bar_actions(link);
        }
        app.update();
        app.update();
        let anchor = app
            .world_mut()
            .query::<(Entity, &DockAnchor)>()
            .iter(app.world())
            .find_map(|(entity, anchor)| (anchor.0 == dock).then_some(entity))
            .ok_or("anchor")?;
        let mut shown = Vec::new();
        for (row, node, parent) in app
            .world_mut()
            .query::<(&BarRow, &Node, &ChildOf)>()
            .iter(app.world())
        {
            if node.display != Display::None {
                shown.push(row.0);
            }
            if row.0 == bar {
                assert_eq!(parent.parent(), anchor, "bar {bar} sits on its dock");
                let vertical = dock != crate::play::BarDock::Bottom;
                assert_eq!(
                    node.flex_direction,
                    if vertical {
                        FlexDirection::Column
                    } else {
                        FlexDirection::Row
                    }
                );
            }
        }
        shown.sort_unstable();
        assert_eq!(
            shown,
            vec![0, bar],
            "only the default and the enabled bar show"
        );
    }
    Ok(())
}
