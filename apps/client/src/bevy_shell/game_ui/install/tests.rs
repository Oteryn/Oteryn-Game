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
