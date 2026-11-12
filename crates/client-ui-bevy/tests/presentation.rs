use bevy::{
    prelude::*,
    text::{FontSize, LineHeight},
    ui::{ComputedNode, UiGlobalTransform},
};
use oteryn_client_ui_bevy as ui;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn layout_app() -> App {
    use bevy::{
        app::{HierarchyPropagatePlugin, PropagateSet},
        camera::{ComputedCameraValues, RenderTargetInfo},
        text::{FontCx, RemSize},
        ui::{
            ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiScale, UiSystems,
            ui_layout_system, ui_surface::UiSurface, update::propagate_ui_target_cameras,
        },
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        ui::GlobalUiPlugin,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .init_resource::<UiScale>()
    .init_resource::<UiSurface>()
    .init_resource::<FontCx>()
    .init_resource::<RemSize>()
    .init_resource::<Assets<TextureAtlasLayout>>()
    .configure_sets(
        PostUpdate,
        (
            UiSystems::Prepare,
            UiSystems::Propagate,
            UiSystems::Content,
            UiSystems::Layout,
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
        propagate_ui_target_cameras.in_set(UiSystems::Prepare),
    )
    .add_systems(
        PostUpdate,
        bevy::ui::widget::update_image_content_size_system.in_set(UiSystems::Content),
    )
    .add_systems(PostUpdate, ui_layout_system.in_set(UiSystems::Layout));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(800, 600),
                    scale_factor: 1.,
                }),
                ..default()
            },
            ..default()
        },
    ));
    app
}
fn rect(app: &App, entity: Entity) -> Result<Rect, Box<dyn std::error::Error>> {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .ok_or("computed size")?
        .size;
    let center = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .ok_or("transform")?
        .transform_point2(Vec2::ZERO);
    Ok(Rect::from_center_size(center, size))
}
#[test]
fn dialog_caption_tracks_header_center_and_ordinary_font_across_dialog_sizes() -> TestResult {
    let mut app = layout_app();
    let root = app
        .world_mut()
        .spawn(Node {
            width: px(685.),
            height: px(100.),
            ..default()
        })
        .id();
    let header = ui::dialog_caption(
        &mut app.world_mut().commands(),
        root,
        "Options",
        Handle::default(),
    );
    app.world_mut().flush();
    let text = app.world().get::<Children>(header).ok_or("caption")?[0];
    // Stand in for the measured glyph bounds, while driving actual native layout.
    app.world_mut().entity_mut(text).insert(Node {
        width: px(80.),
        height: px(13.),
        ..default()
    });
    for width in [685., 700., 305.] {
        app.world_mut().get_mut::<Node>(root).ok_or("dialog")?.width = px(width);
        app.update();
        let bounds = rect(&app, header)?;
        let outer = rect(&app, root)?;
        assert_eq!(bounds.min, outer.min + Vec2::new(10., 0.));
        assert_eq!(bounds.size(), Vec2::new(width - 20., 17.));
        assert!(
            (rect(&app, text)?.center() - bounds.center())
                .abs()
                .max_element()
                <= 0.5
        );
        assert_eq!(
            app.world().get::<TextFont>(text).ok_or("font")?.font_size,
            FontSize::Px(11.)
        );
        assert_eq!(
            app.world().get::<TextColor>(text).ok_or("color")?.0,
            Color::srgb_u8(144, 144, 144)
        );
        assert_eq!(
            app.world().get::<LineHeight>(text).ok_or("line")?,
            &LineHeight::Px(13.)
        );
    }
    Ok(())
}
#[test]
fn row_fillers_share_space_and_table_viewport_respects_remaining_column_height() -> TestResult {
    let mut app = layout_app();
    let page = app
        .world_mut()
        .spawn(Node {
            width: px(505.),
            height: px(456.),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            ..default()
        })
        .id();
    let preset = app
        .world_mut()
        .spawn(Node {
            width: percent(100.),
            height: px(20.),
            column_gap: px(5.),
            ..default()
        })
        .id();
    let mut fill = Node {
        width: percent(100.),
        height: px(20.),
        ..default()
    };
    ui::layout_child(&mut fill, FlexDirection::Row, true, false);
    let combo = app.world_mut().spawn(fill.clone()).id();
    let mut children = vec![combo];
    for width in [30., 35., 45., 45.] {
        let mut fixed = Node {
            width: px(width),
            height: px(20.),
            flex_shrink: 0.,
            ..default()
        };
        ui::layout_child(&mut fixed, FlexDirection::Row, false, false);
        children.push(app.world_mut().spawn(fixed).id());
    }
    app.world_mut().entity_mut(preset).add_children(&children);
    let search = app
        .world_mut()
        .spawn(Node {
            width: percent(100.),
            height: px(20.),
            column_gap: px(5.),
            ..default()
        })
        .id();
    let label = app
        .world_mut()
        .spawn(Node {
            width: px(170.),
            height: px(13.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    let input = app.world_mut().spawn(fill).id();
    let clear = app
        .world_mut()
        .spawn(Node {
            width: px(20.),
            height: px(20.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    app.world_mut()
        .entity_mut(search)
        .add_children(&[label, input, clear]);
    let mut table_row = Node {
        width: percent(100.),
        column_gap: px(4.),
        ..default()
    };
    ui::layout_child(&mut table_row, FlexDirection::Column, true, true);
    let table_row = app.world_mut().spawn(table_row).id();
    let mut viewport = Node {
        width: percent(100.),
        height: percent(100.),
        overflow: Overflow {
            x: OverflowAxis::Clip,
            y: OverflowAxis::Scroll,
        },
        ..default()
    };
    ui::layout_child(&mut viewport, FlexDirection::Row, true, false);
    let table = app
        .world_mut()
        .spawn((bevy::ui_widgets::ListBox, viewport))
        .id();
    let scrollbar = app
        .world_mut()
        .spawn(Node {
            width: px(12.),
            height: percent(100.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    app.world_mut()
        .entity_mut(table_row)
        .add_children(&[table, scrollbar]);
    app.world_mut()
        .entity_mut(page)
        .add_children(&[preset, search, table_row]);
    app.update();
    let page_rect = rect(&app, page)?;
    for entity in children.iter().copied().chain([
        preset, search, label, input, clear, table_row, table, scrollbar,
    ]) {
        let bounds = rect(&app, entity)?;
        assert!(
            bounds.min.cmpge(page_rect.min).all() && bounds.max.cmple(page_rect.max).all(),
            "source child escaped allocated page"
        );
    }
    assert_eq!(rect(&app, combo)?.width(), 330.);
    assert_eq!(rect(&app, input)?.width(), 305.);
    for (entity, width) in children.iter().skip(1).zip([30., 35., 45., 45.]) {
        assert_eq!(rect(&app, *entity)?.width(), width);
    }
    assert_eq!(rect(&app, table_row)?.height(), 406.);
    assert_eq!(rect(&app, table)?.width(), 489.);
    assert_eq!(
        app.world().get::<Node>(page).ok_or("page")?.overflow.x,
        OverflowAxis::Visible
    );
    Ok(())
}
