use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::stash::{StashCommand, StashResponse, stash};
use oteryn_client_ui::stash::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer;
impl Stepper for Peer {
    fn stash_selected(&self) -> bool {
        true
    }
    async fn stash(&mut self, command: &StashCommand) -> Result<StashResponse, SessionError> {
        let intent = stash::decode_stash_intent(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        let reply = stash::StashReply {
            outcome: stash::StashOutcome::Opened,
            stowed_quantity: 0,
            transaction: None,
            page: intent.page,
            has_more: false,
            rows: vec![stash::StashRow {
                handle: [7; 16],
                definition_key: "item.apple".into(),
                definition_revision: "r1".into(),
                quantity: 12,
                migration_required: false,
            }],
        };
        let wire = stash::encode_stash_reply(&reply).map_err(|_| SessionError::SessionUnusable)?;
        Ok(StashResponse {
            status: CommandStatus::Accepted,
            reply: stash::decode_stash_reply(&wire).map_err(|_| SessionError::SessionUnusable)?,
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
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.stash_panel();
            if panel.selected && !panel.pending && !panel.rows.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn stash_panel_request_uses_codec_and_applies_encoded_reply() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer, incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.stash_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.stash_request(Intent::Refresh);
                ready(&link).await?;
                assert!(panel_text(&link.stash_panel()).contains("item.apple"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Stash UI test thread panicked")?
}
