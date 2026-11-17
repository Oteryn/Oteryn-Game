use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::market::{MarketCommand, MarketResponse, market};
use oteryn_client_ui::market::{Intent, panel_text, row_text};
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
fn market_controls_reach_session_and_apply_encoded_reply() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
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
                assert!(link.market_details("42", "3", "101", false, false));
                link.market_request(Intent::CreateOffer);
                ready(&link).await?;
                assert!(panel_text(&link.market_panel()).contains("insufficient funds"));
                assert!(link.market_details("42", "3", "100", false, false));
                link.market_request(Intent::CreateOffer);
                ready(&link).await?;
                assert!(link.market_details("42", "3", "100", true, false));
                for intent in [
                    Intent::CreateOffer,
                    Intent::OwnOffers,
                    Intent::CancelOffer,
                    Intent::Browse,
                    Intent::AcceptOffer,
                ] {
                    if matches!(intent, Intent::CancelOffer | Intent::AcceptOffer) {
                        link.market_select_offer(0);
                    }
                    link.market_request(intent);
                    ready(&link).await?;
                }
                drop(link);
                task.await?;
                let requests = requests.lock().map_err(|_| "request log poisoned")?;
                assert_eq!(
                    requests.iter().map(|r| r.action).collect::<Vec<_>>(),
                    [2, 5, 5, 2, 6, 2, 3, 7, 3, 2, 8, 2]
                );
                for request in requests.iter().filter(|r| matches!(r.action, 7 | 8)) {
                    assert_eq!(request.offer, offer_id());
                }
                assert_eq!((requests[2].amount, requests[2].price), (3, 100));
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Market UI test thread panicked")?
}

// Native HWND creation is Windows-only; exercise shared panel lifetime and input here.
#[cfg(not(windows))]
#[test]
fn depot_market_open_preserves_panels_and_ctrl_m_can_reopen() -> Result<(), Error> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut incoming, _) = play_channel();
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        m.market.selected = true;
        m.depot.open = true;
        m.prey.open = true;
        m.quest.open = true;
    }
    crate::play::feature_0063_market::input::open_from_depot(&link.mailbox, &link.commands);
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::Market(_)
    ));
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        assert!(m.market.open && m.depot.open && m.prey.open && m.quest.open);
        assert!(!m.market_shutdown && !m.depot_shutdown && !m.prey_shutdown && !m.quest_shutdown);
        m.market.open = false;
        m.market.pending = None;
    }
    assert!(link.market_input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    }]));
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::Market(_)
    ));
    let m = crate::play::mailbox::lock(&link.mailbox);
    assert!(m.market.open && m.depot.open && m.prey.open && m.quest.open);
    assert!(!m.market_shutdown && !m.depot_shutdown && !m.prey_shutdown && !m.quest_shutdown);
    Ok(())
}

#[path = "details.rs"]
mod details;

#[test]
fn market_side_filter_preserves_server_offer_identity() -> Result<(), Error> {
    let (link, mut incoming, _) = play_channel();
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        m.market.selected = true;
        m.market.rows = [1, 2]
            .into_iter()
            .map(|side| market::MarketRow {
                offer: {
                    let mut id = offer_id();
                    id[0] = side;
                    id
                },
                definition: 42,
                side,
                amount: 5,
                price: 100,
                state: 1,
                anonymous: false,
            })
            .collect();
    }
    assert!(link.market_filter_side(2));
    link.market_select_offer(0);
    link.market_request(Intent::AcceptOffer);
    let crate::combat_input::PlayCommand::Market(command) = incoming.try_recv()? else {
        return Err("missing market command".into());
    };
    assert_eq!(command.0.action, 8);
    assert_eq!(command.0.offer[0], 2);
    assert!(!link.market_filter_side(3));
    Ok(())
}
