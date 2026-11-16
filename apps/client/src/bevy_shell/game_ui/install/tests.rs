use super::*;
#[test]
fn container_tool_shortcuts_present_the_retained_tools() -> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::owner::{ActivePlay, HostOwner, Phase};
    use oteryn_input_actions::{
        ButtonState, KeyCode as NormalizedKeyCode, Modifier, Modifiers, NormalizedInputEvent,
    };
    for (code, modifiers, title) in [
        (
            5,
            Modifiers::one(Modifier::Control).with(Modifier::Shift),
            "Browse Field",
        ),
        (
            15,
            Modifiers::one(Modifier::Control).with(Modifier::Shift),
            "Depot",
        ),
        (
            4,
            Modifiers::one(Modifier::Control).with(Modifier::Shift),
            "Depot Search",
        ),
        (
            22,
            Modifiers::one(Modifier::Control).with(Modifier::Alt),
            "Container Sort",
        ),
    ] {
        let mut owner = HostOwner::new(None)?;
        let (link, _commands, _outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 0, &[])?;
        view.use_shared_frontend();
        owner.play = Some(ActivePlay { view, link });
        owner.phase = Phase::Playing;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            oteryn_client_ui_bevy::GlobalUiPlugin,
        ))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<super::super::viewport::WorldViewport>()
        .insert_non_send(owner)
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::setup).chain(),
        )
        .add_systems(Update, super::super::project);
        install(&mut app);
        app.update();
        app.world_mut().non_send_mut::<HostOwner>().route_input(
            &[NormalizedInputEvent::Key {
                code: NormalizedKeyCode::new(code)?,
                state: ButtonState::Pressed,
                modifiers,
                repeat: false,
            }],
            false,
        );
        app.update();
        assert!(
            app.world_mut()
                .query::<&Name>()
                .iter(app.world())
                .any(|name| name.as_str() == title),
            "{title} needs an installed presenter"
        );
    }
    Ok(())
}

#[test]
fn parity_a_container_sort_controls_dispatch_both_directions()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::owner::{ActivePlay, Phase};
    use oteryn_client_runtime::container_tree::{ContainerView, ContainerViews};
    for descending in [false, true] {
        let mut owner = HostOwner::new(None)?;
        let (link, mut requests, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_sort_fixture();
        crate::play::PlayLink::bevy_items_fixture(
            &outbox,
            None,
            Some(ContainerViews {
                views: vec![ContainerView {
                    view_id: 0,
                    container_handle: 7.try_into()?,
                    parent_handle: None,
                    capacity: 20,
                    entries: vec![],
                    browse: None,
                }],
            }),
            false,
            false,
        );
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        owner.play = Some(ActivePlay { view, link });
        owner.phase = Phase::Playing;
        assert!(owner.request_container_tool(ContainerTool::Sort, None, None));
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Font>()
            .init_asset::<Image>()
            .insert_non_send(owner)
            .add_systems(Startup, crate::bevy_shell::ui::setup);
        install_container_tools(&mut app);
        app.update();
        let direction = if descending {
            "descending"
        } else {
            "ascending"
        };
        let button = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| {
                (text.0 == format!("Sort container 0 by name {direction}"))
                    .then_some(parent.parent())
            })
            .ok_or("sort direction control")?;
        app.world_mut()
            .trigger(bevy::ui_widgets::Activate { entity: button });
        let crate::combat_input::PlayCommand::Feature(command) = requests.try_recv()? else {
            return Err("sort request".into());
        };
        let request = oteryn_client_runtime::container_sort::Request::decode(&command.payload)
            .ok_or("sort decode")?;
        assert_eq!(request.descending, descending);
    }
    Ok(())
}

/// Tibia Global's container sort menu carries the "Use Manual Sort Mode" checkbox
/// (docs/reference/tibia-manual/controls.md:79). It is the same per-session mode as the
/// ToggleManualSortMode hotkey (Shift+S), shown only when the session negotiated capability 510.
#[test]
fn use_manual_sort_mode_checkbox_in_the_sort_menu_toggles_the_session_mode()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::owner::{ActivePlay, Phase};
    use oteryn_client_runtime::container_tree::{ContainerView, ContainerViews};
    for selected in [false, true] {
        let mut owner = HostOwner::new(None)?;
        let (link, _requests, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_sort_fixture();
        crate::play::PlayLink::bevy_manual_sort_fixture(&outbox, selected);
        crate::play::PlayLink::bevy_items_fixture(
            &outbox,
            None,
            Some(ContainerViews {
                views: vec![ContainerView {
                    view_id: 0,
                    container_handle: 7.try_into()?,
                    parent_handle: None,
                    capacity: 20,
                    entries: vec![],
                    browse: None,
                }],
            }),
            false,
            false,
        );
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        owner.play = Some(ActivePlay { view, link });
        owner.phase = Phase::Playing;
        assert!(owner.request_container_tool(ContainerTool::Sort, None, None));
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Font>()
            .init_asset::<Image>()
            .insert_non_send(owner)
            .add_systems(Startup, crate::bevy_shell::ui::setup);
        install_container_tools(&mut app);
        app.update();
        let checkbox = |app: &mut App| {
            app.world_mut()
                .query::<(&Text, &ChildOf)>()
                .iter(app.world())
                .find_map(|(text, parent)| {
                    (text.0 == "Use Manual Sort Mode").then_some(parent.parent())
                })
        };
        let checked = |app: &mut App, row: Entity| -> Result<bool, Box<dyn std::error::Error>> {
            let skins = app.world().resource::<Skins>();
            let (on, off) = (skins.checked.clone(), skins.unchecked.clone());
            let image = app
                .world_mut()
                .query::<(&ImageNode, &ChildOf)>()
                .iter(app.world())
                .find_map(|(image, parent)| (parent.parent() == row).then(|| image.image.clone()))
                .ok_or("checkbox indicator")?;
            assert!(image == on || image == off);
            Ok(image == on)
        };
        let active = |app: &App| {
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .is_some_and(|play| play.link.container_manual_sort_active())
        };
        let Some(row) = checkbox(&mut app) else {
            assert!(!selected, "510 shows the checkbox");
            continue;
        };
        assert!(selected, "without 510 there is no checkbox");
        assert!(!checked(&mut app, row)?);
        for expected in [true, false] {
            let row = checkbox(&mut app).ok_or("checkbox")?;
            app.world_mut()
                .trigger(bevy::ui_widgets::Activate { entity: row });
            app.update();
            assert_eq!(active(&app), expected);
            let row = checkbox(&mut app).ok_or("checkbox after toggle")?;
            assert_eq!(checked(&mut app, row)?, expected);
        }
    }
    Ok(())
}
