//! Shared connection form. Only the existing native-login owner validates and saves settings.
use super::{
    game_ui::{GameModal, modal::ModalGuard},
    owner::HostOwner,
    ui::Skins,
};
use crate::native_login::{LoginForm, LoginSettings};
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui_bevy as source;
use std::path::PathBuf;
mod bulletin;
mod submission;

pub(super) struct LoginState {
    pub form: LoginForm,
    pub bulletin: bulletin::State,
    pub visible: bool,
    /// Shows the developer connection fields; off by default.
    pub advanced: bool,
    pub revision: u64,
    path: Option<PathBuf>,
    pending_submit: Option<submission::Pending>,
}
impl Default for LoginState {
    fn default() -> Self {
        Self {
            form: LoginForm::new(LoginSettings::default()),
            bulletin: Default::default(),
            visible: false,
            advanced: false,
            revision: 0,
            path: None,
            pending_submit: None,
        }
    }
}
impl LoginState {
    pub fn load(&mut self, path: Option<PathBuf>, lookup: impl Fn(&str) -> Option<String>) {
        let loaded = path
            .as_ref()
            .map_or_else(|| Ok(LoginSettings::default()), |p| LoginSettings::load(p));
        let error = loaded.as_ref().err().cloned();
        self.form = LoginForm::new(loaded.unwrap_or_default().with_env(lookup));
        if let Some(error) = error {
            self.form.feedback(error);
        }
        self.path = path;
        self.revision = self.revision.saturating_add(1);
    }
    pub fn show(&mut self) {
        self.visible = true;
        self.revision = self.revision.saturating_add(1);
    }
    pub fn submit(&mut self) -> Result<crate::NativeLoginConfig, String> {
        if !self.form.can_submit() {
            return Err("Sign-in is already running or connected.".into());
        }
        let saved = self
            .path
            .as_ref()
            .ok_or_else(|| "Cannot locate the login configuration directory.".to_owned())
            .and_then(|p| self.form.settings.save(p));
        if let Err(error) = saved {
            self.form.feedback(error.clone());
            return Err(error);
        }
        self.form.submit()
    }
}
#[derive(Component)]
pub(super) struct LoginDialog;
/// The developer connection fields, shown only while Advanced is on.
#[derive(Component)]
pub(super) struct AdvancedFields;
/// The modal message box that presents login errors and notices.
#[derive(Component)]
pub(super) struct LoginAlert;
#[derive(Component)]
pub(super) struct AlertTitle;
#[derive(Component)]
pub(super) struct AlertText;
#[derive(Component)]
pub(super) struct LoginField(pub usize);
impl LoginField {
    pub fn id(&self) -> &'static str {
        [
            "login.platform_url",
            "login.gateway_url",
            "login.oauth_client_id",
            "login.world_ref",
            "login.character_id",
            "login.dev_root",
        ]
        .get(self.0)
        .copied()
        .unwrap_or("login.unknown")
    }
}
#[derive(Component, Clone, Copy)]
pub(super) enum LoginControl {
    Submit,
    Cancel,
    Advanced,
    DismissAlert,
    News,
    ForgetDevice,
    About,
    Survey,
    Support,
    Cookies,
}
impl LoginControl {
    pub fn id(self) -> &'static str {
        match self {
            Self::News => "login.news",
            Self::ForgetDevice => "login.forget_device",
            Self::About => "login.about",
            Self::Survey => "login.survey",
            Self::Support => "login.support",
            Self::Cookies => "login.cookies",
            Self::Submit => "login.submit",
            Self::Cancel => "login.cancel",
            Self::Advanced => "login.advanced",
            Self::DismissAlert => "login.alert.ok",
        }
    }
}

pub(super) fn install(app: &mut App) {
    bulletin::install(app);
    app.init_resource::<super::ui::window_layout::WindowLayouts>()
        .add_systems(
            Update,
            (super::ui::project_art, super::ui::window_layout::zoom),
        )
        .add_systems(
            PostUpdate,
            (
                submission::finish.after(bevy::text::EditableTextSystems),
                super::ui::window_layout::project.before(bevy::ui::UiSystems::Layout),
            ),
        );
}

fn control_button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: LoginControl,
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
    commands.entity(parent).add_child(button);
    button
}

pub(super) fn setup(mut commands: Commands, skins: Res<Skins>) {
    bulletin::setup(&mut commands, &skins);
    let modal = commands
        .spawn((
            LoginDialog,
            GameModal,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    // Entry-screen art fills the screen behind the login window; the scrim stays without it.
    let background = commands
        .spawn((
            super::ui::ArtSlot::LoginBackground,
            Name::new("login.background"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
        ))
        .id();
    // Tibia Global's entry top bar (1028x33) carries the secondary account actions.
    let top_bar = commands
        .spawn((
            Name::new("login.top_bar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(0.),
                width: px(1028.),
                max_width: percent(100.),
                height: px(33.),
                column_gap: px(5.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    for (control, caption, width) in [
        (LoginControl::News, "News", 60.),
        (LoginControl::About, "About", 60.),
        (LoginControl::Survey, "Survey", 60.),
        (LoginControl::Support, "Support", 70.),
        (LoginControl::Cookies, "Cookie policy", 108.),
        (LoginControl::ForgetDevice, "Sign out device", 108.),
    ] {
        control_button(&mut commands, &skins, top_bar, control, caption, width);
    }
    let panel = commands
        .spawn((
            super::window_drag::DraggableDialog,
            bevy::ui_widgets::ScrollArea,
            Name::new("login.window"),
            TabGroup {
                order: 2,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            // Tibia Global login mask: 280 wide.
            Node {
                width: px(280.),
                max_width: percent(100.),
                max_height: percent(100.),
                padding: UiRect {
                    left: px(10.),
                    right: px(10.),
                    top: px(27.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands
        .entity(modal)
        .add_children(&[background, top_bar, panel]);
    source::dialog_caption(&mut commands, panel, "Login", skins.font.clone());
    let advanced = commands
        .spawn((
            AdvancedFields,
            Name::new("login.advanced_fields"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                display: Display::None,
                ..default()
            },
        ))
        .id();
    for (index, (label, _)) in crate::native_login::FIELDS.iter().enumerate() {
        let label = commands
            .spawn(source::text(
                *label,
                source::font(skins.font.clone(), false),
            ))
            .id();
        // Bevy's text/image intrinsic-size writers must not own the same node.
        // Keep the source chrome on its own parent; the child owns editing/rendering.
        let frame = commands
            .spawn((
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    height: px(20.),
                    min_height: px(20.),
                    ..default()
                },
            ))
            .id();
        let input = commands
            .spawn((
                LoginField(index),
                TextInput,
                EditableText {
                    max_characters: Some(4096),
                    ..EditableText::default()
                },
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(index as i32),
                Node {
                    width: percent(100.),
                    height: px(20.),
                    min_height: px(20.),
                    padding: UiRect::axes(px(4.), px(2.)),
                    ..default()
                },
            ))
            .id();
        commands.entity(frame).add_child(input);
        commands.entity(advanced).add_children(&[label, frame]);
    }
    let toggle = commands
        .spawn(Node {
            justify_content: JustifyContent::Start,
            ..default()
        })
        .id();
    control_button(
        &mut commands,
        &skins,
        toggle,
        LoginControl::Advanced,
        "Advanced",
        86.,
    );
    let footer = commands
        .spawn(Node {
            justify_content: JustifyContent::End,
            column_gap: px(5.),
            ..default()
        })
        .id();
    for (control, caption, width) in [
        (LoginControl::Submit, "Login", 86.),
        (LoginControl::Cancel, "Cancel", 86.),
    ] {
        control_button(&mut commands, &skins, footer, control, caption, width);
    }
    commands
        .entity(panel)
        .add_children(&[advanced, toggle, footer]);
    let alert = commands
        .spawn((
            LoginAlert,
            GameModal,
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let box_panel = commands
        .spawn((
            TabGroup {
                order: 3,
                modal: true,
            },
            Name::new("login.alert"),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(260.),
                max_width: percent(100.),
                max_height: percent(100.),
                padding: UiRect {
                    left: px(10.),
                    right: px(10.),
                    top: px(27.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(10.),
                ..default()
            },
        ))
        .id();
    commands.entity(alert).add_child(box_panel);
    let header = source::dialog_caption(&mut commands, box_panel, "", skins.font.clone());
    commands.entity(header).insert(AlertTitle);
    let text = commands
        .spawn((
            AlertText,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    let actions = commands
        .spawn(Node {
            justify_content: JustifyContent::End,
            ..default()
        })
        .id();
    control_button(
        &mut commands,
        &skins,
        actions,
        LoginControl::DismissAlert,
        "Ok",
        43.,
    );
    commands.entity(box_panel).add_children(&[text, actions]);
}
fn read_fields(owner: &mut HostOwner, fields: &Query<(Entity, &LoginField, &EditableText)>) {
    if owner.login.form.can_submit() {
        let mut values = owner.login.form.settings.values().map(str::to_owned);
        for (_, field, input) in fields {
            if let Some(value) = values.get_mut(field.0) {
                *value = input.value().to_string();
            }
        }
        owner.login.form.settings = LoginSettings::from_values(values);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&LoginControl>,
    guard: ModalGuard,
    fields: Query<(Entity, &LoginField, &EditableText)>,
    mut owner: NonSendMut<HostOwner>,
) {
    if !guard.allows(event.entity) || !owner.login.visible || owner.selection.pending().is_some() {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        match control {
            LoginControl::DismissAlert => owner.login.form.dismiss_alert(),
            LoginControl::Advanced => {
                owner.login.advanced = !owner.login.advanced;
            }
            LoginControl::News => owner.login.bulletin.visible = true,
            LoginControl::ForgetDevice => {
                owner.login.pending_submit = None;
                read_fields(&mut owner, &fields);
                owner.sign_out_device();
            }
            LoginControl::About => owner
                .login
                .form
                .inform("About", crate::about::about_text().replace("\r\n", "\n")),
            // The optional questionnaire reviews its answer and asks for consent in OS dialogs.
            #[cfg(windows)]
            LoginControl::Survey => crate::about::run_survey(),
            #[cfg(not(windows))]
            LoginControl::Survey => owner.login.form.inform(
                "Survey",
                "The optional survey is offered by the Windows client.".into(),
            ),
            LoginControl::Support | LoginControl::Cookies => {
                read_fields(&mut owner, &fields);
                owner.open_web_link(if matches!(control, LoginControl::Support) {
                    crate::web_links::WebLink::SupportTickets
                } else {
                    crate::web_links::WebLink::CookiePolicy
                });
            }
            LoginControl::Submit => {
                submission::request(&mut owner, &fields);
            }
            LoginControl::Cancel => {
                owner.login.pending_submit = None;
                owner.cancel_login();
                owner.login.visible = false;
            }
        }
    }
}
pub(super) fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    fields: Query<(Entity, &LoginField, &EditableText)>,
    focus: Res<InputFocus>,
    dialogs: Query<Entity, With<LoginDialog>>,
    alerts: Query<Entity, With<LoginAlert>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if !owner.login.visible || owner.selection.pending().is_some() {
        keys.clear();
        return;
    }
    if owner.login.form.alert().is_some() && alerts.iter().any(|entity| guard.allows(entity)) {
        for key in keys.read() {
            if key.state == bevy::input::ButtonState::Pressed
                && !key.repeat
                && matches!(
                    key.key_code,
                    KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter
                )
            {
                owner.login.form.dismiss_alert();
            }
        }
        return;
    }
    if !dialogs.iter().any(|entity| guard.allows(entity)) {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed || key.repeat {
            continue;
        }
        match key.key_code {
            KeyCode::Escape => {
                owner.login.pending_submit = None;
                owner.cancel_login();
                owner.login.visible = false;
            }
            KeyCode::Enter | KeyCode::NumpadEnter
                if focus.get().is_some_and(|entity| fields.get(entity).is_ok()) =>
            {
                submission::request(&mut owner, &fields);
            }
            _ => {}
        }
    }
}

type DialogNodes<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, Has<LoginDialog>, Has<LoginAlert>),
    Or<(With<LoginDialog>, With<LoginAlert>, With<AdvancedFields>)>,
>;

// These ECS queries are independent widget projections and focus state.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    mut dialogs: DialogNodes,
    mut fields: Query<(Entity, &LoginField, &mut EditableText)>,
    mut texts: Query<&mut Text>,
    alert_text: Query<Entity, With<AlertText>>,
    alert_title: Query<&Children, With<AlertTitle>>,
    controls: Query<(Entity, &LoginControl)>,
    mut focus: ResMut<InputFocus>,
    mut revision: Local<Option<u64>>,
    mut shown: Local<bool>,
) {
    let visible = owner.login.visible && owner.selection.pending().is_none();
    if *shown && !visible && focus.get().is_some_and(|entity| fields.get(entity).is_ok()) {
        focus.clear();
    }
    *shown = visible;
    let alert = owner.login.form.alert();
    for (mut node, dialog, alert_box) in &mut dialogs {
        let shown = if dialog {
            visible
        } else if alert_box {
            visible && alert.is_some()
        } else {
            owner.login.advanced
        };
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    if let Some((title, message)) = alert {
        let titles = alert_title.iter().flat_map(|children| children.iter());
        let entries = titles
            .map(|entity| (entity, title))
            .chain(alert_text.iter().map(|entity| (entity, message)));
        for (entity, next) in entries {
            if let Ok(mut text) = texts.get_mut(entity)
                && text.0 != next
            {
                text.0 = next.to_owned();
            }
        }
    }
    if visible && *revision != Some(owner.login.revision) {
        for (entity, field, mut text) in &mut fields {
            if let Some(value) = owner.login.form.settings.values().get(field.0) {
                // Replacing EditableText would discard the font styles already synced by Bevy.
                text.pending_edits.clear();
                text.pending_paste = None;
                text.editor_mut().set_text(value);
                text.queue_edit(bevy::text::TextEdit::TextEnd(false));
                text.max_characters = Some(4096);
            }
            if field.0 == 0 && owner.login.advanced {
                focus.set(entity, FocusCause::Navigated);
            }
        }
        if !owner.login.advanced
            && let Some((submit, _)) = controls
                .iter()
                .find(|(_, control)| matches!(control, LoginControl::Submit))
        {
            focus.set(submit, FocusCause::Navigated);
        }
        *revision = Some(owner.login.revision);
    }
    let pending = owner.login.form.pending();
    for (entity, _, _) in &fields {
        if pending {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    for (entity, control) in &controls {
        if matches!(control, LoginControl::Submit) && !owner.login.form.can_submit() {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    // Keep edits in the existing form even when an action is dispatched by keyboard next frame.
    if visible && !pending {
        let mut values = owner.login.form.settings.values().map(str::to_owned);
        for (_, field, text) in &fields {
            if let Some(value) = values.get_mut(field.0) {
                *value = text.value().to_string();
            }
        }
        owner.login.form.settings = LoginSettings::from_values(values);
    }
}

#[cfg(test)]
mod tests;
