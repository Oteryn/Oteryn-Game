use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::spell_action_bar::movement::MovementSpell;
use oteryn_session::{
    ChatDisposition, ChatIntent, ChatOutcome, ChatSpeechMode, CommandStatus, LogoutResult,
    SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn movement_spell_controls_clear_on_logout() {
    let (link, _incoming, outbox) = play_channel();
    outbox.movement_spells_initialize(true);
    assert!(link.chat_public_panel().speech_selected);
    outbox.logged_out();
    assert!(!link.chat_public_panel().speech_selected);
    assert!(!link.chat_public_panel().pending);
}

#[test]
fn ordinary_chat_reply_does_not_clear_pending_movement_spell() -> Result<(), Error> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.movement_spells_initialize(true);
    crate::play::feature_local_movement_spells::input::enqueue(
        &mut crate::play::mailbox::lock(&link.mailbox),
        &link.commands,
        MovementSpell::MagicRope,
    );
    assert_eq!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::MovementSpeech(ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: "exani tera".into(),
        })
    );
    let earlier_say = ChatIntent::Say {
        mode: ChatSpeechMode::Say,
        text: "Earlier ordinary speech".into(),
    };
    let outcome = ChatOutcome {
        command_id: 1,
        status: CommandStatus::Accepted,
        disposition: ChatDisposition::Rejected,
        wait_seconds: 0,
        result_server_sequence: 1,
    };
    outbox.chat_result(&earlier_say, &outcome);
    assert!(link.chat_public_panel().pending);
    assert_eq!(link.chat_public_panel().feedback, None);
    outbox.chat_public_result(
        &ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: "exani tera".into(),
        },
        &outcome,
    );
    assert!(!link.chat_public_panel().pending);
    assert_eq!(
        link.chat_public_panel().feedback.as_deref(),
        Some("The server rejected the chat command")
    );
    Ok(())
}

type PendingSpeech = (ChatIntent, tokio::sync::oneshot::Sender<()>);
struct Peer(
    Arc<Mutex<Vec<ChatIntent>>>,
    Option<tokio::sync::mpsc::UnboundedSender<PendingSpeech>>,
);

impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 7
    }
    async fn chat_public(&mut self, intent: &ChatIntent) -> Result<ChatOutcome, SessionError> {
        let command_id = {
            let mut commands = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            commands.push(intent.clone());
            u64::try_from(commands.len()).map_err(|_| SessionError::SessionUnusable)?
        };
        if let Some(requests) = &self.1 {
            let (complete, wait) = tokio::sync::oneshot::channel();
            requests
                .send((intent.clone(), complete))
                .map_err(|_| SessionError::SessionUnusable)?;
            wait.await.map_err(|_| SessionError::SessionUnusable)?;
        }
        Ok(ChatOutcome {
            command_id,
            status: CommandStatus::Accepted,
            disposition: ChatDisposition::Rejected,
            wait_seconds: 0,
            result_server_sequence: 1,
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
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

async fn wait_for(
    link: &PlayLink,
    predicate: impl Fn(&oteryn_client_ui::chat_public_channels::ChatPanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate(&link.chat_public_panel()) {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[test]
fn movement_spell_controls_reach_session_and_decode_refusal_without_public_rooms()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer(Arc::clone(&commands), None),
                    incoming,
                    outbox,
                ));
                wait_for(&link, |panel| panel.speech_selected).await?;
                assert!(!link.chat_public_panel().selected);
                for (spell, words) in [
                    (MovementSpell::LevitateUp, "exani hur \"up"),
                    (MovementSpell::LevitateDown, "exani hur \"down"),
                    (MovementSpell::MagicRope, "exani tera"),
                ] {
                    crate::play::feature_local_movement_spells::input::enqueue(
                        &mut crate::play::mailbox::lock(&link.mailbox),
                        &link.commands,
                        spell,
                    );
                    wait_for(&link, |panel| !panel.pending && panel.feedback.is_some()).await?;
                    assert_eq!(
                        commands
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .last(),
                        Some(&ChatIntent::Say {
                            mode: ChatSpeechMode::Say,
                            text: words.into(),
                        })
                    );
                }
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "movement spell client thread panicked")?
}

#[test]
fn movement_and_normal_speech_results_keep_their_pending_owner() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (requests, mut pending) = tokio::sync::mpsc::unbounded_channel();
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer(Arc::clone(&commands), Some(requests)),
                    incoming,
                    outbox,
                ));
                wait_for(&link, |panel| {
                    panel.speech_selected && link.chat_panel().selected
                })
                .await?;
                assert!(!link.chat_public_panel().selected);
                link.chat_text("ordinary speech".into());
                link.chat_request(oteryn_client_ui::chat::Mode::Say, "");
                let (ordinary, finish_ordinary) =
                    tokio::time::timeout(Duration::from_secs(3), pending.recv())
                        .await?
                        .ok_or("missing ordinary speech")?;
                assert_eq!(
                    ordinary,
                    ChatIntent::Say {
                        mode: ChatSpeechMode::Say,
                        text: "ordinary speech".into()
                    }
                );
                crate::play::feature_local_movement_spells::input::enqueue(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    MovementSpell::MagicRope,
                );
                assert!(link.chat_panel().pending);
                assert!(link.chat_public_panel().pending);
                finish_ordinary
                    .send(())
                    .map_err(|_| "ordinary speech waiter closed")?;
                let (movement, finish_movement) =
                    tokio::time::timeout(Duration::from_secs(3), pending.recv())
                        .await?
                        .ok_or("missing movement speech")?;
                assert_eq!(
                    movement,
                    ChatIntent::Say {
                        mode: ChatSpeechMode::Say,
                        text: "exani tera".into()
                    }
                );
                assert!(!link.chat_panel().pending);
                assert!(link.chat_panel().feedback.is_some());
                assert!(
                    link.chat_public_panel().pending,
                    "ordinary SAY must not acknowledge queued movement speech"
                );
                assert!(link.chat_public_panel().feedback.is_none());
                finish_movement
                    .send(())
                    .map_err(|_| "movement speech waiter closed")?;
                wait_for(&link, |panel| !panel.pending && panel.feedback.is_some()).await?;
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .as_slice(),
                    &[ordinary, movement]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "interleaved movement speech client thread panicked")?
}
