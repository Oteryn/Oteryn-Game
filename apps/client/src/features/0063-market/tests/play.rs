use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::market::{MarketCommand, MarketResponse, market};
use oteryn_client_ui::market::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer;
impl Stepper for Peer {
    fn market_selected(&self) -> bool {
        true
    }
    async fn market(&mut self, request: &MarketCommand) -> Result<MarketResponse, SessionError> {
        let bytes = market::encode_request(request.0).map_err(|_| SessionError::SessionUnusable)?;
        let decoded = market::decode_request(&bytes).map_err(|_| SessionError::SessionUnusable)?;
        let reply = market::MarketReply {
            action: decoded.action,
            code: 1,
            offer: [0; 16],
            rows: vec![market::MarketRow {
                offer: [9; 16],
                definition: decoded.definition,
                side: 2,
                amount: 5,
                price: 100,
                state: 1,
                anonymous: false,
            }],
        };
        let wire = market::encode_reply(&reply).map_err(|_| SessionError::SessionUnusable)?;
        Ok(MarketResponse {
            status: CommandStatus::Accepted,
            reply: Some(market::decode_reply(&wire).map_err(|_| SessionError::SessionUnusable)?),
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
            let panel = link.market_panel();
            if panel.selected && panel.pending.is_none() && !panel.rows.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn market_controls_reach_session_and_apply_encoded_reply() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer, incoming, outbox));
                while !link.market_panel().selected {
                    tokio::task::yield_now().await;
                }
                {
                    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                    mailbox.market.definition = 42;
                }
                link.market_request(Intent::Browse);
                ready(&link).await?;
                assert!(panel_text(&link.market_panel()).contains("Item 42"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Market UI test thread panicked")?
}
