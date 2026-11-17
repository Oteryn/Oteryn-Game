//! Client diagnostics: redacted log files, the crash-last-time flag, the ECS error context and
//! the diagnostics text and overlay (Options > Diagnostics, or a utility hotkey the player assigns).
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
#[allow(unused_imports)] // The dock registers this content when it lands.
pub(super) use overlay::{DiagnosticsText, OverlayBody, content};

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

/// Bevy's `LogPlugin` with the Oteryn file, ring and redacted stderr layers, at the saved
/// Options level. Tests write no log files.
pub(super) fn log_plugin(owner: &HostOwner) -> bevy::log::LogPlugin {
    let base = if cfg!(test) {
        None
    } else {
        crate::crash_report::user_data_directory()
    };
    logging::plugin(base, owner.settings.preferences.diagnostics.level_name())
}

/// Set with `OTERYN_DIAGNOSTICS_TEST_PANIC=1`: panics once the first diagnostics text exists, to
/// produce a real crash file from a build. The shipped UI has no way to set it.
const TEST_PANIC: &str = "OTERYN_DIAGNOSTICS_TEST_PANIC";

/// Registers diagnostics after `LogPlugin`: the startup line, the crash-last-time flag and the
/// overlay.
pub(super) struct DiagnosticsPlugin;
impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        info!(
            build = crate::CLIENT_BUILD,
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH,
            filter = logging::filter_source(),
            "Oteryn client starting"
        );
        app.insert_resource(detect_last_crash());
        install(app);
        if std::env::var(TEST_PANIC).is_ok_and(|value| value == "1") {
            warn!("{TEST_PANIC} is set: the client will panic on purpose");
            app.add_systems(Update, test_panic.after(overlay::refresh));
        }
    }
}

#[allow(clippy::panic)] // A deliberate crash for crash-file evidence, behind TEST_PANIC only.
fn test_panic(text: Res<overlay::DiagnosticsText>) {
    if !text.0.is_empty() {
        panic!("diagnostics test panic ({TEST_PANIC}=1) token=crash-sample-secret");
    }
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

fn install(app: &mut App) {
    app.set_error_handler(ecs_error)
        .init_resource::<LastCrash>()
        .init_resource::<overlay::DiagnosticsText>()
        .add_systems(Startup, overlay::setup)
        .add_systems(
            Update,
            (apply_log_level, overlay::refresh, overlay::project).chain(),
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
