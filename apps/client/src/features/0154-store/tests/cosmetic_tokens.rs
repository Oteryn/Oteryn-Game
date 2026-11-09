//! Token balance and selected tender cross the actual session loop and Store reducer.
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{PlayLink, Stepper, play_channel, run_session},
};
use oteryn_client_runtime::{
    cui3::store::{self, browse as b},
    server_mechanic::{CommandStatus, ServerMechanicCommand, ServerMechanicResponse},
};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    balance: u64,
    seen: Arc<Mutex<Vec<ServerMechanicCommand>>>,
}
impl Stepper for Peer {
    fn server_mechanic_selected(&self, cap: u32) -> bool {
        matches!(cap, 154 | 479)
    }
    async fn server_mechanic(
        &mut self,
        c: &ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, SessionError> {
        self.seen
            .lock()
            .map_err(|_| SessionError::SessionUnusable)?
            .push(c.clone());
        let payload = match c.command_type {
            6187 => {
                assert_eq!(c.capability, 479);
                self.balance.to_le_bytes().to_vec()
            }
            6118 => {
                let q = b::Query::decode(&c.payload).map_err(|_| SessionError::SessionUnusable)?;
                assert_eq!(q.category, "cosmetics.outfits");
                b::Page {
                    categories: vec![
                        b::Category {
                            key: "cosmetics".into(),
                            parent: String::new(),
                            title: "Cosmetics".into(),
                        },
                        b::Category {
                            key: q.category.clone(),
                            parent: "cosmetics".into(),
                            title: "Outfits".into(),
                        },
                    ],
                    products: vec![b::Product {
                        category: q.category,
                        revision: "reference-price-870".into(),
                        offer: store::StoreOffer {
                            offer_key: "store:outfit:field-surgeon".into(),
                            title: "Field Surgeon (full outfit)".into(),
                            price: 870,
                            currency: "TIBIA_COIN".into(),
                        },
                        artwork: b::Artwork {
                            kind: 2,
                            looks: [1814, 1815],
                        },
                    }],
                    next: String::new(),
                    total: 1000,
                    transferable: 0,
                    transferable_known: false,
                }
                .encode()
                .map_err(|_| SessionError::SessionUnusable)?
            }
            6188 => {
                assert_eq!(c.capability, 479);
                let q = store::decode_purchase_request(&c.payload)
                    .map_err(|_| SessionError::SessionUnusable)?;
                assert_eq!(
                    (q.offer_key.as_str(), q.price, q.currency.as_str()),
                    ("store:outfit:field-surgeon", 870, "COSMETIC_TOKEN")
                );
                self.balance -= q.price;
                store::encode_purchase_result(&store::StorePurchaseResult {
                    code: store::StorePurchaseCode::Accepted,
                    order_id: Some(q.purchase_request_id),
                    operation_id: Some(q.purchase_request_id),
                })
                .map_err(|_| SessionError::SessionUnusable)?
            }
            _ => return Err(SessionError::SessionUnusable),
        };
        Ok(ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        })
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(d).await;
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
async fn wait(
    link: &PlayLink,
    condition: impl Fn(&oteryn_client_ui::store::StorePanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if condition(&link.store_panel()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn cosmetic_tokens_cross_real_session_loop_and_native_confirmation() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let seen = Arc::new(Mutex::new(Vec::new()));
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            balance: 870,
                            seen: Arc::clone(&seen),
                        },
                        incoming,
                        outbox,
                    ));
                    wait(&link, |p| p.selected).await?;
                    link.store_request(6187, vec![]);
                    wait(&link, |p| !p.pending && p.cosmetic_tokens == Some(870)).await?;
                    link.store_browse("cosmetics.outfits", "", 0, 0);
                    wait(&link, |p| !p.pending && p.catalogue.is_some()).await?;
                    assert!(link.store_payment(true));
                    let quote = oteryn_client_ui::store::confirmation_quote(&link.store_panel())
                        .ok_or("quote")?;
                    assert!(link.store_confirm_purchase(&quote, "", "", ""));
                    wait(&link, |p| !p.pending && p.purchase.is_some()).await?;
                    assert_eq!(
                        link.store_panel().purchase.ok_or("purchase")?.code,
                        store::StorePurchaseCode::Accepted
                    );
                    link.store_request(6187, vec![]);
                    wait(&link, |p| !p.pending && p.cosmetic_tokens == Some(0)).await?;
                    let kinds: Vec<_> = seen
                        .lock()
                        .map_err(|_| "seen")?
                        .iter()
                        .map(|c| c.command_type)
                        .collect();
                    assert_eq!(kinds, [6187, 6118, 6188, 6187]);
                    task.abort();
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "token client test thread panicked")?
}
