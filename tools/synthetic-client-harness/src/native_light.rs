//! C26: production task, mailbox, view and scene ambient light projection.
use oteryn_client::play::{PlayView, play_channel, run_session};
use oteryn_renderer::TileCoord;
use oteryn_session::{AppliedDelta, SessionEvent, world_clock::WorldClock};
#[test]
fn native_clock_update_reaches_status_and_scene()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let initial = oteryn_session::world_map::WorldMapViewSnapshot {
                        header: oteryn_session::world_map::MapViewHeader {
                            content_generation: [1; 32],
                            bundle_digest: [2; 32],
                            reset_epoch: 1,
                            origin: oteryn_session::ActorPosition {
                                x: 0,
                                y: 0,
                                floor: 0,
                            },
                            first_visible_floor: 0,
                        },
                        tiles: vec![],
                    };
                    let (link, commands, outbox) = play_channel();
                    let (send, incoming) = tokio::sync::mpsc::unbounded_channel();
                    let task = tokio::spawn(run_session(
                        super::native_map::MapSession {
                            initial,
                            incoming,
                            pending: vec![],
                            appearances: vec![],
                        },
                        commands,
                        outbox,
                    ));
                    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
                    let original = view.scene().sprites().lighting.ambient;
                    for (revision, minute) in [(1, 1439), (2, 0)] {
                        send.send(SessionEvent::WorldClock(AppliedDelta {
                            server_sequence: revision,
                            base_revision: revision - 1,
                            new_revision: revision,
                            value: WorldClock {
                                tibian_minute: minute,
                                light_level: 64,
                                light_color: 215,
                            },
                        }))?;
                        let expected = if minute == 0 { "00:00" } else { "23:59" };
                        tokio::time::timeout(std::time::Duration::from_secs(5), async {
                            loop {
                                view.tick(&link).map_err(|e| format!("{e:?}"))?;
                                if view.world_clock_text().as_deref() == Some(expected) {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            Ok::<_, String>(())
                        })
                        .await??;
                        assert!(view.hud_status().contains(expected));
                        assert_ne!(view.scene().sprites().lighting.ambient, original);
                    }
                    task.abort();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "native light scenario thread failed")?
}
