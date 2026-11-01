use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::loyalty_skills::{LoyaltySkillsPanel, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::loyalty_skills::{LoyaltySkills, SkillLevel};
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::Notify;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Reply {
    status: CommandStatus,
    payload: Vec<u8>,
    release: Option<Arc<Notify>>,
}

impl Reply {
    fn new(status: CommandStatus, payload: Vec<u8>) -> Self {
        Self {
            status,
            payload,
            release: None,
        }
    }
}

struct Peer {
    selected: bool,
    replies: VecDeque<Reply>,
    commands: Arc<Mutex<Vec<Vec<u8>>>>,
    served: Arc<Notify>,
}

impl Peer {
    fn new(selected: bool, replies: impl IntoIterator<Item = Reply>) -> Self {
        Self {
            selected,
            replies: replies.into_iter().collect(),
            commands: Arc::default(),
            served: Arc::default(),
        }
    }
}

impl Stepper for Peer {
    fn loyalty_skills_selected(&self) -> bool {
        self.selected
    }

    async fn loyalty_skills(&mut self, payload: &[u8]) -> Result<GatedCommandResult, SessionError> {
        // Record even an unexpected request so capability-selection failures cannot be
        // hidden by the resulting session error and mailbox reset.
        let sequence = {
            let mut commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
            commands.push(payload.to_vec());
            u64::try_from(commands.len()).map_err(|_| SessionError::SessionUnusable)?
        };
        assert!(payload.is_empty());
        let Some(reply) = self.replies.pop_front() else {
            return Err(SessionError::SessionUnusable);
        };
        if let Some(release) = reply.release {
            release.notified().await;
        }
        Ok(GatedCommandResult {
            command_id: sequence,
            status: reply.status,
            server_sequence: sequence,
            payload: reply.payload,
        })
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.served.notify_one();
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
        Ok(Some(LogoutResult::Accepted))
    }
}

async fn ready(
    link: &PlayLink,
    predicate: impl Fn(&LoyaltySkillsPanel) -> bool,
) -> Result<LoyaltySkillsPanel, Error> {
    Ok(tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.loyalty_skills_panel();
            if predicate(&panel) {
                return panel;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?)
}

fn scenario(case: impl FnOnce() -> Result<(), Error> + Send + 'static) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(case)?
        .join()
        .map_err(|_| "Loyalty skills UI test thread panicked")?
}

fn zero_snapshot() -> LoyaltySkills {
    let mut skills = [SkillLevel {
        base: 100,
        with_loyalty: 100,
    }; 8];
    skills[0] = SkillLevel {
        base: 30,
        with_loyalty: 30,
    };
    LoyaltySkills {
        points: 0,
        bonus_percent: 0,
        title: None,
        skills,
    }
}

fn boosted_snapshot() -> LoyaltySkills {
    let mut snapshot = zero_snapshot();
    snapshot.points = 3_600;
    snapshot.bonus_percent = 50;
    snapshot.title = Some("Keeper of Tibia".into());
    for (skill, with_loyalty) in snapshot
        .skills
        .iter_mut()
        .zip([31, 102, 102, 102, 102, 104, 104, 104])
    {
        skill.with_loyalty = with_loyalty;
    }
    snapshot
}

#[test]
fn loyalty_skills_native_input_refresh_and_replay_use_the_queue_and_clear_on_logout()
-> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let zero = zero_snapshot();
            let boosted = boosted_snapshot();
            let release = Arc::new(Notify::new());
            let peer = Peer::new(
                true,
                [
                    Reply {
                        status: CommandStatus::Accepted,
                        payload: zero.encode().ok_or("zero projection")?,
                        release: Some(Arc::clone(&release)),
                    },
                    Reply::new(
                        CommandStatus::Accepted,
                        boosted.encode().ok_or("boosted projection")?,
                    ),
                    Reply::new(
                        CommandStatus::DuplicateReplay,
                        boosted.encode().ok_or("replay")?,
                    ),
                ],
            );
            let commands = Arc::clone(&peer.commands);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, |panel| panel.selected).await?;
            let open = NormalizedInputEvent::Key {
                code: KeyCode::new(22)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control),
                repeat: false,
            };
            assert!(link.loyalty_skills_input(&[open]));
            assert!(link.loyalty_skills_panel().pending);
            // A second Refresh while the first answer is blocked must not create a
            // duplicate command; the one confirmed reply releases the pending state.
            link.loyalty_skills_request();
            release.notify_one();
            let initial = ready(&link, |panel| panel.snapshot.is_some() && !panel.pending).await?;
            assert!(initial.open);
            assert_eq!(initial.snapshot.as_ref(), Some(&zero));
            assert!(panel_text(&initial).contains("0% skill-point bonus"));
            assert!(panel_text(&initial).contains("No loyalty title"));

            link.loyalty_skills_request();
            let refreshed = ready(&link, |panel| !panel.pending).await?;
            assert_eq!(refreshed.snapshot.as_ref(), Some(&boosted));
            let text = panel_text(&refreshed);
            assert!(text.contains("50% skill-point bonus"));
            assert!(text.contains("Keeper of Tibia"));
            assert!(text.contains("Magic: 30 / 31"));
            assert!(text.contains("Distance: 100 / 104"));

            link.loyalty_skills_request();
            let replayed = ready(&link, |panel| !panel.pending).await?;
            assert_eq!(replayed.snapshot.as_ref(), Some(&boosted));
            assert!(replayed.error.is_none());
            assert_eq!(
                *commands.lock().unwrap_or_else(PoisonError::into_inner),
                vec![Vec::<u8>::new(), vec![], vec![]]
            );
            link.request_logout();
            task.await?;
            assert_eq!(link.loyalty_skills_panel(), LoyaltySkillsPanel::default());
            link.loyalty_skills_request();
            assert_eq!(link.loyalty_skills_panel(), LoyaltySkillsPanel::default());
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn loyalty_skills_refusal_and_malformed_reply_keep_confirmed_values_and_allow_retry()
-> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let confirmed = boosted_snapshot();
            let mut refreshed = confirmed.clone();
            refreshed.points = 7_000;
            refreshed.title = Some("Enlightened of Tibia".into());
            let mut truncated = refreshed.encode().ok_or("malformed fixture")?;
            truncated.pop();
            let peer = Peer::new(
                true,
                [
                    Reply::new(
                        CommandStatus::Accepted,
                        confirmed.encode().ok_or("initial")?,
                    ),
                    // A syntactically valid payload accompanying a refusal is not an
                    // authoritative replacement for the last accepted snapshot.
                    Reply::new(
                        CommandStatus::Rejected,
                        refreshed.encode().ok_or("refused")?,
                    ),
                    Reply::new(CommandStatus::Accepted, truncated),
                    Reply::new(CommandStatus::Accepted, refreshed.encode().ok_or("retry")?),
                ],
            );
            let commands = Arc::clone(&peer.commands);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, |panel| panel.selected).await?;
            link.loyalty_skills_request();
            ready(&link, |panel| panel.snapshot.is_some() && !panel.pending).await?;
            for _ in 0..2 {
                link.loyalty_skills_request();
                let refused = ready(&link, |panel| panel.error.is_some() && !panel.pending).await?;
                assert_eq!(refused.snapshot.as_ref(), Some(&confirmed));
                assert!(refused.selected);
            }
            link.loyalty_skills_request();
            let recovered = ready(&link, |panel| !panel.pending).await?;
            assert_eq!(recovered.snapshot.as_ref(), Some(&refreshed));
            assert!(recovered.error.is_none());
            assert!(panel_text(&recovered).contains("Enlightened of Tibia"));
            assert_eq!(
                commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len(),
                4
            );
            link.request_logout();
            task.await?;
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn loyalty_skills_unselected_session_receives_no_skills_command() -> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let peer = Peer::new(false, []);
            let commands = Arc::clone(&peer.commands);
            let served = Arc::clone(&peer.served);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            tokio::time::timeout(Duration::from_secs(3), served.notified()).await?;
            link.loyalty_skills_request();
            link.loyalty_skills_request();
            let panel = link.loyalty_skills_panel();
            assert!(!panel.selected);
            assert!(!panel.pending);
            assert!(panel.snapshot.is_none());
            assert!(panel_text(&panel).contains("unavailable"));
            link.request_logout();
            task.await?;
            assert!(
                commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .is_empty()
            );
            assert_eq!(link.loyalty_skills_panel(), LoyaltySkillsPanel::default());
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn loyalty_skills_closed_command_queue_releases_pending_request() {
    let (link, incoming, outbox) = play_channel();
    outbox.loyalty_skills_initialize(true);
    drop(incoming);
    link.loyalty_skills_request();
    let panel = link.loyalty_skills_panel();
    assert!(!panel.pending);
    assert_eq!(panel.error, Some("Session ended"));
    assert!(panel.snapshot.is_none());
}
