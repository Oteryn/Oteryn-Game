//! Source-derived primitives only; actions edit the production owner's transient draft.
use super::owner::{GraphicsDraft, HostOwner};
use bevy::{
    input_focus::{FocusCause, InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Activate, ValueChange},
};
use oteryn_client_ui_bevy as source;
use std::path::Path;

const SKIN: &str = "images/skin/classic/";
const FILES: [&str; 7] = [
    "images/skin/classic/button-grey-up.png",
    "images/skin/classic/button-grey-down.png",
    "images/skin/classic/checkbox-checked.png",
    "images/skin/classic/checkbox-unchecked.png",
    "images/skin/classic/dialog-frame-borderimage.png",
    "images/1pixel-down-frame.png",
    "fonts/verdanab.ttf",
];
pub(super) fn validate_assets(root: &Path) -> Result<(), String> {
    for name in FILES {
        let path = root
            .join(name)
            .canonicalize()
            .map_err(|_| format!("Missing external UI asset: {name}"))?;
        if !path.starts_with(root) || !path.is_file() {
            return Err(format!("UI asset leaves --ui-assets directory: {name}"));
        }
    }
    Ok(())
}
#[derive(Resource)]
pub(super) struct Skins {
    pub(super) buttons: source::ButtonImages,
    pub(super) checked: Handle<Image>,
    pub(super) unchecked: Handle<Image>,
    pub(super) frame: Handle<Image>,
    pub(super) inset: Handle<Image>,
    pub(super) font: Handle<Font>,
}
#[derive(Component, Clone, Copy, Debug)]
pub(super) enum Control {
    CastChat,
    Cast,
    ExivaSettings,
    HouseMoveOut,
    GuildhallLeadership,
    CyclopediaTrackers,
    CyclopediaMap,
    BossDifficulty,
    BossRewardChest,
    AnimusMastery,
    BrowseField,
    ContainerSorting,
    Options,
    HirelingServices,
    FurnitureRotation,
    HouseTransfer,
    HouseBrowsing,
    GuildWars,
    Market,
    Party,
    PlayerReport,
    Quests,
    Spells,
    StatusDetails,
    StaffTools,
    WorldEvent,
    TeamFinder,
    Training,
    Store,
    Login,
    CancelLogin,
    Fullscreen,
    Vsync,
    IntegerScaling,
    Apply,
    Ok,
    Cancel,
}
impl Control {
    pub fn id(self) -> &'static str {
        match self {
            Self::CastChat => "shell.cast_chat",
            Self::Cast => "shell.cast_control",
            Self::ExivaSettings => "shell.exiva_settings",
            Self::HouseMoveOut => "shell.house_move_out",
            Self::GuildhallLeadership => "shell.guildhall_leadership",
            Self::CyclopediaTrackers => "shell.cyclopedia_trackers",
            Self::CyclopediaMap => "shell.cyclopedia_map",
            Self::BossDifficulty => "shell.boss_difficulty",
            Self::BossRewardChest => "shell.boss_reward_chest",
            Self::AnimusMastery => "shell.animus_mastery",
            Self::BrowseField => "shell.browse_field",
            Self::ContainerSorting => "shell.container_sorting",
            Self::Options => "shell.options",
            Self::HirelingServices => "shell.hireling_services",
            Self::FurnitureRotation => "shell.furniture_rotation",
            Self::HouseTransfer => "shell.house_transfer",
            Self::HouseBrowsing => "shell.house_browsing",
            Self::GuildWars => "shell.guild_wars",
            Self::Market => "shell.market",
            Self::Party => "shell.party",
            Self::PlayerReport => "shell.player_report",
            Self::Quests => "shell.quests",
            Self::Spells => "shell.spells",
            Self::StatusDetails => "shell.status_details",
            Self::StaffTools => "shell.staff_tools",
            Self::WorldEvent => "shell.world_event",
            Self::TeamFinder => "shell.team_finder",
            Self::Training => "shell.training",
            Self::Store => "shell.store",
            Self::Login => "shell.sign_in",
            Self::CancelLogin => "shell.cancel_sign_in",
            Self::Fullscreen => "graphics.fullscreen",
            Self::Vsync => "graphics.vsync",
            Self::IntegerScaling => "graphics.integer_scaling",
            Self::Apply => "graphics.apply",
            Self::Ok => "graphics.ok",
            Self::Cancel => "graphics.cancel",
        }
    }
    fn checked(self, draft: GraphicsDraft) -> bool {
        match self {
            Self::Fullscreen => draft.fullscreen,
            Self::Vsync => draft.vsync,
            Self::IntegerScaling => draft.integer_scaling,
            _ => false,
        }
    }
}
#[derive(Component)]
pub(super) struct ShellToolbar;
#[derive(Component)]
pub(super) struct Dialog;
#[derive(Component)]
pub(super) struct Status;
#[derive(Component)]
pub(super) struct Feedback;
#[derive(Component)]
pub(super) struct ScopeLabel;
#[derive(Component)]
pub(super) struct Indicator(Entity);

pub(super) fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: bevy::camera::ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
    ));
    let skins = Skins {
        buttons: source::ButtonImages {
            up: assets.load(format!("{SKIN}button-grey-up.png")),
            down: assets.load(format!("{SKIN}button-grey-down.png")),
        },
        checked: assets.load(format!("{SKIN}checkbox-checked.png")),
        unchecked: assets.load(format!("{SKIN}checkbox-unchecked.png")),
        frame: assets.load(format!("{SKIN}dialog-frame-borderimage.png")),
        inset: assets.load("images/1pixel-down-frame.png"),
        font: assets.load("fonts/verdanab.ttf"),
    };
    let shell = commands
        .spawn((
            ShellToolbar,
            TabGroup::new(0),
            Node {
                position_type: PositionType::Absolute,
                top: px(8.),
                left: px(8.),
                right: px(8.),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .id();
    let status = commands
        .spawn((
            source::text(
                "Offline — no admitted character or world.",
                source::font(skins.font.clone(), false),
            ),
            Status,
        ))
        .insert(TextLayout::default())
        .id();
    let toolbar = commands
        .spawn(Node {
            column_gap: px(6.),
            row_gap: px(3.),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    let shell_feedback = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            Feedback,
        ))
        .insert(TextLayout::default())
        .id();
    commands
        .entity(shell)
        .add_children(&[status, toolbar, shell_feedback]);
    for (control, caption, width) in [
        (Control::Cast, "Cast settings", 100.),
        (Control::HouseMoveOut, "House move-out (Ctrl+Alt+O)", 210.),
        (Control::CyclopediaMap, "Cyclopedia", 90.),
        (Control::BossDifficulty, "Boss difficulty", 120.),
        (
            Control::BossRewardChest,
            "Boss rewards (Ctrl+Shift+A)",
            210.,
        ),
        (Control::BrowseField, "Browse Field", 100.),
        (Control::Login, "Sign in", 80.),
        (Control::CancelLogin, "Cancel sign-in", 100.),
        (Control::GuildhallLeadership, "Guildhall (Ctrl+Alt+G)", 190.),
        (Control::ContainerSorting, "Container sorting", 130.),
        (Control::Options, "Options", 80.),
        (Control::CastChat, "Cast chat (Ctrl+Alt+Shift+H)", 220.),
        (Control::ExivaSettings, "Exiva settings", 110.),
        (
            Control::HirelingServices,
            "Hireling services (Ctrl+Shift+H)",
            240.,
        ),
        (Control::FurnitureRotation, "Furniture rotation", 140.),
        (Control::HouseTransfer, "House transfer", 110.),
        (Control::HouseBrowsing, "Houses", 100.),
        (Control::GuildWars, "Guild wars (Ctrl+Alt+W)", 160.),
        (Control::CyclopediaTrackers, "Cyclopedia trackers", 145.),
        (Control::AnimusMastery, "Animus Mastery (Ctrl+A)", 140.),
        (Control::Market, "Market", 80.),
        (Control::Party, "Party", 80.),
        (Control::PlayerReport, "Reports (Ctrl+Alt+R)", 170.),
        (Control::Quests, "Quest Log", 80.),
        (Control::Spells, "Spellbook", 80.),
        (Control::StatusDetails, "Status details", 110.),
        (Control::StaffTools, "Staff tools (Ctrl+Alt+0)", 180.),
        (Control::WorldEvent, "World events", 100.),
        (Control::TeamFinder, "Team Finder (Ctrl+Alt+T)", 180.),
        (Control::Training, "Training", 80.),
        (Control::Store, "Store", 80.),
    ] {
        let button = action_button(&mut commands, &skins, control, caption, width);
        commands.entity(toolbar).add_child(button);
    }
    // Full-window modal scrim blocks world pointer input while settings are open.
    let modal = commands
        .spawn((
            Dialog,
            super::game_ui::GameModal,
            GlobalZIndex(100),
            Node {
                width: percent(100.),
                height: percent(100.),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    let panel = commands
        .spawn((
            super::window_drag::DraggableDialog,
            TabGroup {
                order: 1,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(740.),
                height: px(550.),
                max_width: percent(98.),
                max_height: percent(98.),
                padding: UiRect {
                    left: px(14.),
                    right: px(14.),
                    top: px(27.),
                    bottom: px(14.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(modal).add_child(panel);
    source::dialog_caption(&mut commands, panel, "Options", skins.font.clone());
    let content = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            Node {
                column_gap: px(10.),
                flex_grow: 1.,
                flex_basis: px(0.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let nav = commands
        .spawn((
            source::navigation_outer(),
            source::frame(skins.inset.clone(), [1.; 4]),
        ))
        .id();
    let graphics = commands
        .spawn((source::button(skins.buttons.clone(), 115.), Checked))
        .id();
    let caption = commands
        .spawn(source::button_caption("Graphics", skins.font.clone()))
        .id();
    commands.entity(graphics).add_child(caption);
    commands.entity(nav).add_child(graphics);
    let page = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            min_width: px(0.),
            flex_grow: 1.,
            row_gap: px(10.),
            ..default()
        })
        .id();
    let title = commands
        .spawn(source::text(
            "Graphics",
            source::font(skins.font.clone(), false),
        ))
        .id();
    let scope = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            ScopeLabel,
        ))
        .id();
    commands.entity(page).add_children(&[title, scope]);
    for (control, caption) in [
        (Control::Fullscreen, "Full Screen"),
        (
            Control::IntegerScaling,
            "Scale Using Only Integral Multiples",
        ),
        (Control::Vsync, "Vertical Synchronisation (V-Sync)"),
    ] {
        let mut node = source::checkbox_row(true);
        node.width = percent(100.);
        let row = commands
            .spawn((
                source::checkbox(node),
                source::frame(skins.inset.clone(), [1.; 4]),
                control,
            ))
            .observe(toggle)
            .id();
        let indicator = commands
            .spawn((
                source::checkbox_indicator(skins.unchecked.clone()),
                Indicator(row),
            ))
            .id();
        let label = commands
            .spawn(source::text(
                caption,
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(row).add_children(&[indicator, label]);
        commands.entity(page).add_child(row);
    }
    let unsupported = commands
        .spawn((
            source::text(
                "Renderer selection, antialiasing and other pages are unavailable in this host.",
                source::font(skins.font.clone(), false),
            ),
            InteractionDisabled,
        ))
        .insert((
            TextLayout::default(),
            TextColor(source::DISABLED_TEXT_COLOR),
        ))
        .id();
    commands.entity(page).add_child(unsupported);
    let feedback = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            Feedback,
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(page).add_child(feedback);
    commands.entity(content).add_children(&[nav, page]);
    let footer = commands
        .spawn(Node {
            column_gap: px(8.),
            justify_content: JustifyContent::End,
            ..default()
        })
        .id();
    for (control, caption) in [
        (Control::Ok, "Ok"),
        (Control::Apply, "Apply"),
        (Control::Cancel, "Cancel"),
    ] {
        let button = action_button(&mut commands, &skins, control, caption, 60.);
        commands.entity(footer).add_child(button);
    }
    commands.entity(panel).add_children(&[content, footer]);
    commands.insert_resource(skins);
}
fn action_button(
    commands: &mut Commands,
    skins: &Skins,
    control: Control,
    caption: &str,
    width: f32,
) -> Entity {
    let button = commands
        .spawn((source::button(skins.buttons.clone(), width), control))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    button
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::game_ui::modal::ModalGuard,
) {
    if !modal.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if owner.modal_active()
            && matches!(
                control,
                Control::CastChat | Control::Cast | Control::ExivaSettings | Control::HirelingServices | Control::FurnitureRotation | Control::HouseMoveOut | Control::HouseTransfer | Control::HouseBrowsing | Control::GuildhallLeadership | Control::GuildWars | Control::CyclopediaTrackers | Control::CyclopediaMap | Control::BossDifficulty | Control::BossRewardChest | Control::AnimusMastery | Control::BrowseField | Control::ContainerSorting | Control::Options
                    | Control::Market
                    | Control::Party
                    | Control::PlayerReport
                    | Control::Quests
                    | Control::Spells
                    | Control::StatusDetails
                    | Control::StaffTools
                    | Control::WorldEvent
                    | Control::Training
                    | Control::Store
                    | Control::Login
                    | Control::CancelLogin
            )
        {
            return;
        }
        match control {
            Control::CastChat => super::game_ui::cast_chat::toggle(&mut owner),
            Control::Cast => super::game_ui::cast_control::open(&owner),
            Control::ExivaSettings => super::game_ui::open_exiva_settings(&owner),
            Control::HouseMoveOut => super::game_ui::house_move_out::open(&mut owner),
            Control::GuildhallLeadership => {
                super::game_ui::guildhall_leadership::toggle(&mut owner)
            }
            Control::CyclopediaTrackers => super::game_ui::cyclopedia_trackers::open(&owner),
            Control::CyclopediaMap => super::game_ui::cyclopedia_map::open(&mut owner),
            Control::BossDifficulty => super::game_ui::boss_difficulty::open(&owner),
            Control::BossRewardChest => super::game_ui::open_boss_reward_chest(&mut owner),
            Control::AnimusMastery => super::game_ui::animus_mastery::open(&owner),
            Control::BrowseField => super::game_ui::browse_field::open(&owner),
            Control::ContainerSorting => super::game_ui::open_container_sorting(&mut owner),
            Control::Options => owner.open_graphics(),
            Control::HirelingServices => super::game_ui::hireling_services::toggle(&owner),
            Control::FurnitureRotation => super::game_ui::toggle_furniture_rotation(&mut owner),
            Control::HouseTransfer => super::game_ui::house_transfer::open(&owner),
            Control::HouseBrowsing => super::game_ui::house_browsing::open(&mut owner),
            Control::GuildWars => super::game_ui::open_guild_wars(&owner),
            Control::Market => super::game_ui::open_market(&mut owner),
            Control::Party => super::game_ui::open_party(&mut owner),
            Control::PlayerReport => super::game_ui::player_report_report::open(&mut owner),
            Control::Quests => super::game_ui::open_quests(&mut owner),
            Control::Spells => super::game_ui::open_spells(&mut owner),
            Control::StatusDetails => super::game_ui::status_details::toggle(&owner),
            Control::StaffTools => super::game_ui::staff_tools::open(&mut owner),
            Control::WorldEvent => super::game_ui::world_event_window::open(&owner),
            Control::TeamFinder => super::game_ui::open_team_finder(&mut owner),
            Control::Training => super::game_ui::training::open(&owner),
            Control::Store => super::game_ui::store::open(&mut owner),
            Control::Login => owner.request_login(),
            Control::CancelLogin => owner.cancel_login(),
            Control::Apply => owner.apply(false),
            Control::Ok => owner.apply(true),
            Control::Cancel => owner.cancel(),
            _ => {}
        }
    }
}
fn toggle(
    event: On<ValueChange<bool>>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::game_ui::modal::ModalGuard,
) {
    if !modal.allows(event.source) {
        return;
    }
    if let Ok(control) = controls.get(event.source) {
        owner.edit(|draft| match control {
            Control::Fullscreen => draft.fullscreen = event.value,
            Control::Vsync => draft.vsync = event.value,
            Control::IntegerScaling => draft.integer_scaling = event.value,
            _ => {}
        });
    }
}
// Bevy injects the independent ECS queries/resources in this system signature.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut dialogs: Query<&mut Node, With<Dialog>>,
    mut text: Query<(&mut Text, Has<Status>, Has<Feedback>, Has<ScopeLabel>)>,
    controls: Query<(Entity, &Control, Has<Checked>)>,
    mut indicators: Query<(&Indicator, &mut ImageNode)>,
    mut focus: ResMut<InputFocus>,
    mut shown: Local<bool>,
    full_options: Option<Res<super::game_ui::options::Installed>>,
) {
    let open = owner.draft().is_some() && full_options.is_none();
    if open != *shown {
        if open {
            if let Some((entity, _, _)) = controls
                .iter()
                .find(|(_, control, _)| matches!(control, Control::Fullscreen))
            {
                focus.set(entity, FocusCause::Navigated);
            }
        } else {
            focus.clear();
        }
        *shown = open;
    }
    for mut node in &mut dialogs {
        node.display = if open { Display::Flex } else { Display::None };
    }
    for (mut text, status, feedback, scope) in &mut text {
        let next = if status {
            Some(owner.status.as_str())
        } else if feedback {
            Some(owner.settings.feedback.as_str())
        } else if scope {
            Some(owner.settings.scope_label())
        } else {
            None
        };
        if let Some(next) = next
            && text.0 != next
        {
            text.0 = next.into();
        }
    }
    if let Some(draft) = owner.draft() {
        for (entity, control, checked) in &controls {
            if matches!(
                control,
                Control::Fullscreen | Control::Vsync | Control::IntegerScaling
            ) {
                if control.checked(draft) && !checked {
                    commands.entity(entity).insert(Checked);
                }
                if !control.checked(draft) && checked {
                    commands.entity(entity).remove::<Checked>();
                }
            }
        }
        for (indicator, mut image) in &mut indicators {
            if let Ok((_, control, _)) = controls.get(indicator.0) {
                let next = if control.checked(draft) {
                    &skins.checked
                } else {
                    &skins.unchecked
                };
                if image.image != *next {
                    image.image = next.clone();
                }
            }
        }
    }
}

/// Escape discards the transient draft on every desktop platform.
pub(super) fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut owner: NonSendMut<HostOwner>,
    full_options: Option<Res<super::game_ui::options::Installed>>,
    modal: super::game_ui::modal::ModalGuard,
    dialogs: Query<Entity, With<Dialog>>,
) {
    if full_options.is_some() || !dialogs.iter().any(|entity| modal.allows(entity)) {
        keys.clear();
        return;
    }
    if owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
    {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if owner.draft().is_some()
            && key.key_code == KeyCode::Escape
            && key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
        {
            owner.cancel();
        }
    }
}

/// Keep admission controls outside the in-game HUD once play is installed.
// One disjoint query changes the toolbar, status and admission button nodes together.
#[allow(clippy::type_complexity)]
pub(super) fn project_toolbar(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Query<Entity, With<super::game_ui::ToolbarRegion>>,
    mut nodes: Query<
        (
            Entity,
            &mut Node,
            Has<ShellToolbar>,
            Has<Status>,
            Option<&Control>,
            Option<&ChildOf>,
        ),
        Or<(With<ShellToolbar>, With<Status>, With<Control>)>,
    >,
) {
    use Control::{BossRewardChest, Market, Party, Quests, Spells};
    let playing = owner.play.is_some();
    let staff = super::game_ui::staff_tools::toolbar_visible(&owner);
    for (entity, mut node, toolbar, status, control, parent) in &mut nodes {
        if toolbar {
            if playing {
                if let Ok(region) = regions.single() {
                    if parent.is_none_or(|parent| parent.parent() != region) {
                        commands.entity(entity).insert(ChildOf(region));
                    }
                    node.position_type = PositionType::Relative;
                    node.width = percent(100.);
                    node.max_width = percent(100.);
                    node.overflow = Overflow::clip();
                    node.top = Val::Auto;
                    node.bottom = Val::Auto;
                    node.left = Val::Auto;
                    node.right = Val::Auto;
                }
            } else {
                if parent.is_some() {
                    commands.entity(entity).remove::<ChildOf>();
                }
                node.position_type = PositionType::Absolute;
                node.width = Val::Auto;
                node.max_width = Val::Auto;
                node.top = px(8.);
                node.bottom = Val::Auto;
                node.left = px(8.);
                node.right = px(8.);
            }
        } else if matches!(
            control,
            Some(Control::StatusDetails | Control::CastChat | Control::Cast | Control::StaffTools | Control::PlayerReport | Control::WorldEvent | Control::ExivaSettings | Control::TeamFinder | Control::Training | Control::HirelingServices | Control::FurnitureRotation | Control::HouseMoveOut | Control::HouseTransfer | Control::HouseBrowsing | Control::GuildhallLeadership | Control::GuildWars | Control::CyclopediaTrackers | Control::CyclopediaMap | Control::BossDifficulty | Control::BossRewardChest | Control::AnimusMastery | Control::BrowseField | Control::ContainerSorting | Control::Market | Control::Party | Control::Quests | Control::Spells)
        ) {
            node.display = if playing && (staff || !matches!(control, Some(Control::StaffTools))) {
                Display::Flex
            } else {
                Display::None
            };
        } else if status || matches!(control, Some(Control::Login | Control::CancelLogin)) {
            node.display = if playing {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}
