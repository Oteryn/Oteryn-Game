//! The Interface page chooses the client language; the saved choice is used from the next start.
use super::*;
use bevy::ui_widgets::Activate;
use oteryn_client_ui::localization::Language;

#[derive(Component, Clone, Copy)]
pub(super) struct Choice(Language);

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &crate::play::ReferencePreferences,
) {
    section::label(
        commands,
        parent,
        skins,
        tr!("Client Language (used after the client restarts)"),
    );
    let bar = section::bar(commands, parent);
    for language in Language::ALL {
        let button = section::action(
            commands,
            bar,
            skins,
            language.name(),
            100.,
            &format!("interface.language.{}", language.code()),
            draft.language() == language,
        );
        commands
            .entity(button)
            .insert(Choice(language))
            .observe(select);
    }
}

fn select(
    event: On<Activate>,
    choices: Query<&Choice>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Interface) {
        return;
    }
    let Ok(&Choice(language)) = choices.get(event.entity) else {
        return;
    };
    owner.edit_preferences(|draft| draft.set_language(language));
    state.revision = state.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn language_choice_applies_and_saves_the_client_language() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-language.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Interface, Some(path.clone()))?;
            let button = app
                .world_mut()
                .spawn(Choice(Language::De))
                .observe(select)
                .id();
            app.world_mut().trigger(Activate { entity: button });
            let owner = app.world().non_send::<HostOwner>();
            assert_eq!(
                owner.preferences_draft().ok_or("draft")?.language(),
                Language::De
            );
            assert_ne!(owner.settings.preferences.language(), Language::De);
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
            assert_eq!(stored.language(), Language::De);
            Ok(())
        })();
        fixture.close()?;
        result
    }
}
