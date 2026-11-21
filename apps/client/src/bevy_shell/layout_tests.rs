//! Real Taffy ComputedNode layout and public audit, with no render/winit plugin.
use super::{audit, owner::HostOwner, ui};
use bevy::{
    app::{HierarchyPropagatePlugin, PropagateSet},
    asset::AssetPlugin,
    camera::{ComputedCameraValues, RenderTargetInfo},
    input_focus::InputFocus,
    prelude::*,
    text::{FontCx, RemSize},
    ui::{
        ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems, ui_layout_system,
        ui_surface::UiSurface, update::propagate_ui_target_cameras,
    },
};

#[test]
fn computed_audit_keeps_footer_inside_small_window_at_large_interface_scale()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("oteryn-bevy-layout-{}.json", std::process::id()));
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<UiScale>()
    .init_resource::<UiSurface>()
    .init_resource::<FontCx>()
    .init_resource::<RemSize>()
    .init_resource::<InputFocus>()
    .init_resource::<Assets<TextureAtlasLayout>>()
    .insert_non_send(HostOwner::new(None)?)
    .insert_resource(audit::AuditOutput::new(Some(path.clone())))
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
    .add_systems(Startup, ui::setup)
    .add_systems(Update, ui::project)
    .add_systems(
        PostUpdate,
        propagate_ui_target_cameras.in_set(UiSystems::Prepare),
    )
    .add_systems(
        PostUpdate,
        bevy::ui::widget::update_image_content_size_system.in_set(UiSystems::Content),
    )
    .add_systems(PostUpdate, ui_layout_system.in_set(UiSystems::Layout))
    .add_systems(PostUpdate, audit::write.after(UiSystems::Layout));
    app.world_mut().spawn((
        Window {
            resolution: (1024, 720).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    for (width, height, scale) in [(1024, 720, 1.), (1024, 720, 2.), (800, 400, 1.)] {
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        for mut camera in app
            .world_mut()
            .query::<&mut Camera>()
            .iter_mut(app.world_mut())
        {
            camera.computed = ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(width, height),
                    scale_factor: 1.,
                }),
                ..default()
            };
        }
        for mut window in app
            .world_mut()
            .query::<&mut Window>()
            .iter_mut(app.world_mut())
        {
            window.resolution.set_physical_resolution(width, height);
        }
        app.update();
        app.update();
        let audit: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let controls = audit["controls"].as_array().ok_or("controls")?;
        for id in ["graphics.apply", "graphics.ok", "graphics.cancel"] {
            let control = controls
                .iter()
                .find(|c| c["id"] == id)
                .ok_or("footer control")?;
            let rect = control["bounds_physical"]
                .as_array()
                .ok_or("computed rect")?;
            let y = rect[1].as_f64().ok_or("y")?;
            let h = rect[3].as_f64().ok_or("height")?;
            assert!(
                y >= 0. && y + h <= f64::from(height),
                "{id} {y}+{h} outside {height} at UI scale {scale}"
            );
            assert!(h > 0.);
        }
        assert_eq!(audit["lifecycle"], "offline");
        assert!(audit["draft"]["vsync"].is_boolean());
        assert!(audit["accepted"]["fullscreen"].is_boolean());
    }
    std::fs::remove_file(path)?;
    Ok(())
}
