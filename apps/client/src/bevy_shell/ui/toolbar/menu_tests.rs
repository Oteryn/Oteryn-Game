use super::*;

#[test]
fn menu_handle_restores_grid_and_touch_zoom_changes_hud_scale()
-> Result<(), Box<dyn std::error::Error>> {
    use bevy::input::touch::{TouchInput, TouchPhase};
    let mut owner = HostOwner::new(None)?;
    let (link, _, _) = crate::play::play_channel();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::input::InputPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<SidebarWidgets>()
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            |mut commands: Commands, skins: Res<Skins>| {
                let parent = commands.spawn(Node::default()).id();
                spawn(&mut commands, &skins, parent);
            },
        )
            .chain(),
    )
    .add_systems(Update, project);
    app.update();
    let named = |app: &mut App, name: &str| -> Result<Entity, String> {
        app.world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(entity, n)| (n.as_str() == name).then_some(entity))
            .ok_or_else(|| format!("missing {name}"))
    };
    let grid = named(&mut app, "Control buttons")?;
    let toggle = named(&mut app, "Toggle top menu")?;
    for display in [Display::None, Display::Flex] {
        app.world_mut().trigger(Activate { entity: toggle });
        app.update();
        assert_eq!(
            app.world().get::<Node>(grid).ok_or("grid")?.display,
            display
        );
    }
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(TouchInput {
        phase: TouchPhase::Started,
        position: Vec2::ZERO,
        window,
        force: None,
        id: 1,
    });
    app.update();
    let original = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .interface_scale;
    for (name, scale) in [("Zoom in", original + 50), ("Zoom out", original)] {
        let entity = named(&mut app, name)?;
        assert_ne!(
            app.world().get::<Node>(entity).ok_or("zoom")?.display,
            Display::None
        );
        app.world_mut().trigger(Activate { entity });
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .settings
                .preferences
                .interface_scale,
            scale
        );
    }
    Ok(())
}
