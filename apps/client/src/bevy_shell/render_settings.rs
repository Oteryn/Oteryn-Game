//! The wgpu settings the host starts Bevy's renderer with.
use bevy::{
    DefaultPlugins,
    app::{PluginGroup, PluginGroupBuilder},
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
};
use std::ffi::OsString;

pub(super) fn plugins(smoke: bool, engine: oteryn_renderer::GraphicsEngine) -> PluginGroupBuilder {
    let mut settings = select(smoke, |name| std::env::var_os(name));
    if !smoke && std::env::var_os("WGPU_BACKEND").is_none() {
        settings.backends = match engine {
            oteryn_renderer::GraphicsEngine::Vulkan => Some(Backends::VULKAN),
            oteryn_renderer::GraphicsEngine::Direct3D12 if cfg!(windows) => Some(Backends::DX12),
            oteryn_renderer::GraphicsEngine::Direct3D12 => settings.backends,
        };
    }
    DefaultPlugins.set(RenderPlugin {
        render_creation: settings.into(),
        ..Default::default()
    })
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

#[cfg(test)]
mod tests {
    use super::*;

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
