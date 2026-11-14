use super::super::*;
use crate::play::{
    PlayView, feature_0450_server_modal::protocol::BevyServerModalFixture, play_channel,
};
use bevy::{
    asset::AssetPlugin,
    input::InputPlugin,
    input_focus::InputDispatchPlugin,
    text::{RemSize, TextPlugin},
    ui_widgets::TextInputPlugin,
    window::WindowPlugin,
};
use oteryn_client_runtime::server_modal::input::InputValue;
use oteryn_client_ui_bevy as source;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn app_with_choices(
    choices: bool,
) -> Result<(App, Entity, BevyServerModalFixture), Box<dyn std::error::Error>> {
    let (link, _commands, _outbox) = play_channel();
    let mut fixture = link.bevy_server_modal_input_fixture()?;
    if choices {
        fixture.with_choices();
    }
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
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
    .add_systems(
        Startup,
        (
            super::super::super::super::ui::setup,
            super::super::super::setup,
        )
            .chain(),
    )
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
        .after(super::super::super::super::ui::setup)
        .before(super::super::super::setup),
    );
    install(&mut app);
    app.update();
    let editor = app
        .world_mut()
        .query_filtered::<Entity, With<ValueField>>()
        .single(app.world())?;
    let mut text = app
        .world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?;
    text.editor_mut().set_text("1");
    text.queue_edit(bevy::text::TextEdit::TextEnd(false));
    app.update();
    Ok((app, editor, fixture))
}
fn button(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(id, action)| matches!(action.kind, ActionKind::Button(1)).then_some(id))
        .ok_or("modal submit".into())
}
#[test]
fn modal_button_waits_for_native_editor_edits_before_driver_acceptance() -> TestResult {
    let (mut app, editor, mut fixture) = app_with_choices(false)?;
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Insert("7".into()));
    let submit = button(&mut app)?;
    app.world_mut().trigger(Activate { entity: submit });
    app.update();
    let answer = fixture.accepted_input()?.ok_or("accepted driver answer")?;
    assert_eq!(answer.value, Some(InputValue::Amount(17)));
    assert!(fixture.accepted_input()?.is_none());
    Ok(())
}
#[test]
fn modal_paste_waits_and_cancels_on_session_closure() -> TestResult {
    use std::sync::{Arc, Mutex};
    for close in [false, true] {
        let (mut app, editor, mut fixture) = app_with_choices(false)?;
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
        let submit = button(&mut app)?;
        app.world_mut().trigger(Activate { entity: submit });
        app.update();
        assert!(
            fixture.accepted_input()?.is_none(),
            "old modal value submitted before clipboard completion"
        );
        if close {
            app.world_mut().non_send_mut::<HostOwner>().close();
        }
        *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("12".into()));
        app.update();
        if close {
            assert!(fixture.accepted_input()?.is_none());
        } else {
            assert_eq!(
                fixture.accepted_input()?.ok_or("answer")?.value,
                Some(InputValue::Amount(12))
            );
        }
    }
    Ok(())
}

#[test]
fn deferred_modal_reply_cancels_on_choice_remember_and_descriptor_changes() -> TestResult {
    use std::sync::{Arc, Mutex};
    for change in 0..3 {
        let (mut app, editor, mut fixture) = app_with_choices(change == 0)?;
        if change == 0 {
            let choice = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(id, action)| matches!(action.kind, ActionKind::Choice(7)).then_some(id))
                .ok_or("first choice")?;
            app.world_mut().trigger(Activate { entity: choice });
        }
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
        let submit = button(&mut app)?;
        app.world_mut().trigger(Activate { entity: submit });
        app.update();
        assert!(fixture.accepted_input()?.is_none());
        if change == 0 {
            let choice = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(id, action)| matches!(action.kind, ActionKind::Choice(9)).then_some(id))
                .ok_or("second choice")?;
            app.world_mut().trigger(Activate { entity: choice });
        } else if change == 1 {
            let remember = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(id, action)| matches!(action.kind, ActionKind::Remember).then_some(id))
                .ok_or("remember")?;
            app.world_mut().trigger(ValueChange {
                source: remember,
                value: true,
                is_final: true,
            });
        } else {
            // A new driver may reuse the same dialog ID; its published revision remains distinct.
            drop(fixture);
            let link = &app
                .world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link;
            fixture = link.bevy_server_modal_input_fixture()?;
        }
        *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("12".into()));
        app.update();
        assert!(
            fixture.accepted_input()?.is_none(),
            "deferred reply must not adopt a changed choice/Remember/descriptor"
        );
        assert!(app.world().resource::<State>().pending.is_none());
        if change < 2 {
            let submit = button(&mut app)?;
            app.world_mut().trigger(Activate { entity: submit });
            app.update();
            let answer = fixture.accepted_input()?.ok_or("fresh button answer")?;
            assert_eq!(answer.value, Some(InputValue::Amount(12)));
            assert_eq!(answer.dont_show_again, change == 1);
            assert_eq!(answer.answer.choice_id, (change == 0).then_some(9));
        }
    }
    Ok(())
}
