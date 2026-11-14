use super::*;
#[test]
fn settings_tools_window_shows_owner_data_only() {
    let settings = crate::play::SettingsState::new(None);
    assert_eq!(Snapshot::read(&settings).preferences, settings.preferences);
}
#[test]
fn settings_tools_actions_are_bound_and_confirmed() {
    assert!(Action::Import.destructive());
    assert!(Action::Defaults.destructive());
    assert!(!Action::Export.destructive());
}
#[test]
fn settings_tools_rejected_when_session_replaced_or_target_changed() {
    let mut session = Admission::new(7);
    assert!(!session.admit(8, 0, false, false));
    assert!(!session.admit(7, 1, false, false));
    assert!(!session.admit(7, 0, true, false));
    assert!(session.admit(7, 0, false, false));
    assert!(!session.admit(7, 0, false, false));
}
#[test]
fn settings_tools_window_registered() {
    assert!(include_str!("../mod.rs").contains("settings_tools::install(app)"));
    assert!(include_str!("../../ui.rs").contains("Control::SettingsTools"));
}

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<App, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    open(&mut owner);
    let mut app = App::new();
    app.insert_resource(State {
        bound: Some(Admission::new(owner.generation())),
        snapshot: Some(Snapshot::read(&owner.settings)),
        page: Some(SettingsPage::Help),
        draft: Draft::read(&owner.settings.preferences),
        ..default()
    })
    .insert_non_send(owner);
    Ok(app)
}
fn click(app: &mut App, kind: Kind, revision: u64) {
    let generation = app.world().non_send::<HostOwner>().generation();
    let button = app
        .world_mut()
        .spawn(Control {
            generation,
            revision,
            kind,
        })
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: button });
}
#[test]
fn settings_tools_confirmation_and_duplicate_reach_retained_owner() -> TestResult {
    let mut app = fixture()?;
    let path = std::env::temp_dir().join(format!("settings-tools-{}.json", std::process::id()));
    // An existing path must fail rather than replace data.
    std::fs::write(&path, "preserve")?;
    app.world_mut().resource_mut::<State>().draft.path = path.display().to_string();
    click(&mut app, Kind::Action(Action::Export), 0);
    assert_eq!(std::fs::read_to_string(&path)?, "preserve");
    std::fs::remove_file(&path)?;
    let feedback = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .feedback
        .clone();
    click(&mut app, Kind::Action(Action::Export), 0);
    assert!(!path.exists(), "duplicate revision must not execute");
    assert_eq!(
        app.world().non_send::<HostOwner>().settings.feedback,
        feedback
    );
    let snapshot = Snapshot::read(&app.world().non_send::<HostOwner>().settings);
    app.world_mut().resource_mut::<State>().snapshot = Some(snapshot);
    click(&mut app, Kind::Action(Action::Export), 1);
    assert!(path.exists());
    let saved = crate::play::PreferenceStore::read_transfer(&path)?;
    assert_eq!(
        saved,
        app.world().non_send::<HostOwner>().settings.preferences
    );
    std::fs::remove_file(&path)?;
    Ok(())
}
#[test]
fn settings_tools_close_ignores_modal_and_preserves_pending_capture() -> TestResult {
    let mut app = fixture()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .request_screenshot();
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(300), Node::default()));
    click(&mut app, Kind::Action(Action::Close), 99);
    let owner = app.world().non_send::<HostOwner>();
    assert!(!owner.settings.open);
    assert!(owner.settings.screenshot_pending());
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    assert!(visible(app.world().non_send::<HostOwner>()));
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .screenshot_pending()
    );
    Ok(())
}
#[test]
fn settings_tools_edited_confirmation_cannot_replace_settings() -> TestResult {
    let mut app = fixture()?;
    click(&mut app, Kind::Action(Action::Defaults), 0);
    assert!(app.world().resource::<State>().confirm.is_some());
    app.world_mut().resource_mut::<State>().draft.path = "changed target".into();
    let before = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .clone();
    let feedback = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .feedback
        .clone();
    click(&mut app, Kind::Confirm, 1);
    assert_eq!(
        app.world().non_send::<HostOwner>().settings.preferences,
        before
    );
    assert_eq!(
        app.world().non_send::<HostOwner>().settings.feedback,
        feedback
    );
    Ok(())
}
#[test]
fn settings_tools_project_matches_help_and_screenshot_owner() -> TestResult {
    let mut app = fixture()?;
    app.insert_resource(Skins {
        buttons: source::ButtonImages {
            up: default(),
            down: default(),
        },
        checked: default(),
        unchecked: default(),
        frame: default(),
        inset: default(),
        font: default(),
    })
    .add_systems(Update, project);
    app.world_mut().non_send_mut::<HostOwner>().settings.page = SettingsPage::HelpGuide;
    let expected = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .help_text(crate::play::help::Topic::Faq);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == expected)
    );
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.page = SettingsPage::Screenshots;
        owner.settings.preferences.screenshots.folder = "/actual-owner-folder".into();
    }
    app.update();
    assert!(
        app.world_mut()
            .query::<&EditableText>()
            .iter(app.world())
            .any(|t| t.editor.text() == "/actual-owner-folder")
    );
    Ok(())
}
#[test]
fn settings_tools_stale_modal_and_changed_page_do_not_submit() -> TestResult {
    let mut app = fixture()?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .clone();
    let stale = app
        .world_mut()
        .spawn(Control {
            generation: 1,
            revision: 0,
            kind: Kind::Action(Action::Defaults),
        })
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: stale });
    assert!(app.world().resource::<State>().confirm.is_none());
    app.world_mut().non_send_mut::<HostOwner>().settings.page = SettingsPage::Language;
    click(&mut app, Kind::Action(Action::Defaults), 0);
    assert!(app.world().resource::<State>().confirm.is_none());
    app.world_mut().non_send_mut::<HostOwner>().settings.page = SettingsPage::Help;
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(300), Node::default()));
    click(&mut app, Kind::Action(Action::Defaults), 0);
    assert!(app.world().resource::<State>().confirm.is_none());
    assert_eq!(
        app.world().non_send::<HostOwner>().settings.preferences,
        before
    );
    Ok(())
}
#[test]
fn settings_tools_owner_chord_reopens_and_escape_closes_pending() -> TestResult {
    use oteryn_input_actions::{
        ButtonState, KeyCode as InputKey, Modifier, Modifiers, NormalizedInputEvent,
    };
    let mut owner = HostOwner::new(None)?;
    owner.settings.page = SettingsPage::Help;
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code: InputKey::new(18)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
            repeat: false,
        }],
        false,
    );
    assert!(visible(&owner));
    owner.settings.request_screenshot();
    let generation = owner.generation();
    let mut app = App::new();
    app.insert_non_send(owner)
        .insert_resource(State {
            bound: Some(Admission::new(generation)),
            ..default()
        })
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, keyboard);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    assert!(!owner.settings.open);
    assert!(owner.settings.screenshot_pending());
    Ok(())
}
#[test]
fn settings_tools_confirmed_import_uses_owner_backup_and_rejects_duplicate() -> TestResult {
    let directory =
        std::env::temp_dir().join(format!("settings-tools-import-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let result = (|| -> TestResult {
        let mut app = fixture()?;
        let path = directory.join("preferences.json");
        let transfer = directory.join("transfer.json");
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.settings = SettingsState::new(Some(path));
            owner.settings.open = true;
            owner.settings.page = SettingsPage::Help;
            let mut p = owner.settings.preferences.clone();
            p.audio.master_volume = 13;
            owner.settings.apply(p.clone());
            owner.settings.export_to(&transfer)?;
            p.audio.master_volume = 27;
            owner.settings.apply(p);
        }
        let snapshot = Snapshot::read(&app.world().non_send::<HostOwner>().settings);
        {
            let mut state = app.world_mut().resource_mut::<State>();
            state.snapshot = Some(snapshot);
            state.draft.path = transfer.display().to_string();
        }
        click(&mut app, Kind::Action(Action::Import), 0);
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .settings
                .preferences
                .audio
                .master_volume,
            27
        );
        click(&mut app, Kind::Confirm, 1);
        let owner = app.world().non_send::<HostOwner>();
        assert_eq!(owner.settings.preferences.audio.master_volume, 13);
        let backup = owner
            .settings
            .settings_backup_path()
            .ok_or("backup target")?;
        assert_eq!(
            crate::play::PreferenceStore::read_transfer(&backup)?
                .audio
                .master_volume,
            27
        );
        let feedback = owner.settings.feedback.clone();
        click(&mut app, Kind::Confirm, 1);
        assert_eq!(
            app.world().non_send::<HostOwner>().settings.feedback,
            feedback
        );
        assert_eq!(
            crate::play::PreferenceStore::read_transfer(&backup)?
                .audio
                .master_volume,
            27
        );
        Ok(())
    })();
    std::fs::remove_dir_all(directory)?;
    result
}
#[test]
fn settings_tools_capture_request_is_scoped_to_owner_generation() {
    let mut owner = SettingsState::new(None);
    owner.request_screenshot_bound(3);
    assert!(owner.screenshot_pending());
    owner.bind_screenshot_session(3);
    assert!(owner.screenshot_pending());
    owner.bind_screenshot_session(4);
    assert!(!owner.screenshot_pending());
}
