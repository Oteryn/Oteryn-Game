//! Row 147: General Hotkeys key bindings; each action gets a primary and a secondary chord.
use super::*;
use crate::play::{PreferenceBinding, ReferencePreferences, preference_bindings};
use bevy::{text::EditableText, ui_widgets::Activate};

#[derive(Component)]
pub(super) struct ChordField;
#[derive(Component, Clone, Copy)]
pub(super) enum Action {
    Pick(&'static str),
    Primary,
    Secondary,
    ClearSecondary,
    Default,
}

/// The bindings a keyboard chord can edit; the mouse-backed world actions have no chord.
fn keyboard_bindings() -> Vec<PreferenceBinding> {
    let mut bindings = preference_bindings().unwrap_or_default();
    bindings.retain(|binding| !binding.default_chord.starts_with("MOUSE"));
    bindings
}

/// "0013-paced-movement-move-north" reads as "paced movement move north".
fn caption(binding: &PreferenceBinding, p: &ReferencePreferences) -> String {
    let name = binding
        .id
        .split_once('-')
        .filter(|(number, _)| number.chars().all(|c| c.is_ascii_digit()))
        .map_or(binding.id, |(_, rest)| rest)
        .replace('-', " ");
    let primary = match p.binding_chord(binding) {
        "" => "—",
        chord => chord,
    };
    match p.binding_secondary(binding) {
        Some(secondary) => format!("{name}: {primary} / {secondary}"),
        None => format!("{name}: {primary}"),
    }
}

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &ReferencePreferences,
    selected: Option<&'static str>,
) {
    section::label(
        commands,
        parent,
        skins,
        "Key Bindings: choose an action, type a chord (for example CTRL+SHIFT+F1), then set it.",
    );
    let list = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            Node {
                width: percent(100.),
                height: px(180.),
                flex_shrink: 0.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(list);
    let bindings = keyboard_bindings();
    for binding in &bindings {
        let button = section::action(
            commands,
            list,
            skins,
            &caption(binding, draft),
            400.,
            &format!("hotkeys.binding.{}", binding.id),
            selected == Some(binding.id),
        );
        commands
            .entity(button)
            .insert(Action::Pick(binding.id))
            .observe(activate);
    }
    let selected = selected.and_then(|id| bindings.iter().find(|binding| binding.id == id));
    let chord = selected.map_or("", |binding| draft.binding_chord(binding));
    // Extra action-bar slots keep a single key, so they offer no secondary controls.
    let single = selected.is_some_and(PreferenceBinding::is_extra_action_bar_slot);
    let field = section::field(commands, parent, skins, "hotkeys.binding.chord", chord);
    commands.entity(field).insert(ChordField);
    let bar = section::bar(commands, parent);
    for (text, action, id) in [
        ("Set Primary", Action::Primary, "hotkeys.binding.primary"),
        (
            "Set Secondary",
            Action::Secondary,
            "hotkeys.binding.secondary",
        ),
        (
            "Clear Secondary",
            Action::ClearSecondary,
            "hotkeys.binding.clear",
        ),
        ("Default", Action::Default, "hotkeys.binding.default"),
    ] {
        if single && matches!(action, Action::Secondary | Action::ClearSecondary) {
            continue;
        }
        let button = section::action(commands, bar, skins, text, 110., id, false);
        commands.entity(button).insert(action).observe(activate);
    }
}

/// Restores an action's default primary chord; an action without one becomes unbound again.
/// Extra action-bar slots are unbound in their own action-bar storage.
fn restore(p: &mut ReferencePreferences, binding: &PreferenceBinding) -> Result<(), String> {
    if binding.is_extra_action_bar_slot() {
        return p.rebind(binding.id, "").map_err(|error| error.to_string());
    }
    if binding.default_chord.is_empty() {
        p.keybindings.remove(binding.id);
        return Ok(());
    }
    p.rebind(binding.id, binding.default_chord)
        .map_err(|error| error.to_string())
}

pub(super) fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    fields: Query<&EditableText, With<ChordField>>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::GeneralHotkeys) {
        return;
    }
    let Ok(&action) = actions.get(event.entity) else {
        return;
    };
    if let Action::Pick(id) = action {
        state.binding = Some(id);
        state.revision = state.revision.wrapping_add(1);
        return;
    }
    let Some(binding) = state.binding.and_then(|id| {
        keyboard_bindings()
            .into_iter()
            .find(|binding| binding.id == id)
    }) else {
        owner.settings.feedback = "Choose an action first".into();
        return;
    };
    let chord = fields
        .iter()
        .next()
        .map(|field| field.value().to_string().trim().to_ascii_uppercase())
        .unwrap_or_default();
    section::try_edit(
        &mut owner,
        &mut state,
        |p| match action {
            Action::Primary => p
                .rebind(binding.id, &chord)
                .map_err(|error| error.to_string()),
            Action::Secondary | Action::ClearSecondary if binding.is_extra_action_bar_slot() => {
                Err("Action bar slots take a single key".into())
            }
            Action::Secondary => p
                .rebind_secondary(binding.id, &chord)
                .map_err(|error| error.to_string()),
            Action::ClearSecondary => p
                .rebind_secondary(binding.id, "")
                .map_err(|error| error.to_string()),
            Action::Default | Action::Pick(_) => restore(p, &binding),
        },
        "Key binding changed; Apply or OK to keep it",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::{ButtonState, NormalizedInputEvent};
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn press(app: &mut App, action: Action) {
        let button = app.world_mut().spawn(action).observe(activate).id();
        app.world_mut().trigger(Activate { entity: button });
    }
    fn key(chord: &str) -> Result<NormalizedInputEvent, String> {
        let (code, modifiers) =
            crate::play::feature_local_reference_preferences::settings::parse_action_chord(chord)
                .ok_or(format!("chord {chord}"))?;
        Ok(NormalizedInputEvent::Key {
            code,
            modifiers,
            state: ButtonState::Pressed,
            repeat: false,
        })
    }

    #[test]
    fn rebinding_an_action_applies_saves_and_routes_the_new_chord() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-keys.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::GeneralHotkeys, Some(path.clone()))?;
            let field = app
                .world_mut()
                .spawn((ChordField, EditableText::new("ctrl+alt+shift+f11")))
                .id();
            press(&mut app, Action::Primary);
            assert_eq!(
                app.world().non_send::<HostOwner>().settings.feedback,
                "Choose an action first"
            );
            press(&mut app, Action::Pick("0165-hireling-open"));
            press(&mut app, Action::Primary);
            app.world_mut()
                .get_mut::<EditableText>(field)
                .ok_or("field")?
                .editor_mut()
                .set_text("CTRL+ALT+SHIFT+F10");
            press(&mut app, Action::Secondary);
            let owner = app.world().non_send::<HostOwner>();
            let draft = owner.preferences_draft().ok_or("draft")?;
            assert_eq!(
                draft
                    .keybindings
                    .get("0165-hireling-open")
                    .map(String::as_str),
                Some("CTRL+ALT+SHIFT+F11")
            );
            assert!(
                !owner
                    .settings
                    .preferences
                    .keybindings
                    .contains_key("0165-hireling-open")
            );
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
            assert_eq!(
                stored
                    .secondary_keybindings
                    .get("0165-hireling-open")
                    .map(String::as_str),
                Some("CTRL+ALT+SHIFT+F10")
            );
            // The new chords reach the feature as its own Ctrl+Shift+H; the old one is retired.
            let opened = key("CTRL+SHIFT+H")?;
            assert_eq!(
                stored.route_input(&[key("CTRL+ALT+SHIFT+F11")?])?,
                std::slice::from_ref(&opened)
            );
            assert_eq!(stored.route_input(&[key("CTRL+ALT+SHIFT+F10")?])?, [opened]);
            assert!(stored.route_input(&[key("CTRL+SHIFT+H")?])?.is_empty());
            // Default restores the registered chord.
            let mut app = super::super::tests::app(Page::GeneralHotkeys, Some(path.clone()))?;
            app.world_mut().resource_mut::<OptionsState>().binding = Some("0165-hireling-open");
            press(&mut app, Action::Default);
            let owner = app.world().non_send::<HostOwner>();
            assert_eq!(
                owner
                    .preferences_draft()
                    .ok_or("draft")?
                    .keybindings
                    .get("0165-hireling-open")
                    .map(String::as_str),
                Some("CTRL+SHIFT+H")
            );
            Ok(())
        })();
        fixture.close()?;
        result
    }

    #[test]
    fn extra_action_bar_slots_default_and_secondary_use_their_own_storage() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-keys-slots.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::GeneralHotkeys, Some(path.clone()))?;
            let field = app
                .world_mut()
                .spawn((ChordField, EditableText::new("CTRL+ALT+SHIFT+F9")))
                .id();
            for slot in [5, 36, 450] {
                let id = keyboard_bindings()
                    .into_iter()
                    .find(|binding| binding.id == format!("local-spell-action-bar-spell-{slot}"))
                    .ok_or("slot binding")?
                    .id;
                press(&mut app, Action::Pick(id));
                press(&mut app, Action::Primary);
                let chord = |app: &App| -> Result<String, Box<dyn std::error::Error>> {
                    let owner = app.world().non_send::<HostOwner>();
                    let draft = owner.preferences_draft().ok_or("draft")?;
                    let binding = keyboard_bindings()
                        .into_iter()
                        .find(|binding| binding.id == id)
                        .ok_or("binding")?;
                    Ok(draft.binding_chord(&binding).to_owned())
                };
                assert_eq!(chord(&app)?, "CTRL+ALT+SHIFT+F9", "slot {slot}");
                app.world_mut()
                    .get_mut::<EditableText>(field)
                    .ok_or("field")?
                    .editor_mut()
                    .set_text("CTRL+ALT+SHIFT+F8");
                press(&mut app, Action::Secondary);
                assert_eq!(
                    app.world().non_send::<HostOwner>().settings.feedback,
                    "Action bar slots take a single key"
                );
                press(&mut app, Action::Default);
                assert_eq!(chord(&app)?, "", "slot {slot}");
                app.world_mut().non_send_mut::<HostOwner>().apply(false);
                let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
                assert!(stored.action_keys.0.is_empty(), "slot {slot}");
                assert!(stored.secondary_keybindings.is_empty(), "slot {slot}");
                app.world_mut()
                    .get_mut::<EditableText>(field)
                    .ok_or("field")?
                    .editor_mut()
                    .set_text("CTRL+ALT+SHIFT+F9");
            }
            Ok(())
        })();
        fixture.close()?;
        result
    }

    #[test]
    fn mouse_actions_are_not_offered_by_the_keyboard_editor() -> TestResult {
        let mut app = super::super::tests::app(Page::GeneralHotkeys, None)?;
        app.world_mut().spawn((ChordField, EditableText::new("F9")));
        for id in [
            "0011-world-interaction-click",
            "0011-world-interaction-context",
        ] {
            assert!(keyboard_bindings().iter().all(|binding| binding.id != id));
            press(&mut app, Action::Pick(id));
            press(&mut app, Action::Primary);
            assert_eq!(
                app.world().non_send::<HostOwner>().settings.feedback,
                "Choose an action first"
            );
        }
        Ok(())
    }
}
