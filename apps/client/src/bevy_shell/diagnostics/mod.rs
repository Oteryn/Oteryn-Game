//! Client diagnostics: redacted log files, the crash-last-time flag, the ECS error context and
//! the diagnostics overlay (Options > Diagnostics, or Ctrl+Shift+D).
mod logging;
mod overlay;
#[cfg(test)]
mod tests;

use super::owner::{HostOwner, Phase};
use bevy::{
    ecs::error::{BevyError, ErrorContext, Severity},
    log::{info, warn},
    prelude::*,
};
use std::path::PathBuf;

pub(super) use overlay::options_section;

/// Set on the start after a crash, for the UI to say "The client crashed last time; log saved
/// at ...". It is never cleared by the host; the UI decides when it has shown it.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::bevy_shell) struct LastCrash(pub Option<PathBuf>);
impl LastCrash {
    pub fn message(&self) -> Option<String> {
        self.0.as_ref().map(|path| {
            format!(
                "The client crashed last time; log saved at {}",
                path.display()
            )
        })
    }
}

/// Starts file logging before the host owners exist, so their errors reach the log.
pub(super) fn start_logging() {
    let base = crate::crash_report::user_data_directory();
    let source = logging::install(base.as_deref(), "info");
    info!(
        build = crate::CLIENT_BUILD,
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        filter = source,
        "Oteryn client starting"
    );
}

/// Reports a crash file written since the last start, once.
pub(super) fn detect_last_crash() -> LastCrash {
    let Some(directory) = crate::crash_report::user_data_directory()
        .map(|base| base.join(crate::crash_report::CRASH_DIRECTORY))
    else {
        return LastCrash::default();
    };
    match crate::crash_report::take_unseen_crash(&directory) {
        Ok(Some(path)) => {
            warn!(
                "The client crashed last time; log saved at {}",
                path.display()
            );
            LastCrash(Some(path))
        }
        Ok(None) => LastCrash::default(),
        Err(error) => {
            warn!("Cannot read the crash log directory: {error}");
            LastCrash::default()
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.set_error_handler(ecs_error)
        .init_resource::<LastCrash>()
        .init_resource::<overlay::Samples>()
        .init_resource::<overlay::OverlayText>()
        .add_systems(Startup, overlay::setup)
        .add_systems(
            Update,
            (
                toggle_hotkey,
                apply_log_level,
                overlay::sample,
                overlay::refresh,
                overlay::project,
            )
                .chain(),
        );
}

/// Bevy's fallback handler with the failing system or command named in the log first; the
/// default stays fail-fast, so a panicking error still panics after it is logged.
fn ecs_error(error: BevyError, context: ErrorContext) {
    if error.severity() == Severity::Panic {
        bevy::log::error!(
            target: "oteryn_client::ecs",
            "ECS {} `{}` failed: {error}",
            context.kind(),
            context.name()
        );
    }
    bevy::ecs::error::match_severity(error, context);
}

/// The lifecycle state a crash file and the overlay report.
pub(super) fn lifecycle(owner: &HostOwner) -> &'static str {
    match owner.phase {
        Phase::Playing => "playing",
        Phase::Closed => "closing",
        _ if owner.selection.pending().is_some() => "selection",
        Phase::Entering => "entering",
        Phase::Offline => "login",
    }
}

/// Ctrl+Shift+D shows or hides the overlay and saves the choice like the Options checkbox.
fn toggle_hotkey(keys: Option<Res<ButtonInput<KeyCode>>>, mut owner: NonSendMut<HostOwner>) {
    let Some(keys) = keys else {
        return;
    };
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if !(control && shift && keys.just_pressed(KeyCode::KeyD)) {
        return;
    }
    let mut preferences = owner.settings.preferences.clone();
    let show = !preferences.diagnostics.overlay;
    preferences.diagnostics.overlay = show;
    owner.settings.apply(preferences);
    // An open Options draft follows, so its Apply does not undo the hotkey.
    if owner.preferences_draft().is_some() {
        owner.edit_preferences(|draft| draft.diagnostics.overlay = show);
    }
}

fn apply_log_level(owner: NonSend<HostOwner>, mut applied: Local<Option<u8>>) {
    let diagnostics = owner.settings.preferences.diagnostics;
    if *applied == Some(diagnostics.log_level) {
        return;
    }
    let first = applied.is_none();
    *applied = Some(diagnostics.log_level);
    logging::set_level(diagnostics.level_name());
    if !first {
        info!("Log level set to {}", diagnostics.level_name());
    }
}
