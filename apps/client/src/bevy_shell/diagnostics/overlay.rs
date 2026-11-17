//! The diagnostics overlay: a movable, resizable monospace panel refreshed twice a second from
//! the client's own state, plus the Options > Diagnostics buttons.
use super::super::{
    game_ui::{GameModal, modal::ModalGuard},
    owner::{HostOwner, Phase},
    ui::Skins,
    window_drag::DraggableDialog,
};
use bevy::{
    ecs::entity::Entities,
    log::{info, warn},
    picking::events::PointerDrag,
    prelude::*,
    render::renderer::RenderAdapterInfo,
    ui_widgets::Activate,
    window::PrimaryWindow,
};
use oteryn_client_ui_bevy as source;
use std::time::Duration;

/// Overlay text refresh period.
pub(super) const REFRESH: Duration = Duration::from_millis(500);
/// Log lines added to "Copy diagnostics".
pub(super) const COPIED_LOG_LINES: usize = 50;
const MIN_SIZE: Vec2 = Vec2::new(220., 120.);

#[derive(Component)]
pub(super) struct Overlay;
#[derive(Component)]
pub(super) struct OverlayBody;
#[derive(Component)]
struct ResizeGrip;
#[derive(Component, Clone, Copy)]
pub(super) enum DiagnosticsButton {
    OpenLogFolder,
    CopyDiagnostics,
}

/// Frame times since the last refresh.
#[derive(Resource, Default)]
pub(super) struct Samples {
    frames: u32,
    total: Duration,
    max: Duration,
    since_refresh: Duration,
    refreshed: bool,
}
/// The current overlay text; "Copy diagnostics" copies the same text.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub(super) struct OverlayText(pub String);

pub(super) fn setup(mut commands: Commands) {
    let font = TextFont::from_font_size(11.);
    let panel = commands
        .spawn((
            Overlay,
            DraggableDialog,
            Name::new("diagnostics.overlay"),
            Node {
                position_type: PositionType::Absolute,
                left: px(8.),
                top: px(24.),
                width: px(380.),
                height: px(330.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.82)),
            BorderColor::all(Color::srgb(0.35, 0.35, 0.35)),
            GlobalZIndex(400),
        ))
        .id();
    let caption = commands
        .spawn((
            Text::new("Diagnostics — Ctrl+Shift+D"),
            font.clone(),
            TextColor(Color::srgb(0.9, 0.8, 0.5)),
            Node {
                height: px(17.),
                flex_shrink: 0.,
                padding: UiRect::horizontal(px(4.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
            Pickable::IGNORE,
        ))
        .id();
    let body = commands
        .spawn((
            OverlayBody,
            Text::new(""),
            font,
            TextColor(Color::srgb(0.85, 0.95, 0.85)),
            Node {
                padding: UiRect::all(px(4.)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let grip = commands
        .spawn((
            ResizeGrip,
            Name::new("diagnostics.overlay.resize"),
            Node {
                position_type: PositionType::Absolute,
                right: px(0.),
                bottom: px(0.),
                width: px(12.),
                height: px(12.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.45, 0.45, 0.45)),
        ))
        .observe(resize)
        .id();
    commands.entity(panel).add_children(&[caption, body, grip]);
}

fn resize(
    mut event: On<PointerDrag>,
    scale: Res<UiScale>,
    parents: Query<&ChildOf>,
    mut panels: Query<(&mut Node, &ComputedNode), With<Overlay>>,
) {
    event.propagate(false);
    let Ok(parent) = parents.get(event.entity) else {
        return;
    };
    let Ok((mut node, computed)) = panels.get_mut(parent.parent()) else {
        return;
    };
    let factor = if scale.0.is_finite() && scale.0 > 0. {
        scale.0
    } else {
        1.
    };
    let current = computed.size * computed.inverse_scale_factor;
    let size = (current + event.delta / factor).max(MIN_SIZE);
    if size.is_finite() {
        node.width = px(size.x);
        node.height = px(size.y);
    }
}

/// Counts every frame's real duration.
pub(super) fn sample(time: Res<Time<Real>>, mut samples: ResMut<Samples>) {
    let delta = time.delta();
    samples.frames = samples.frames.saturating_add(1);
    samples.total += delta;
    samples.max = samples.max.max(delta);
    samples.since_refresh += delta;
}

/// The values the overlay shows; a value the client does not have is `None` and its line is
/// left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Snapshot {
    pub frames: Option<(f32, f32, f32)>,
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
    pub counters: crate::crash_report::LogCounters,
    pub open_windows: Vec<String>,
    pub log_level: &'static str,
    pub last_crash: Option<String>,
}
impl Snapshot {
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some((fps, average, max)) = self.frames {
            lines.push(format!(
                "FPS {fps:.0}  frame avg {average:.1} ms  max {max:.1} ms"
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
        lines.push(format!("Open windows {}", self.open_windows.len()));
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

/// Process resident memory from `/proc`; other platforms leave the line out.
fn resident_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
        let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kib * 1024)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
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

/// A dialog's layout, visibility and name, for the open-windows line.
type ModalRow = (
    &'static Node,
    Option<&'static Visibility>,
    Option<&'static Name>,
);

#[allow(clippy::too_many_arguments)] // One read of each real source the overlay names.
/// Rebuilds the text and the crash context every [`REFRESH`].
pub(super) fn refresh(
    owner: NonSend<HostOwner>,
    entities: &Entities,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    adapter: Option<Res<RenderAdapterInfo>>,
    modals: Query<ModalRow, With<GameModal>>,
    last_crash: Res<super::LastCrash>,
    mut samples: ResMut<Samples>,
    mut text: ResMut<OverlayText>,
) {
    if samples.refreshed && samples.since_refresh < REFRESH {
        return;
    }
    let frames = (samples.frames > 0 && !samples.total.is_zero()).then(|| {
        let seconds = samples.total.as_secs_f32();
        (
            samples.frames as f32 / seconds,
            seconds * 1000. / samples.frames as f32,
            samples.max.as_secs_f32() * 1000.,
        )
    });
    *samples = Samples {
        refreshed: true,
        ..default()
    };
    let open_windows: Vec<String> = modals
        .iter()
        .filter(|(node, visibility, _)| {
            node.display != Display::None && *visibility != Some(&Visibility::Hidden)
        })
        .map(|(_, _, name)| name.map_or_else(|| "unnamed".to_owned(), |name| name.to_string()))
        .collect();
    let adapter = adapter.map(|info| {
        format!(
            "{} ({:?}, {} {})",
            info.name, info.backend, info.driver, info.driver_info
        )
    });
    let play = owner.play.as_ref();
    let snapshot = Snapshot {
        frames,
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
        counters: crate::crash_report::log_counters(),
        open_windows: open_windows.clone(),
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
        open_windows,
    });
}

pub(super) fn project(
    owner: NonSend<HostOwner>,
    text: Res<OverlayText>,
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
    if display == Display::None {
        return;
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
    text: Res<OverlayText>,
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
