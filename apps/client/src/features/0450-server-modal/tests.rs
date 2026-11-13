use super::*;
use crate::combat_input::{CombatCommand, CombatReport};
use oteryn_client_runtime::{
    raw_feature::FeatureCommand,
    server_modal::{Dialog, Image, Reply},
};
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Default)]
pub(super) struct Peer {
    pub(super) answers: usize,
    terminal: bool,
    queries: Vec<Vec<u8>>,
    query_results: VecDeque<Result<GatedCommandResult, SessionError>>,
    after_answer_calls: Arc<AtomicUsize>,
}
impl Peer {
    fn record_after_answer_call(&self) {
        if self.terminal && self.answers > 0 {
            self.after_answer_calls.fetch_add(1, Ordering::SeqCst);
        }
    }
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 450
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        self.record_after_answer_call();
        assert_eq!(command.capability, 450);
        if command.kind == 736 {
            self.queries.push(command.payload.clone());
            if let Some(result) = self.query_results.pop_front() {
                return result;
            }
        }
        let reply = match command.kind {
            736 => Reply::Dialog(Dialog {
                id: 73,
                title: "Choose".into(),
                message: "Server-defined options".into(),
                choices: vec![(9, "Nine".into())],
                buttons: vec![(4, "Accept".into()), (8, "Cancel".into())],
                cancel_button: Some(8),
                image: Some(Image {
                    width: 1,
                    height: 1,
                    rgba: vec![255, 0, 0, 255],
                }),
            }),
            737 => {
                self.answers += 1;
                let answer =
                    Answer::decode(&command.payload).map_err(|_| SessionError::SessionUnusable)?;
                if self.terminal {
                    Reply::AnsweredAndLoggedOut(answer)
                } else {
                    Reply::Answered(answer)
                }
            }
            _ => return Err(SessionError::SessionUnusable),
        };
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply.encode().map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        self.record_after_answer_call();
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        self.record_after_answer_call();
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        self.record_after_answer_call();
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        self.record_after_answer_call();
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        self.record_after_answer_call();
        Ok(None)
    }
}
#[test]
fn server_modal_session_caller_delivers_inline_image_and_exact_answer_ack()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut peer = Peer::default();
            let mut driver = Driver::new(true);
            assert_eq!(driver.poll(&mut peer).await?, None);
            let dialog = driver
                .panel
                .active
                .as_ref()
                .ok_or("missing native descriptor")?;
            assert_eq!(
                dialog.image.as_ref().ok_or("image")?.rgba,
                vec![255, 0, 0, 255]
            );
            let answer = Answer {
                dialog_id: 73,
                button_id: 4,
                choice_id: Some(9),
            };
            let (tx, rx) = oneshot::channel();
            driver.sender.try_send((
                Answer {
                    choice_id: Some(10),
                    ..answer
                },
                tx,
            ))?;
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(!rx.await?);
            assert_eq!(peer.answers, 0);
            let (tx, rx) = oneshot::channel();
            driver.sender.try_send((answer, tx))?;
            assert_eq!(
                driver.poll(&mut peer).await?,
                Some(Completion::Answered(answer))
            );
            assert!(rx.await?);
            assert_eq!(peer.answers, 1);
            assert!(driver.panel.active.is_none());
            let sender = driver.sender.clone();
            drop(driver);
            assert!(sender.is_closed());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}

fn query_result(status: CommandStatus, payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    }
}

#[test]
fn server_modal_polls_again_after_an_empty_reply() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut peer = Peer {
                query_results: VecDeque::from([Ok(query_result(
                    CommandStatus::Accepted,
                    Reply::None.encode()?,
                ))]),
                ..Peer::default()
            };
            let mut driver = Driver::new(true);
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(driver.panel.active.is_none());
            driver.next_query = Instant::now();
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(driver.panel.active.is_some());
            assert_eq!(
                peer.queries,
                vec![
                    oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY.to_vec(),
                    oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY.to_vec(),
                ]
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}

#[test]
fn server_modal_retries_a_rejected_opt_in_with_the_legacy_query()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut peer = Peer {
                query_results: VecDeque::from([Ok(query_result(CommandStatus::Rejected, vec![]))]),
                ..Peer::default()
            };
            let mut driver = Driver::new(true);
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(!driver.panel.logged_out);
            driver.next_query = Instant::now();
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(driver.panel.active.is_some());
            assert_eq!(
                peer.queries,
                vec![
                    oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY.to_vec(),
                    vec![],
                ]
            );
            let answer = Answer {
                dialog_id: 73,
                button_id: 4,
                choice_id: Some(9),
            };
            let (tx, rx) = oneshot::channel();
            driver.sender.try_send((answer, tx))?;
            assert_eq!(
                driver.poll(&mut peer).await?,
                Some(Completion::Answered(answer))
            );
            assert!(rx.await?);
            assert!(!driver.panel.logged_out);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}

#[test]
fn server_modal_transport_loss_never_falls_back_or_logs_out()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut peer = Peer {
                query_results: VecDeque::from([Err(SessionError::SessionUnusable)]),
                ..Peer::default()
            };
            let mut driver = Driver::new(true);
            assert!(matches!(
                driver.poll(&mut peer).await,
                Err(SessionError::SessionUnusable)
            ));
            assert_eq!(peer.queries.len(), 1);
            assert_eq!(driver.panel.terminal_replies, ui::TerminalReplies::Probing);
            assert!(!driver.panel.logged_out);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}

#[test]
fn terminal_modal_ack_ends_the_session_before_queued_gameplay_or_idle_io() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let mut peer = Peer {
                        terminal: true,
                        ..Peer::default()
                    };
                    let after_answer_calls = Arc::clone(&peer.after_answer_calls);
                    let mut driver = Driver::new(true);
                    assert_eq!(driver.poll(&mut peer).await?, None);
                    let answer = Answer {
                        dialog_id: 73,
                        button_id: 4,
                        choice_id: Some(9),
                    };
                    let (tx, rx) = oneshot::channel();
                    driver.sender.try_send((answer, tx))?;
                    let (link, commands, outbox) = crate::play::play_channel();
                    link.request(crate::input::StepDir::North);
                    crate::play::session_loop::run_session_with_modal(
                        peer, commands, outbox, driver,
                    )
                    .await;
                    assert!(rx.await?);
                    assert_eq!(after_answer_calls.load(Ordering::SeqCst), 0);
                    let mut logged_out = false;
                    while let Some(event) = link.poll() {
                        match event {
                            crate::play::PlayEvent::LoggedOut => logged_out = true,
                            crate::play::PlayEvent::Ended(_) => {
                                return Err(
                                    "terminal modal returned to login as unavailable".into()
                                );
                            }
                            _ => {}
                        }
                    }
                    assert!(logged_out);
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "terminal modal test thread panicked".to_owned())?
}

/// Serves one dialog that asks for an amount and records the raw answer it receives.
#[derive(Default)]
pub(super) struct AmountPeer {
    pub(super) answer: Option<Vec<u8>>,
}
impl Stepper for AmountPeer {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 450 | 490)
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        use oteryn_client_runtime::server_modal::input::{
            DialogInput, InputAnswer, InputKind, InputValue, encode_dialog_with_input,
        };
        let payload = if command.kind == 736 {
            let dialog = Dialog {
                id: 31,
                title: "Amount".into(),
                message: "How many?".into(),
                choices: Vec::new(),
                buttons: vec![(1, "Ok".into()), (2, "Cancel".into())],
                cancel_button: Some(2),
                image: None,
            };
            let input = DialogInput {
                kind: InputKind::Amount,
                minimum: 1,
                maximum: 20,
                initial: Some(InputValue::Amount(5)),
            };
            encode_dialog_with_input(&dialog, Some(&input))
                .map_err(|_| SessionError::SessionUnusable)?
        } else {
            self.answer = Some(command.payload.clone());
            let sent = InputAnswer::decode(&command.payload)
                .map_err(|_| SessionError::SessionUnusable)?;
            Reply::Answered(sent.answer)
                .encode()
                .map_err(|_| SessionError::SessionUnusable)?
        };
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
        })
    }
    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}

#[test]
fn c0490_driver_sends_the_entered_value_and_the_remember_choice()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_modal::input::{InputAnswer, InputValue};
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut peer = AmountPeer::default();
            let mut driver = Driver::new(true).with_input(true);
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(driver.panel.active_input.is_some());
            *driver.entry.lock().map_err(|_| "entry")? =
                oteryn_client_ui::server_modal_input::Entry {
                    value: Some(InputValue::Amount(12)),
                    dont_show_again: true,
                };
            let answer = Answer {
                dialog_id: 31,
                button_id: 1,
                choice_id: None,
            };
            let (tx, rx) = oneshot::channel();
            driver.sender.try_send((answer, tx))?;
            assert_eq!(
                driver.poll(&mut peer).await?,
                Some(Completion::Answered(answer))
            );
            assert!(rx.await?);
            let sent = InputAnswer::decode(&peer.answer.ok_or("no answer")?)
                .map_err(|_| "answer wire")?;
            assert_eq!(sent.value, Some(InputValue::Amount(12)));
            assert!(sent.dont_show_again);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
