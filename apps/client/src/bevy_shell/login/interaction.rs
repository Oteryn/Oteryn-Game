//! Login dialog input: field reads, button activation and keyboard.
use super::*;

pub(super) fn read_fields(
    owner: &mut HostOwner,
    fields: &Query<(Entity, &LoginField, &EditableText)>,
) {
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

pub(super) fn activate(
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

pub(in crate::bevy_shell) fn keyboard(
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
