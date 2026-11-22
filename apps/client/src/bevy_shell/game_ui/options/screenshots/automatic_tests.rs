use super::*;
use crate::play::screenshot::CaptureAction;
use bevy::ui_widgets::ValueChange;

#[test]
fn impact_records_and_incoming_player_hits_drive_independent_capture_choices()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    use oteryn_session::analyser_transport::{AnalyserDamageSource, AnalyserFact, AnalyserImpact};
    let n = 20_u32.try_into()?;
    let damage = AnalyserFact::Impact(AnalyserImpact::Damage {
        value: n,
        element: DamageElement::Physical,
    });
    let heal = AnalyserFact::Impact(AnalyserImpact::Healing(n));
    let pvp = AnalyserFact::DamageInput {
        value: n,
        element: DamageElement::Physical,
        source: AnalyserDamageSource::Player,
    };
    for (enabled, id) in [
        "screenshots.damage_record",
        "screenshots.healing_record",
        "screenshots.pvp_attack",
    ]
    .into_iter()
    .enumerate()
    {
        let mut app = super::super::tests::app(Page::Screenshots, None)?;
        let checkbox = app
            .world_mut()
            .spawn(Control::Check(id))
            .observe(super::super::toggle)
            .id();
        app.world_mut().trigger(ValueChange {
            source: checkbox,
            value: true,
            is_final: true,
        });
        let (link, _, outbox) = crate::play::play_channel();
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.apply(true);
        let now = std::time::Instant::now();
        owner.settings.prepare_screenshots(now, Some(&link));
        for (i, fact) in [
            damage.clone(),
            heal.clone(),
            pvp.clone(),
            damage.clone(),
            heal.clone(),
            pvp.clone(),
        ]
        .into_iter()
        .enumerate()
        {
            let seq = i as u64 + 1;
            outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
                oteryn_session::AppliedDelta {
                    server_sequence: seq,
                    base_revision: seq - 1,
                    new_revision: seq,
                    value: vec![fact],
                },
            )]);
            assert_eq!(
                matches!(
                    owner.settings.prepare_screenshots(now, Some(&link)),
                    CaptureAction::Save
                ),
                i == enabled,
                "{id} fact {i}"
            );
            owner.settings.cancel_screenshot();
        }
    }
    Ok(())
}
#[test]
fn low_health_capture_fires_on_crossing_and_rearms_after_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = super::super::tests::app(Page::Screenshots, None)?;
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("screenshots.low_health"))
        .observe(super::super::toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: true,
        is_final: true,
    });
    let (link, _, outbox) = crate::play::play_channel();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.apply(true);
    for (i, (health, expected)) in [(100, false), (4, true), (3, false), (100, false), (4, true)]
        .into_iter()
        .enumerate()
    {
        let seq = i as u64 + 1;
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: seq,
                base_revision: seq - 1,
                new_revision: seq,
                value: oteryn_session::ActorVitals {
                    health,
                    max_health: 100,
                    ..default()
                },
            },
        )]);
        assert_eq!(
            matches!(
                owner
                    .settings
                    .prepare_screenshots(std::time::Instant::now(), Some(&link)),
                CaptureAction::Save
            ),
            expected
        );
        owner.settings.cancel_screenshot();
    }
    Ok(())
}
#[test]
fn disabling_automatic_capture_suppresses_an_enabled_death_trigger()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = super::super::tests::app(Page::Screenshots, None)?;
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.on_death = true;
        owner.cancel();
        owner.open_graphics();
    }
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("screenshots.automatic"))
        .observe(super::super::toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: false,
        is_final: true,
    });
    let (link, _, outbox) = crate::play::play_channel();
    let now = std::time::Instant::now();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    assert!(owner.settings.preferences.screenshots.on_death);
    owner.apply(true);
    for (seq, health) in [(1, 100), (2, 0)] {
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: seq,
                base_revision: seq - 1,
                new_revision: seq,
                value: oteryn_session::ActorVitals {
                    health,
                    max_health: 100,
                    ..default()
                },
            },
        )]);
        assert!(matches!(
            owner.settings.prepare_screenshots(now, Some(&link)),
            CaptureAction::Idle
        ));
    }
    Ok(())
}

#[test]
fn unchecking_death_capture_and_applying_prevents_a_death_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = super::super::tests::app(Page::Screenshots, None)?;
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.on_death = true;
        owner.cancel();
        owner.open_graphics();
    }
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("screenshots.death"))
        .observe(super::super::toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: false,
        is_final: true,
    });
    let (link, _, outbox) = crate::play::play_channel();
    let now = std::time::Instant::now();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    assert!(owner.settings.preferences.screenshots.on_death);
    owner.apply(true);
    for (seq, health) in [(1, 100), (2, 0)] {
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: seq,
                base_revision: seq - 1,
                new_revision: seq,
                value: oteryn_session::ActorVitals {
                    health,
                    max_health: 100,
                    ..default()
                },
            },
        )]);
        assert!(matches!(
            owner.settings.prepare_screenshots(now, Some(&link)),
            CaptureAction::Idle
        ));
    }
    Ok(())
}

#[test]
fn character_impact_records_survive_restart_and_master_cancels_queued_automatic_captures()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::analyser_transport::{AnalyserFact, AnalyserImpact};
    let root = tempfile::tempdir()?;
    let base = root.path().join("prefs.json");
    for (character, healing, expected) in [
        (1, 100_u32, true),
        (1, 80, false),
        (2, 80, true),
        (1, 101, true),
    ] {
        let mut state = crate::play::SettingsState::new(Some(base.clone()));
        state.select_character(Some([character; 16]));
        let mut preferences = state.preferences.clone();
        preferences.screenshots.healing_record = true;
        preferences.screenshots.automatic_disabled = false;
        state.apply(preferences);
        let (link, _commands, outbox) = crate::play::play_channel();
        let now = std::time::Instant::now();
        state.prepare_screenshots(now, Some(&link));
        outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
            oteryn_session::AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(
                    healing.try_into()?,
                ))],
            },
        )]);
        assert_eq!(
            matches!(
                state.prepare_screenshots(now, Some(&link)),
                CaptureAction::Save
            ),
            expected
        );
        if character == 1 && healing == 80 {
            outbox.bevy_hud_fixture(&[], None);
            link.analyser_open();
            assert!(link.analyser_select("impact"));
            link.analyser_reset();
            state.prepare_screenshots(now, Some(&link));
            outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
                oteryn_session::AppliedDelta {
                    server_sequence: 2,
                    base_revision: 1,
                    new_revision: 2,
                    value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(
                        1_u32.try_into()?,
                    ))],
                },
            )]);
            assert!(matches!(
                state.prepare_screenshots(now, Some(&link)),
                CaptureAction::Save
            ));
        }
        let mut preferences = state.preferences.clone();
        preferences.screenshots.automatic_disabled = true;
        state.apply(preferences);
        assert!(matches!(
            state.prepare_screenshots(now, Some(&link)),
            CaptureAction::Idle
        ));
        state.request_screenshot();
        assert!(matches!(
            state.prepare_screenshots(now, Some(&link)),
            CaptureAction::Save
        ));
    }
    Ok(())
}

#[test]
fn burst_of_impact_records_saves_the_last_record_for_the_outgoing_character()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::analyser_transport::{AnalyserFact, AnalyserImpact};
    let root = tempfile::tempdir()?;
    let base = root.path().join("prefs.json");
    let mut state = crate::play::SettingsState::new(Some(base.clone()));
    state.select_character(Some([1; 16]));
    bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    state.use_background_preferences();
    let (link, _commands, outbox) = crate::play::play_channel();
    let now = std::time::Instant::now();
    for value in 1_u32..=12 {
        outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
            oteryn_session::AppliedDelta {
                server_sequence: value.into(),
                base_revision: (value - 1).into(),
                new_revision: value.into(),
                value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(
                    value.try_into()?,
                ))],
            },
        )]);
        state.prepare_screenshots(now, Some(&link));
    }
    state.select_character(Some([2; 16]));
    let deadline = now + std::time::Duration::from_secs(10);
    while state.local_preference_writes_pending() {
        state.poll_background_save();
        if std::time::Instant::now() >= deadline {
            return Err("character preference writer stalled".into());
        }
        std::thread::yield_now();
    }
    assert_eq!(state.preferences.impact_records, [0, 0]);
    let mut reopened = crate::play::SettingsState::new(Some(base));
    reopened.select_character(Some([1; 16]));
    assert_eq!(reopened.preferences.impact_records, [0, 12]);
    Ok(())
}

#[test]
fn failed_background_impact_reset_retries_without_another_hit()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let base = root.path().join("prefs.json");
    let identity = [1; 16];
    let mut state = crate::play::SettingsState::new(Some(base.clone()));
    state.select_character(Some(identity));
    let (link, _rx, outbox) = crate::play::play_channel();
    outbox.bevy_hud_fixture(&[], None);
    let now = std::time::Instant::now();
    outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: vec![oteryn_session::analyser_transport::AnalyserFact::Impact(
                oteryn_session::analyser_transport::AnalyserImpact::Healing(100u32.try_into()?),
            )],
        },
    )]);
    state.prepare_screenshots(now, Some(&link));
    assert_eq!(state.preferences.impact_records, [0, 100]);
    let file = crate::play::PreferenceStore::character_path(&base, identity);
    let saved = std::fs::read(&file)?;
    std::fs::write(&file, "invalid")?;
    bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    state.use_background_preferences();
    link.analyser_open();
    assert!(link.analyser_select("impact"));
    link.analyser_reset();
    state.prepare_screenshots(now, Some(&link));
    let deadline = now + std::time::Duration::from_secs(10);
    loop {
        if state.poll_background_save().error.is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            return Err("expected write failure".into());
        }
        std::thread::yield_now();
    }
    std::fs::write(&file, saved)?;
    state.prepare_screenshots(now, Some(&link));
    while state.local_preference_writes_pending() {
        state.poll_background_save();
        if std::time::Instant::now() > deadline {
            return Err("retry stalled".into());
        }
        std::thread::yield_now();
    }
    let mut reopened = crate::play::SettingsState::new(Some(base));
    reopened.select_character(Some(identity));
    assert_eq!(reopened.preferences.impact_records, [0, 0]);
    Ok(())
}
