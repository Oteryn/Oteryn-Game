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
    let fixture = crate::play::screenshot::TestDir::new()?;
    let folder = fixture.path().to_path_buf();
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
    publish(display)?;
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut source)?
            .is_empty()
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
    Ok(())
}

#[test]
fn manual_screenshot_caller_saves_real_pixels_and_selected_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let folder = fixture.path().to_path_buf();
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
    Ok(())
}

#[test]
fn screenshot_backlog_and_death_trigger_follow_server_vitals()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_session::{ActorVitals, AppliedDelta, SessionEvent};
    use std::time::{Duration, Instant};
    let fixture = crate::play::screenshot::TestDir::new()?;
    let folder = fixture.path().to_path_buf();
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
    Ok(())
}

#[test]
fn paired_rescue_vitals_trigger_and_rearm_low_health_capture()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_session::{ActorCooldownUpdate, ActorVitals, AppliedDelta, SessionEvent};
    use std::time::{Duration, Instant};

    let fixture = crate::play::screenshot::TestDir::new()?;
    let folder = fixture.path();
    let mut state = SettingsState::new(Some(folder.join("settings.json")));
    let mut preferences = state.preferences.clone();
    preferences.screenshots.folder = folder.to_string_lossy().into_owned();
    preferences.screenshots.on_low_health = true;
    state.apply(preferences);
    let (link, _, outbox) = play_channel();
    let mut source = Source(Vec::new());
    let now = Instant::now();
    let mut captures = Vec::new();

    // The ordinary 4% event distinguishes a missed initial trigger from a missed rearm.
    for (step, (health, paired)) in [(100, false), (30, true), (40, false), (200, true), (40, false)]
        .into_iter()
        .enumerate()
    {
        let revision = step as u64;
        let vitals = AppliedDelta {
            server_sequence: 2 * revision + 2,
            base_revision: revision,
            new_revision: revision + 1,
            value: ActorVitals {
                health,
                max_health: 1_000,
                ..Default::default()
            },
        };
        let event = if paired {
            SessionEvent::GiftOfLife {
                cooldowns: AppliedDelta {
                    server_sequence: vitals.server_sequence - 1,
                    base_revision: revision / 2,
                    new_revision: revision / 2 + 1,
                    value: ActorCooldownUpdate {
                        value: oteryn_protocol_oteryn::presentation::ActorCooldowns {
                            server_now_ms: revision * 1_000,
                            rtt_ms: None,
                            entries: Vec::new(),
                        },
                        received_at: now,
                        snapshot: false,
                    },
                },
                vitals,
            }
        } else {
            SessionEvent::ActorVitals(vitals)
        };
        outbox.push(vec![event]);
        captures.push(
            state
                .process_screenshots(now + Duration::from_secs(revision), Some(&link), &mut source)?
                .len(),
        );
    }

    assert_eq!(captures, [0, 1, 0, 0, 1]);
    Ok(())
}

#[test]
fn paired_gift_of_life_rearms_low_health_screenshot() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{
        ActorCooldownUpdate, ActorCooldowns, ActorVitals, AppliedDelta, SessionEvent,
    };
    use std::time::Instant;

    let fixture = crate::play::screenshot::TestDir::new()?;
    let mut state = SettingsState::new(None);
    let mut preferences = state.preferences.clone();
    preferences.screenshots.folder = fixture.path().to_string_lossy().into_owned();
    preferences.screenshots.on_low_health = true;
    state.apply(preferences);
    let (link, _, outbox) = crate::play::play_channel();
    let mut source = Source(Vec::new());
    let now = Instant::now();
    let mut capture_count = || -> std::io::Result<usize> {
        Ok(state.process_screenshots(now, Some(&link), &mut source)?.len())
    };
    let vitals = |server_sequence, revision, health| AppliedDelta {
        server_sequence,
        base_revision: revision - 1,
        new_revision: revision,
        value: ActorVitals {
            health,
            max_health: 100,
            ..Default::default()
        },
    };
    outbox.push(vec![SessionEvent::ActorVitals(vitals(1, 1, 30))]);
    assert_eq!(capture_count()?, 0);
    outbox.push(vec![SessionEvent::ActorVitals(vitals(2, 2, 3))]);
    assert_eq!(capture_count()?, 1);
    outbox.push(vec![SessionEvent::GiftOfLife {
        cooldowns: AppliedDelta {
            server_sequence: 3,
            base_revision: 0,
            new_revision: 1,
            value: ActorCooldownUpdate {
                value: ActorCooldowns {
                    server_now_ms: 0,
                    rtt_ms: None,
                    entries: vec![],
                },
                received_at: now,
                snapshot: false,
            },
        },
        vitals: vitals(4, 3, 20),
    }]);
    assert_eq!(capture_count()?, 0);
    outbox.push(vec![SessionEvent::ActorVitals(vitals(5, 4, 3))]);
    assert_eq!(capture_count()?, 1);
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

#[test]
fn screenshot_alias_manual_capture_uses_the_effective_absolute_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let mut state = SettingsState::new(None);
    let mut preferences = state.preferences.clone();
    preferences.screenshots.directory = root.to_string_lossy().into_owned();
    preferences.screenshots.folder = "ignored-relative-capture".into();
    state.apply(preferences);
    state.request_screenshot();
    let path = state
        .take_screenshot(&mut Source(Vec::new()))?
        .ok_or("capture missing")?;
    let actual = path.parent().ok_or("capture directory")?.to_path_buf();
    std::fs::remove_file(path)?;
    assert_eq!(actual, root);
    Ok(())
}

#[test]
fn screenshot_alias_pixel_writer_rejects_relative_folders() -> Result<(), Box<dyn std::error::Error>>
{
    crate::play::screenshot::with_relative_capture_path(|folder, _| {
        let result = Pixels::new(1, 1, vec![0, 0, 0, 255])?.save(folder);
        assert!(result.is_err_and(|error| error.kind() == std::io::ErrorKind::InvalidInput));
        assert!(!std::path::Path::new(folder).exists());
        Ok(())
    })
}

#[test]
fn tracked_drop_requests_screenshot_on_confirmed_loot() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::analyser::{AnalyserFact, AnalyserItemCount};
    let folder = tempfile::tempdir()?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.analyser_initialize(true);
    link.analyser_open();
    let mut state = SettingsState::new(None);
    state.preferences.screenshots.directory = folder.path().to_string_lossy().into_owned();
    state.preferences.screenshots.on_valuable_loot = true;
    let now = std::time::Instant::now();
    let loot = |seq| {
        oteryn_session::SessionEvent::AnalyserFacts(oteryn_session::AppliedDelta {
            server_sequence: seq,
            base_revision: seq - 1,
            new_revision: seq,
            value: vec![AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![AnalyserItemCount {
                    item_type: std::num::NonZeroU32::MIN,
                    count: std::num::NonZeroU16::MIN,
                }],
                gold: 0,
            }],
        })
    };
    outbox.push(vec![loot(1)]);
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .is_empty()
    );
    assert!(link.analyser_select("loot-tracker"));
    link.analyser_track_item("1", true);
    outbox.push(vec![loot(2)]);
    assert_eq!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .len(),
        1
    );
    link.analyser_track_item("1", false);
    link.analyser_auto_value("5");
    outbox.push(vec![loot(3)]);
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .is_empty()
    );
    link.analyser_resolve_items();
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("catalogue request".into());
    };
    assert_eq!(command.kind, 734);
    outbox.analyser_catalogue_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 4,
        payload: oteryn_session::analyser_catalogue::encode_reply(&[
            oteryn_session::analyser_catalogue::ItemInfo {
                item_type: 1,
                name: "Known drop".into(),
                coin_worth: 100,
            },
        ])?,
    });
    assert_eq!(
        link.analyser_panel()
            .loot
            .get(&1)
            .ok_or("tracked drop")?
            .count,
        3
    );
    assert!(link.analyser_panel().auto_tracked.contains(&1));
    assert_eq!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .len(),
        1,
        "capture the drop that first meets the auto-track threshold"
    );
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .is_empty()
    );
    outbox.push(vec![loot(4)]);
    assert_eq!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .len(),
        1
    );
    let mut prefs = state.preferences.clone();
    prefs.screenshots.on_valuable_loot = false;
    state.apply(prefs.clone());
    outbox.push(vec![loot(5)]);
    prefs.screenshots.on_valuable_loot = true;
    state.apply(prefs);
    assert!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .is_empty(),
        "loot received before enabling the switch"
    );
    outbox.push(vec![loot(6)]);
    assert_eq!(
        state
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .len(),
        1
    );
    Ok(())
}

#[test]
fn tracked_bestiary_thresholds_and_boss_kills_trigger_independently()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_tracker::{
        TrackerEntry, TrackerSnapshot, encode_domain_snapshot,
    };
    let folder = tempfile::tempdir()?;
    for key in [
        "on_bestiary_unlock",
        "on_bestiary_complete",
        "on_boss_defeat",
    ] {
        let (link, _, outbox) = crate::play::play_channel();
        outbox.cyclopedia_tracker_initialize(true);
        let mut state = SettingsState::new(None);
        let mut settings = serde_json::to_value(&state.preferences.screenshots)?;
        settings[key] = true.into();
        state.preferences.screenshots = serde_json::from_value(settings)?;
        state.preferences.screenshots.directory = folder.path().to_string_lossy().into_owned();
        for (kills, boss_kills, event) in [
            (1, 1, ""),
            (2, 1, "on_bestiary_unlock"),
            (3, 1, "on_bestiary_unlock"),
            (4, 1, "on_bestiary_complete"),
            (4, 2, "on_boss_defeat"),
        ] {
            let tracker = |boss: bool, kill_count| TrackerSnapshot {
                boss,
                entries: vec![TrackerEntry {
                    creature_key: if boss { "ferumbras" } else { "rat" }.into(),
                    kill_count,
                    thresholds: [2, 3, 4],
                    completed: kill_count >= 4,
                }],
            };
            outbox.cyclopedia_tracker_push(
                encode_domain_snapshot(&tracker(false, kills), &tracker(true, boss_kills))
                    .ok_or("tracker")?,
            );
            let paths = state.process_screenshots(
                std::time::Instant::now(),
                Some(&link),
                &mut Source(vec![]),
            )?;
            assert_eq!(paths.len(), usize::from(key == event));
        }
    }
    Ok(())
}

#[test]
fn impact_capture_records_survive_character_relogin() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::analyser_transport::{AnalyserFact, AnalyserImpact};
    use std::num::NonZeroU32;
    let folder = tempfile::tempdir()?;
    for (id, key) in [(1, "on_record_damage"), (2, "on_record_healing")] {
        for (session, values) in [
            (1, vec![(100, true), (100, false), (99, false)]),
            (2, vec![(80, false), (101, true)]),
        ] {
            let mut settings = SettingsState::new(Some(folder.path().join("settings.json")));
            settings.select_character(Some([id; 16]));
            if session == 1 {
                let mut draft = settings.preferences.clone();
                let mut json = serde_json::to_value(&draft.screenshots)?;
                json[key] = true.into();
                draft.screenshots = serde_json::from_value(json)?;
                draft.screenshots.directory = folder.path().to_string_lossy().into_owned();
                settings.apply(draft);
            }
            let (link, _, outbox) = crate::play::play_channel();
            let now = std::time::Instant::now();
            settings.process_screenshots(now, Some(&link), &mut Source(vec![]))?;
            for (i, (amount, record)) in values.into_iter().enumerate() {
                let value = NonZeroU32::new(amount).ok_or("amount")?;
                let impact = if id == 1 {
                    AnalyserImpact::Damage {
                        value,
                        element: oteryn_protocol_oteryn::damage_element::DamageElement::Physical,
                    }
                } else {
                    AnalyserImpact::Healing(value)
                };
                outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
                    oteryn_session::AppliedDelta {
                        server_sequence: i as u64 + 1,
                        base_revision: i as u64,
                        new_revision: i as u64 + 1,
                        value: vec![AnalyserFact::Impact(impact)],
                    },
                )]);
                assert_eq!(
                    settings
                        .process_screenshots(now, Some(&link), &mut Source(vec![]))?
                        .len(),
                    usize::from(record)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn untracked_boss_kill_uses_confirmed_bosstiary_progress() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_protocol_oteryn::boss_encounter::{BosstiaryProgressEntry, encode_bosstiary_list};
    let folder = tempfile::tempdir()?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.boss_encounter_initialize(true);
    let mut settings = SettingsState::new(None);
    settings.preferences.screenshots.on_boss_defeat = true;
    settings.preferences.screenshots.directory = folder.path().to_string_lossy().into_owned();
    let start = std::time::Instant::now();
    for (index, count) in [(0_u64, 2), (1, 3), (2, 3)] {
        let now = start + std::time::Duration::from_secs(index * 6);
        assert!(
            settings
                .process_screenshots(now, Some(&link), &mut Source(vec![]))?
                .is_empty()
        );
        let crate::combat_input::PlayCommand::Feature(query) = commands.try_recv()? else {
            return Err("boss progress query".into());
        };
        assert_eq!((query.capability, query.kind), (117, 133));
        outbox.boss_encounter_result(&oteryn_session::GatedCommandResult {
            command_id: index + 1,
            server_sequence: index + 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_bosstiary_list(
                index,
                &[],
                false,
                &[BosstiaryProgressEntry {
                    boss_key: "oteryn:creature.ferumbras".into(),
                    kill_count: count,
                }],
                0,
                false,
            )
            .map_err(|e| format!("{e:?}"))?,
        });
        assert_eq!(
            settings
                .process_screenshots(now, Some(&link), &mut Source(vec![]))?
                .len(),
            usize::from(index == 1)
        );
    }
    Ok(())
}

#[test]
fn manual_bosstiary_reply_after_scope_transfer_belongs_to_the_open_panel()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.boss_encounter_initialize(true);
    let mut settings = SettingsState::new(None);
    settings.preferences.screenshots.on_boss_defeat = true;
    settings.process_screenshots(std::time::Instant::now(), Some(&link), &mut Source(vec![]))?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    outbox.screenshot_activity_handoff()?;
    link.boss_encounter_request(oteryn_client_ui::boss_encounter::Intent::Refresh);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    let payload =
        oteryn_protocol_oteryn::boss_encounter::encode_bosstiary_list(3, &[], false, &[], 0, false)
            .map_err(|error| format!("{error:?}"))?;
    outbox.boss_encounter_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 4,
        status: oteryn_session::CommandStatus::Accepted,
        payload,
    });
    assert!(link.boss_encounter_panel().snapshot.is_some());
    Ok(())
}

impl crate::play::Outbox {
    pub(crate) fn screenshot_activity_handoff(&self) -> Result<(), String> {
        use oteryn_session::activity_scope::{
            ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
        };
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let mut successor = id;
        successor[15] = 2;
        let payload = encode_activity_scope(&ActivityScopeSnapshot {
            handoff: id,
            source_session: id,
            session: successor,
            scope: ActivityScope::Instance {
                world: id,
                instance: id,
            },
            scope_generation: 1,
            connection_generation: 1,
            lease_generation: 2,
            selection: vec![117],
            requires_ack: false,
        })
        .map_err(|e| format!("{e:?}"))?;
        self.push(vec![oteryn_session::SessionEvent::GatedSnapshotReplaced(
            oteryn_session::AppliedDelta {
                server_sequence: 3,
                base_revision: 0,
                new_revision: 1,
                value: oteryn_session::GatedSnapshot {
                    domain_id: 99,
                    snapshot_type: 1,
                    revision: 1,
                    payload,
                },
            },
        )]);
        Ok(())
    }
}

#[test]
fn ending_session_preserves_impact_record_earned_after_explicit_reset()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::analyser_transport::{AnalyserFact, AnalyserImpact};
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("settings.json");
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.select_character(Some([1; 16]));
    let (link, _commands, outbox) = crate::play::play_channel();
    outbox.analyser_initialize(true);
    let now = std::time::Instant::now();
    settings.prepare_screenshots(now, Some(&link));
    link.analyser_reset_high();
    outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(
                75u32.try_into()?,
            ))],
        },
    )]);
    settings.prepare_screenshots(now, Some(&link));
    assert_eq!(settings.preferences.impact_records, [0, 75]);
    outbox.logged_out();
    settings.prepare_screenshots(now, Some(&link));
    let mut reopened = SettingsState::new(Some(path));
    reopened.select_character(Some([1; 16]));
    assert_eq!(reopened.preferences.impact_records, [0, 75]);
    Ok(())
}

#[cfg(windows)]
#[test]
fn capture_directory_has_only_an_inheritable_owner_ace() -> std::io::Result<()> {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("screenshots");
    crate::play::screenshot::ensure_screenshot_directory(&path)?;
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"
            $ErrorActionPreference = 'Stop'
            $acl = Get-Acl -LiteralPath $env:CAPTURE_TEST_DIRECTORY
            $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
            $rules = $acl.GetAccessRules($true, $true, $sid.GetType())
            if (!$acl.AreAccessRulesProtected -or $rules.Count -ne 1) { exit 1 }
            $rule = $rules[0]
            if ($rule.IdentityReference -ne $sid -or $rule.IsInherited -or
                $rule.AccessControlType -ne 'Allow' -or [int]$rule.InheritanceFlags -ne 3 -or
                $rule.FileSystemRights -ne 'FullControl') { exit 2 }
        "#,
        ])
        .env("CAPTURE_TEST_DIRECTORY", path)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn personal_boss_defeat_wire_requests_screenshot() -> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let (link, _, outbox) = crate::play::play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .capture_observations
        .personal_boss_events = true;
    let mut settings = SettingsState::new(None);
    settings.preferences.screenshots.on_boss_defeat = true;
    settings.preferences.screenshots.directory = folder.path().to_string_lossy().into_owned();
    let now = std::time::Instant::now();
    assert!(
        settings
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .is_empty()
    );
    let value = oteryn_protocol_oteryn::zones_nologout::decode_server_message(b"\x2a\x04boss")
        .map_err(|error| format!("{error:?}"))?;
    outbox.push(vec![oteryn_session::SessionEvent::ServerMessage(
        oteryn_session::AppliedDelta {
            base_revision: 1,
            new_revision: 2,
            server_sequence: 2,
            value,
        },
    )]);
    assert_eq!(
        settings
            .process_screenshots(now, Some(&link), &mut Source(vec![]))?
            .len(),
        1
    );
    Ok(())
}

#[test]
fn each_personal_defeat_before_the_first_frame_requests_one_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let (link, _, outbox) = crate::play::play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .capture_observations
        .personal_boss_events = true;
    let mut settings = SettingsState::new(None);
    settings.preferences.screenshots.on_boss_defeat = true;
    settings.preferences.screenshots.directory = folder.path().to_string_lossy().into_owned();
    let value = oteryn_protocol_oteryn::zones_nologout::decode_server_message(b"\x2a\x04boss")
        .map_err(|error| format!("{error:?}"))?;
    for revision in [2, 8, 8] {
        outbox.push(vec![oteryn_session::SessionEvent::ServerMessage(
            oteryn_session::AppliedDelta {
                base_revision: revision - 1,
                new_revision: revision,
                server_sequence: revision,
                value: value.clone(),
            },
        )]);
    }
    let now = std::time::Instant::now();
    for expected in [1, 1, 0] {
        assert_eq!(
            settings
                .process_screenshots(now, Some(&link), &mut Source(vec![]))?
                .len(),
            expected
        );
    }
    Ok(())
}
