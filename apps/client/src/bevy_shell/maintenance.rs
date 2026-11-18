//! Updates and repair window the launcher opens once the client handed maintenance off
//! (`--updates`): it checks the signed stable release, then updates, verifies or repairs the
//! installation on request. The window cannot close while an operation runs.
use super::ui::EXTERNAL_FONT as FONT;
use crate::updater::{Action, maintain};
use bevy::{
    asset::AssetPlugin,
    prelude::*,
    ui_widgets::{Activate, Button},
    window::WindowCloseRequested,
};
use oteryn_client_ui_bevy as source;
use oteryn_platform_client::updates::UpdateError;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, TryRecvError},
    },
};

type Outcome = Result<String, UpdateError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Working,
    Choose,
    OfferRepair,
    Done,
}
struct Maintenance {
    directory: PathBuf,
    stage: Stage,
    pending: Option<(Action, Receiver<Outcome>)>,
    report: String,
    /// The latest finished operation; the launcher reports it after the window closes.
    outcome: Arc<Mutex<Option<Outcome>>>,
    /// Starts one operation; the window uses [`start`], tests a deterministic operation.
    start: fn(PathBuf, Action) -> Receiver<Outcome>,
}
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    Update,
    Verify,
    Repair,
    Close,
}
#[derive(Component)]
struct Report;

/// Opens the window over `directory` and returns the last operation's result.
pub fn run(directory: &Path, args: &[OsString]) -> Outcome {
    let assets = ui_assets(directory, args);
    let outcome = Arc::new(Mutex::new(None));
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: assets
                    .as_deref()
                    .unwrap_or(directory)
                    .to_string_lossy()
                    .into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                close_when_requested: false,
                primary_window: Some(Window {
                    title: "Oteryn updates and repair".into(),
                    resolution: (560, 320).into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)));
    if assets.is_some() {
        app.insert_resource(MaintenanceFont);
    }
    install(
        &mut app,
        Maintenance {
            directory: directory.to_path_buf(),
            stage: Stage::Working,
            pending: Some((Action::Check, start(directory.to_path_buf(), Action::Check))),
            report: "Checking the signed stable release…".into(),
            outcome: outcome.clone(),
            start,
        },
    );
    let _ = app.run();
    outcome
        .lock()
        .ok()
        .and_then(|mut outcome| outcome.take())
        .unwrap_or(Err(UpdateError::Unavailable))
}

/// The UI-art root holding the font: `--ui-assets`, `OTERYN_UI_ASSETS`, the active release,
/// then the installation directory.
fn ui_assets(directory: &Path, args: &[OsString]) -> Option<PathBuf> {
    let explicit = args
        .iter()
        .position(|argument| argument == "--ui-assets")
        .and_then(|at| args.get(at + 1))
        .map(PathBuf::from);
    let release = std::fs::read_to_string(directory.join("current.txt"))
        .ok()
        .map(|current| current.trim().to_owned())
        .filter(|current| crate::updater::release_directory(current))
        .map(|current| directory.join("releases").join(current).join("ui-assets"));
    explicit
        .into_iter()
        .chain(std::env::var_os("OTERYN_UI_ASSETS").map(PathBuf::from))
        .chain(release)
        .chain(Some(directory.join("ui-assets")))
        .find(|root| root.join(FONT).is_file())
}

/// Runs one operation on its own thread; a failed spawn drops the sender, which the window
/// reports as a stopped operation.
fn start(directory: PathBuf, action: Action) -> Receiver<Outcome> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("launcher-maintenance".into())
        .spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|_| UpdateError::Unavailable)
                .and_then(|runtime| runtime.block_on(maintain(&directory, "stable", action)));
            let _ = tx.send(result);
        });
    rx
}

fn install(app: &mut App, state: Maintenance) {
    app.add_message::<AppExit>()
        .add_message::<WindowCloseRequested>()
        .insert_non_send(state)
        .add_systems(Startup, setup)
        .add_systems(Update, (poll, close_requested, project).chain());
}

#[derive(Resource)]
struct MaintenanceFont;

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    external_font: Option<Res<MaintenanceFont>>,
) {
    let font = if external_font.is_some() {
        assets.load(FONT)
    } else {
        Handle::default()
    };
    commands.spawn(Camera2d);
    let root = commands
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            padding: px(16.).all(),
            row_gap: px(10.),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let report = commands
        .spawn((Report, source::text("", source::font(font.clone(), false))))
        .id();
    commands.entity(root).add_child(report);
    for (choice, label) in [
        (Choice::Update, "Download and install update"),
        (Choice::Verify, "Verify installed files"),
        (
            Choice::Repair,
            "Download and repair from the signed release",
        ),
        (Choice::Close, "Close"),
    ] {
        let button = commands
            .spawn((
                choice,
                Button,
                Node {
                    width: px(360.),
                    height: px(24.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::text(label, source::font(font.clone(), false)))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(root).add_child(button);
    }
}

/// Takes a finished operation and moves to the next choice.
fn poll(mut state: NonSendMut<Maintenance>) {
    let Some((action, receiver)) = &state.pending else {
        return;
    };
    let (action, result) = match receiver.try_recv() {
        Ok(result) => (*action, result),
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => (*action, Err(UpdateError::Unavailable)),
    };
    state.pending = None;
    if let Ok(mut outcome) = state.outcome.lock() {
        *outcome = Some(result.clone());
    }
    (state.stage, state.report) = match (action, result) {
        (Action::Check, Ok(version)) => (
            Stage::Choose,
            format!("{version}\n\nDownload and install the update, or verify the installed files."),
        ),
        (Action::Verify, Err(_)) => (
            Stage::OfferRepair,
            "Verification failed. Download and repair from the signed release?".into(),
        ),
        (_, Ok(report)) => (Stage::Done, report),
        (_, Err(error)) => (Stage::Done, error.to_string()),
    };
}

fn activate(
    event: On<Activate>,
    choices: Query<&Choice>,
    mut state: NonSendMut<Maintenance>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    let action = match (*choice, state.stage) {
        (Choice::Close, Stage::Working) => return,
        (Choice::Close, _) => {
            exit.write(AppExit::Success);
            return;
        }
        (Choice::Update, Stage::Choose) => Action::Update,
        (Choice::Verify, Stage::Choose) => Action::Verify,
        (Choice::Repair, Stage::OfferRepair) => Action::Repair,
        _ => return,
    };
    state.stage = Stage::Working;
    state.report = match action {
        Action::Update => "Downloading and installing the verified update…",
        Action::Verify => "Verifying the installed files…",
        _ => "Downloading and repairing from the signed release…",
    }
    .into();
    state.pending = Some((action, (state.start)(state.directory.clone(), action)));
}

/// Closing the window waits for a running operation to finish.
fn close_requested(
    mut requests: MessageReader<WindowCloseRequested>,
    state: NonSend<Maintenance>,
    mut exit: MessageWriter<AppExit>,
) {
    if requests.read().count() > 0 && state.stage != Stage::Working {
        exit.write(AppExit::Success);
    }
}

fn project(
    state: NonSend<Maintenance>,
    mut reports: Query<&mut Text, With<Report>>,
    mut buttons: Query<(&Choice, &mut Node)>,
) {
    for mut report in &mut reports {
        if report.0 != state.report {
            report.0.clone_from(&state.report);
        }
    }
    for (choice, mut node) in &mut buttons {
        let shown = match choice {
            Choice::Update | Choice::Verify => state.stage == Stage::Choose,
            Choice::Repair => state.stage == Stage::OfferRepair,
            Choice::Close => state.stage != Stage::Working,
        };
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
#[path = "maintenance_tests.rs"]
mod tests;
