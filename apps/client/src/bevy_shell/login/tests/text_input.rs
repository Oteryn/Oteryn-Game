use super::super::*;
use bevy::{
    asset::AssetPlugin,
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputDispatchPlugin,
    text::{RemSize, TextPlugin},
    ui_widgets::TextInputPlugin,
    window::WindowPlugin,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn app() -> Result<(App, Entity), Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.request_login();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        InputPlugin,
        InputDispatchPlugin,
        WindowPlugin::default(),
        TextPlugin,
        TextInputPlugin,
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<UiScale>()
    .init_resource::<RemSize>()
    .init_resource::<bevy::picking::events::PointerState>()
    .add_message::<bevy::picking::events::PointerRelease>()
    .insert_non_send(owner)
    .add_systems(Startup, (super::super::super::ui::setup, setup).chain())
    .add_systems(Update, (keyboard, project).chain())
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/assets/FiraMono-subset.ttf"
            ))
            .to_vec(),
        ));
    app.add_systems(
        Startup,
        (move |mut skins: ResMut<Skins>| {
            skins.font = font.clone();
        })
        .after(super::super::super::ui::setup)
        .before(setup),
    );
    install(&mut app);
    app.update();
    let editor = app
        .world_mut()
        .query::<(Entity, &LoginField)>()
        .iter(app.world())
        .find_map(|(id, field)| (field.0 == 0).then_some(id))
        .ok_or("platform editor")?;
    let mut text = app
        .world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?;
    text.editor_mut().set_text("bad");
    text.queue_edit(bevy::text::TextEdit::TextEnd(false));
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    Ok((app, editor))
}
fn key(app: &mut App, key_code: KeyCode, logical_key: Key, text: Option<&str>) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}
#[test]
fn login_enter_submits_the_last_character_from_the_native_editor() -> TestResult {
    let (mut app, _) = app()?;
    key(
        &mut app,
        KeyCode::Digit1,
        Key::Character("!".into()),
        Some("!"),
    );
    key(&mut app, KeyCode::Enter, Key::Enter, None);
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .login
            .form
            .settings
            .platform_url,
        "bad!"
    );
    assert!(!app.world().non_send::<HostOwner>().login.form.pending());
    Ok(())
}
#[test]
fn login_button_waits_for_paste_and_cancels_when_form_closes() -> TestResult {
    use std::sync::{Arc, Mutex};
    for close in [false, true] {
        let (mut app, editor) = app()?;
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?
            .queue_edit(bevy::text::TextEdit::SelectAll);
        app.update();
        let pending = Arc::new(Mutex::new(None));
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?
            .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
            &pending,
        )));
        let submit = app
            .world_mut()
            .query::<(Entity, &LoginControl)>()
            .iter(app.world())
            .find_map(|(id, control)| matches!(control, LoginControl::Submit).then_some(id))
            .ok_or("submit")?;
        let message = app
            .world()
            .non_send::<HostOwner>()
            .login
            .form
            .message()
            .to_owned();
        app.world_mut().trigger(Activate { entity: submit });
        app.update();
        assert_eq!(
            app.world().non_send::<HostOwner>().login.form.message(),
            message,
            "paste must finish before validation/submission"
        );
        if close {
            app.world_mut().non_send_mut::<HostOwner>().close();
        }
        *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("still invalid!".into()));
        app.update();
        let owner = app.world().non_send::<HostOwner>();
        if close {
            assert_eq!(owner.login.form.message(), message);
        } else {
            assert_eq!(owner.login.form.settings.platform_url, "still invalid!");
            assert_ne!(owner.login.form.message(), message);
        }
    }
    Ok(())
}
