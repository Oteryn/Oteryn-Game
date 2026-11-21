use bevy::{
    prelude::*,
    ui::{ComputedNode, widget::NodeImageMode},
};
use oteryn_client_ui_bevy as ui;

#[test]
fn chrome_covers_border_box_without_moving_body_in_native_layout()
-> Result<(), Box<dyn std::error::Error>> {
    use bevy::{
        app::{HierarchyPropagatePlugin, PropagateSet},
        camera::{ComputedCameraValues, RenderTargetInfo},
        text::{FontCx, RemSize},
        ui::{
            ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiGlobalTransform, UiScale,
            UiSystems, ui_layout_system, ui_surface::UiSurface,
            update::propagate_ui_target_cameras,
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
    let mut source = Image::default();
    source.texture_descriptor.size.width = 98;
    source.texture_descriptor.size.height = 40;
    let source = app.world_mut().resource_mut::<Assets<Image>>().add(source);
    let root = app
        .world_mut()
        .spawn((
            ui::frame(source.clone(), [1.; 4]),
            Node {
                width: px(43.),
                height: px(20.),
                border: px(1.).all(),
                padding: px(3.).all(),
                ..default()
            },
        ))
        .id();
    let body = app
        .world_mut()
        .spawn(Node {
            width: px(10.),
            height: px(6.),
            ..default()
        })
        .id();
    app.world_mut().entity_mut(root).add_child(body);
    let option_row = app
        .world_mut()
        .spawn((
            ui::frame(source, [1.; 4]),
            Node {
                width: px(505.),
                ..ui::checkbox_row(true)
            },
        ))
        .id();

    app.update();
    app.update(); // newly attached chrome participates in the next native layout
    assert_eq!(
        app.world()
            .get::<ComputedNode>(option_row)
            .ok_or("option row")?
            .size
            .y,
        22.,
        "source-holder image must not impose its texture aspect ratio on a framed row"
    );
    let root_center = app
        .world()
        .get::<UiGlobalTransform>(root)
        .ok_or("root transform")?
        .transform_point2(Vec2::ZERO);
    let outer_min = root_center - Vec2::new(43., 20.) / 2.;
    let body_center = app
        .world()
        .get::<UiGlobalTransform>(body)
        .ok_or("body transform")?
        .transform_point2(Vec2::ZERO);
    assert_eq!(
        body_center - Vec2::new(10., 6.) / 2.,
        outer_min + Vec2::splat(4.)
    );
    let mut count = 0;
    for child in app.world().get::<Children>(root).ok_or("children")? {
        if *child == body {
            continue;
        }
        let node = app
            .world()
            .get::<ComputedNode>(*child)
            .ok_or("piece size")?;
        let transform = app
            .world()
            .get::<UiGlobalTransform>(*child)
            .ok_or("piece transform")?;
        let min = transform.transform_point2(Vec2::ZERO) - node.size / 2.;
        let max = min + node.size;
        assert!(min.cmpge(outer_min).all() && max.cmple(outer_min + Vec2::new(43., 20.)).all());
        if count == 0 {
            assert_eq!(min, outer_min);
            assert_eq!(node.size, Vec2::ONE);
        }
        count += 1;
    }
    assert_eq!(count, 9);
    Ok(())
}

#[test]
fn shrinking_source_keeps_pixel_edges_and_reuses_chrome_on_resize()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ui::GlobalUiPlugin));
    let mut images = Assets::<Image>::default();
    let mut source = Image::default();
    source.texture_descriptor.size.width = 98;
    source.texture_descriptor.size.height = 40;
    let source = images.add(source);
    app.insert_resource(images);
    for inset in [1., 2.] {
        let root = app
            .world_mut()
            .spawn((
                ui::frame(source.clone(), [inset; 4]),
                Node {
                    width: px(43.),
                    height: px(20.),
                    border: px(inset).all(),
                    ..default()
                },
                ComputedNode {
                    size: Vec2::new(43., 20.),
                    border: BorderRect::all(inset),
                    ..default()
                },
            ))
            .id();
        app.update();
        let children = app
            .world()
            .get::<Children>(root)
            .map(|c| c.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        assert_eq!(
            children.len(),
            9,
            "native chrome needs fixed-size cropped pieces, not proportional borders"
        );
        for size in [
            Vec2::new(43., 20.),
            Vec2::new(34., 20.),
            Vec2::new(113., 20.),
            Vec2::new(519., 470.),
        ] {
            app.world_mut()
                .get_mut::<ComputedNode>(root)
                .ok_or("computed node")?
                .size = size;
            app.update();
            assert_eq!(
                app.world()
                    .get::<Children>(root)
                    .ok_or("children")?
                    .iter()
                    .collect::<Vec<_>>(),
                children
            );
            for (index, child) in children.iter().enumerate() {
                let node = app.world().get::<Node>(*child).ok_or("piece geometry")?;
                let image = app.world().get::<ImageNode>(*child).ok_or("piece image")?;
                assert_eq!(image.image, source);
                assert!(matches!(
                    image.image_mode,
                    NodeImageMode::Tiled {
                        stretch_value: 1.,
                        ..
                    }
                ));
                assert!(
                    app.world()
                        .get::<Pickable>(*child)
                        .is_some_and(|p| !p.is_hoverable && !p.should_block_lower)
                );
                assert!(
                    app.world()
                        .get::<bevy::input_focus::tab_navigation::TabIndex>(*child)
                        .is_none()
                );
                let crop = image.rect.ok_or("source crop")?;
                let column = index % 3;
                let row = index / 3;
                // 0.439px and 0.878px were the old 1px/2px edge sizes at 43x20.
                // The emitted native nodes must retain full source-pixel widths.
                if column != 1 {
                    assert_eq!(node.width, px(inset));
                    assert_eq!(crop.width(), inset);
                }
                if row != 1 {
                    assert_eq!(node.height, px(inset));
                    assert_eq!(crop.height(), inset);
                }
                assert_eq!(node.left, px([0., inset, size.x - inset][column] - inset));
                assert_eq!(node.top, px([0., inset, size.y - inset][row] - inset));
            }
        }
        app.world_mut().despawn(root);
        for child in children {
            assert!(
                app.world().get_entity(child).is_err(),
                "chrome must follow parent lifetime"
            );
        }
    }
    Ok(())
}
