use super::*;

#[test]
fn srvcat_modal_enter_double_click_and_escape_use_server_defaults() -> TestResult {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerClick},
        pointer::{Location, PointerButton, PointerId},
    };
    for trigger in 0..3 {
        let (mut app, _, mut fixture) = app_with_choices(true)?;
        app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        fixture.with_defaults();
        app.update();
        let editor = app
            .world_mut()
            .query_filtered::<Entity, With<ValueField>>()
            .single(app.world())?;
        {
            let mut text = app
                .world_mut()
                .get_mut::<EditableText>(editor)
                .ok_or("editor")?;
            text.editor_mut().set_text("1");
            text.queue_edit(bevy::text::TextEdit::TextEnd(false));
        }
        app.update();
        if trigger == 1 {
            let entity = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(entity, action)| {
                    matches!(action.kind, ActionKind::Choice(9)).then_some(entity)
                })
                .ok_or("second choice")?;
            app.world_mut()
                .get_mut::<EditableText>(editor)
                .ok_or("editor")?
                .queue_edit(bevy::text::TextEdit::Insert("7".into()));
            let pointer = Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ZERO,
                },
            );
            app.world_mut()
                .trigger(bevy::picking::events::PointerPress {
                    entity,
                    pointer: pointer.clone(),
                    button: PointerButton::Primary,
                    count: 2,
                    hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
                });
            app.world_mut().trigger(PointerClick {
                entity,
                pointer,
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
                duration: std::time::Duration::ZERO,
                count: 2,
            });
        } else {
            if trigger == 0 {
                let help = app
                    .world_mut()
                    .query::<(Entity, &Action)>()
                    .iter(app.world())
                    .find_map(|(entity, action)| {
                        matches!(action.kind, ActionKind::Button(8)).then_some(entity)
                    })
                    .ok_or("nondefault focused button")?;
                app.world_mut()
                    .resource_mut::<InputFocus>()
                    .set(help, FocusCause::Navigated);
            }
            app.world_mut().write_message(KeyboardInput {
                key_code: if trigger == 0 {
                    KeyCode::Enter
                } else {
                    KeyCode::Escape
                },
                logical_key: if trigger == 0 {
                    Key::Enter
                } else {
                    Key::Escape
                },
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        }
        app.update();
        let answer = fixture.accepted_input()?.ok_or("default answer")?;
        assert_eq!(answer.answer.button_id, if trigger == 2 { 2 } else { 1 });
        assert_eq!(
            answer.answer.choice_id,
            Some(if trigger == 1 { 9 } else { 7 })
        );
        assert_eq!(
            answer.value,
            if trigger == 2 {
                None
            } else {
                Some(InputValue::Amount(if trigger == 1 { 17 } else { 1 }))
            }
        );
        assert!(fixture.accepted_input()?.is_none());
    }
    Ok(())
}

#[test]
fn srvcat_legacy_modal_enter_keeps_focused_button_activation() -> TestResult {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, _, mut fixture) = app_with_choices(true)?;
    app.add_plugins(bevy::ui_widgets::ButtonPlugin);
    let confirm = button(&mut app)?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(confirm, FocusCause::Navigated);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    let answer = fixture.accepted_input()?.ok_or("legacy focused Enter")?;
    assert_eq!(answer.answer.button_id, 1);
    assert_eq!(answer.answer.choice_id, Some(7));
    assert_eq!(answer.value, Some(InputValue::Amount(1)));
    Ok(())
}
