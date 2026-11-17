//! The Tibia Global control buttons at the bottom of the fixed sidebar panel: Store on top of
//! a five-column grid of 20x20 icon buttons, with Options, Logout and the button manager in a
//! 42 px column on the right. Every toolbar entry has a grid button; the button manager picks
//! which are shown.
use super::{Control, HostOwner, TOOLBAR, open_toolbar};
use crate::bevy_shell::{
    game_ui::{
        art,
        hud_panes::{SidebarWidgets, Widget},
    },
    ui::Skins,
};
use bevy::{prelude::*, ui_widgets::Activate};

/// Tibia Global control widget geometry (ControlButtons.qml).
const LEFT_WIDTH: f32 = 108.;
const RIGHT_WIDTH: f32 = 42.;
const BUTTON: f32 = 20.;
const PITCH: f32 = 22.;
const COLUMNS: u16 = 5;

/// The grid buttons shown until the player picks their own set in the button manager: the
/// Tibia Global `controlButtonsOptions` defaults in their order, each mapped to the Oteryn
/// feature that serves it (`tibia.*` ids are the [`EXTRA`] buttons with no toolbar entry).
pub(in crate::bevy_shell) const DEFAULT: &[&str] = &[
    "shell.skills",            // skillsWidget
    "battle",                  // battleListWidget
    "shell.spells",            // spellListWidget
    "vip",                     // vipWidget
    "tibia.lens_help",         // lenshelpFunction
    "shell.quests",            // questDialog
    "shell.notices",           // compendiumDialog
    "cyclopedia",              // cyclopediaDialog
    "highscores",              // highscoresDialog
    "tibia.player_guide",      // playerGuide
    "bars",                    // manageShortcuts
    "wheel",                   // skillWheelDialog
    "tasks",                   // taskboard
    "prey",                    // preyDialog
    "tibia.prey_widget",       // preyWidget
    "shell.analysers",         // analyticsSelectorWidget
    "bestiary_tracker",        // bestiaryTrackerWidget
    "tibia.imbuement_tracker", // imbuementTrackerWidget
    "forge",                   // exaltationForgeDialog
    "proficiency",             // weaponProficiency
    "bosstiary",               // bosstiaryDialog
    "bosstiary_tracker",       // bosstiaryTrackerWidget
    "tibia.boss_slots",        // bossslotsDialog
    "tibia.friends",           // friendsDialog
    "rewards",                 // rewardWallDialog
];
/// A Tibia control button with no toolbar entry, opening the Oteryn view that serves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) enum Extra {
    /// Lens help mode (features/local-lens-help).
    LensHelp,
    /// The guide pages in the Notices window (feature_dialogs/guide.rs).
    PlayerGuide,
    /// The prey window (game_ui/prey.rs).
    PreyWidget,
    /// The equipped-item imbuement tracker (game_ui/imbuement_tracker.rs).
    ImbuementTracker,
    /// The boss slots of the Bosstiary window (game_ui/bosstiary.rs).
    BossSlots,
    /// The Account Friends form (features/0043-vip/account.rs).
    Friends,
}
impl Extra {
    pub(in crate::bevy_shell) fn id(self) -> &'static str {
        EXTRA
            .iter()
            .find(|entry| entry.2 == self)
            .map_or("", |entry| entry.0)
    }
}
/// The [`Extra`] buttons: grid id, label and view.
pub(in crate::bevy_shell) const EXTRA: &[(&str, &str, Extra)] = &[
    ("tibia.boss_slots", "Boss Slots", Extra::BossSlots),
    ("tibia.friends", "Friends", Extra::Friends),
    (
        "tibia.imbuement_tracker",
        "Imbuement Tracker",
        Extra::ImbuementTracker,
    ),
    ("tibia.lens_help", "Lens Help", Extra::LensHelp),
    ("tibia.player_guide", "Player Guide", Extra::PlayerGuide),
    ("tibia.prey_widget", "Prey Widget", Extra::PreyWidget),
];
/// Entries that sit in the fixed columns instead of the grid.
const FIXED: &[&str] = &["shell.options", "store"];

/// One control button.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::bevy_shell) enum ControlButton {
    Grid(Control),
    Extra(Extra),
    Store,
    Options,
    Logout,
    Manage,
}
/// The grid that holds the [`ControlButton::Grid`] buttons.
#[derive(Component)]
pub(in crate::bevy_shell) struct ControlGrid;

/// The art key of a toolbar entry: `control-` and its label in lowercase words.
pub(in crate::bevy_shell) fn key(label: &str) -> String {
    let words: Vec<String> = label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    format!("control-{}", words.join("-"))
}
/// Two-letter text fallback for an entry without its icon.
fn short(label: &str) -> String {
    let mut words = label.split_whitespace();
    match (words.next(), words.next()) {
        (Some(first), Some(second)) => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
        (Some(first), None) => first.chars().take(2).collect(),
        _ => String::new(),
    }
}

/// Spawns the control buttons into the sidebar panel's toolbar region.
pub(in crate::bevy_shell) fn spawn(commands: &mut Commands, skins: &Skins, region: Entity) {
    let widget = commands
        .spawn((
            Name::new("Control buttons"),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
        ))
        .id();
    let left = commands
        .spawn(Node {
            width: px(LEFT_WIDTH),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            row_gap: px(PITCH - BUTTON),
            ..default()
        })
        .id();
    let store = fixed(
        commands,
        skins,
        ControlButton::Store,
        "control-store",
        "Store",
    );
    commands
        .entity(store)
        .entry::<Node>()
        .and_modify(|mut node| node.width = px(LEFT_WIDTH));
    let grid = commands
        .spawn((
            ControlGrid,
            Node {
                width: px(LEFT_WIDTH),
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::px(COLUMNS, BUTTON),
                grid_auto_rows: vec![GridTrack::px(BUTTON)],
                column_gap: px(PITCH - BUTTON),
                row_gap: px(PITCH - BUTTON),
                ..default()
            },
        ))
        .id();
    for &(label, control) in TOOLBAR {
        if FIXED.contains(&control.id()) {
            continue;
        }
        let button = art::spawn(
            commands,
            &skins.font,
            key(label),
            &short(label),
            Vec2::splat(BUTTON),
        );
        commands
            .entity(button)
            .insert((ControlButton::Grid(control), bevy::ui_widgets::Button))
            .observe(activate);
        commands.entity(grid).add_child(button);
    }
    for &(_, label, extra) in EXTRA {
        let button = art::spawn(
            commands,
            &skins.font,
            key(label),
            &short(label),
            Vec2::splat(BUTTON),
        );
        commands
            .entity(button)
            .insert((ControlButton::Extra(extra), bevy::ui_widgets::Button))
            .observe(activate);
        commands.entity(grid).add_child(button);
    }
    commands.entity(left).add_children(&[store, grid]);
    let right = commands
        .spawn(Node {
            width: px(RIGHT_WIDTH),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            row_gap: px(PITCH - BUTTON),
            ..default()
        })
        .id();
    for (button, key, text) in [
        (ControlButton::Options, "control-options", "Options"),
        (ControlButton::Logout, "control-logout", "Logout"),
        (ControlButton::Manage, "control-manage", "Edit"),
    ] {
        let entity = fixed(commands, skins, button, key, text);
        commands.entity(right).add_child(entity);
    }
    commands.entity(widget).add_children(&[left, right]);
    commands.entity(region).add_child(widget);
}
fn fixed(
    commands: &mut Commands,
    skins: &Skins,
    button: ControlButton,
    key: &str,
    text: &str,
) -> Entity {
    let entity = art::spawn(
        commands,
        &skins.font,
        key,
        text,
        Vec2::new(RIGHT_WIDTH, BUTTON),
    );
    commands
        .entity(entity)
        .insert((button, bevy::ui_widgets::Button))
        .observe(activate);
    entity
}

/// The grid buttons the player has enabled, in their order.
fn enabled(owner: &HostOwner) -> Vec<String> {
    owner
        .settings
        .preferences
        .toolbar_order
        .clone()
        .unwrap_or_else(|| DEFAULT.iter().map(|id| (*id).to_owned()).collect())
}

fn activate(
    event: On<Activate>,
    buttons: Query<&ControlButton>,
    mut owner: NonSendMut<HostOwner>,
    modal: crate::bevy_shell::game_ui::modal::ModalGuard,
    mut widgets: ResMut<SidebarWidgets>,
    mut manager: ResMut<super::Manager>,
) {
    if !modal.allows(event.entity) || owner.modal_active() || owner.play.is_none() {
        return;
    }
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    match *button {
        ControlButton::Store => {
            open_toolbar(&mut owner, Control::Shortcut("store"));
        }
        ControlButton::Options => {
            open_toolbar(&mut owner, Control::Options);
        }
        ControlButton::Logout => owner.request_logout_confirmation(),
        ControlButton::Manage => manager.open = true,
        // The skills and analytics buttons toggle their sidebar widgets; opening the skills
        // widget also asks the server for fresh values.
        ControlButton::Grid(Control::Skills) => {
            toggle_skills(&mut owner, &mut widgets);
        }
        ControlButton::Grid(Control::Analysers) => {
            widgets.toggle(Widget::AnalyticsSelector);
        }
        ControlButton::Grid(control) => {
            open_toolbar(&mut owner, control);
        }
        ControlButton::Extra(extra) => open_extra(&mut owner, extra),
    }
}

fn toggle_skills(owner: &mut HostOwner, widgets: &mut SidebarWidgets) {
    if widgets.toggle(Widget::Skills) {
        open_toolbar(owner, Control::Skills);
    }
}

fn open_extra(owner: &mut HostOwner, extra: Extra) {
    match extra {
        Extra::PlayerGuide => {
            open_toolbar(owner, Control::Notices);
        }
        Extra::PreyWidget => {
            open_toolbar(owner, Control::Shortcut("prey"));
        }
        Extra::BossSlots => {
            if !owner.settings.preferences.panel_visible("bosstiary") {
                open_toolbar(owner, Control::Panel("bosstiary"));
            }
            if let Some(play) = &owner.play {
                play.link.boss_encounter_request(
                    oteryn_client_ui::boss_encounter::Intent::SlotsDetailed,
                );
            }
        }
        Extra::LensHelp | Extra::ImbuementTracker | Extra::Friends => {
            let Some(play) = &owner.play else { return };
            match extra {
                Extra::LensHelp => {
                    play.link.lens_toggle();
                }
                Extra::ImbuementTracker => play.link.open_imbue_tracker(),
                _ => play.link.account_friends_open(),
            }
        }
    }
}

/// Shows the enabled grid buttons in the player's order.
pub(in crate::bevy_shell) fn project(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    mut widgets: ResMut<SidebarWidgets>,
    grids: Query<(Entity, &Children), With<ControlGrid>>,
    mut buttons: Query<(&ControlButton, &mut Node)>,
) {
    let owner = &mut *owner;
    if let Some(play) = &owner.play {
        let toggles = owner.settings.take_skills_toggles(&play.link);
        for _ in 0..toggles {
            toggle_skills(owner, &mut widgets);
        }
    }
    let enabled = enabled(owner);
    for (grid, children) in &grids {
        let mut order = Vec::new();
        for child in children.iter() {
            let Ok((button, mut node)) = buttons.get_mut(child) else {
                continue;
            };
            let id = match button {
                ControlButton::Grid(control) => control.id(),
                ControlButton::Extra(extra) => extra.id(),
                _ => continue,
            };
            let rank = enabled.iter().position(|enabled| enabled == id);
            let display = if rank.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            order.push((rank.unwrap_or(usize::MAX), child));
        }
        let current: Vec<Entity> = order.iter().map(|(_, entity)| *entity).collect();
        order.sort_by_key(|(rank, _)| *rank);
        if order.iter().map(|(_, entity)| *entity).ne(current) {
            let sorted: Vec<Entity> = order.into_iter().map(|(_, entity)| entity).collect();
            commands.entity(grid).insert_children(0, &sorted);
        }
    }
}
