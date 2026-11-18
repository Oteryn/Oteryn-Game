//! Signed release maintenance, separate from gameplay presentations.
use super::{
    game_ui::modal::ModalGuard,
    owner::{HostOwner, Phase},
    ui::Skins,
};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
};
#[cfg(test)]
mod tests;
mod widgets;
mod worker;

#[derive(Default)]
struct State {
    open: bool,
    directory: Option<PathBuf>,
    ui_assets: Option<PathBuf>,
    pending: Option<Receiver<worker::Event>>,
    report: String,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Open,
    Check,
    Download,
    Maintain,
    Close,
}
#[derive(Component)]
struct Dialog;
#[derive(Component)]
struct Status;

pub(super) fn install(app: &mut App, ui_assets: Option<PathBuf>) {
    app.add_message::<AppExit>().insert_non_send(State {
        directory: std::env::current_exe().ok().and_then(|path| worker::installation(&path)),
        ui_assets,
        report: "Stable release updates use the build-pinned signing root.\nCheck / Retry checks metadata; Download verifies a package without activating it.\nClose client and maintain opens the launcher for update, verification or repair.".into(),
        ..Default::default()
    }).add_systems(Startup, widgets::setup.after(super::ui::setup))
        .add_systems(Update, project.before(super::input::route))
        .add_observer(activate);
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    actions: Query<&Action>,
    mut state: NonSendMut<State>,
    mut owner: NonSendMut<HostOwner>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) {
        return;
    }
    if *action == Action::Close {
        state.open = false;
        return;
    }
    if owner.phase != Phase::Offline {
        return;
    }
    if *action == Action::Open {
        state.open = true;
        return;
    }
    if state.pending.is_some() {
        return;
    }
    let Some(directory) = state.directory.clone() else {
        state.report = "No managed installation found. Use the trusted Oteryn installer, then retry from the installed client.".into();
        return;
    };
    if *action == Action::Maintain {
        match handoff(&directory, state.ui_assets.as_deref()) {
            Ok(()) => {
                owner.close();
                exit.write(AppExit::Success);
            }
            Err(error) => {
                state.report =
                    format!("Could not open launcher: {error}. Retry or run the trusted installer.")
            }
        }
    } else {
        match worker::start(directory, *action == Action::Download) {
            Ok(receiver) => {
                state.report = "Starting signed update check…".into();
                state.pending = Some(receiver);
            }
            Err(error) => state.report = error,
        }
    }
}
fn handoff(
    directory: &std::path::Path,
    ui_assets: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let launcher = if cfg!(windows) {
        "oteryn-launcher.exe"
    } else {
        "oteryn-launcher"
    };
    let mut command = std::process::Command::new(directory.join(launcher));
    command
        .current_dir(directory)
        .args(["--after-client", "--updates"]);
    if let Some(ui_assets) = ui_assets {
        command.arg("--ui-assets").arg(ui_assets);
    }
    command.spawn()?;
    Ok(())
}
fn project(
    mut commands: Commands,
    mut state: NonSendMut<State>,
    owner: NonSend<HostOwner>,
    mut dialogs: Query<&mut Node, (With<Dialog>, Without<Action>)>,
    mut actions: Query<(Entity, &Action, &mut Node), Without<Dialog>>,
    mut status: Query<&mut Text, With<Status>>,
) {
    loop {
        let event = state.pending.as_ref().map(Receiver::try_recv);
        match event {
            Some(Ok(worker::Event::Progress(report))) => state.report = report,
            Some(Ok(worker::Event::Finished(result))) => {
                state.report = result.unwrap_or_else(|error| format!("{error}. Active release unchanged. Check / Retry to try again. Configuration errors require a trusted build with a signing root; storage errors require free space and write access."));
                state.pending = None;
            }
            Some(Err(TryRecvError::Disconnected)) => {
                state.pending = None;
                state.report =
                    "Updater worker stopped. Active release unchanged. Check / Retry.".into();
            }
            _ => break,
        }
    }
    let offline = owner.phase == Phase::Offline;
    for mut node in &mut dialogs {
        node.display = if state.open && offline {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (entity, action, mut node) in &mut actions {
        if *action == Action::Open {
            node.display = if offline {
                Display::Flex
            } else {
                Display::None
            };
        }
        if matches!(action, Action::Check | Action::Download | Action::Maintain) {
            if state.pending.is_some() {
                commands.entity(entity).insert(InteractionDisabled);
            } else {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
        }
    }
    for mut text in &mut status {
        text.0.clone_from(&state.report);
    }
}
