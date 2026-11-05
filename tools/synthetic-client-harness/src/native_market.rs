use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::market::{Intent, panel_text, row_text};
use oteryn_client_ui::market::{MarketCommand, MarketResponse, market};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

type Error = Box<dyn std::error::Error + Send + Sync>;
fn offer_id() -> [u8; 16] {
    let mut id = [9; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
struct Peer(Arc<Mutex<Vec<market::MarketRequest>>>);
impl Stepper for Peer {
    fn market_selected(&self) -> bool {
        true
    }
    async fn market(&mut self, request: &MarketCommand) -> Result<MarketResponse, SessionError> {
        let bytes = market::encode_request(request.0).map_err(|_| SessionError::SessionUnusable)?;
        let decoded = market::decode_request(&bytes).map_err(|_| SessionError::SessionUnusable)?;
        self.0
            .lock()
            .map_err(|_| SessionError::SessionUnusable)?
            .push(decoded);
        let reply = market::MarketReply {
            action: decoded.action,
            code: if decoded.price == 101 { 2 } else { 1 },
            offer: [0; 16],
            rows: vec![market::MarketRow {
                offer: offer_id(),
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
fn native_market_returns_to_wares_and_pages_via_public_controls() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let requests = Arc::new(Mutex::new(Vec::new()));
                let task = tokio::spawn(run_session(Peer(requests.clone()), incoming, outbox));
                while !link.market_panel().selected {
                    tokio::task::yield_now().await;
                }
                assert!(link.market_details("42", "3", "100", true, false));
                link.market_request(Intent::Browse);
                ready(&link).await?;
                assert!(row_text(&link.market_panel().rows[0]).contains("Item 42"));
                assert!(panel_text(&link.market_panel()).contains("Total: 300"));
                // Unlike the app's trade test, this scenario exercises public wares navigation.
                link.market_request(Intent::Wares);
                ready(&link).await?;
                assert_eq!(link.market_panel().definition, 0);
                link.market_request(Intent::NextPage);
                ready(&link).await?;
                link.market_request(Intent::Refresh);
                ready(&link).await?;
                drop(link);
                task.await?;
                let requests = requests.lock().map_err(|_| "request log poisoned")?;
                assert_eq!(
                    requests
                        .iter()
                        .map(|r| (r.action, r.definition, r.page))
                        .collect::<Vec<_>>(),
                    [(2, 42, 0), (1, 0, 0), (1, 0, 1), (1, 0, 1)]
                );
                Ok::<(), Error>(())
            })
        })?
        .join()
        .map_err(|_| "Market UI test thread panicked")?
}
