//! The wgpu settings the host starts Bevy's renderer with.
use bevy::render::settings::{Backends, WgpuSettings};
use std::ffi::OsString;

/// A `--smoke` packaging run must start on a runner without a GPU, so it asks wgpu for the
/// software fallback adapter (WARP through DX12 on Windows), unless the environment already
/// chose one through `WGPU_FORCE_FALLBACK_ADAPTER` or `WGPU_BACKEND`. Normal runs keep Bevy's
/// defaults.
pub(super) fn select(smoke: bool, env: impl Fn(&str) -> Option<OsString>) -> WgpuSettings {
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
