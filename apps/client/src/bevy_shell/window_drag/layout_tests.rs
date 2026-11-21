use super::*;
use bevy::{
    app::{HierarchyPropagatePlugin, PropagateSet},
    camera::{ComputedCameraValues, RenderTargetInfo},
    text::{FontCx, RemSize},
    ui::{
        ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems, ui_layout_system,
        ui_surface::UiSurface, update::propagate_ui_target_cameras,
    },
};
#[test]
fn actual_taffy_layout_drag_respects_dpi_interface_scale_resize_and_growth() -> Result {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .insert_resource(UiScale(2.))
    .init_resource::<UiSurface>()
    .init_resource::<FontCx>()
    .init_resource::<RemSize>()
    .configure_sets(
        PostUpdate,
        (UiSystems::Prepare, UiSystems::Propagate, UiSystems::Layout).chain(),
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
    .add_systems(PostUpdate, ui_layout_system.in_set(UiSystems::Layout));
    super::super::super::game_ui::modal::install(&mut app);
    install(&mut app);
    let mut window = Window {
        resolution: (1600, 1200).into(),
        focused: true,
        ..default()
    };
    window.resolution.set_scale_factor_override(Some(2.));
    let window = app.world_mut().spawn((window, PrimaryWindow)).id();
    let camera = app
        .world_mut()
        .spawn((
            Camera2d,
            IsDefaultUiCamera,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1600, 1200),
                        scale_factor: 2.,
                    }),
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    let root = app
        .world_mut()
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let panel = app
        .world_mut()
        .spawn((
            DraggableDialog,
            super::super::super::game_ui::GameModal,
            GlobalZIndex(100),
            Node {
                width: px(200.),
                height: px(100.),
                flex_shrink: 0.,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    app.update();
    app.update();
    let before = app
        .world()
        .get::<UiGlobalTransform>(panel)
        .ok_or("layout")?
        .translation;
    assert_eq!(before, Vec2::new(800., 600.));
    assert_eq!(
        app.world().get::<ComputedNode>(panel).ok_or("size")?.size(),
        Vec2::new(800., 400.)
    );
    let origin = Vec2::new(220., 210.);
    press_at(
        &mut app,
        panel,
        window,
        origin,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        origin,
        Vec2::new(100., 50.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(50., 25.));
    app.update();
    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(panel)
            .ok_or("moved layout")?
            .translation,
        before + Vec2::new(200., 100.)
    );
    move_to(
        &mut app,
        panel,
        window,
        origin,
        Vec2::splat(5000.),
        PointerId::Mouse,
        PointerButton::Primary,
        false,
    )?;
    app.update();
    let node = app.world().get::<ComputedNode>(panel).ok_or("size")?;
    let center = app
        .world()
        .get::<UiGlobalTransform>(panel)
        .ok_or("bounded layout")?
        .translation;
    assert_eq!(center + node.size() * 0.5, Vec2::new(1600., 1200.));
    app.world_mut()
        .get_mut::<Window>(window)
        .ok_or("window")?
        .resolution
        .set_physical_resolution(1000, 800);
    app.world_mut()
        .get_mut::<Camera>(camera)
        .ok_or("camera")?
        .computed
        .target_info = Some(RenderTargetInfo {
        physical_size: UVec2::new(1000, 800),
        scale_factor: 2.,
    });
    app.update();
    app.update();
    assert_eq!(translation(&app, panel)?, Vec2::ZERO);
    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(panel)
            .ok_or("resized layout")?
            .translation,
        Vec2::new(500., 400.)
    );
    app.world_mut().get_mut::<Node>(panel).ok_or("panel")?.width = px(280.);
    app.update();
    app.update();
    let node = app.world().get::<ComputedNode>(panel).ok_or("grown size")?;
    let center = app
        .world()
        .get::<UiGlobalTransform>(panel)
        .ok_or("grown layout")?
        .translation;
    assert_eq!(
        (center - node.size() * 0.5).x,
        0.,
        "an oversized panel keeps its caption's left edge reachable"
    );
    Ok(())
}
