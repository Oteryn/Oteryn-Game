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
