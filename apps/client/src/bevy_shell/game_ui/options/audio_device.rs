//! Row 155: the Sound page selects the audio output device; empty means the system default.
use super::*;
use bevy::ui_widgets::Activate;

#[derive(Component)]
pub(super) struct Device(pub(super) String);

/// The system default, then the host's devices; a saved device that is missing stays listed.
fn choices(saved: &str) -> Vec<String> {
    let mut names = crate::play::audio_device_names();
    if !saved.is_empty() && !names.iter().any(|name| name == saved) {
        names.push(saved.to_owned());
    }
    std::iter::once(String::new()).chain(names).collect()
}

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &crate::play::ReferencePreferences,
) {
    section::label(commands, parent, skins, "Audio Output Device");
    let bar = section::bar(commands, parent);
    for name in choices(&draft.audio_device) {
        let caption = if name.is_empty() {
            "System default"
        } else {
            name.as_str()
        };
        let button = section::action(
            commands,
            bar,
            skins,
            caption,
            300.,
            &format!("sound.device.{caption}"),
            name == draft.audio_device,
        );
        commands.entity(button).insert(Device(name)).observe(select);
    }
}

fn select(
    event: On<Activate>,
    devices: Query<&Device>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Sound) {
        return;
    }
    let Ok(device) = devices.get(event.entity) else {
        return;
    };
    let name = device.0.clone();
    owner.edit_preferences(|draft| draft.audio_device = name);
    state.revision = state.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn device_choice_applies_saves_and_reaches_the_audio_preferences() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-device.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Sound, Some(path.clone()))?;
            assert_eq!(choices("Headset").first().map(String::as_str), Some(""));
            assert!(choices("Headset").iter().any(|name| name == "Headset"));
            let button = app
                .world_mut()
                .spawn(Device("Headset".into()))
                .observe(select)
                .id();
            app.world_mut().trigger(Activate { entity: button });
            let owner = app.world().non_send::<HostOwner>();
            assert_eq!(
                owner.preferences_draft().ok_or("draft")?.audio_device,
                "Headset"
            );
            assert!(owner.settings.preferences.audio_device.is_empty());
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let owner = app.world().non_send::<HostOwner>();
            assert_eq!(owner.settings.preferences.audio_device, "Headset");
            let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
            assert_eq!(stored.audio_device, "Headset");
            // The audio driver opens the device named by the play view's preferences.
            let names = vec!["Speakers".to_owned(), "Headset".to_owned()];
            assert_eq!(
                crate::play::select_audio_device(&stored.audio_device, &names),
                Ok(Some(1))
            );
            Ok(())
        })();
        result
    }
}
