use super::*;
use bevy::{
    asset::{AssetPlugin, RenderAssetUsages},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui_widgets::Activate,
};

fn synthetic_skins(mut skins: ResMut<Skins>, mut images: ResMut<Assets<Image>>) {
    // Original test pixels only; no client skin files enter this fixture.
    let image = images.add(Image::new_fill(
        Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[53, 47, 41, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    ));
    skins.frame = image.clone();
    skins.inset = image.clone();
    skins.buttons = source::ButtonImages {
        up: image.clone(),
        down: image,
    };
}
fn nearest_frame(world: &World, entity: Entity) -> Option<Entity> {
    let mut next = Some(entity);
    while let Some(entity) = next {
        if world.get::<source::Frame>(entity).is_some() {
            return Some(entity);
        }
        next = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    None
}
fn rendered_pieces(world: &World, frame: Entity) -> Vec<Entity> {
    world
        .get::<Children>(frame)
        .map(|children| {
            children
                .iter()
                .filter(|entity| {
                    world
                        .get::<ImageNode>(*entity)
                        .is_some_and(|image| image.rect.is_some() && image.color != Color::NONE)
                        && world
                            .get::<Node>(*entity)
                            .is_some_and(|node| node.display != Display::None)
                        && world
                            .get::<Visibility>(*entity)
                            .is_none_or(|visible| *visible != Visibility::Hidden)
                })
                .collect()
        })
        .unwrap_or_default()
}
#[test]
fn navigation_frame_pieces_survive_actual_page_activation() -> Result<(), Box<dyn std::error::Error>>
{
    let mut owner = HostOwner::new(None)?;
    owner.open_graphics();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .insert_non_send(owner)
    .insert_resource(OptionsState {
        page: Page::Controls,
        advanced: true,
        ..default()
    })
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            synthetic_skins,
            widgets::setup,
        )
            .chain(),
    )
    .add_systems(Update, projection::project);
    app.update();
    let navigation = app
        .world_mut()
        .query_filtered::<Entity, With<Navigation>>()
        .single(app.world())?;
    let frame = nearest_frame(app.world(), navigation).ok_or("navigation frame")?;
    {
        let mut computed = app
            .world_mut()
            .get_mut::<ComputedNode>(frame)
            .ok_or("computed frame")?;
        computed.size = Vec2::new(129., 400.);
        computed.inverse_scale_factor = 1.;
    }
    app.update();
    let before = rendered_pieces(app.world(), frame);
    assert_eq!(
        before.len(),
        9,
        "fixture must exercise all real frame pieces before a page change"
    );
    let target = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, Control::Page(Page::Graphics)).then_some(entity)
        })
        .ok_or("Graphics navigation button")?;
    app.world_mut().trigger(Activate { entity: target });
    app.update();
    assert_eq!(app.world().resource::<OptionsState>().page, Page::Graphics);
    let after = rendered_pieces(app.world(), frame);
    assert_eq!(
        after.len(),
        9,
        "page navigation must not delete the frame's rendered children"
    );
    assert_eq!(
        after, before,
        "the persistent frame pieces must survive, not be recreated to mask their deletion"
    );
    assert_eq!(
        app.world().get::<Name>(frame).map(Name::as_str),
        Some("options.navigation.frame")
    );
    Ok(())
}

#[test]
fn a_hotkey_options_request_turns_the_open_options_to_the_hotkey_page()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .insert_non_send(HostOwner::new(None)?)
    .insert_resource(OptionsState::default())
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            synthetic_skins,
            widgets::setup,
        )
            .chain(),
    )
    .add_systems(Update, projection::project);
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.update();
    let state = app.world().resource::<OptionsState>();
    assert_eq!((state.page, state.advanced), (Page::Basic, false));
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .show_hotkey_options();
    app.update();
    let state = app.world().resource::<OptionsState>();
    assert_eq!((state.page, state.advanced), (Page::Controls, true));
    let hotkeys = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .any(|control| matches!(control, Control::Page(Page::Controls)));
    assert!(hotkeys, "the advanced navigation shows the hotkey page");
    Ok(())
}

#[test]
fn typing_an_action_name_filters_the_general_hotkey_list() -> Result<(), Box<dyn std::error::Error>>
{
    let mut app = options_page(Page::GeneralHotkeys)?;
    let search = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(e, name)| (name.as_str() == "hotkeys.binding.search").then_some(e))
        .ok_or("search field missing")?;
    app.world_mut()
        .get_mut::<bevy::text::EditableText>(search)
        .ok_or("search text")?
        .editor_mut()
        .set_text("HiReLiNg");
    app.update();
    let visible = app
        .world_mut()
        .query::<(&keys::Action, &Node)>()
        .iter(app.world())
        .filter_map(|(action, node)| match action {
            keys::Action::Pick(id) if node.display != Display::None => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(visible.contains(&"0165-hireling-open"));
    assert!(visible.iter().all(|id| id.contains("hireling")));
    Ok(())
}

pub(super) fn options_page(page: Page) -> Result<App, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.open_graphics();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .add_message::<bevy::input::keyboard::KeyboardInput>()
    .insert_non_send(owner);
    app.add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.insert_resource(OptionsState {
        page,
        advanced: true,
        ..default()
    });
    app.update();
    Ok(app)
}

#[test]
fn open_screenshot_folder_reports_an_unusable_configured_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = options_page(Page::Screenshots)?;
    let fixture = tempfile::tempdir()?;
    let file = fixture.path().join("file");
    std::fs::write(&file, b"existing")?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .screenshots
        .directory = file.to_string_lossy().into_owned();
    let button = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(e, name)| (name.as_str() == "screenshots.open-folder").then_some(e))
        .ok_or("open folder button")?;
    app.world_mut().trigger(Activate { entity: button });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !app
        .world()
        .non_send::<HostOwner>()
        .settings
        .feedback
        .contains("Could not open screenshots folder")
        && std::time::Instant::now() < deadline
    {
        app.update();
        std::thread::yield_now();
    }
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .feedback
            .contains("Could not open screenshots folder")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn opening_a_new_screenshot_directory_keeps_it_private() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let mut app = options_page(Page::Screenshots)?;
    let fixture = tempfile::tempdir()?;
    let folder = fixture.path().join("screens");
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .screenshots
        .directory = folder.to_string_lossy().into_owned();
    let button = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(e, n)| (n.as_str() == "screenshots.open-folder").then_some(e))
        .ok_or("open folder")?;
    app.world_mut().trigger(Activate { entity: button });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !folder.exists() && std::time::Instant::now() < deadline {
        app.update();
        std::thread::yield_now();
    }
    assert_eq!(std::fs::metadata(folder)?.permissions().mode() & 0o077, 0);
    Ok(())
}
