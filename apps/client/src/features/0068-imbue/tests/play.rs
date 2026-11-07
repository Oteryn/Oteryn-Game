use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::imbue::{ImbueCommand, ImbueResponse, imbuement};
use oteryn_client_ui::imbue::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    requests: Arc<Mutex<Vec<imbuement::ImbuementRequest>>>,
}

impl Stepper for Peer {
    fn imbue_selected(&self) -> bool {
        true
    }
    async fn imbue(&mut self, command: &ImbueCommand) -> Result<ImbueResponse, SessionError> {
        let bytes = imbuement::encode_imbuement_request(&command.0)
            .map_err(|_| SessionError::SessionUnusable)?;
        let request = imbuement::decode_imbuement_request(&bytes)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);
        let reply = imbuement::ImbuementReply {
            result: if matches!(request.action, imbuement::ImbuementAction::Apply { .. }) {
                imbuement::ImbuementResult::MaterialsMissing
            } else {
                imbuement::ImbuementResult::Ok
            },
            window: matches!(request.action, imbuement::ImbuementAction::Query).then(|| {
                imbuement::ImbuementWindow {
                    slot_count: 2,
                    applied: vec![imbuement::AppliedImbuement {
                        slot: 0,
                        family: 2,
                        tier: 1,
                        remaining_ms: 5000,
                    }],
                    offers: vec![imbuement::ImbuementOffer {
                        family: 3,
                        tier: 2,
                        gold: 400,
                        requires_premium: true,
                    }],
                    clear_gold: 75,
                }
            }),
        };
        let encoded =
            imbuement::encode_imbuement_reply(&reply).map_err(|_| SessionError::SessionUnusable)?;
        Ok(ImbueResponse {
            status: CommandStatus::Accepted,
            reply: Some(
                imbuement::decode_imbuement_reply(&encoded)
                    .map_err(|_| SessionError::SessionUnusable)?,
            ),
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

async fn ready(
    link: &PlayLink,
    requests: &Arc<Mutex<Vec<imbuement::ImbuementRequest>>>,
    count: usize,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if link.imbue_panel().selected
                && !link.imbue_panel().pending
                && requests
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len()
                    == count
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[test]
fn imbuement_panel_queries_and_sends_every_wire_action() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let requests = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    requests: Arc::clone(&requests),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, &requests, 0).await?;
                link.imbue_item(NonZeroU64::new(42).ok_or("item")?);
                link.open_imbue();
                ready(&link, &requests, 1).await?;
                assert!(panel_text(&link.imbue_panel()).contains("400 gold (premium)"));
                link.imbue_request(Intent::Apply);
                ready(&link, &requests, 2).await?;
                assert!(panel_text(&link.imbue_panel()).contains("materials are missing"));
                link.imbue_request(Intent::Clear);
                ready(&link, &requests, 3).await?;
                let actions = requests
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .iter()
                    .map(|request| request.action)
                    .collect::<Vec<_>>();
                assert_eq!(
                    actions,
                    vec![
                        imbuement::ImbuementAction::Query,
                        imbuement::ImbuementAction::Apply {
                            slot: 0,
                            imbuement: 0
                        },
                        imbuement::ImbuementAction::Clear { slot: 0 }
                    ]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Imbuement UI test thread panicked")?
}
