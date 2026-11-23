use crate::play::screenshot::CaptureAction;
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
use super::*;
use bevy::ui_widgets::ValueChange;
use oteryn_session::{
    AppliedDelta, SessionEvent,
    analyser_transport::{AnalyserFact, AnalyserItemCount},
};
#[test]
fn low_health_crossing_requests_one_capture() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = super::super::tests::app(Page::Screenshots, None)?;
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("screenshots.low-health"))
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
    for (seq, health, save) in [(1, 100, false), (2, 4, true), (3, 1, false), (4, 0, false)] {
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
            save
        );
        owner.settings.cancel_screenshot();
    }
    Ok(())
}
#[test]
fn master_switch_preserves_death_selection_and_suppresses_capture()
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
    assert!(owner.settings.preferences.screenshots.on_death);
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
fn tracked_loot_capture_uses_confirmed_drops_and_current_tracker()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = super::super::tests::app(Page::Screenshots, None)?;
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("screenshots.valuable-loot"))
        .observe(super::super::toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: true,
        is_final: true,
    });
    let (link, _, outbox) = crate::play::play_channel();
    outbox.bevy_hud_fixture(&[], None);
    link.analyser_open();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.apply(true);
    let now = std::time::Instant::now();
    for seq in 1..=2 {
        outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
            server_sequence: seq,
            base_revision: seq - 1,
            new_revision: seq,
            value: vec![AnalyserFact::KillLoot {
                race: None,
                gold: 0,
                corpse_items: vec![AnalyserItemCount {
                    item_type: 22_u32.try_into()?,
                    count: 1_u16.try_into()?,
                }],
            }],
        })]);
        assert_eq!(
            matches!(
                owner.settings.prepare_screenshots(now, Some(&link)),
                crate::play::screenshot::CaptureAction::Save
            ),
            seq == 2
        );
        assert!(link.analyser_select("loot-tracker"));
        link.analyser_track_item("22", true);
    }
    Ok(())
}
