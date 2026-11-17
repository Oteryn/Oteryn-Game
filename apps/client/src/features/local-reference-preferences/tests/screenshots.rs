use crate::play::{
    SettingsState,
    screenshot::{CaptureSource, Pixels},
};

struct Source(Vec<bool>);
impl CaptureSource for Source {
    fn capture(&mut self, game_only: bool) -> std::io::Result<Pixels> {
        self.0.push(game_only);
        Pixels::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255])
    }
}

#[test]
fn screenshot_advance_triggers_use_decoded_server_skill_display()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_client_runtime::skill_display::SkillDisplay;
    let folder = std::env::temp_dir().join(format!("t071-skills-{}", std::process::id()));
    let mut state = SettingsState::new(Some(folder.join("settings.json")));
    let mut preferences = state.preferences.clone();
    preferences.screenshots.folder = folder.to_string_lossy().into_owned();
    preferences.screenshots.on_level = true;
    preferences.screenshots.on_skill = true;
    state.apply(preferences);
    let (link, mut commands, outbox) = play_channel();
    outbox.skill_display_initialize(true);
    let mut display = SkillDisplay {
        level: 1,
        ..Default::default()
    };
    let publish = |display: SkillDisplay| -> Result<(), Box<dyn std::error::Error>> {
        outbox.skill_progress_result(&oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: display.encode().ok_or("invalid display")?,
        });
        Ok(())
    };
    publish(display)?;
    let mut source = Source(Vec::new());
    let now = std::time::Instant::now();
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut source)?
            .is_empty()
    );
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if command.capability == 370 && command.kind == 640)
    );
    display.level = 2;
    publish(display)?;
    assert_eq!(
        state
            .process_screenshots(now, Some(&link), &mut source)?
            .len(),
        1
    );
    display.trained.skills[0].level = 1;
    display.loyalty_levels[0] = 1;
    publish(display)?;
    assert_eq!(
        state
            .process_screenshots(now, Some(&link), &mut source)?
            .len(),
        1
    );
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut source)?
            .is_empty()
    );
    assert!(commands.try_recv().is_err());
    std::fs::remove_dir_all(folder)?;
    Ok(())
}

#[test]
fn manual_screenshot_caller_saves_real_pixels_and_selected_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::temp_dir().join(format!("t070-{}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
    let mut state = SettingsState::new(Some(folder.join("settings.json")));
    let mut preferences = state.preferences.clone();
    preferences.screenshots.folder = folder.to_string_lossy().into_owned();
    preferences.screenshots.game_only = true;
    state.apply(preferences);
    let mut source = Source(Vec::new());
    assert!(state.take_screenshot(&mut source)?.is_none());
    state.request_screenshot();
    let path = state
        .take_screenshot(&mut source)?
        .ok_or("missing screenshot")?;
    let bytes = std::fs::read(&path)?;
    assert_eq!(&bytes[..2], b"BM");
    assert_eq!(&bytes[54..60], &[0, 0, 255, 0, 255, 0]);
    assert_eq!(source.0, [true]);
    let mut preferences = state.preferences.clone();
    preferences.screenshots.game_only = false;
    state.apply(preferences);
    state.request_screenshot();
    assert_ne!(
        state
            .take_screenshot(&mut source)?
            .ok_or("missing window capture")?,
        path
    );
    assert_eq!(source.0, [true, false]);
    assert!(
        !SettingsState::new(Some(folder.join("settings.json")))
            .preferences
            .screenshots
            .game_only
    );
    std::fs::remove_dir_all(folder)?;
    Ok(())
}

#[test]
fn screenshot_backlog_and_death_trigger_follow_server_vitals()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_session::{ActorVitals, AppliedDelta, SessionEvent};
    use std::time::{Duration, Instant};
    let folder = std::env::temp_dir().join(format!("t071-{}", std::process::id()));
    let mut state = SettingsState::new(Some(folder.join("settings.json")));
    let mut preferences = state.preferences.clone();
    preferences.screenshots.folder = folder.to_string_lossy().into_owned();
    preferences.screenshots.backlog_frames = 2;
    preferences.screenshots.on_death = true;
    state.apply(preferences);
    let (link, _commands, outbox) = play_channel();
    let vitals = |health| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 100,
                ..Default::default()
            },
        })
    };
    let now = Instant::now();
    let mut source = Source(Vec::new());
    outbox.push(vec![vitals(100)]);
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut source)?
            .is_empty()
    );
    assert!(
        state
            .process_screenshots(now + Duration::from_millis(500), Some(&link), &mut source)?
            .is_empty()
    );
    assert_eq!(source.0.len(), 1);
    state.process_screenshots(now + Duration::from_secs(1), Some(&link), &mut source)?;
    state.process_screenshots(now + Duration::from_secs(2), Some(&link), &mut source)?;
    outbox.push(vec![vitals(0)]);
    let saved =
        state.process_screenshots(now + Duration::from_millis(2500), Some(&link), &mut source)?;
    assert_eq!(saved.len(), 3); // Two bounded pre-event frames and the event frame.
    for path in saved {
        assert_eq!(&std::fs::read(path)?[..2], b"BM");
    }
    assert!(
        state
            .process_screenshots(now + Duration::from_millis(2600), Some(&link), &mut source)?
            .is_empty()
    );
    // A death followed by a respawn in one mailbox batch still produces one capture.
    outbox.push(vec![vitals(100), vitals(0), vitals(100)]);
    assert_eq!(
        state
            .process_screenshots(now + Duration::from_millis(2700), Some(&link), &mut source)?
            .len(),
        1
    );
    state.process_screenshots(now + Duration::from_secs(3), None, &mut source)?;
    state.request_screenshot();
    assert_eq!(
        state
            .process_screenshots(now + Duration::from_secs(4), Some(&link), &mut source)?
            .len(),
        1
    );
    std::fs::remove_dir_all(folder)?;
    Ok(())
}

#[test]
fn screenshot_editors_reconcile_destinations_and_independent_backlog_modes() {
    use crate::play::ScreenshotPreferences;
    let original = ScreenshotPreferences {
        directory: std::env::temp_dir().to_string_lossy().into_owned(),
        backlog_seconds: 3,
        ..Default::default()
    };
    let effective = original.effective();
    assert_eq!(effective.folder, original.directory);
    assert_eq!(effective.backlog_frames, 3);
    let mut edited = effective;
    edited.folder = std::env::temp_dir()
        .join("captures")
        .to_string_lossy()
        .into_owned();
    edited.backlog_frames = 2;
    let edited = edited.reconciled(&original.effective());
    assert_eq!(edited.directory, edited.folder);
    assert_eq!(edited.backlog_seconds, 0);
    assert_eq!(edited.effective().backlog_frames, 2);
    let mut disabled = edited.effective();
    disabled.backlog_frames = 0;
    let disabled = disabled.reconciled(&edited.effective());
    assert_eq!(disabled.effective().backlog_frames, 0);
    assert_eq!(disabled.backlog_seconds, 0);
    assert_eq!(original.backlog_seconds, 3);
}
