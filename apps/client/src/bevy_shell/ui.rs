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
    Options,
    Panel(&'static str),
    Analysers,
    Inspection,
    Loot,
    Notices,
    Skills,
    Market,
    Party,
    Quests,
    Spells,
    Login,
    CancelLogin,
    Fullscreen,
    Vsync,
    IntegerScaling,
    Apply,
    Ok,
    Cancel,
}
// Label text doubles as the button caption; entries are ordered by label.
const TOOLBAR: &[(&str, Control)] = &[
    ("Analysers", Control::Analysers),
    ("Battle", Control::Panel("battle")),
    ("Chat", Control::Panel("chat")),
    ("Inspection", Control::Inspection),
    ("Inventory", Control::Panel("inventory")),
    ("Loot Containers", Control::Loot),
    ("Market", Control::Market),
    ("Minimap", Control::Panel("minimap")),
    ("Notices", Control::Notices),
    ("Options", Control::Options),
    ("Party", Control::Party),
    ("Quest Log", Control::Quests),
    ("Skills", Control::Skills),
    ("Spellbook", Control::Spells),
    ("VIP", Control::Panel("vip")),
];
fn open_toolbar(owner: &mut HostOwner, control: Control) -> bool {
    if owner.modal_active() {
        return false;
    }
    match control {
        Control::Options => owner.open_graphics(),
        Control::Market => super::game_ui::open_market(owner),
        Control::Party => super::game_ui::open_party(owner),
        Control::Quests => super::game_ui::open_quests(owner),
        Control::Spells => super::game_ui::open_spells(owner),
        Control::Panel(id) => {
            if let Some(play) = &owner.play {
                let mut preferences = owner.settings.preferences.clone();
                if !preferences.panel_visible(id) {
                    preferences.visible_panels.push(id.into());
                    owner.settings.apply(preferences);
                }
                if id == "vip" {
                    play.link
                        .vip_request(oteryn_client_ui::vip::Intent::Query(0));
                }
            }
        }
        Control::Analysers => {
            if let Some(play) = &owner.play {
                play.link.analyser_open();
            }
        }
        Control::Inspection => {
            if let Some(play) = &owner.play {
                play.link.inspection_set_open(true);
            }
        }
        Control::Loot => {
            if let Some(play) = &owner.play {
                play.link.discovery_open(98);
            }
        }
        Control::Notices => {
            if let Some(play) = &owner.play {
                play.link.notices_set_open(true);
            }
        }
        Control::Skills => {
            if let Some(play) = &owner.play {
                play.link.skills_widget_request();
            }
        }
        _ => return false,
    }
    true
}
impl Control {
    pub fn id(self) -> &'static str {
        match self {
            Self::Panel(id) => id,
            Self::Analysers => "shell.analysers",
            Self::Inspection => "shell.inspection",
            Self::Loot => "shell.loot",
            Self::Notices => "shell.notices",
            Self::Skills => "shell.skills",
            Self::Options => "shell.options",
            Self::Market => "shell.market",
            Self::Party => "shell.party",
            Self::Quests => "shell.quests",
            Self::Spells => "shell.spells",
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
        (Control::Login, "Sign in", 80.),
        (Control::CancelLogin, "Cancel sign-in", 100.),
    ] {
        let button = action_button(&mut commands, &skins, control, caption, width);
        commands.entity(toolbar).add_child(button);
    }
    for &(caption, control) in TOOLBAR {
        let button = action_button(&mut commands, &skins, control, caption, 100.);
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
                Control::Options
                    | Control::Market
                    | Control::Party
                    | Control::Quests
                    | Control::Spells
                    | Control::Login
                    | Control::CancelLogin
            )
        {
            return;
        }
        match control {
            control if TOOLBAR.iter().any(|entry| entry.1.id() == control.id()) => {
                let _ = open_toolbar(&mut owner, *control);
            }
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
    let playing = owner.play.is_some();
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
            Some(control) if !matches!(control, Control::Options) && TOOLBAR.iter().any(|entry| entry.1.id() == control.id())
        ) {
            node.display = if playing {
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

#[cfg(test)]
mod toolbar_tests {
    use super::*;
    #[test]
    fn toolbar_registry_sorted() {
        assert!(TOOLBAR.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
    #[test]
    fn toolbar_opens_window() -> Result<(), Box<dyn std::error::Error>> {
        for label in [
            "Analysers",
            "Battle",
            "Chat",
            "Inspection",
            "Inventory",
            "Loot Containers",
            "Market",
            "Minimap",
            "Notices",
            "Options",
            "Party",
            "Quest Log",
            "Skills",
            "Spellbook",
            "VIP",
        ] {
            let mut owner = HostOwner::new(None)?;
            owner.settings.preferences.visible_panels.clear();
            let (link, mut commands, outbox) = crate::play::play_channel();
            outbox.bevy_hud_skills_fixture()?;
            owner.play = Some(crate::bevy_shell::owner::ActivePlay {
                view: crate::play::PlayView::new(
                    oteryn_renderer::TileCoord::new(100, 100),
                    7,
                    &[],
                )?,
                link,
            });
            let entry = TOOLBAR
                .iter()
                .find(|entry| entry.0 == label)
                .ok_or("missing toolbar entry")?;
            let mut app = App::new();
            app.insert_non_send(owner);
            let button = app
                .world_mut()
                .spawn((entry.1, Node::default()))
                .observe(activate)
                .id();
            app.world_mut().trigger(Activate { entity: button });
            let owner = app.world().non_send::<HostOwner>();
            match entry.1 {
                Control::Panel(id) => assert!(owner.settings.preferences.panel_visible(id)),
                Control::Options => assert!(owner.draft().is_some()),
                Control::Analysers => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.analyser_panel().open)
                ),
                Control::Skills => assert!(commands.try_recv().is_ok()),
                Control::Inspection => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.inspection_panel().open)
                ),
                Control::Loot => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.discovery_panel().loot.open)
                ),
                Control::Notices => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.notices_panel().open)
                ),
                Control::Market => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.market_panel().open)
                ),
                Control::Party => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.party_is_open())
                ),
                Control::Quests => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.quest_window_open())
                ),
                Control::Spells => assert!(
                    owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.spellbook_window_open())
                ),
                _ => return Err("unexpected toolbar action".into()),
            }
        }
        Ok(())
    }
}
