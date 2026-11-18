//! Row 149: General Hotkeys hotkey sets (select, add, rename, remove) and Chat On / Chat Off.
use super::*;
use bevy::{text::EditableText, ui_widgets::Activate};

#[derive(Component)]
pub(super) struct NameField;
#[derive(Component, Clone)]
pub(super) enum Action {
    Select(String),
    Add,
    Rename,
    Remove,
    Chat(bool),
}

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &crate::play::ReferencePreferences,
) {
    let sets = &draft.hotkey_contexts;
    section::label(commands, parent, skins, "Hotkey Set");
    let bar = section::bar(commands, parent);
    for name in sets.presets.keys() {
        let button = section::action(
            commands,
            bar,
            skins,
            name,
            120.,
            &format!("hotkeys.set.{name}"),
            *name == sets.selected,
        );
        commands
            .entity(button)
            .insert(Action::Select(name.clone()))
            .observe(activate);
    }
    section::label(
        commands,
        parent,
        skins,
        "Set name (up to 32 characters; at most 8 sets)",
    );
    let field = section::field(commands, parent, skins, "hotkeys.set.name", &sets.selected);
    commands.entity(field).insert(NameField);
    let bar = section::bar(commands, parent);
    for (caption, action, id) in [
        ("Add", Action::Add, "hotkeys.set.add"),
        ("Rename", Action::Rename, "hotkeys.set.rename"),
        ("Remove", Action::Remove, "hotkeys.set.remove"),
    ] {
        let button = section::action(commands, bar, skins, caption, 90., id, false);
        commands.entity(button).insert(action).observe(activate);
    }
    section::label(
        commands,
        parent,
        skins,
        "Chat On and Chat Off keep separate key bindings in every set; edit the mode selected here.",
    );
    let bar = section::bar(commands, parent);
    for (caption, chat_on) in [("Chat On", true), ("Chat Off", false)] {
        let button = section::action(
            commands,
            bar,
            skins,
            caption,
            90.,
            &format!("hotkeys.chat.{chat_on}"),
            sets.chat_on == chat_on,
        );
        commands
            .entity(button)
            .insert(Action::Chat(chat_on))
            .observe(activate);
    }
}

pub(super) fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    fields: Query<&EditableText, With<NameField>>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::GeneralHotkeys) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let name = fields
        .iter()
        .next()
        .map(|field| field.value().to_string())
        .unwrap_or_default();
    let action = action.clone();
    section::try_edit(
        &mut owner,
        &mut state,
        |p| {
            let selected = p.hotkey_contexts.selected.clone();
            let result = match action {
                Action::Select(name) => p.select_hotkey_profile(&name),
                Action::Add => p.add_hotkey_profile(&name),
                Action::Rename => p.rename_hotkey_profile(&selected, &name),
                Action::Remove => p.remove_hotkey_profile(&selected),
                Action::Chat(chat_on) => p.select_hotkey_context(chat_on),
            };
            result.map_err(|error| error.to_string())
        },
        "Hotkey set changed; Apply or OK to keep it",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn press(app: &mut App, action: Action) {
        let button = app.world_mut().spawn(action).observe(activate).id();
        app.world_mut().trigger(Activate { entity: button });
    }

    #[test]
    fn hotkey_sets_add_select_and_chat_mode_apply_and_route_keys() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-sets.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::GeneralHotkeys, Some(path.clone()))?;
            app.world_mut()
                .spawn((NameField, EditableText::new("Hunt")));
            press(&mut app, Action::Add);
            press(&mut app, Action::Chat(false));
            let draft = app
                .world()
                .non_send::<HostOwner>()
                .preferences_draft()
                .cloned()
                .ok_or("draft")?;
            assert_eq!(draft.hotkey_contexts.selected, "Hunt");
            assert!(draft.hotkey_contexts.presets.contains_key("Default"));
            assert!(!draft.hotkey_contexts.chat_on);
            press(&mut app, Action::Select("Default".into()));
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
            assert_eq!(stored.hotkey_contexts.selected, "Default");
            assert_eq!(stored.hotkey_contexts.presets.len(), 2);
            assert!(!stored.hotkey_contexts.chat_on);
            // Removing the last set is refused and leaves the draft as it was.
            let mut app = super::super::tests::app(Page::GeneralHotkeys, Some(path.clone()))?;
            press(&mut app, Action::Remove);
            press(&mut app, Action::Remove);
            let owner = app.world().non_send::<HostOwner>();
            assert_eq!(
                owner
                    .preferences_draft()
                    .ok_or("draft")?
                    .hotkey_contexts
                    .presets
                    .len(),
                1
            );
            assert_eq!(
                owner.settings.feedback,
                "Invalid preferences or conflicting key binding"
            );
            Ok(())
        })();
        result
    }
}
