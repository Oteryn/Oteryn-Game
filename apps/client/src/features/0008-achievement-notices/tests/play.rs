use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::achievement_notices::{
    AchievementEarned, AchievementNoticesPanel, AchievementWatermark, notices_panel_text,
};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    AppliedDelta, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    selected: bool,
    incoming: UnboundedReceiver<Vec<SessionEvent>>,
    queued: Vec<SessionEvent>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        self.selected && capability == 8
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (self.selected && domain == 13).then(Vec::new)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        match tokio::time::timeout(duration, self.incoming.recv()).await {
            Ok(Some(events)) => {
                self.queued.extend(events);
                Ok(())
            }
            Ok(None) => Err(SessionError::SessionUnusable),
            Err(_) => Ok(()),
        }
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.queued)
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}
async fn ready(
    link: &PlayLink,
    predicate: impl Fn(&AchievementNoticesPanel) -> bool,
) -> Result<AchievementNoticesPanel, Error> {
    Ok(tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let panel = link.achievement_notices_panel();
            if predicate(&panel) {
                break panel;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?)
}

#[test]
fn achievement_notices_play_input_live_push_reopen_and_session_cleanup() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                for (selected, disconnect) in [(false, false), (true, false), (true, true)] {
                    let (sender, incoming) = unbounded_channel();
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            selected,
                            incoming,
                            queued: vec![],
                        },
                        commands,
                        outbox,
                    ));
                    sender.send(vec![])?;
                    if selected {
                        ready(&link, |p| p.watermark.is_some()).await?;
                    }
                    let key = NormalizedInputEvent::Key {
                        code: KeyCode::new(16)?,
                        state: ButtonState::Pressed,
                        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                        repeat: false,
                    };
                    assert!(link.achievement_notices_input(std::slice::from_ref(&key)));
                    if !selected {
                        assert!(
                            notices_panel_text(&link.achievement_notices_panel())
                                .contains("unavailable")
                        );
                        link.request_logout();
                        task.await?;
                        assert_eq!(
                            link.achievement_notices_panel(),
                            AchievementNoticesPanel::default()
                        );
                        continue;
                    }
                    assert!(
                        notices_panel_text(&link.achievement_notices_panel())
                            .contains("Achievements: 0")
                    );
                    for n in 1..=2 {
                        sender.send(vec![SessionEvent::AchievementEarned(AppliedDelta {
                            server_sequence: 40 + u64::from(n),
                            base_revision: u64::from(n - 1),
                            new_revision: u64::from(n),
                            value: AchievementEarned {
                                key: format!("oteryn:achievement/award-{n}"),
                                name: format!("Award {n}"),
                                watermark: AchievementWatermark {
                                    fact_count: n,
                                    total_points: n * 5,
                                },
                            },
                        })])?;
                    }
                    let panel = ready(&link, |p| p.recent.len() == 2).await?;
                    assert!(panel.open);
                    let text = notices_panel_text(&panel);
                    assert!(text.find("Award 1") < text.find("Award 2"));
                    assert!(text.contains("Total points: 10"));
                    crate::play::mailbox::lock(&link.mailbox)
                        .achievement_notices
                        .open = false;
                    assert!(link.achievement_notices_input(&[key]));
                    assert_eq!(link.achievement_notices_panel().recent, panel.recent);
                    if disconnect {
                        drop(sender);
                    } else {
                        link.request_logout();
                    }
                    task.await?;
                    assert_eq!(
                        link.achievement_notices_panel(),
                        AchievementNoticesPanel::default()
                    );
                }
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "achievement scenario panicked")?
}

#[test]
fn nx6_wayfarer_server_payload_reaches_native_notice_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                // Domain 13/type 1 payload for the actual catalogue key, name and
                // three points. Decode the server bytes before entering play.
                let earned = oteryn_session::achievement_notices::AchievementEarned {
                    key: "oteryn:achievement/wayfarer".into(),
                    name: "Wayfarer".into(),
                    watermark: AchievementWatermark {
                        fact_count: 1,
                        total_points: 3,
                    },
                };
                let payload =
                    b"\x0a\x1boteryn:achievement/wayfarer\x12\x08Wayfarer\x18\x01\x20\x03";
                let decoded =
                    oteryn_session::achievement_notices::decode_achievement_earned(payload)
                        .map_err(|e| format!("{e:?}"))?;
                let (sender, incoming) = unbounded_channel();
                let (link, commands, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        selected: true,
                        incoming,
                        queued: vec![],
                    },
                    commands,
                    outbox,
                ));
                sender.send(vec![])?;
                ready(&link, |p| p.watermark.is_some()).await?;
                let key = NormalizedInputEvent::Key {
                    code: KeyCode::new(16)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                    repeat: false,
                };
                assert!(link.achievement_notices_input(&[key]));
                sender.send(vec![SessionEvent::AchievementEarned(AppliedDelta {
                    server_sequence: 41,
                    base_revision: 0,
                    new_revision: 1,
                    value: decoded,
                })])?;
                let panel = ready(&link, |p| p.recent.len() == 1).await?;
                assert!(panel.open);
                assert_eq!(panel.recent.front(), Some(&earned));
                let text = notices_panel_text(&panel);
                assert!(text.contains("Wayfarer"));
                assert!(text.contains("Total points: 3"));
                link.request_logout();
                task.await?;
                assert_eq!(
                    link.achievement_notices_panel(),
                    AchievementNoticesPanel::default()
                );
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Wayfarer play thread panicked")?
}

#[test]
fn recovery_snapshot_replaces_achievement_totals_without_replaying_notices() {
    let (_sender, incoming) = unbounded_channel();
    let mut peer = Peer {
        selected: true,
        incoming,
        queued: vec![],
    };
    let (link, _commands, outbox) = play_channel();
    outbox.achievement_notices_publish(&mut peer);
    let expected = AchievementWatermark {
        fact_count: 3,
        total_points: 15,
    };
    peer.queued
        .push(SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 42,
            base_revision: 0,
            new_revision: 3,
            value: oteryn_session::GatedSnapshot {
                domain_id: 13,
                snapshot_type: 1,
                revision: 3,
                payload: oteryn_session::achievement_notices::encode_achievement_notices_snapshot(
                    expected,
                ),
            },
        }));
    outbox.achievement_notices_publish(&mut peer);
    let panel = link.achievement_notices_panel();
    assert_eq!(panel.watermark, Some(expected));
    assert!(panel.recent.is_empty());
    #[cfg(feature = "bevy-shell")]
    assert!(
        crate::play::mailbox::lock(&outbox.0)
            .achievement_banners
            .is_empty()
    );
}

#[test]
fn recovery_totals_follow_event_order_with_live_achievement_notices() {
    let (_sender, incoming) = unbounded_channel();
    let mut peer = Peer {
        selected: true,
        incoming,
        queued: vec![],
    };
    let (link, _commands, outbox) = play_channel();
    outbox.achievement_notices_publish(&mut peer);
    let earned = |count| {
        SessionEvent::AchievementEarned(AppliedDelta {
            server_sequence: u64::from(count),
            base_revision: 0,
            new_revision: u64::from(count),
            value: AchievementEarned {
                key: format!("award-{count}"),
                name: format!("Award {count}"),
                watermark: AchievementWatermark {
                    fact_count: count,
                    total_points: count * 5,
                },
            },
        })
    };
    peer.queued = vec![
        earned(1),
        SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 3,
            base_revision: 1,
            new_revision: 3,
            value: oteryn_session::GatedSnapshot {
                domain_id: 13,
                snapshot_type: 1,
                revision: 3,
                payload: oteryn_session::achievement_notices::encode_achievement_notices_snapshot(
                    AchievementWatermark {
                        fact_count: 3,
                        total_points: 15,
                    },
                ),
            },
        }),
    ];
    outbox.achievement_notices_publish(&mut peer);
    assert_eq!(
        link.achievement_notices_panel().watermark,
        Some(AchievementWatermark {
            fact_count: 3,
            total_points: 15
        })
    );
    assert_eq!(link.achievement_notices_panel().recent.len(), 1);
    peer.queued = vec![earned(4)];
    outbox.achievement_notices_publish(&mut peer);
    assert_eq!(
        link.achievement_notices_panel().watermark,
        Some(AchievementWatermark {
            fact_count: 4,
            total_points: 20
        })
    );
    assert_eq!(link.achievement_notices_panel().recent.len(), 2);
}
