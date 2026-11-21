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
/// The inventory widget state: minimised to the Soul and Cap boxes, and the blessings panel
/// opened from its Blessings button (closed by default).
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct EquipmentWidget {
    pub minimised: bool,
    pub blessings: bool,
}
#[path = "hud/skills.rs"]
mod skills;
#[path = "hud/status_icons.rs"]
mod status_icons;
#[path = "hud/top_bar.rs"]
mod top_bar;
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
    /// Opens or closes the battle list menu (sort, lists, list editor).
    BattleMenu,
    /// Toggles a type filter of the active battle list.
    BattleFilter(battle::Filter),
    Analyser(oteryn_client_ui::analyser::Tool),
}
/// A combat control of the inventory widget; it is kept across frames, so its generation and
/// secure target follow the live session.
#[derive(Component)]
struct CombatControl;
/// The combat column of the inventory widget (Inventory.qml): a 42 px column with the Stop
/// button over 20x20 buttons in three rows, filled top to bottom.
const COMBAT_COLUMN: f32 = 42.;
const COMBAT_BUTTON: f32 = 20.;
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
    app.init_resource::<DevOverlay>()
        .init_resource::<battle::Menu>()
        .init_resource::<EquipmentWidget>();
    vip::install(app);
    app.add_systems(
        Startup,
        setup.after(super::setup).after(super::super::ui::setup),
    )
    .add_systems(
        Update,
        (
            project,
            battle::project,
            status_icons::project,
            top_bar::project,
        )
            .chain(),
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
#[derive(Component)]
struct SereneIcon;
mod layout;
use layout::setup;
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
    mut owner: NonSendMut<HostOwner>,
    mut menu: ResMut<battle::Menu>,
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
    // Battle list filters edit the saved active list; the session mailbox
    // takes them over on the next settings sync.
    match action.command {
        Command::BattleMenu => {
            menu.0 = !menu.0;
            return;
        }
        Command::BattleFilter(filter) => {
            let Some(active) = owner
                .play
                .as_ref()
                .map(|play| play.link.active_battle_list())
            else {
                return;
            };
            let mut preferences = owner.settings.preferences.clone();
            let Some(list) = preferences.battle_lists.get_mut(active) else {
                return;
            };
            match filter {
                battle::Filter::Players => list.players = !list.players,
                battle::Filter::Monsters => list.creatures = !list.creatures,
            }
            owner.settings.apply(preferences);
            return;
        }
        Command::BattleList(_) | Command::BattleListEditor => menu.0 = false,
        _ => {}
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
        Command::BattleMenu | Command::BattleFilter(_) => {}
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
        Query<&mut Visibility, With<SereneIcon>>,
    ),
    mut fills: Query<(&VitalFill, &mut Node, &mut BackgroundColor), Without<MonkRow>>,
    mut actions: Query<(Entity, &mut HudAction, Has<InteractionDisabled>), Without<CombatControl>>,
    mut controls: Query<
        (&mut HudAction, &mut BorderColor, &mut Node),
        (With<CombatControl>, Without<VitalFill>, Without<MonkRow>),
    >,
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
    let serene_visibility = if vitals.is_some_and(|v| v.serene) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut monk.2 {
        visibility.set_if_neq(serene_visibility);
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
    let combat = play.link.battle_panel().combat;
    let pvp = play.link.pvp_expert_panel();
    let expert = pvp.confirmed.filter(|_| pvp.selected);
    for (mut action, mut border, mut node) in &mut controls {
        action.generation = generation;
        let (selected, shown) = match action.command {
            Command::Fight(mode) => (combat.is_some_and(|c| c.modes.fight_mode == mode), true),
            Command::Chase(mode) => (combat.is_some_and(|c| c.modes.chase == mode), true),
            Command::Secure(_) | Command::Expert(_) => match expert {
                Some(state) => {
                    let controls = battle::pvp_controls(state);
                    let index = match action.command {
                        Command::Expert(mode) => usize::from(mode) + 1,
                        _ => 0,
                    };
                    if index == 0 {
                        action.command = controls[0].0;
                    }
                    (controls.get(index).is_some_and(|control| control.2), true)
                }
                None => (false, false),
            },
            _ => (false, true),
        };
        let next = BorderColor::all(if selected {
            Color::srgb_u8(230, 230, 230)
        } else {
            Color::NONE
        });
        if *border != next {
            *border = next;
        }
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
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
