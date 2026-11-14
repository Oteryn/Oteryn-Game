use super::tests::{TestResult, click, field, fixture};
use super::*;
#[test]
fn input_profiles_all_editor_actions_use_existing_persistence() -> TestResult {
    let path = std::env::temp_dir().join(format!(
        "input-profiles-persistence-{}.json",
        std::process::id()
    ));
    let result = (|| -> TestResult {
        let mut app = fixture()?;
        app.world_mut().non_send_mut::<HostOwner>().settings =
            crate::play::SettingsState::new(Some(path.clone()));
        open(&mut app.world_mut().non_send_mut::<HostOwner>());
        app.update();
        field(&mut app, 0, "Raid");
        click(&mut app, Kind::Copy)?;
        app.update();
        field(&mut app, 0, "Raid renamed");
        click(&mut app, Kind::Rename)?;
        app.update();
        click(&mut app, Kind::Profile("Default".into()))?;
        app.update();
        click(&mut app, Kind::Profile("Raid renamed".into()))?;
        app.update();
        click(&mut app, Kind::Action("0147-exiva-settings-open".into()))?;
        app.update();
        field(&mut app, 1, "SUPER+ALT+F11");
        click(&mut app, Kind::Primary)?;
        app.update();
        field(&mut app, 2, "SUPER+ALT+F12");
        click(&mut app, Kind::Secondary)?;
        app.update();
        let binding = bindings()
            .into_iter()
            .find(|b| b.id == "0147-exiva-settings-open")
            .ok_or("binding")?;
        {
            let owner = app.world().non_send::<HostOwner>();
            assert_eq!(
                owner.settings.preferences.binding_chord(&binding),
                "SUPER+ALT+F11"
            );
            assert_eq!(
                owner.settings.preferences.binding_secondary(&binding),
                Some("SUPER+ALT+F12")
            );
        }
        field(&mut app, 2, "");
        click(&mut app, Kind::Secondary)?;
        app.update();
        click(&mut app, Kind::Context(true))?;
        app.update();
        assert_ne!(
            app.world()
                .non_send::<HostOwner>()
                .settings
                .preferences
                .binding_chord(&binding),
            "SUPER+ALT+F11"
        );
        click(&mut app, Kind::Context(false))?;
        app.update();
        click(&mut app, Kind::ShortcutAdd("market".into()))?;
        app.update();
        let row = app
            .world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .shortcuts
            .len()
            - 1;
        click(&mut app, Kind::ShortcutMove(row, row - 1))?;
        app.update();
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .settings
                .preferences
                .shortcuts[row - 1],
            "market"
        );
        click(&mut app, Kind::ShortcutMove(row - 1, row))?;
        app.update();
        click(&mut app, Kind::ShortcutRemove(row))?;
        app.update();
        let expected = app
            .world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .clone();
        assert_eq!(expected.hotkey_contexts.selected, "Raid renamed");
        assert_eq!(expected.binding_chord(&binding), "SUPER+ALT+F11");
        assert_eq!(expected.binding_secondary(&binding), None);
        assert!(!expected.shortcuts.contains(&"market".into()));
        let loaded = crate::play::SettingsState::new(Some(path.clone()));
        assert_eq!(loaded.preferences, expected);
        assert_eq!(
            app.world().non_send::<HostOwner>().settings.feedback,
            "Applied and saved"
        );
        Ok(())
    })();
    let _ = std::fs::remove_file(&path);
    result
}
