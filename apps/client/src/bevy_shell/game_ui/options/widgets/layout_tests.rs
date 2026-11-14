//! Real glyph shaping and Taffy layout; no GPU or proprietary font/skin fixture.
use super::*;
use bevy::{
    app::{HierarchyPropagatePlugin, PropagateSet},
    asset::{AssetPlugin, RenderAssetUsages},
    camera::{ComputedCameraValues, RenderTargetInfo},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    text::{TextLayoutInfo, TextPlugin},
    ui::{
        ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems, ui_layout_system,
        ui_surface::UiSurface, update::propagate_ui_target_cameras,
    },
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(
    mut skins: ResMut<Skins>,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
) {
    skins.font = fonts.add(Font::from_bytes(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/assets/FiraMono-subset.ttf"
        ))
        .to_vec(),
    ));
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
fn app() -> Result<App, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.open_graphics();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TextPlugin,
        source::GlobalUiPlugin,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .init_asset::<Image>()
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<UiScale>()
    .init_resource::<UiSurface>()
    .init_resource::<InputFocus>()
    .insert_non_send(owner)
    .insert_resource(OptionsState {
        page: Page::Graphics,
        advanced: true,
        ..default()
    })
    .configure_sets(
        PostUpdate,
        (
            UiSystems::Prepare,
            UiSystems::Propagate,
            UiSystems::Content,
            UiSystems::Layout,
            UiSystems::PostLayout,
        )
            .chain(),
    )
    .configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default().in_set(UiSystems::Propagate),
    )
    .configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default().in_set(UiSystems::Propagate),
    )
    .add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, fixture, setup).chain(),
    )
    .add_systems(Update, super::super::projection::project)
    .add_systems(
        PostUpdate,
        propagate_ui_target_cameras.in_set(UiSystems::Prepare),
    )
    .add_systems(
        PostUpdate,
        (
            bevy::ui::widget::update_image_content_size_system,
            bevy::ui::widget::measure_text_system
                .after(bevy::text::detect_text_needs_rerender)
                .after(bevy::text::load_font_assets_into_font_collection),
        )
            .in_set(UiSystems::Content),
    )
    .add_systems(PostUpdate, ui_layout_system.in_set(UiSystems::Layout))
    .add_systems(
        PostUpdate,
        bevy::ui::widget::text_system.in_set(UiSystems::PostLayout),
    );
    app.world_mut().spawn((
        Window {
            resolution: (1024, 720).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.update();
    for mut camera in app
        .world_mut()
        .query::<&mut Camera>()
        .iter_mut(app.world_mut())
    {
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(1024, 720),
                scale_factor: 1.,
            }),
            ..default()
        };
    }
    for _ in 0..4 {
        app.update();
    }
    Ok(app)
}
fn row(app: &mut App, id: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == id).then_some(entity))
        .ok_or_else(|| format!("missing {id}").into())
}
fn caption(app: &App, row: Entity) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world()
        .get::<Children>(row)
        .ok_or("children")?
        .iter()
        .find(|entity| app.world().get::<Text>(*entity).is_some())
        .ok_or_else(|| "caption".into())
}
fn visible_glyph_right(layout: &TextLayoutInfo) -> f32 {
    // TextLayoutInfo.size uses Parley's full_width, including trailing spaces
    // beyond a soft break. Bound the actual rasterized glyphs instead.
    layout
        .glyphs
        .iter()
        .map(|glyph| glyph.position.x + glyph.atlas_info.rect.width() / 2.)
        .fold(0., f32::max)
}
#[test]
fn real_font_checkbox_labels_use_available_width_in_the_standard_options_dialog() -> TestResult {
    let mut app = app()?;
    for id in [
        "graphics.fullscreen",
        "graphics.vsync",
        "graphics.integer_scaling",
        "options.advanced",
    ] {
        let entity = row(&mut app, id)?;
        let text = caption(&app, entity)?;
        let shaped = app
            .world()
            .get::<TextLayoutInfo>(text)
            .ok_or("shaped caption")?;
        let row_size = app
            .world()
            .get::<ComputedNode>(entity)
            .ok_or("row geometry")?
            .size();
        let text_size = app
            .world()
            .get::<ComputedNode>(text)
            .ok_or("text geometry")?
            .size();
        assert!(
            !shaped.glyphs.is_empty(),
            "{id}: the fixture must shape real glyphs"
        );
        assert!(
            shaped.size.y <= 14.,
            "{id}: normal-width caption wraps: {:?}, text={text_size:?}, row={row_size:?}",
            shaped.size
        );
        assert_eq!(
            row_size.y,
            if id == "options.advanced" { 16. } else { 22. },
            "{id}: source row height differs: {row_size:?}"
        );
        assert!(
            visible_glyph_right(shaped) <= text_size.x + 1.,
            "{id}: glyphs overflow available text width"
        );
    }
    Ok(())
}
#[test]
fn real_font_checkbox_labels_still_wrap_when_the_available_row_is_genuinely_narrow() -> TestResult {
    let mut app = app()?;
    let entity = row(&mut app, "graphics.integer_scaling")?;
    let text = caption(&app, entity)?;
    app.world_mut().get_mut::<Node>(entity).ok_or("row")?.width = px(100.);
    for _ in 0..3 {
        app.update();
    }
    let shaped = app
        .world()
        .get::<TextLayoutInfo>(text)
        .ok_or("shaped caption")?;
    let row_size = app
        .world()
        .get::<ComputedNode>(entity)
        .ok_or("row geometry")?
        .size();
    let text_size = app
        .world()
        .get::<ComputedNode>(text)
        .ok_or("text geometry")?
        .size();
    assert!(!shaped.glyphs.is_empty());
    assert!(
        shaped.size.y > 22.,
        "narrow caption must retain multiline layout"
    );
    assert!(
        row_size.y >= shaped.size.y,
        "row must grow to contain wrapped text"
    );
    assert!(
        visible_glyph_right(shaped) <= text_size.x + 1.,
        "narrow text must not overflow horizontally: shaped={:?}, text={text_size:?}, row={row_size:?}",
        shaped.size
    );
    Ok(())
}
