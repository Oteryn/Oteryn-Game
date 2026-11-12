//! Shared connection form. Only the existing native-login owner validates and saves settings.
use super::{owner::HostOwner, ui::Skins};
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

pub(super) struct LoginState {
    pub form: LoginForm,
    pub visible: bool,
    pub revision: u64,
    path: Option<PathBuf>,
}
impl Default for LoginState {
    fn default() -> Self {
        Self {
            form: LoginForm::new(LoginSettings::default()),
            visible: false,
            revision: 0,
            path: None,
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
#[derive(Component)]
pub(super) struct LoginMessage;
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
}
impl LoginControl {
    pub fn id(self) -> &'static str {
        match self {
            Self::Submit => "login.submit",
            Self::Cancel => "login.cancel",
        }
    }
}

pub(super) fn setup(mut commands: Commands, skins: Res<Skins>) {
    let modal = commands
        .spawn((
            LoginDialog,
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
    let panel = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            TabGroup {
                order: 2,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(685.),
                max_width: percent(100.),
                max_height: percent(100.),
                padding: UiRect {
                    left: px(16.),
                    right: px(16.),
                    top: px(29.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(modal).add_child(panel);
    source::dialog_caption(
        &mut commands,
        panel,
        "Connection Settings",
        skins.font.clone(),
    );
    for (index, (label, _)) in crate::native_login::FIELDS.iter().enumerate() {
        let label = commands
            .spawn(source::text(
                *label,
                source::font(skins.font.clone(), false),
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
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    height: px(25.),
                    min_height: px(25.),
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_children(&[label, input]);
    }
    let message = commands
        .spawn((
            LoginMessage,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    let footer = commands
        .spawn(Node {
            justify_content: JustifyContent::End,
            column_gap: px(5.),
            ..default()
        })
        .id();
    for (control, caption, width) in [
        (LoginControl::Submit, "Save and sign in", 130.),
        (LoginControl::Cancel, "Cancel", 60.),
    ] {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), width), control))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    commands.entity(panel).add_children(&[message, footer]);
}
fn read_fields(owner: &mut HostOwner, fields: &Query<(&LoginField, &EditableText)>) {
    if owner.login.form.can_submit() {
        let mut values = owner.login.form.settings.values().map(str::to_owned);
        for (field, input) in fields {
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
    fields: Query<(&LoginField, &EditableText)>,
    mut owner: NonSendMut<HostOwner>,
) {
    if !owner.login.visible || owner.selection.pending().is_some() {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        match control {
            LoginControl::Submit => {
                read_fields(&mut owner, &fields);
                owner.submit_login();
            }
            LoginControl::Cancel => {
                owner.cancel_login();
                owner.login.visible = false;
            }
        }
    }
}
pub(super) fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    fields: Query<(&LoginField, &EditableText)>,
    focus: Res<InputFocus>,
    mut owner: NonSendMut<HostOwner>,
) {
    if !owner.login.visible || owner.selection.pending().is_some() {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed || key.repeat {
            continue;
        }
        match key.key_code {
            KeyCode::Escape => {
                owner.cancel_login();
                owner.login.visible = false;
            }
            KeyCode::Enter | KeyCode::NumpadEnter
                if focus.get().is_some_and(|entity| fields.get(entity).is_ok()) =>
            {
                read_fields(&mut owner, &fields);
                owner.submit_login();
            }
            _ => {}
        }
    }
}

// These ECS queries are independent widget projections and focus state.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    mut dialogs: Query<&mut Node, With<LoginDialog>>,
    mut fields: Query<(Entity, &LoginField, &mut EditableText)>,
    mut messages: Query<&mut Text, With<LoginMessage>>,
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
    for mut node in &mut dialogs {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if visible && *revision != Some(owner.login.revision) {
        for (entity, field, mut text) in &mut fields {
            if let Some(value) = owner.login.form.settings.values().get(field.0) {
                *text = EditableText::new(value);
                text.max_characters = Some(4096);
            }
            if field.0 == 0 {
                focus.set(entity, FocusCause::Navigated);
            }
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
    for mut message in &mut messages {
        if message.0 != owner.login.form.message() {
            message.0 = owner.login.form.message().to_owned();
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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> std::io::Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "bevy-login-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path)?;
            Ok(Self(path))
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn settings(temp: &Temp) -> Result<LoginSettings, Box<dyn std::error::Error>> {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let path = temp.0.join("root.der");
        std::fs::write(&path, certificate.cert.der())?;
        Ok(LoginSettings {
            platform_url: "http://127.0.0.1:18564".into(),
            gateway_url: "http://127.0.0.1:18565".into(),
            oauth_client_id: "native-test".into(),
            dev_root: path.to_string_lossy().into_owned(),
            ..Default::default()
        })
    }
    #[test]
    fn shared_login_request_exposes_form_on_every_native_platform() -> TestResult {
        let mut host = HostOwner::new(None)?;
        host.request_login();
        assert!(host.login.visible);
        assert!(host.modal_active());
        assert!(!host.status.contains("admission is unavailable"));
        host.close();
        assert!(!host.login.visible);
        assert!(!host.modal_active());
        Ok(())
    }
    #[test]
    fn malformed_login_settings_and_invalid_submit_remain_visible_without_admission() -> TestResult
    {
        let temp = Temp::new()?;
        let path = temp.0.join("login.json");
        std::fs::write(&path, b"not-json")?;
        let mut host = HostOwner::new(None)?;
        host.configure_login(Some(path.clone()), |_| None);
        host.request_login();
        assert!(host.login.form.message().contains("invalid"));
        host.submit_login();
        assert_eq!(host.phase, super::super::owner::Phase::Offline);
        assert!(!host.login.form.pending());
        assert!(host.login.visible);
        assert_eq!(std::fs::read(path)?, b"not-json");
        Ok(())
    }
    #[test]
    fn existing_login_owner_saves_public_configuration_and_rejects_blocked_save() -> TestResult {
        let temp = Temp::new()?;
        let path = temp.0.join("login.json");
        let mut state = LoginState::default();
        state.load(Some(path.clone()), |_| None);
        state.form.settings = settings(&temp)?;
        let expected = state.form.settings.clone();
        let config = state.submit()?;
        assert_eq!(config.platform_url, expected.platform_url);
        assert!(state.form.pending());
        assert!(LoginSettings::load(&path)? == expected);
        assert!(state.submit().is_err());
        state
            .form
            .finish(Err(crate::GameplayEntryError::NativeProtocolUnavailable));
        std::fs::remove_file(&path)?;
        std::fs::create_dir(&path)?;
        assert!(state.submit().is_err());
        assert!(!state.form.pending());
        assert!(state.form.message().contains("Cannot save"));
        Ok(())
    }
    #[test]
    fn text_input_values_and_native_activate_reach_existing_validation() -> TestResult {
        use bevy::asset::AssetPlugin;
        let temp = Temp::new()?;
        let mut host = HostOwner::new(None)?;
        host.configure_login(Some(temp.0.join("login.json")), |_| None);
        host.request_login();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            oteryn_client_ui_bevy::GlobalUiPlugin,
        ))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(host)
        .add_systems(Startup, (super::super::ui::setup, setup).chain())
        .add_systems(Update, project);
        app.update();
        for (field, mut input) in app
            .world_mut()
            .query::<(&LoginField, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            if field.0 == 0 {
                *input = EditableText::new("https://example.invalid?token=refused");
            }
        }
        let submit = app
            .world_mut()
            .query::<(Entity, &LoginControl)>()
            .iter(app.world())
            .find_map(|(entity, control)| matches!(control, LoginControl::Submit).then_some(entity))
            .ok_or("submit")?;
        app.world_mut().trigger(Activate { entity: submit });
        app.update();
        let host = app.world().non_send::<HostOwner>();
        assert_eq!(host.phase, super::super::owner::Phase::Offline);
        assert!(!host.login.form.pending());
        assert!(host.login.visible);
        assert_eq!(
            host.login.form.settings.platform_url,
            "https://example.invalid?token=refused"
        );
        assert!(!temp.0.join("login.json").exists());
        Ok(())
    }
}
