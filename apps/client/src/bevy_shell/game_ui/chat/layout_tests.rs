//! The chat in a dock region of any shape, with real glyph shaping and Taffy layout.
use super::widgets::{self, TAB_HEIGHT};
use super::*;
use crate::bevy_shell::gameplay_dialogs::scale::{self, Module, ModuleScale};
use bevy::{
    app::{HierarchyPropagatePlugin, PropagateSet},
    asset::{AssetPlugin, RenderAssetUsages},
    camera::{ComputedCameraValues, RenderTargetInfo},
    ecs::system::RunSystemOnce,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    text::TextPlugin,
    ui::{
        ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiGlobalTransform, UiSystems,
        ui_layout_system, ui_surface::UiSurface, update::propagate_ui_target_cameras,
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

/// A window of 1280x720 whose chat dock region is `region` logical pixels, with fourteen open
/// tabs and the chat at `zoom`.
fn app(region: Vec2, zoom: f32) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut module_scale = ModuleScale::default();
    module_scale.set(Module::Chat, zoom);
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
    .insert_resource(module_scale)
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
        PostUpdate,
        (
            scale::apply.before(UiSystems::Prepare),
            propagate_ui_target_cameras.in_set(UiSystems::Prepare),
            bevy::ui::widget::measure_text_system
                .after(bevy::text::detect_text_needs_rerender)
                .after(bevy::text::load_font_assets_into_font_collection)
                .in_set(UiSystems::Content),
            ui_layout_system.in_set(UiSystems::Layout),
            bevy::ui::widget::text_system.in_set(UiSystems::PostLayout),
        ),
    );
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.world_mut()
        .run_system_once(crate::bevy_shell::ui::setup)
        .map_err(|error| format!("{error:?}"))?;
    app.world_mut()
        .run_system_once(fixture)
        .map_err(|error| format!("{error:?}"))?;
    let chat = app
        .world_mut()
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(10.),
            top: px(10.),
            width: px(region.x),
            height: px(region.y),
            ..default()
        })
        .id();
    let other = app.world_mut().spawn(Node::default()).id();
    app.insert_resource(GameRegions {
        vitals: other,
        minimap: other,
        equipment: other,
        containers: other,
        battle: other,
        skills: other,
        vip: other,
        analyzers: other,
        chat,
        bars: other,
        world: other,
    });
    app.world_mut()
        .run_system_once(widgets::setup)
        .map_err(|error| format!("{error:?}"))?;
    let tabs = app
        .world_mut()
        .query_filtered::<Entity, With<Tabs>>()
        .single(app.world())?;
    for _ in 0..14 {
        let tab = app
            .world_mut()
            .spawn(widgets::fixed(Vec2::new(96., TAB_HEIGHT)))
            .id();
        app.world_mut().entity_mut(tabs).add_child(tab);
    }
    app.update();
    for mut camera in app
        .world_mut()
        .query::<&mut Camera>()
        .iter_mut(app.world_mut())
    {
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(1280, 720),
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

/// The logical rectangle of `entity`.
fn rect(app: &App, entity: Entity) -> Result<Rect, Box<dyn std::error::Error>> {
    let node = app.world().get::<ComputedNode>(entity).ok_or("geometry")?;
    let transform = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .ok_or("transform")?;
    Ok(Rect::from_center_size(
        transform.translation,
        node.size() * node.inverse_scale_factor,
    ))
}
fn single<F: bevy::ecs::query::QueryFilter>(
    app: &mut App,
) -> Result<Entity, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<Entity, F>()
        .single(app.world())?)
}

fn assert_docked(region: Vec2, zoom: f32) -> TestResult {
    let mut app = app(region, zoom)?;
    let chat = app.world().resource::<GameRegions>().chat;
    let bounds = rect(&app, chat)?;
    assert_eq!(bounds.size(), region);
    let row = single::<With<ComposeRow>>(&mut app)?;
    let input = rect(&app, row)?;
    // The input row sits at the bottom of the region, at its scaled height.
    assert!(
        (input.max.y - bounds.max.y).abs() < 0.5,
        "{input:?} in {bounds:?}"
    );
    assert_eq!(input.height(), 16. * zoom);
    let mode = single::<With<ChatModeButton>>(&mut app)?;
    let mode = rect(&app, mode)?;
    assert!(mode.max.x <= bounds.max.x + 0.5, "{mode:?} in {bounds:?}");
    assert!(mode.min.x >= bounds.min.x - 0.5, "{mode:?} in {bounds:?}");
    // The fourteen tabs overflow the strip, which scrolls instead of widening the module.
    let tabs = single::<With<Tabs>>(&mut app)?;
    let strip = rect(&app, tabs)?;
    assert!(strip.max.x <= bounds.max.x + 0.5, "{strip:?} in {bounds:?}");
    assert_eq!(strip.height(), TAB_HEIGHT * zoom);
    let content = app
        .world()
        .get::<ComputedNode>(tabs)
        .ok_or("tabs")?
        .content_size;
    assert!(content.x > strip.width(), "{content:?} {strip:?}");
    // The transcript keeps the space between the tab strip and the input row.
    let history = single::<With<HistoryScroll>>(&mut app)?;
    let history = rect(&app, history)?;
    assert!(history.min.y >= strip.max.y - 0.5 && history.max.y <= input.min.y + 0.5);
    assert!(history.width() <= bounds.width() + 0.5);
    Ok(())
}

#[test]
fn chat_fits_a_tall_side_column_dock() -> TestResult {
    assert_docked(Vec2::new(300., 700.), 1.)
}

#[test]
fn chat_fits_a_wide_shallow_bottom_dock() -> TestResult {
    assert_docked(Vec2::new(1200., 150.), 1.)
}

#[test]
fn chat_at_150_percent_scales_tabs_input_and_fonts() -> TestResult {
    assert_docked(Vec2::new(300., 700.), 1.5)?;
    let mut app = app(Vec2::new(300., 700.), 1.5)?;
    let tabs = single::<With<Tabs>>(&mut app)?;
    let first = app
        .world()
        .get::<Children>(tabs)
        .and_then(|children| children.first().copied())
        .ok_or("tab")?;
    assert_eq!(rect(&app, first)?.width(), 144.);
    let mut fonts = app
        .world_mut()
        .query::<(&scale::ScaledFont, &bevy::text::TextFont)>();
    let mut checked = 0;
    for (scaled, font) in fonts.iter(app.world()) {
        assert_eq!(font.font_size, bevy::text::FontSize::Px(scaled.size * 1.5));
        checked += 1;
    }
    assert!(checked > 0);
    Ok(())
}
