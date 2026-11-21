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
