//! The diagnostics text provider and its plain fixed panel, refreshed twice a second from the
//! client's own state, plus the Options > Diagnostics buttons. The panel content can be
//! registered in a window dock through [`content`].
use super::super::{
    game_ui::modal::ModalGuard,
    owner::{HostOwner, Phase},
    ui::Skins,
};
use crate::play::PerformancePacer;
use bevy::{
    ecs::entity::Entities,
    log::{info, warn},
    prelude::*,
    render::renderer::RenderAdapterInfo,
    ui_widgets::Activate,
    window::PrimaryWindow,
};
use oteryn_client_ui_bevy as source;
use std::time::{Duration, Instant};

/// Text refresh period.
pub(super) const REFRESH: Duration = Duration::from_millis(500);
/// Log lines added to "Copy diagnostics".
pub(super) const COPIED_LOG_LINES: usize = 50;

/// The fixed panel the overlay option shows.
#[derive(Component)]
pub(in crate::bevy_shell) struct Overlay;
/// A text node that shows [`DiagnosticsText`]; [`content`] spawns one.
#[derive(Component)]
pub(in crate::bevy_shell) struct OverlayBody;
#[derive(Component, Clone, Copy)]
pub(super) enum DiagnosticsButton {
    OpenLogFolder,
    CopyDiagnostics,
}

/// The current diagnostics lines, one per row; "Copy diagnostics" copies the same text.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct DiagnosticsText(pub String);

/// Spawns the monospace text content that shows [`DiagnosticsText`], for the fixed panel here
/// or a dock window.
pub(in crate::bevy_shell) fn content(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            OverlayBody,
            Name::new("diagnostics.text"),
            Text::new(""),
            TextFont::from_font_size(11.),
            TextColor(Color::srgb(0.85, 0.95, 0.85)),
            Node {
                padding: UiRect::all(px(4.)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn setup(mut commands: Commands) {
    let body = content(&mut commands);
    commands
        .spawn((
            Overlay,
            Name::new("diagnostics.overlay"),
            Node {
                position_type: PositionType::Absolute,
                right: px(8.),
                top: px(24.),
                width: px(380.),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.82)),
            GlobalZIndex(400),
            Pickable::IGNORE,
        ))
        .add_child(body);
}

/// The values the overlay shows; a value the client does not have is `None` and its line is
/// left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Snapshot {
    pub frames: Option<(u64, Duration)>,
    pub entities: u32,
    pub connection: &'static str,
    pub lifecycle: &'static str,
    pub latency: Option<(Duration, Duration)>,
    pub channel: Option<String>,
    pub character: Option<String>,
    pub position: Option<(i32, i32, i16)>,
    pub window: Option<(u32, u32, f32)>,
    pub ui_scale: f32,
    pub adapter: Option<String>,
    pub memory: Option<u64>,
    /// Session bytes `(sent, received)` since admission.
    pub traffic: Option<(u64, u64)>,
    pub counters: crate::crash_report::LogCounters,
    pub log_level: &'static str,
    pub last_crash: Option<String>,
}
impl Snapshot {
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some((fps, max)) = self.frames {
            lines.push(format!(
                "FPS {fps}  frame avg {:.1} ms  max {:.1} ms",
                1000. / fps.max(1) as f64,
                max.as_secs_f64() * 1000.
            ));
        }
        lines.push(format!("Entities {}", self.entities));
        lines.push(format!(
            "Connection {}  ({})",
            self.connection, self.lifecycle
        ));
        if let Some((round_trip, age)) = self.latency {
            lines.push(format!(
                "Command round trip {} ms  ({} s ago)",
                round_trip.as_millis(),
                age.as_secs()
            ));
        }
        if let Some((sent, received)) = self.traffic {
            lines.push(format!(
                "Traffic out {:.1} KiB  in {:.1} KiB",
                sent as f64 / 1024.,
                received as f64 / 1024.
            ));
        }
        if let Some(channel) = &self.channel {
            lines.push(channel.clone());
        }
        if let Some(character) = &self.character {
            lines.push(format!("Character {character}"));
        }
        if let Some((x, y, z)) = self.position {
            lines.push(format!("Position {x}, {y}, {z}"));
        }
        if let Some((width, height, factor)) = self.window {
            lines.push(format!(
                "Window {width}x{height}  scale {factor:.2}  UI {:.0}%",
                self.ui_scale * 100.
            ));
        }
        lines.push(format!("Build {}", crate::CLIENT_BUILD));
        if let Some(adapter) = &self.adapter {
            lines.push(format!("Renderer {adapter}"));
        }
        if let Some(memory) = self.memory {
            lines.push(format!("Memory RSS {:.1} MiB", memory as f64 / 1_048_576.));
        }
        lines.push(format!(
            "Log {}  warnings {}  errors {}",
            self.log_level, self.counters.warnings, self.counters.errors
        ));
        if let Some(error) = &self.counters.last_error {
            lines.push(format!("Last error {error}"));
        }
        if let Some(crash) = &self.last_crash {
            lines.push(crash.clone());
        }
        lines
    }
}

fn connection(phase: Phase) -> &'static str {
    match phase {
        Phase::Offline => "offline",
        Phase::Entering => "connecting",
        Phase::Playing => "connected",
        Phase::Closed => "closed",
    }
}

/// Process resident memory: `/proc` on Linux, the working set on Windows; other platforms
/// leave the line out.
fn resident_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
        let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kib * 1024)
    }
    #[cfg(windows)]
    {
        windows_working_set()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        None
    }
}

#[cfg(windows)]
#[allow(unsafe_code)] // One read into an owned, sized struct; the pseudo-handle needs no close.
fn windows_working_set() -> Option<u64> {
    use windows_sys::Win32::System::{
        ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };
    let size = u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size,
        ..Default::default()
    };
    // SAFETY: GetCurrentProcess returns a pseudo-handle valid for this process, and `counters`
    // is an initialized struct of exactly `size` bytes the call writes into.
    if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) } == 0 {
        return None;
    }
    u64::try_from(counters.WorkingSetSize).ok()
}

fn character_text(identity: [u8; 16]) -> String {
    let hex: String = identity.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

#[allow(clippy::too_many_arguments)] // One read of each real source the overlay names.
/// Rebuilds the text and the crash context every [`REFRESH`] of real time.
pub(super) fn refresh(
    owner: NonSend<HostOwner>,
    pacer: Option<NonSend<PerformancePacer>>,
    entities: &Entities,
    time: Res<Time<Real>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    adapter: Option<Res<RenderAdapterInfo>>,
    last_crash: Res<super::LastCrash>,
    mut refreshed: Local<Option<Duration>>,
    mut text: ResMut<DiagnosticsText>,
) {
    let now = time.elapsed();
    if refreshed.is_some_and(|at| now.saturating_sub(at) < REFRESH) {
        return;
    }
    *refreshed = Some(now);
    let adapter = adapter.map(|info| {
        format!(
            "{} ({:?}, {} {})",
            info.name, info.backend, info.driver, info.driver_info
        )
    });
    let play = owner.play.as_ref();
    let snapshot = Snapshot {
        frames: pacer.and_then(|pacer| pacer.measured(Instant::now())),
        entities: entities.count_spawned(),
        connection: connection(owner.phase),
        lifecycle: super::lifecycle(&owner),
        latency: play
            .and_then(|play| play.link.performance_command_latency())
            .map(|(at, round_trip)| (round_trip, at.elapsed())),
        channel: play.and_then(|play| play.view.served_channel_text()),
        character: play
            .and_then(|play| play.view.preference_character())
            .map(character_text),
        position: play.map(|play| play.view.satellite_player()),
        window: windows.single().ok().map(|window| {
            (
                window.physical_width(),
                window.physical_height(),
                window.scale_factor(),
            )
        }),
        ui_scale: scale.map_or(1., |scale| scale.0),
        adapter: adapter.clone(),
        memory: resident_memory(),
        traffic: play.and_then(|play| play.link.traffic_bytes()),
        counters: crate::crash_report::log_counters(),
        log_level: owner.settings.preferences.diagnostics.level_name(),
        last_crash: last_crash.message(),
    };
    let next = snapshot.lines().join("\n");
    if text.0 != next {
        text.0 = next;
    }
    crate::crash_report::set_crash_context(crate::crash_report::CrashContext {
        lifecycle: snapshot.lifecycle.to_owned(),
        adapter,
    });
}

pub(super) fn project(
    owner: NonSend<HostOwner>,
    text: Res<DiagnosticsText>,
    mut panels: Query<&mut Node, With<Overlay>>,
    mut bodies: Query<&mut Text, With<OverlayBody>>,
) {
    let display = if owner.settings.preferences.diagnostics.overlay {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut panels {
        if node.display != display {
            node.display = display;
        }
    }
    for mut body in &mut bodies {
        if body.0 != text.0 {
            body.0.clone_from(&text.0);
        }
    }
}

/// The overlay text and the last log lines, redacted, for a bug report.
pub(super) fn clipboard_text(overlay: &str) -> String {
    let mut text = format!("Oteryn client diagnostics\n{overlay}\n\nLast log lines:\n");
    for line in crate::crash_report::recent_log_lines(COPIED_LOG_LINES) {
        text.push_str(&line);
        text.push('\n');
    }
    crate::crash_report::redact(&text)
}

/// The platform file browser, with the folder as one argument and no shell.
pub(super) fn folder_command(folder: &std::path::Path) -> std::process::Command {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut command = std::process::Command::new(program);
    command.arg(folder);
    command
}

/// Options > Diagnostics: the two actions beside the level and overlay rows.
pub(in crate::bevy_shell) fn options_section(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
) {
    for (action, caption, name) in [
        (
            DiagnosticsButton::OpenLogFolder,
            "Open log folder",
            "diagnostics.open-log-folder",
        ),
        (
            DiagnosticsButton::CopyDiagnostics,
            "Copy diagnostics to clipboard",
            "diagnostics.copy",
        ),
    ] {
        let button = commands
            .spawn((
                action,
                source::button(skins.buttons.clone(), 190.),
                Name::new(name),
            ))
            .observe(activate)
            .id();
        let label = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(label);
        commands.entity(parent).add_child(button);
    }
}

fn activate(
    event: On<Activate>,
    buttons: Query<&DiagnosticsButton>,
    guard: ModalGuard,
    text: Res<DiagnosticsText>,
    clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    match buttons.get(event.entity) {
        Ok(DiagnosticsButton::OpenLogFolder) => open_log_folder(),
        Ok(DiagnosticsButton::CopyDiagnostics) => {
            let Some(mut clipboard) = clipboard else {
                warn!("Copy diagnostics: no clipboard is available");
                return;
            };
            match clipboard.set_text(clipboard_text(&text.0)) {
                Ok(()) => info!("Diagnostics copied to the clipboard"),
                Err(error) => warn!("Copy diagnostics failed: {error}"),
            }
        }
        Err(_) => {}
    }
}

fn open_log_folder() {
    let Some(folder) = super::logging::directory().or_else(|| {
        crate::crash_report::user_data_directory()
            .map(|base| base.join(crate::crash_report::LOG_DIRECTORY))
    }) else {
        warn!("Open log folder: the client user-data directory is unavailable");
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&folder) {
        warn!(
            "Open log folder: cannot create {}: {error}",
            folder.display()
        );
        return;
    }
    match folder_command(&folder)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(error) => warn!("Open log folder {} failed: {error}", folder.display()),
    }
}
