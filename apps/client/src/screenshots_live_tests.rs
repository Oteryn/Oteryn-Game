use crate::play::play_channel;
use crate::screenshots::CaptureController;
use oteryn_session::{ActorVitals, AppliedDelta, SessionEvent};

#[test]
fn live_death_events_reach_capture_caller_even_when_respawn_is_coalesced()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    let mut capture = CaptureController::default();
    capture.policy.death = true;
    capture.observe(link.screenshot_facts());
    let vitals = |health| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })
    };
    outbox.push(vec![vitals(0), vitals(150)]);
    assert_eq!(link.screenshot_facts().deaths, 1);
    capture.observe(link.screenshot_facts());
    assert!(capture.pending());
    capture.cancel();
    outbox.push(vec![vitals(150)]);
    capture.observe(link.screenshot_facts());
    assert!(!capture.pending());
    Ok(())
}

#[test]
fn server_skill_reply_and_achievement_delta_reach_capture_caller()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::achievement_notices::{
        AchievementEarned, AchievementWatermark, decode_achievement_earned,
        encode_achievement_earned,
    };
    use oteryn_session::{CommandStatus, GatedCommandResult};
    let (link, _, outbox) = play_channel();
    outbox.skill_display_initialize(true);
    let mut capture = CaptureController::default();
    capture.policy.level = true;
    let result = |level| -> Result<GatedCommandResult, Box<dyn std::error::Error>> {
        let full = oteryn_session::skill_display::SkillDisplay {
            level,
            ..Default::default()
        };
        Ok(GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: full.encode().ok_or("skill encode")?,
        })
    };
    outbox.skill_progress_result(&result(8)?);
    capture.observe(link.screenshot_facts());
    assert!(!capture.pending());
    outbox.skill_progress_result(&result(9)?);
    capture.observe(link.screenshot_facts());
    assert!(capture.pending());
    capture.cancel();
    capture.policy.achievement = true;
    crate::play::mailbox::lock(&link.mailbox)
        .achievement_notices
        .selected = true;
    for count in 1..=2 {
        let value = AchievementEarned {
            key: format!("oteryn:achievement/capture{count}"),
            name: format!("Capture {count}"),
            watermark: AchievementWatermark {
                fact_count: count,
                total_points: count,
            },
        };
        let bytes = encode_achievement_earned(&value).map_err(|e| format!("{e:?}"))?;
        let decoded = decode_achievement_earned(&bytes).map_err(|e| format!("{e:?}"))?;
        outbox.push(vec![SessionEvent::AchievementEarned(AppliedDelta {
            server_sequence: count.into(),
            base_revision: (count - 1).into(),
            new_revision: count.into(),
            value: decoded,
        })]);
        capture.observe(link.screenshot_facts());
        assert_eq!(capture.pending(), count == 2);
    }
    Ok(())
}

#[test]
fn screenshot_settings_survive_restart_and_save_full_and_game_only_frames()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        play::{ScreenshotPreferences, SettingsState},
        screenshots::{Frame, Rect},
    };
    let root = std::env::temp_dir().join(format!("screenshot-settings-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let settings_path = root.join("settings.json");
    let directory = root.join("chosen");
    let mut settings = SettingsState::new(Some(settings_path.clone()));
    let mut preferences = settings.preferences.clone();
    preferences.screenshots.directory = directory.to_str().ok_or("directory")?.into();
    preferences.screenshots.game_only = true;
    settings.apply(preferences.clone());
    let restored = SettingsState::new(Some(settings_path));
    assert_eq!(restored.preferences.screenshots, preferences.screenshots);
    let pixels: Vec<u8> = (0..24).collect();
    let frame = || Frame {
        width: 3,
        height: 2,
        bgra: &pixels,
        game: Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 2,
        },
    };
    let mut capture = CaptureController::default();
    capture.manual();
    let cropped = restored.preferences.screenshots.save_frame(
        &mut capture,
        &root.join("fallback"),
        frame(),
    )?;
    assert!(!capture.pending());
    assert_eq!(cropped.parent(), Some(directory.as_path()));
    let bytes = std::fs::read(&cropped)?;
    assert_eq!(&bytes[18..22], &1i32.to_le_bytes());
    assert_eq!(&bytes[22..26], &(-2i32).to_le_bytes());
    assert_eq!(&bytes[54..], &[&pixels[4..8], &pixels[16..20]].concat());
    let mut full: ScreenshotPreferences = restored.preferences.screenshots.clone();
    full.game_only = false;
    full.directory.clear();
    capture.manual();
    let entire = full.save_frame(&mut capture, &root.join("fallback"), frame())?;
    assert!(!capture.pending());
    assert_eq!(entire.parent(), Some(root.join("fallback").as_path()));
    let bytes = std::fs::read(&entire)?;
    assert_eq!(&bytes[18..22], &3i32.to_le_bytes());
    assert_eq!(&bytes[54..], pixels.as_slice());
    capture.manual();
    let second = full.save_frame(&mut capture, &root.join("fallback"), frame())?;
    assert_ne!(second, entire);
    let mut invalid = preferences.clone();
    invalid.screenshots.directory = "relative-folder".into();
    settings.apply(invalid);
    assert_eq!(settings.preferences, preferences);
    let blocked = root.join("not-a-directory");
    std::fs::write(&blocked, b"preserve")?;
    full.directory = blocked.to_str().ok_or("blocked directory")?.into();
    capture.manual();
    assert!(full.save_frame(&mut capture, &root, frame()).is_err());
    assert!(capture.pending());
    assert_eq!(std::fs::read(blocked)?, b"preserve");
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn screenshot_backlog_saves_preceding_frames_on_confirmed_death_and_persists_policy()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    use crate::screenshots::{CaptureBacklog, Frame, Rect};
    use std::time::{Duration, Instant};
    let root = std::env::temp_dir().join(format!("screenshot-backlog-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let settings_path = root.join("settings.json");
    let mut settings = SettingsState::new(Some(settings_path.clone()));
    let mut preferences = settings.preferences.clone();
    preferences.screenshots.backlog_seconds = 2;
    preferences.screenshots.on_death = true;
    settings.apply(preferences.clone());
    let restored = SettingsState::new(Some(settings_path));
    assert_eq!(restored.preferences.screenshots, preferences.screenshots);
    let prefs = &restored.preferences.screenshots;
    let mut capture = CaptureController::default();
    prefs.apply_policy(&mut capture);
    let (link, _, outbox) = play_channel();
    outbox.vitals(Some(ActorVitals {
        health: 100,
        max_health: 100,
        ..Default::default()
    }));
    capture.observe(link.screenshot_facts());
    let mut backlog = CaptureBacklog::default();
    let now = Instant::now();
    assert!(backlog.sample_due(now, prefs.backlog_seconds));
    assert!(!backlog.sample_due(now + Duration::from_millis(500), prefs.backlog_seconds));
    let area = Rect {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    for pixel in 0..3u8 {
        let pixels = [pixel, 0, 0, 255];
        assert!(
            backlog
                .process(
                    &mut capture,
                    prefs,
                    &root,
                    Frame {
                        width: 1,
                        height: 1,
                        bgra: &pixels,
                        game: area
                    },
                    now + Duration::from_secs(u64::from(pixel))
                )?
                .is_empty()
        );
    }
    outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: ActorVitals {
            health: 0,
            max_health: 100,
            ..Default::default()
        },
    })]);
    capture.observe(link.screenshot_facts());
    assert!(capture.pending());
    let pixels = [3, 0, 0, 255];
    let paths = backlog.process(
        &mut capture,
        prefs,
        &root,
        Frame {
            width: 1,
            height: 1,
            bgra: &pixels,
            game: area,
        },
        now + Duration::from_secs(3),
    )?;
    assert_eq!(paths.len(), 3);
    for (path, expected) in paths.iter().zip([1, 2, 3]) {
        assert_eq!(std::fs::read(path)?[54], expected);
    }
    assert!(!capture.pending());
    capture.manual();
    let paths = backlog.process(
        &mut capture,
        prefs,
        &root,
        Frame {
            width: 1,
            height: 1,
            bgra: &pixels,
            game: area,
        },
        now + Duration::from_secs(6),
    )?;
    assert_eq!(paths.len(), 1, "expired backlog frames must not be saved");
    assert!(!backlog.sample_due(now + Duration::from_secs(7), 0));
    let mut disabled = prefs.clone();
    disabled.backlog_seconds = 0;
    capture.manual();
    assert_eq!(
        backlog
            .process(
                &mut capture,
                &disabled,
                &root,
                Frame {
                    width: 1,
                    height: 1,
                    bgra: &pixels,
                    game: area
                },
                now + Duration::from_secs(7)
            )?
            .len(),
        1
    );
    let mut invalid = preferences.clone();
    invalid.screenshots.backlog_seconds = 11;
    settings.apply(invalid);
    assert_eq!(settings.preferences, preferences);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
