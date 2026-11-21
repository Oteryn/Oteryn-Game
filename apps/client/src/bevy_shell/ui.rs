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
const FILES: [&str; 6] = [
    "images/skin/classic/button-grey-up.png",
    "images/skin/classic/button-grey-down.png",
    "images/skin/classic/checkbox-checked.png",
    "images/skin/classic/checkbox-unchecked.png",
    "images/skin/classic/dialog-frame-borderimage.png",
    "images/1pixel-down-frame.png",
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
/// Optional Tibia Global art. These files are not required by `validate_assets`: until a slot's
/// file loads, the slot keeps its fallback (a caption child or the parent's plain backdrop).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(super) enum ArtSlot {
    /// Entry-screen background behind the login window.
    LoginBackground,
    /// Market category icon, keyed by the server's category name.
    MarketCategory(String),
}
impl ArtSlot {
    pub fn path(&self) -> String {
        match self {
            Self::LoginBackground => "images/login/background.png".into(),
            Self::MarketCategory(name) => {
                let key: String = name
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() {
                            c.to_ascii_lowercase()
                        } else {
                            '-'
                        }
                    })
                    .collect();
                format!("images/market/category-{key}.png")
            }
        }
    }
}
/// The caption a slot shows while its art file is absent.
#[derive(Component)]
pub(super) struct ArtFallback;
/// Loads each new slot's file; once the image is available the slot shows and its sibling
/// fallback caption hides.
// One node query covers the slots and their fallbacks; Bevy injects the queries.
#[allow(clippy::type_complexity)]
pub(super) fn project_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    added: Query<(Entity, &ArtSlot), Without<ImageNode>>,
    slots: Query<(Entity, &ImageNode, Option<&ChildOf>), With<ArtSlot>>,
    children: Query<&Children>,
    mut nodes: Query<(&mut Node, Has<ArtFallback>), Or<(With<ArtSlot>, With<ArtFallback>)>>,
) {
    for (entity, slot) in &added {
        commands
            .entity(entity)
            .insert(ImageNode::new(assets.load(slot.path())));
    }
    for (entity, image, parent) in &slots {
        if !assets.is_loaded(&image.image) {
            continue;
        }
        let siblings = parent
            .and_then(|parent| children.get(parent.parent()).ok())
            .into_iter()
            .flat_map(|children| children.iter());
        for target in std::iter::once(entity).chain(siblings) {
            if let Ok((mut node, fallback)) = nodes.get_mut(target) {
                let display = if fallback {
                    Display::None
                } else {
                    Display::Flex
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
    }
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
    ChangeChannel,
    Options,
    ManageButtons,
    Shortcut(&'static str),
    Panel(&'static str),
    Analysers,
    CyclopediaMap,
    Depot,
    Inspection,
    Loot,
    Notices,
    Skills,
    Social,
    Market,
    Party,
    Quests,
    Spells,
    StoreInbox,
    UnjustifiedPoints,
    Login,
    CancelLogin,
    Fullscreen,
    Vsync,
    IntegerScaling,
    Apply,
    Ok,
    Cancel,
}
pub(super) mod toolbar;
pub(in crate::bevy_shell) mod window_layout;
use toolbar::{TOOLBAR, open_toolbar};
pub(super) use toolbar::{project_manager, project_toolbar};
impl Control {
    pub fn id(self) -> &'static str {
        match self {
            Self::Panel(id) | Self::Shortcut(id) => id,
            Self::ManageButtons => "shell.manage_buttons",
            Self::Depot => "shell.depot",
            Self::Analysers => "shell.analysers",
            Self::ChangeChannel => "shell.change_channel",
            Self::CyclopediaMap => "shell.cyclopedia_map",
            Self::Inspection => "shell.inspection",
            Self::Loot => "shell.loot",
            Self::Notices => "shell.notices",
            Self::Skills => "shell.skills",
            Self::Social => "shell.social",
            Self::Options => "shell.options",
            Self::Market => "shell.market",
            Self::Party => "shell.party",
            Self::Quests => "shell.quests",
            Self::Spells => "shell.spells",
            Self::StoreInbox => "shell.store_inbox",
            Self::UnjustifiedPoints => "shell.unjustified_points",
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
pub(super) enum Feedback {
    Settings,
    Admission,
}
#[derive(Component)]
pub(super) struct ScopeLabel;
#[derive(Component)]
pub(super) struct Indicator(Entity);

#[derive(Resource)]
pub(crate) struct BuiltInFont;

pub(super) const EXTERNAL_FONT: &str = "fonts/verdanab.ttf";

pub(crate) fn install_font_fallback(app: &mut App, assets: &std::path::Path) {
    if !assets.join(EXTERNAL_FONT).is_file() {
        app.insert_resource(BuiltInFont);
    }
}

pub(crate) fn ui_font(assets: &AssetServer, fallback: bool) -> Handle<Font> {
    if fallback {
        Handle::default()
    } else {
        assets.load(EXTERNAL_FONT)
    }
}

pub(super) fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fallback: Option<Res<BuiltInFont>>,
) {
    commands.init_resource::<toolbar::Manager>();
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
        font: ui_font(&assets, fallback.is_some()),
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
            Feedback::Admission,
        ))
        .insert(TextLayout::default())
        .id();
    commands
        .entity(shell)
        .add_children(&[toolbar, shell_feedback]);
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
    let manage = action_button(
        &mut commands,
        &skins,
        Control::ManageButtons,
        "Manage Buttons",
        120.,
    );
    commands.entity(toolbar).add_child(manage);
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
            Name::new("options.window"),
            // Tibia Global options: 685 wide; a 129px category column beside a 470px page.
            crate::bevy_shell::ui::window_layout::window(
                "options",
                Node {
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
            ),
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
            Feedback::Settings,
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
mod controls;
#[cfg(test)]
use controls::activate;
use controls::{action_button, toggle};
// Bevy injects the independent ECS queries/resources in this system signature.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut dialogs: Query<&mut Node, With<Dialog>>,
    mut text: Query<(&mut Text, Option<&Feedback>, Has<ScopeLabel>)>,
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
    for (mut text, feedback, scope) in &mut text {
        let next = match feedback {
            Some(Feedback::Admission)
                if owner.phase == super::owner::Phase::Entering && !owner.login.visible =>
            {
                owner.status.as_str()
            }
            Some(Feedback::Admission) => "",
            Some(Feedback::Settings) => owner.settings.feedback.as_str(),
            None if scope => owner.settings.scope_label(),
            None => continue,
        };
        if text.0 != next {
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

#[cfg(test)]
mod toolbar_tests;

#[cfg(test)]
pub(super) fn assert_editor_acquires_focus(world: &mut World, field: Entity) {
    use bevy::input_focus::{AcquireFocus, InputFocus, tab_navigation::acquire_focus_tab_index};
    world.add_observer(acquire_focus_tab_index);
    world.trigger(AcquireFocus {
        focused_entity: field,
        window: Entity::PLACEHOLDER,
    });
    assert_eq!(world.resource::<InputFocus>().get(), Some(field));
}
