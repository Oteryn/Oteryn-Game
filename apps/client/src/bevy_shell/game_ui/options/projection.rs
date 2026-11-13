use super::*;
use bevy::ecs::system::SystemParam;
#[derive(SystemParam)]
pub(super) struct Captions<'w, 's> {
    numbers: Query<'w, 's, (&'static NumberCaption, &'static mut Text)>,
    choices: Query<'w, 's, (&'static ChoiceCaption, &'static mut Text), Without<NumberCaption>>,
    feedback: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<Feedback>,
            Without<NumberCaption>,
            Without<ChoiceCaption>,
        ),
    >,
}
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{SliderRange, SliderValue};

// Each query owns a distinct widget role, with explicit exclusions for mutable UI components.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    mut state: ResMut<OptionsState>,
    skins: Res<Skins>,
    checks: Res<CheckSkins>,
    mut focus: ResMut<InputFocus>,
    navigation: Query<Entity, With<Navigation>>,
    bodies: Query<Entity, With<PageBody>>,
    controls: Query<(Entity, &Control, Option<&SliderValue>)>,
    mut dialogs: Query<&mut Node, (With<OptionsDialog>, Without<ChoicePopup>, Without<Thumb>)>,
    mut popups: Query<(&ChoicePopup, &mut Node), (Without<OptionsDialog>, Without<Thumb>)>,
    mut thumbs: Query<(&Thumb, &mut Node), (Without<OptionsDialog>, Without<ChoicePopup>)>,
    mut indicators: Query<(&Indicator, &mut ImageNode)>,
    mut captions: Captions<'_, '_>,
    mut held: Local<Option<(u64, Page, bool)>>,
) {
    let draft = owner.preferences_draft();
    let visible = draft.is_some();
    for mut dialog in &mut dialogs {
        dialog.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Some(draft) = draft else {
        state.generation = None;
        state.choice = None;
        *held = None;
        if focus
            .get()
            .is_some_and(|entity| controls.get(entity).is_ok())
        {
            focus.clear();
        }
        return;
    };
    let generation = owner.generation();
    let target = (generation, state.page, state.advanced);
    let rebuild = held.as_ref() != Some(&target);
    if rebuild {
        if focus
            .get()
            .is_some_and(|entity| controls.get(entity).is_ok())
        {
            focus.clear();
        }
        state.choice = None;
        state.generation = Some(generation);
        for parent in &navigation {
            commands.entity(parent).despawn_children();
            let pages = if state.advanced {
                &Page::ADVANCED[..]
            } else {
                &Page::BASIC_NAV[..]
            };
            for page in pages {
                let caption = if page.child() {
                    format!("   {}", page.label())
                } else {
                    format!("▾ {}", page.label())
                };
                let button = widgets::button(
                    &mut commands,
                    &skins,
                    &caption,
                    115.,
                    Control::Page(*page),
                    &format!("options.page.{page:?}"),
                );
                if *page == state.page {
                    commands.entity(button).insert(Checked);
                }
                commands.entity(parent).add_child(button);
            }
        }
        for parent in &bodies {
            commands.entity(parent).despawn_children();
            widgets::page(&mut commands, parent, state.page, &skins, &checks, draft);
        }
        *held = Some(target);
    }
    let blocked = top_server_dialog(&owner);
    for (entity, control, slider) in &controls {
        // Dynamic page/navigation controls are already queued for despawn on a rebuild.
        if rebuild
            && !matches!(
                control,
                Control::Advanced | Control::Reset | Control::Apply | Control::Ok | Control::Cancel
            )
        {
            continue;
        }
        let value = match *control {
            Control::Advanced => Some(Value::Check(state.advanced)),
            Control::Page(page) => Some(Value::Check(page == state.page)),
            Control::Check(id) | Control::Number(id) | Control::Choice(id) => {
                metadata::find(state.page, id).map(|row| row.value(draft))
            }
            Control::Select(id, index) => metadata::find(state.page, id)
                .map(|row| Value::Check(row.value(draft) == Value::Choice(index))),
            _ => None,
        };
        match value {
            Some(Value::Check(true)) => {
                commands.entity(entity).insert(Checked);
            }
            Some(Value::Check(false)) => {
                commands.entity(entity).remove::<Checked>();
            }
            Some(Value::Number(value)) if slider.is_some_and(|old| old.0 != f32::from(value)) => {
                commands
                    .entity(entity)
                    .insert(SliderValue(f32::from(value)));
            }
            _ => {}
        }
        if blocked {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    for (indicator, mut image) in &mut indicators {
        let Ok((_, control, _)) = controls.get(indicator.0) else {
            continue;
        };
        let checked = match *control {
            Control::Advanced => state.advanced,
            Control::Check(id) => metadata::find(state.page, id)
                .is_some_and(|row| row.value(draft) == Value::Check(true)),
            _ => false,
        };
        let next = if checked {
            &checks.checked
        } else {
            &checks.unchecked
        };
        if image.image != *next {
            image.image = next.clone();
        }
    }
    for (caption, mut text) in &mut captions.numbers {
        if let Some(row) = metadata::find(state.page, caption.0)
            && let Value::Number(value) = row.value(draft)
        {
            let next = value.to_string();
            if text.0 != next {
                text.0 = next;
            }
        }
    }
    for (caption, mut text) in &mut captions.choices {
        if let Some(row) = metadata::find(state.page, caption.0)
            && let Editor::Choice { read, values, .. } = row.editor
        {
            let next = values.get(read(draft)).copied().unwrap_or("");
            if text.0 != next {
                text.0 = next.into();
            }
        }
    }
    for (popup, mut node) in &mut popups {
        node.display = if state.choice == Some(popup.0) && !blocked {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (thumb, mut node) in &mut thumbs {
        let Ok((_, Control::Number(id), _)) = controls.get(thumb.0) else {
            continue;
        };
        let Some(row) = metadata::find(state.page, id) else {
            continue;
        };
        let Editor::Number { read, min, max, .. } = row.editor else {
            continue;
        };
        let range = SliderRange::new(f32::from(min), f32::from(max));
        node.left = percent(range.thumb_position(f32::from(read(draft))) * 100.);
    }
    for mut text in &mut captions.feedback {
        if text.0 != owner.settings.feedback {
            text.0.clone_from(&owner.settings.feedback);
        }
    }
}
