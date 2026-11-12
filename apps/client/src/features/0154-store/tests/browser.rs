use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{PlayLink, Stepper, mailbox::lock, play_channel, run_session},
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
    seen: Arc<Mutex<Vec<ServerMechanicCommand>>>,
}
fn id() -> [u8; 16] {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
impl Stepper for Peer {
    fn server_mechanic_selected(&self, cap: u32) -> bool {
        cap == 154
    }
    async fn server_mechanic(
        &mut self,
        c: &ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, SessionError> {
        assert_eq!(c.capability, 154);
        self.seen
            .lock()
            .map_err(|_| SessionError::SessionUnusable)?
            .push(c.clone());
        let payload = match c.command_type {
            6118 => {
                let q = b::Query::decode(&c.payload).map_err(|_| SessionError::SessionUnusable)?;
                assert_eq!(
                    (q.category.as_str(), q.search.as_str(), q.sort),
                    ("featured", "Sword", 3)
                );
                let categories = vec![
                    b::Category {
                        key: "featured".into(),
                        parent: String::new(),
                        title: "Home".into(),
                    },
                    b::Category {
                        key: "consumables".into(),
                        parent: String::new(),
                        title: "Consumables".into(),
                    },
                    b::Category {
                        key: "consumables.exercise-weapons".into(),
                        parent: "consumables".into(),
                        title: "Exercise Weapons".into(),
                    },
                ];
                let products = (0..2)
                    .map(|n| b::Product {
                        category: "consumables.exercise-weapons".into(),
                        revision: format!("rev-{n}"),
                        offer: store::StoreOffer {
                            offer_key: format!("store:sword-{n}"),
                            title: format!("Sword {n}"),
                            price: 25 + n,
                            currency: "COINS".into(),
                        },
                        artwork: b::Artwork {
                            kind: 1,
                            looks: [28552; 2],
                        },
                    })
                    .collect();
                b::Page {
                    categories,
                    products,
                    next: String::new(),
                    total: 100,
                    transferable: 0,
                    transferable_known: false,
                }
                .encode()
                .map_err(|_| SessionError::SessionUnusable)?
            }
            185 => {
                let q = store::decode_purchase_request(&c.payload)
                    .map_err(|_| SessionError::SessionUnusable)?;
                assert_eq!(
                    (q.offer_key.as_str(), q.catalog_revision.as_str(), q.price),
                    ("store:sword-1", "rev-1", 26)
                );
                store::encode_purchase_result(&store::StorePurchaseResult {
                    code: store::StorePurchaseCode::Accepted,
                    order_id: Some(id()),
                    operation_id: Some(id()),
                })
                .map_err(|_| SessionError::SessionUnusable)?
            }
            _ => return Err(SessionError::SessionUnusable),
        };
        Ok(ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
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
async fn ready(link: &PlayLink, loaded: bool) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let p = link.store_panel();
            if p.selected && !p.pending && (!loaded || p.browser.page.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn store_browser_caller_preserves_selected_quote_through_native_confirmation() -> Result<(), Error>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let seen = Arc::new(Mutex::new(Vec::new()));
                    let (link, commands, outbox) = play_channel();
                    let task =
                        tokio::spawn(run_session(Peer { seen: seen.clone() }, commands, outbox));
                    ready(&link, false).await?;
                    link.store_browse("featured", "Sword", 3, 0);
                    link.store_browse("featured", "Sword", 3, 0);
                    ready(&link, true).await?;
                    assert_eq!(seen.lock().map_err(|_| "seen")?.len(), 1);
                    assert!(oteryn_client_ui::store::select_product(
                        &mut lock(&link.mailbox).store,
                        1
                    ));
                    let quote = link.store_panel().catalogue.ok_or("quote")?;
                    assert!(!link.store_confirm_purchase(
                        &store::StoreCataloguePage {
                            catalog_revision: "stale".into(),
                            ..quote.clone()
                        },
                        "",
                        "1",
                        ""
                    ));
                    assert!(link.store_confirm_purchase(&quote, "", "1", ""));
                    assert!(!link.store_confirm_purchase(&quote, "", "1", ""));
                    ready(&link, true).await?;
                    assert_eq!(
                        link.store_panel().purchase.ok_or("result")?.code,
                        store::StorePurchaseCode::Accepted
                    );
                    assert_eq!(seen.lock().map_err(|_| "seen")?.len(), 2);
                    task.abort();
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "store browser caller panicked")?
}
