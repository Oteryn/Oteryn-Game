//! Source-skinned HUD over the retained admitted session owners.
use super::super::{owner::HostOwner, ui::Skins};
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use oteryn_client_ui::battle::Action;
use oteryn_client_ui_bevy as source;
use oteryn_session::{ChaseMode, EntityRef, FightMode};

#[path = "hud/battle.rs"]
mod battle;
#[path = "hud/blessings.rs"]
mod blessings;
#[path = "hud/skills.rs"]
mod skills;
#[path = "hud/status_icons.rs"]
mod status_icons;
#[path = "hud/vip.rs"]
mod vip;
#[derive(Clone, Copy, Component)]
struct HudAction {
    generation: u64,
    command: Command,
}
#[derive(Clone, Copy)]
enum Command {
    Attack(EntityRef),
    Stop,
    Fight(FightMode),
    Chase(ChaseMode),
    Secure(bool),
    Expert(u8),
    BattleList(usize),
    BattleListEditor,
    Analyser(oteryn_client_ui::analyser::Tool),
}
#[derive(Component)]
struct VitalLabel(bool);
#[derive(Component)]
struct VitalFill(bool);
#[derive(Clone, Copy, Component)]
enum InfoText {
    Analyser,
    Status,
}

pub(super) fn install(app: &mut App) {
    blessings::spawn(app);
    skills::install(app);
    app.init_resource::<DevOverlay>();
    vip::install(app);
    app.add_systems(
        Startup,
        setup.after(super::setup).after(super::super::ui::setup),
    )
    .add_systems(
        Update,
        (project, battle::project, status_icons::project).chain(),
    )
    .add_observer(activate);
}
/// The developer overlay over the game window (status line, world and channel ids, combat
/// state); it is shown only when `OTERYN_DEV_OVERLAY=1`.
#[derive(Resource, Clone, Copy)]
pub(in crate::bevy_shell) struct DevOverlay(pub bool);
impl Default for DevOverlay {
    fn default() -> Self {
        Self(std::env::var("OTERYN_DEV_OVERLAY").is_ok_and(|value| value == "1"))
    }
}
/// HitpointsMana.qml: a 12x11 symbol, a 94x11 bar and the current value in a 42 px column.
const STATUS_ICON: Vec2 = Vec2::new(12., 11.);
const STATUS_BAR: Vec2 = Vec2::new(94., 11.);
const STATUS_VALUE: f32 = 42.;
/// The monk row of the status widget, shown only while the character has harmony or serene.
#[derive(Component)]
struct MonkRow;
#[derive(Component)]
struct MonkLabel;
fn setup(
    mut commands: Commands,
    regions: Res<super::GameRegions>,
    sidebar: Option<Res<super::Sidebar>>,
    skins: Res<Skins>,
    overlay: Res<DevOverlay>,
) {
    commands.entity(regions.vitals).insert(Node {
        width: percent(100.),
        flex_shrink: 0.,
        flex_direction: FlexDirection::Column,
        row_gap: px(2.),
        ..default()
    });
    let font = source::font(skins.font.clone(), false);
    let row = |commands: &mut Commands| {
        commands
            .spawn(Node {
                width: percent(100.),
                height: px(STATUS_ICON.y),
                flex_shrink: 0.,
                column_gap: px(2.),
                align_items: AlignItems::Center,
                ..default()
            })
            .id()
    };
    for (mana, color, key) in [
        (
            false,
            crate::bevy_shell::health_circle::health_colour(0, 0),
            "status-health",
        ),
        (true, Color::srgb_u8(0, 70, 180), "status-mana"),
    ] {
        let line = row(&mut commands);
        let icon = super::art::fixed(&mut commands, &skins.font, key, STATUS_ICON);
        let track = commands
            .spawn((
                Node {
                    width: px(STATUS_BAR.x),
                    height: px(STATUS_BAR.y),
                    flex_shrink: 0.,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(30, 30, 30)),
            ))
            .id();
        let fill = commands
            .spawn((
                VitalFill(mana),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.),
                    top: px(0.),
                    bottom: px(0.),
                    width: percent(0.),
                    ..default()
                },
                BackgroundColor(color),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(fill);
        let label = commands
            .spawn((
                VitalLabel(mana),
                source::text("—", font.clone()),
                Node {
                    width: px(STATUS_VALUE),
                    flex_shrink: 0.,
                    margin: UiRect::left(Val::Auto),
                    ..default()
                },
            ))
            .insert((
                TextLayout::justify(Justify::Center),
                TextColor(Color::WHITE),
            ))
            .id();
        commands.entity(line).add_children(&[icon, track, label]);
        commands.entity(regions.vitals).add_child(line);
    }
    let monk = row(&mut commands);
    commands.entity(monk).insert(MonkRow);
    commands.entity(monk).entry::<Node>().and_modify(|mut node| {
        node.display = Display::None;
    });
    let harmony = super::art::fixed(&mut commands, &skins.font, "status-harmony", STATUS_ICON);
    let label = commands
        .spawn((
            MonkLabel,
            source::text("", font.clone()),
        ))
        .insert(TextColor(Color::WHITE))
        .id();
    let serene = super::art::fixed(&mut commands, &skins.font, "status-serene", STATUS_ICON);
    commands
        .entity(monk)
        .add_children(&[harmony, label, serene]);
    commands.entity(regions.vitals).add_child(monk);
    status_icons::setup(&mut commands, regions.vitals);
    if overlay.0 {
        let status = commands
            .spawn((
                InfoText::Status,
                source::text("", font.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(4.),
                    top: px(4.),
                    right: px(4.),
                    ..default()
                },
                GlobalZIndex(5),
            ))
            .insert((TextColor(Color::srgb(1., 1., 0.6)), TextLayout::default()))
            .id();
        commands.entity(regions.world).add_child(status);
    }
    // The analytics selector widget: one button per analyser and its report.
    let Some(sidebar) = sidebar else {
        return;
    };
    let tools = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(2.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    for tool in oteryn_client_ui::analyser::Tool::ALL {
        let selector = button(
            &mut commands,
            &skins,
            0,
            Command::Analyser(tool),
            tool.title(),
            78.,
        );
        commands.entity(tools).add_child(selector);
    }
    let text = commands
        .spawn((
            InfoText::Analyser,
            source::text("", font),
            Node {
                width: percent(100.),
                padding: UiRect::all(px(3.)),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    commands
        .entity(sidebar.analytics)
        .add_children(&[tools, text]);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    command: Command,
    label: &str,
    width: f32,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            HudAction {
                generation,
                command,
            },
        ))
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    entity
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    actions: Query<&HudAction>,
    owner: NonSend<HostOwner>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if owner.modal_active() || action.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else { return };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match action.command {
        Command::Attack(target) => play.link.battle_request(
            if play
                .link
                .battle_panel()
                .combat
                .is_some_and(|combat| combat.target == Some(target))
            {
                Action::Stop
            } else {
                Action::Attack(target)
            },
        ),
        Command::Stop => play.link.battle_request(Action::Stop),
        Command::Fight(mode) => play.link.battle_request(Action::Fight(mode)),
        Command::Chase(mode) => play.link.battle_request(Action::Chase(mode)),
        Command::Secure(secure) => battle::request_pvp(&play.link, Some(secure), None),
        Command::Expert(mode) => battle::request_pvp(&play.link, None, Some(mode)),
        Command::BattleList(index) => {
            let _ = play.link.battle_select_list(index);
        }
        Command::BattleListEditor => play.link.battle_list_editor_open(),
        Command::Analyser(tool) => {
            super::analyser::open(&owner, tool);
        }
    }
}
// Bevy's independent queries separate projected vitals, public text and action state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn project(
    mut commands: Commands,
    (owner, playback): (
        NonSend<HostOwner>,
        Option<NonSend<crate::bevy_shell::audio::Playback>>,
    ),
    mut labels: Query<(&VitalLabel, &mut Text), (Without<InfoText>, Without<MonkLabel>)>,
    mut info: Query<(&InfoText, &mut Text), (Without<VitalLabel>, Without<MonkLabel>)>,
    mut monk: (
        Query<&mut Text, With<MonkLabel>>,
        Query<&mut Node, (With<MonkRow>, Without<VitalFill>)>,
    ),
    mut fills: Query<(&VitalFill, &mut Node, &mut BackgroundColor), Without<MonkRow>>,
    mut actions: Query<(Entity, &mut HudAction, Has<InteractionDisabled>)>,
    mut entered: Local<Option<u64>>,
) {
    let Some(play) = &owner.play else {
        *entered = None;
        return;
    };
    let generation = owner.generation();
    if *entered != Some(generation) {
        play.link.skills_widget_request();
        *entered = Some(generation);
    }
    let vitals = play.link.actor_vitals();
    for (label, mut text) in &mut labels {
        let value = vitals.map(|v| {
            if label.0 {
                (v.mana, v.max_mana)
            } else {
                (v.health, v.max_health)
            }
        });
        // Tibia shows the current value only.
        let next = value.map_or_else(|| "—".into(), |(current, _)| current.to_string());
        if text.0 != next {
            text.0 = next;
        }
    }
    for (fill, mut node, mut color) in &mut fills {
        let value = vitals.map(|v| {
            if fill.0 {
                (v.mana, v.max_mana)
            } else {
                (v.health, v.max_health)
            }
        });
        if !fill.0 {
            let (current, maximum) = value.unwrap_or((0, 0));
            color.0 = crate::bevy_shell::health_circle::health_colour(current, maximum);
        }
        let width = value
            .filter(|(_, max)| *max > 0)
            .map_or(0., |(current, max)| {
                current.min(max) as f32 / max as f32 * 100.
            });
        let width = percent(width);
        if node.width != width {
            node.width = width;
        }
    }
    let harmony = vitals
        .filter(|v| v.harmony > 0 || v.serene)
        .map(|v| v.harmony.to_string());
    for mut text in &mut monk.0 {
        let next = harmony.clone().unwrap_or_default();
        if text.0 != next {
            text.0 = next;
        }
    }
    for mut node in &mut monk.1 {
        let display = if harmony.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (kind, mut text) in &mut info {
        let next = match kind {
            InfoText::Status => {
                if owner.settings.preferences.compact_status {
                    oteryn_client_ui::spell_action_bar::vitals_text(vitals.as_ref())
                } else {
                    let line =
                        status_line(play, playback.as_ref().and_then(|audio| audio.status()));
                    format!("{line}\n{}", play.view.combat_status())
                }
            }
            InfoText::Analyser => play.link.analyser_text().replace("\r\n", "\n"),
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    let blocked = owner.modal_active() || play.view.gameplay_input_blocked(&play.link);
    let analyser_blocked = !play.link.analyser_panel().selected;
    for (entity, mut action, disabled) in &mut actions {
        if matches!(action.command, Command::Analyser(_)) {
            action.generation = generation;
            let blocked = blocked || analyser_blocked;
            if blocked && !disabled {
                commands.entity(entity).insert(InteractionDisabled);
            }
            if !blocked && disabled {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
        }
    }
}
/// The combined status line: custom hotkeys, item movement, the HUD status, audio, the
/// movement keys and movement feedback.
pub(super) fn status_line(
    play: &crate::bevy_shell::owner::ActivePlay,
    audio: Option<&str>,
) -> String {
    let settings = play.view.preferences().audio;
    let audio = format!(
        "Audio {}%{} ({})",
        settings.master_volume,
        if settings.muted { " muted" } else { "" },
        audio.unwrap_or("ready")
    );
    let status = lfmt!(
        "{movement} — {} — {audio} — Ctrl+arrows: Turn; Ctrl+F: Follow; Esc: Stop — {}",
        play.view.hud_status(),
        play.link.movement_feedback().unwrap_or(""),
        movement = play.link.item_movement_status(),
        audio = audio
    );
    format!("{} — {status}", play.link.custom_hotkey_status())
}
#[cfg(test)]
#[path = "hud/tests.rs"]
mod tests;

#[derive(Component)]
struct VipNameLabel {
    generation: u64,
    entry: u64,
}
#[derive(Component)]
struct VipMetadata {
    generation: u64,
    entry: u64,
    description: bool,
}
fn vip_icon_text(icon: Option<u8>) -> &'static str {
    const ICONS: [&str; 20] = [
        "", "♥", "☠", "ϟ", "⊕", "★", "$", "▲", "×", "✓", "●", "◆", "♠", "♣", "♦", "♫", "☼", "☾",
        "⚑", "!",
    ];
    icon.and_then(|index| ICONS.get(usize::from(index)).copied())
        .unwrap_or("")
}
