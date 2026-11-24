//! The wgpu settings the host starts Bevy's renderer with.
use super::owner::HostOwner;
use bevy::{
    DefaultPlugins,
    app::{PluginGroup, PluginGroupBuilder},
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
};
use bevy::{prelude::*, window::PrimaryWindow};
use std::ffi::OsString;

pub(super) fn plugins(smoke: bool, engine: oteryn_renderer::GraphicsEngine) -> PluginGroupBuilder {
    let settings = startup_settings(
        smoke,
        engine,
        |name| std::env::var_os(name),
        |backend| {
            // Bevy initializes its native adapter before winit creates the primary window.
            let instance = bevy::render::renderer::RenderInstance::new(Default::default());
            !bevy_tasks::block_on(instance.enumerate_adapters(backend)).is_empty()
        },
    );
    DefaultPlugins.set(RenderPlugin {
        render_creation: settings.into(),
        ..Default::default()
    })
}

fn startup_settings(
    smoke: bool,
    engine: oteryn_renderer::GraphicsEngine,
    env: impl Fn(&str) -> Option<OsString>,
    available: impl FnOnce(Backends) -> bool,
) -> WgpuSettings {
    let mut settings = select(smoke, &env);
    if smoke
        || [
            "WGPU_BACKEND",
            "WGPU_FORCE_FALLBACK_ADAPTER",
            "WGPU_ADAPTER_NAME",
        ]
        .into_iter()
        .any(|key| env(key).is_some())
    {
        return settings;
    }
    let preferred = match engine {
        oteryn_renderer::GraphicsEngine::Vulkan => Backends::VULKAN,
        oteryn_renderer::GraphicsEngine::Direct3D12 if cfg!(windows) => Backends::DX12,
        oteryn_renderer::GraphicsEngine::Direct3D12 => return settings,
    };
    if settings.backends.is_some_and(|all| all.contains(preferred)) && available(preferred) {
        settings.backends = Some(preferred);
    }
    settings
}

/// A `--smoke` packaging run must start on a runner without a GPU, so it asks wgpu for the
/// software fallback adapter (WARP through DX12 on Windows), unless the environment already
/// chose one through `WGPU_FORCE_FALLBACK_ADAPTER` or `WGPU_BACKEND`. Normal runs keep Bevy's
/// defaults.
fn select(smoke: bool, env: impl Fn(&str) -> Option<OsString>) -> WgpuSettings {
    let mut settings = WgpuSettings::default();
    if smoke {
        if env("WGPU_FORCE_FALLBACK_ADAPTER").is_none() {
            settings.force_fallback_adapter = true;
        }
        if cfg!(windows) && env("WGPU_BACKEND").is_none() {
            settings.backends = Some(Backends::DX12);
        }
    }
    settings
}

pub(super) fn apply_window(
    preferences: &crate::play::ReferencePreferences,
    window: &mut bevy::window::Window,
) {
    use bevy::window::{PresentMode, WindowMode};
    let mode = if preferences.graphics.fullscreen {
        WindowMode::BorderlessFullscreen(
            preferences
                .graphics
                .monitor
                .checked_sub(1)
                .map_or(bevy::window::MonitorSelection::Current, |index| {
                    bevy::window::MonitorSelection::Index(usize::from(index))
                }),
        )
    } else {
        WindowMode::Windowed
    };
    let present = if preferences.graphics.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    if window.mode != mode {
        window.mode = mode;
    }
    if window.present_mode != present {
        window.present_mode = present;
    }
}

pub(super) fn project_window(
    owner: NonSend<HostOwner>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
    mut sampling: Query<&mut Msaa, With<Camera2d>>,
    mut previous_display: Local<Option<(bool, u16)>>,
    mut previous_vsync: Local<Option<bool>>,
) {
    let samples = Msaa::from_samples(owner.settings.preferences.graphics.antialiasing.samples());
    for mut msaa in &mut sampling {
        msaa.set_if_neq(samples);
    }
    if let Ok(mut window) = window.single_mut() {
        let graphics = owner.settings.preferences.graphics;
        let display = (graphics.fullscreen, graphics.monitor);
        let display_changed = *previous_display != Some(display);
        if display_changed {
            if !graphics.fullscreen
                && (previous_display.is_some()
                    || !matches!(window.position, bevy::window::WindowPosition::At(_)))
                && let Some(index) = graphics.monitor.checked_sub(1)
            {
                window.position = bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Index(usize::from(index)),
                );
            }
            *previous_display = Some(display);
        }
        if display_changed || *previous_vsync != Some(graphics.vsync) {
            apply_window(&owner.settings.preferences, &mut window);
            *previous_vsync = Some(graphics.vsync);
        }
    }
    let next = owner.settings.preferences.interface_scale as f32 / 100.;
    if scale.0 != next {
        scale.0 = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_preferred_adapter_preserves_all_platform_fallbacks() {
        let settings = startup_settings(
            false,
            oteryn_renderer::GraphicsEngine::Vulkan,
            |_| None,
            |_| false,
        );
        assert_eq!(settings.backends, WgpuSettings::default().backends);
    }

    #[test]
    fn smoke_run_forces_the_software_fallback_adapter() {
        let settings = select(true, |_| None);
        assert!(settings.force_fallback_adapter);
        let expected = if cfg!(windows) {
            Some(Backends::DX12)
        } else {
            WgpuSettings::default().backends
        };
        assert_eq!(settings.backends, expected);
    }

    #[test]
    fn normal_run_keeps_the_default_render_settings() {
        let settings = select(false, |_| None);
        let default = WgpuSettings::default();
        assert_eq!(
            settings.force_fallback_adapter,
            default.force_fallback_adapter
        );
        assert_eq!(settings.backends, default.backends);
    }

    #[test]
    fn smoke_run_leaves_an_adapter_chosen_in_the_environment_to_wgpu() {
        let settings = select(true, |_| Some(OsString::from("0")));
        let default = WgpuSettings::default();
        assert_eq!(
            settings.force_fallback_adapter,
            default.force_fallback_adapter
        );
        assert_eq!(settings.backends, default.backends);
    }
}
