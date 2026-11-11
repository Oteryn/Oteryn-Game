use crate::{
    combat_input::PlayCommand,
    play::{PlayView, mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    cyclopedia_map as wire,
    raw_feature::{CommandStatus, GatedCommandResult},
    tutorial_notices::TutorialNotice,
};
use oteryn_client_ui::cyclopedia_map::Intent;
use oteryn_session::{AppliedDelta, SessionEvent};
use std::time::{Duration, Instant};

fn discovery(sequence: u64) -> SessionEvent {
    SessionEvent::TutorialNotice(AppliedDelta {
        server_sequence: sequence,
        base_revision: sequence - 1,
        new_revision: sequence,
        value: TutorialNotice::StatusText(super::super::protocol::DISCOVERY_NOTICE.into()),
    })
}

#[test]
fn discovery_notice_reaches_banner_once_without_an_open_map() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let (link, _commands, outbox) = play_channel();
            let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
                .map_err(|e| e.to_string())?;
            outbox.push(vec![SessionEvent::TutorialNotice(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: TutorialNotice::StatusText("Another tutorial notice".into()),
            })]);
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(view.notification_text(false).is_empty());
            outbox.push(vec![discovery(2)]);
            assert!(!link.cyclopedia_map_panel().open);
            // A destination may replace the map before the next native tick. The
            // committed discovery belongs to the session and must survive that change.
            lock(&link.mailbox).cancel_activity_requests();
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                view.notification_text(false),
                format!("{}\r\n", super::super::protocol::DISCOVERY_NOTICE)
            );
            assert!(view.notification_text(true).is_empty());
            view.configure_notifications(false, true);
            assert!(view.notification_text(false).is_empty());
            view.configure_notifications(true, true);
            assert!(!view.notification_text(false).is_empty());
            assert!(
                view.game_notifications
                    .text_at(false, Instant::now() + Duration::from_secs(9))
                    .is_empty()
            );
            outbox.push(vec![discovery(2), discovery(1)]);
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(view.notification_text(false).is_empty());
            outbox.push(vec![discovery(3)]);
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(!view.notification_text(false).is_empty());
            outbox.push(vec![discovery(4)]);
            outbox.logged_out();
            let ended = link.cyclopedia_map_panel();
            assert!(ended.discovery_banner.is_none());
            assert_eq!(ended.discovery_notice_sequence, 0);
            let (next_link, _commands, _outbox) = play_channel();
            let mut next = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
                .map_err(|e| e.to_string())?;
            next.tick(&next_link).map_err(|e| format!("{e:?}"))?;
            assert!(next.notification_text(false).is_empty());
            Ok(())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "discovery banner scenario panicked")?
}

#[test]
fn reset_waits_for_authoritative_reply_and_snapshots_never_show_discovery_banner()
-> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let (link, mut commands, outbox) = play_channel();
            lock(&link.mailbox).properties.cyclopedia_map.selected = true;
            let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
                .map_err(|e| e.to_string())?;
            let completed = wire::MapResult {
                code: 0,
                revision: 8,
                discovered: true,
                found: 7,
                area_percent: 100,
                pool: 0,
                epoch: 3,
                improved: false,
                visible: Vec::new(),
            };
            for (intent, state) in [
                (Intent::Query, completed.clone()),
                (
                    Intent::Shuffle,
                    wire::MapResult {
                        revision: 9,
                        discovered: false,
                        found: 0,
                        area_percent: 0,
                        visible: vec![(100, 200, -7)],
                        ..completed.clone()
                    },
                ),
                (Intent::Query, completed.clone()),
            ] {
                let before = link.cyclopedia_map_panel();
                link.cyclopedia_map_request(intent);
                let PlayCommand::Feature(command) =
                    commands.try_recv().map_err(|e| e.to_string())?
                else {
                    return Err("expected map command".into());
                };
                if intent == Intent::Shuffle {
                    assert_eq!((command.capability, command.kind), (57, 73));
                    assert_eq!(
                        wire::decode_request(command.kind, &command.payload)
                            .map_err(|e| format!("{e:?}"))?
                            .operation,
                        2
                    );
                    assert_eq!(link.cyclopedia_map_panel().map, before.map);
                    assert!(!link.cyclopedia_map_panel().hide_pois);
                }
                outbox.properties_result(
                    &command,
                    &GatedCommandResult {
                        command_id: 1,
                        server_sequence: 1,
                        status: CommandStatus::Accepted,
                        payload: wire::encode_result(&state).map_err(|e| format!("{e:?}"))?,
                    },
                );
                let PlayCommand::Feature(speed) = commands.try_recv().map_err(|e| e.to_string())?
                else {
                    return Err("expected speed query".into());
                };
                assert_eq!(speed.kind, wire::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY);
                outbox.properties_result(
                    &speed,
                    &GatedCommandResult {
                        command_id: 2,
                        server_sequence: 2,
                        status: CommandStatus::Accepted,
                        payload: 0u16.to_le_bytes().to_vec(),
                    },
                );
                assert_eq!(link.cyclopedia_map_panel().map, Some(state));
                link.cyclopedia_map_open();
                view.tick(&link).map_err(|e| format!("{e:?}"))?;
                assert!(view.notification_text(false).is_empty());
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "discovery reset scenario panicked")?
}

fn snapshot_markers(sequence: u64) -> [SessionEvent; 2] {
    use oteryn_protocol_oteryn::{
        tutorial_notices::{STATE_DOMAIN_TUTORIAL_NOTICES, TUTORIAL_NOTICES_SNAPSHOT},
        zones_nologout::{SERVER_MESSAGES_EMPTY_SNAPSHOT, STATE_DOMAIN_SERVER_MESSAGES},
    };
    // Session emits one ordered marker per domain in the same validated snapshot.
    [
        (STATE_DOMAIN_TUTORIAL_NOTICES, TUTORIAL_NOTICES_SNAPSHOT),
        (STATE_DOMAIN_SERVER_MESSAGES, SERVER_MESSAGES_EMPTY_SNAPSHOT),
    ]
    .map(|(domain_id, payload)| {
        SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: sequence,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id,
                snapshot_type: 1,
                revision: 1,
                payload: payload.to_vec(),
            },
        })
    })
}

fn handoff_scenario(
    run: impl FnOnce() -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(run)
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "discovery handoff scenario panicked")?
}

#[test]
fn discovery_notice_accepts_lower_sequences_after_house_and_activity_snapshots()
-> Result<(), String> {
    handoff_scenario(|| {
        // House entry starts at 1; House exit and activity destinations start at 0.
        for baseline in [0, 1] {
            let (link, _commands, outbox) = play_channel();
            let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
                .map_err(|e| e.to_string())?;
            outbox.push(vec![discovery(100)]);
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(!view.notification_text(false).is_empty());
            assert!(
                view.game_notifications
                    .text_at(false, Instant::now() + Duration::from_secs(9))
                    .is_empty()
            );
            outbox.push(snapshot_markers(baseline).into());
            assert_eq!(
                link.cyclopedia_map_panel().discovery_notice_sequence,
                baseline
            );
            assert!(link.cyclopedia_map_panel().discovery_banner.is_none());
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(view.notification_text(false).is_empty());
            outbox.push(vec![discovery(baseline + 1)]);
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                view.notification_text(false),
                format!("{}\r\n", super::super::protocol::DISCOVERY_NOTICE)
            );
            assert_eq!(
                link.cyclopedia_map_panel().discovery_notice_sequence,
                baseline + 1
            );
        }
        Ok(())
    })
}

#[test]
fn discovery_handoff_preserves_queued_banner_and_reduces_mixed_batches_in_order()
-> Result<(), String> {
    handoff_scenario(|| {
        for baseline in [0, 1] {
            let (link, _commands, outbox) = play_channel();
            let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
                .map_err(|e| e.to_string())?;
            let mut events = vec![discovery(100)];
            events.extend(snapshot_markers(baseline));
            outbox.push(events);
            let panel = link.cyclopedia_map_panel();
            assert_eq!(panel.discovery_notice_sequence, baseline);
            assert_eq!(panel.discovery_banner, Some(100));
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert!(!view.notification_text(false).is_empty());
            assert!(
                view.game_notifications
                    .text_at(false, Instant::now() + Duration::from_secs(9))
                    .is_empty()
            );
            let mut events = vec![discovery(101)];
            events.extend(snapshot_markers(baseline));
            events.extend([discovery(baseline + 2), discovery(baseline + 1)]);
            outbox.push(events);
            let panel = link.cyclopedia_map_panel();
            assert_eq!(panel.discovery_notice_sequence, baseline + 2);
            assert_eq!(panel.discovery_banner, Some(baseline + 2));
            view.tick(&link).map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                view.notification_text(false),
                format!("{}\r\n", super::super::protocol::DISCOVERY_NOTICE)
            );
            assert!(link.cyclopedia_map_panel().discovery_banner.is_none());
        }
        Ok(())
    })
}

#[test]
fn ordinary_snapshot_keeps_discovery_replay_watermark() -> Result<(), String> {
    handoff_scenario(|| {
        let (link, _commands, outbox) = play_channel();
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
            .map_err(|e| e.to_string())?;
        outbox.push(vec![discovery(100)]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert!(!view.notification_text(false).is_empty());
        assert!(
            view.game_notifications
                .text_at(false, Instant::now() + Duration::from_secs(9))
                .is_empty()
        );
        let mut events = Vec::from(snapshot_markers(101));
        events.extend([discovery(100), discovery(99)]);
        outbox.push(events);
        assert_eq!(link.cyclopedia_map_panel().discovery_notice_sequence, 100);
        assert!(link.cyclopedia_map_panel().discovery_banner.is_none());
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert!(view.notification_text(false).is_empty());
        outbox.push(vec![discovery(102)]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert!(!view.notification_text(false).is_empty());
        Ok(())
    })
}

#[test]
fn discovery_sequence_can_be_reused_after_handoff_but_not_replayed_within_it() -> Result<(), String>
{
    handoff_scenario(|| {
        let (link, _commands, outbox) = play_channel();
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])
            .map_err(|e| e.to_string())?;
        outbox.push(vec![discovery(2)]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert!(!view.notification_text(false).is_empty());
        assert!(
            view.game_notifications
                .text_at(false, Instant::now() + Duration::from_secs(9))
                .is_empty()
        );
        let mut events = Vec::from(snapshot_markers(0));
        events.push(discovery(2));
        outbox.push(events);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            view.notification_text(false),
            format!("{}\r\n", super::super::protocol::DISCOVERY_NOTICE)
        );
        assert!(
            view.game_notifications
                .text_at(false, Instant::now() + Duration::from_secs(9))
                .is_empty()
        );
        outbox.push(vec![discovery(2), discovery(1)]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        assert!(view.notification_text(false).is_empty());
        assert_eq!(link.cyclopedia_map_panel().discovery_notice_sequence, 2);
        Ok(())
    })
}
