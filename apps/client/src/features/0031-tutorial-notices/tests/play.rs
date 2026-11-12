use crate::play::{mailbox::lock, play_channel};
use oteryn_client_runtime::tutorial_notices::{ServerSaveCountdown, TutorialNotice};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{AppliedDelta, SessionEvent};
#[test]
fn pushed_notices_are_reachable_from_play_and_refresh_while_open()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    outbox.notices_initialize(true, true);
    let delta = |value| AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value,
    };
    outbox.push(vec![SessionEvent::TutorialNotice(delta(
        TutorialNotice::Hint(2),
    ))]);
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(17)?,
        state: ButtonState::Pressed,
        repeat: false,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
    };
    assert!(link.notices_input(&[event]));
    assert!(link.notices_panel().open);
    outbox.push(vec![SessionEvent::ServerSaveCountdown(AppliedDelta {
        server_sequence: 2,
        base_revision: 0,
        new_revision: 1,
        value: ServerSaveCountdown {
            ending_epoch: 4,
            minutes_remaining: 5,
        },
    })]);
    let text = oteryn_client_ui::tutorial_notices::panel_text(&link.notices_panel());
    assert!(text.contains("vocation"));
    assert!(text.contains("5 minutes"));
    lock(&link.mailbox).notices.open = false;
    assert!(!link.notices_panel().open);
    assert_eq!(link.notices_panel().tutorial.len(), 1);
    let (fresh, _, _) = play_channel();
    assert!(fresh.notices_panel().tutorial.is_empty());
    assert_eq!(fresh.notices_panel().save_minutes, None);
    Ok(())
}

/// Synthetic session driver exercises the normal idle push path, without a window or GPU.
struct NoticePeer {
    incoming: tokio::sync::mpsc::UnboundedReceiver<SessionEvent>,
    queued: Vec<SessionEvent>,
}
impl crate::play::Stepper for NoticePeer {
    async fn combat(
        &mut self,
        _command: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 31 | 75)
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.queued)
    }
    async fn serve(
        &mut self,
        _duration: std::time::Duration,
    ) -> Result<(), oteryn_session::SessionError> {
        match self.incoming.recv().await {
            Some(event) => {
                self.queued.push(event);
                Ok(())
            }
            None => Err(oteryn_session::SessionError::SessionUnusable),
        }
    }
    async fn step(
        &mut self,
        _direction: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
}
#[test]
fn synthetic_idle_session_publishes_both_notices_to_play()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (sender, incoming) = tokio::sync::mpsc::unbounded_channel();
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(crate::play::run_session(
                        NoticePeer {
                            incoming,
                            queued: Vec::new(),
                        },
                        commands,
                        outbox,
                    ));
                    sender.send(SessionEvent::TutorialNotice(AppliedDelta {
                        server_sequence: 1,
                        base_revision: 0,
                        new_revision: 1,
                        value: TutorialNotice::StatusText("Welcome to Oteryn!".into()),
                    }))?;
                    sender.send(SessionEvent::ServerSaveCountdown(AppliedDelta {
                        server_sequence: 2,
                        base_revision: 0,
                        new_revision: 1,
                        value: ServerSaveCountdown {
                            ending_epoch: 8,
                            minutes_remaining: 2,
                        },
                    }))?;
                    tokio::time::timeout(std::time::Duration::from_secs(2), async {
                        while link.notices_panel().save_minutes != Some(2) {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    let panel = link.notices_panel();
                    assert!(panel.tutorial_selected && panel.save_selected);
                    assert_eq!(
                        panel.tutorial.front().map(String::as_str),
                        Some("Welcome to Oteryn!")
                    );
                    drop(sender);
                    task.await?;
                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "notice scenario thread failed")?
}
