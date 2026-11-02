use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::animus_mastery::{AnimusPanel, Intent, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::animus_mastery::{AnimusPage, MAX_ANIMUS_PAGE_KEYS, encode_page};
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
    offset: u16,
    status: CommandStatus,
    payload: Vec<u8>,
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
    fn animus_selected(&self) -> bool {
        self.selected
    }

    async fn animus(&mut self, payload: &[u8]) -> Result<GatedCommandResult, SessionError> {
        let Some(reply) = self.replies.pop_front() else {
            return Err(SessionError::SessionUnusable);
        };
        assert_eq!(
            Some(payload),
            AnimusPage {
                offset: reply.offset,
                limit: MAX_ANIMUS_PAGE_KEYS,
            }
            .encode()
            .as_ref()
            .map(|bytes| bytes.as_slice())
        );
        let sequence = {
            let mut commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
            commands.push(payload.to_vec());
            u64::try_from(commands.len()).map_err(|_| SessionError::SessionUnusable)?
        };
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
    predicate: impl Fn(&AnimusPanel) -> bool,
) -> Result<AnimusPanel, Error> {
    Ok(tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.animus_panel();
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
        .map_err(|_| "Animus UI test thread panicked")?
}

#[test]
fn animus_native_input_pages_and_replay_use_the_session_queue_and_clear_on_logout()
-> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let first = (0..MAX_ANIMUS_PAGE_KEYS)
                .map(|index| format!("oteryn:creature.species{index:03}"))
                .collect::<Vec<_>>();
            let last = vec!["oteryn:creature.species050".to_owned()];
            let peer = Peer::new(
                true,
                [
                    Reply {
                        offset: 0,
                        status: CommandStatus::Accepted,
                        payload: encode_page(51, &first).ok_or("first page")?,
                    },
                    Reply {
                        offset: 50,
                        status: CommandStatus::DuplicateReplay,
                        payload: encode_page(51, &last).ok_or("last page")?,
                    },
                    Reply {
                        offset: 0,
                        status: CommandStatus::Accepted,
                        payload: encode_page(51, &first).ok_or("previous page")?,
                    },
                ],
            );
            let commands = Arc::clone(&peer.commands);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, |panel| panel.selected).await?;
            let open = NormalizedInputEvent::Key {
                code: KeyCode::new(4)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control),
                repeat: false,
            };
            assert!(link.animus_input(&[open]));
            let first_page = ready(&link, |panel| panel.loaded && panel.pending.is_none()).await?;
            assert!(first_page.open);
            assert_eq!(first_page.keys, first);
            assert_eq!(first_page.total, 51);
            assert!(!first_page.has_previous());
            assert!(first_page.has_next());
            link.animus_request(Intent::Next);
            let last_page =
                ready(&link, |panel| panel.offset == 50 && panel.pending.is_none()).await?;
            assert_eq!(last_page.keys, last);
            assert!(last_page.has_previous());
            assert!(!last_page.has_next());
            assert!(panel_text(&last_page).contains("species050"));
            link.animus_request(Intent::Previous);
            let previous =
                ready(&link, |panel| panel.offset == 0 && panel.pending.is_none()).await?;
            assert_eq!(previous.keys, first);
            assert_eq!(
                *commands.lock().unwrap_or_else(PoisonError::into_inner),
                vec![vec![0, 0, 0, 50], vec![0, 50, 0, 50], vec![0, 0, 0, 50]]
            );
            link.request_logout();
            task.await?;
            assert_eq!(link.animus_panel(), AnimusPanel::default());
            link.animus_request(Intent::Refresh);
            assert!(link.animus_panel().pending.is_none());
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn animus_refused_and_malformed_pages_keep_confirmed_species_and_allow_retry() -> Result<(), Error>
{
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let first = (0..MAX_ANIMUS_PAGE_KEYS)
                .map(|index| format!("oteryn:creature.species{index:03}"))
                .collect::<Vec<_>>();
            let last = vec!["oteryn:creature.species050".to_owned()];
            let peer = Peer::new(
                true,
                [
                    Reply {
                        offset: 0,
                        status: CommandStatus::Accepted,
                        payload: encode_page(51, &first).ok_or("initial page")?,
                    },
                    Reply {
                        offset: 50,
                        status: CommandStatus::Rejected,
                        payload: vec![],
                    },
                    Reply {
                        offset: 50,
                        status: CommandStatus::Accepted,
                        payload: vec![51, 0, 1],
                    },
                    Reply {
                        offset: 50,
                        status: CommandStatus::Accepted,
                        payload: encode_page(51, &last).ok_or("retry page")?,
                    },
                ],
            );
            let commands = Arc::clone(&peer.commands);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, |panel| panel.selected).await?;
            link.animus_request(Intent::Refresh);
            ready(&link, |panel| panel.loaded && panel.pending.is_none()).await?;
            for _ in 0..2 {
                link.animus_request(Intent::Next);
                let refused = ready(&link, |panel| {
                    panel.error.is_some() && panel.pending.is_none()
                })
                .await?;
                assert_eq!(refused.offset, 0);
                assert_eq!(refused.keys, first);
                assert_eq!(refused.total, 51);
                assert!(refused.has_next());
            }
            link.animus_request(Intent::Next);
            let recovered =
                ready(&link, |panel| panel.offset == 50 && panel.pending.is_none()).await?;
            assert_eq!(recovered.keys, last);
            assert!(recovered.error.is_none());
            assert_eq!(
                commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len(),
                4
            );
            drop(link);
            task.await?;
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn animus_unselected_session_never_receives_a_mastery_command() -> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let peer = Peer::new(false, []);
            let commands = Arc::clone(&peer.commands);
            let served = Arc::clone(&peer.served);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            tokio::time::timeout(Duration::from_secs(3), served.notified()).await?;
            for intent in [Intent::Refresh, Intent::Previous, Intent::Next] {
                link.animus_request(intent);
            }
            assert!(!link.animus_panel().selected);
            assert!(link.animus_panel().pending.is_none());
            assert!(panel_text(&link.animus_panel()).contains("unavailable"));
            link.request_logout();
            task.await?;
            assert!(
                commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .is_empty()
            );
            assert_eq!(link.animus_panel(), AnimusPanel::default());
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn animus_closed_command_queue_releases_pending_request() {
    let (link, incoming, outbox) = play_channel();
    outbox.animus_initialize(true);
    drop(incoming);
    link.animus_request(Intent::Refresh);
    let panel = link.animus_panel();
    assert!(panel.pending.is_none());
    assert_eq!(panel.error, Some("Session ended"));
    assert!(!panel.loaded);
}
