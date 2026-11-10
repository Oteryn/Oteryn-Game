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
