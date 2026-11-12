//! C29: production input, session task, mailbox and depot grid projection.
use oteryn_client::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, play_channel, run_session},
};
use oteryn_client_ui::depot::{Intent, deposit_items};
use oteryn_session::{
    DepotAction, DepotEntry, DepotIntent, DepotOutcome, DepotReply, ItemEntry, LogoutResult,
    SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    num::{NonZeroU32, NonZeroU64},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct DepotPeer {
    carried: bool,
    stored: bool,
    refuse_next: bool,
    bag_open: bool,
    withdrawn: bool,
}
fn bag() -> ItemEntry {
    ItemEntry {
        handle: NonZeroU64::new(7).unwrap_or(NonZeroU64::MIN),
        ..item()
    }
}
fn item() -> ItemEntry {
    ItemEntry {
        handle: NonZeroU64::MIN,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}
impl Stepper for DepotPeer {
    fn feature_selected(&self, cap: u32) -> bool {
        cap == 37
    }
    fn depot_items(&self) -> oteryn_client_ui::depot::Items {
        let mut items = oteryn_client_ui::depot::Items {
            containers_selected: true,
            ..Default::default()
        };
        items.inventory.main_backpack = Some(bag());
        if self.carried {
            if self.withdrawn {
                items.inventory.entries.push(item());
            } else if self.bag_open {
                items.views.push(item());
            }
        }
        items
    }
    async fn depot_open_container(
        &mut self,
        handle: oteryn_session::ItemHandle,
    ) -> Result<Option<oteryn_client_ui::depot::ContainerViewOutcome>, SessionError> {
        assert_eq!(handle, bag().handle);
        self.bag_open = true;
        Ok(Some(oteryn_client_ui::depot::ContainerViewOutcome::Opened))
    }
    async fn depot(&mut self, intent: DepotIntent) -> Result<Option<DepotReply>, SessionError> {
        let outcome = match intent.action {
            DepotAction::Browse => DepotOutcome::Opened,
            DepotAction::Deposit(handle) => {
                assert_eq!(handle, item().handle);
                assert_eq!(intent.town, Some(3));
                self.carried = false;
                self.stored = true;
                DepotOutcome::Moved
            }
            DepotAction::Withdraw(71) if self.refuse_next => {
                self.refuse_next = false;
                DepotOutcome::Overweight
            }
            DepotAction::Withdraw(71) => {
                self.withdrawn = true;
                self.carried = true;
                self.stored = false;
                DepotOutcome::Moved
            }
            _ => DepotOutcome::Stale,
        };
        if !matches!(outcome, DepotOutcome::Opened | DepotOutcome::Moved) {
            return Ok(Some(DepotReply {
                outcome,
                town: None,
                box_number: intent.box_number,
                page: intent.page,
                total: 0,
                entries: vec![],
            }));
        }
        Ok(Some(DepotReply {
            outcome,
            town: Some(3),
            box_number: intent.box_number,
            page: intent.page,
            total: u32::from(self.stored && intent.box_number == 1),
            entries: if self.stored && intent.box_number == 1 {
                vec![DepotEntry {
                    ordinal: 71,
                    definition: 1,
                    count: 1,
                }]
            } else {
                vec![]
            },
        }))
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}
async fn ready(link: &oteryn_client::play::PlayLink, text: &str) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let p = link.depot_panel();
            if !p.pending && p.feedback == text {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn native_depot_browse_deposit_switch_box_refusal_withdraw_and_logout() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        DepotPeer {
                            carried: true,
                            stored: false,
                            refuse_next: true,
                            bag_open: false,
                            withdrawn: false,
                        },
                        commands,
                        outbox,
                    ));
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while !link.depot_panel().selected {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    link.open_depot();
                    ready(&link, "Depot box loaded").await?;
                    assert_eq!(deposit_items(&link.depot_panel()), vec![bag()]);
                    link.depot_open_container(bag().handle);
                    ready(&link, "Container loaded").await?;
                    assert_eq!(deposit_items(&link.depot_panel()), vec![bag(), item()]);
                    link.depot_request(Intent::Deposit(item().handle));
                    ready(&link, "Item moved").await?;
                    assert_eq!(deposit_items(&link.depot_panel()), vec![bag()]);
                    assert_eq!(
                        link.depot_panel().reply.as_ref().map(|r| r.entries.len()),
                        Some(1)
                    );
                    link.depot_request(Intent::Browse {
                        box_number: 2,
                        page: 0,
                    });
                    ready(&link, "Depot box loaded").await?;
                    assert!(
                        link.depot_panel()
                            .reply
                            .as_ref()
                            .is_some_and(|r| r.entries.is_empty())
                    );
                    link.depot_request(Intent::Browse {
                        box_number: 1,
                        page: 0,
                    });
                    ready(&link, "Depot box loaded").await?;
                    link.depot_request(Intent::Withdraw(0));
                    ready(&link, "You cannot carry this weight").await?;
                    assert_eq!(deposit_items(&link.depot_panel()), vec![bag()]);
                    assert!(
                        link.depot_panel()
                            .reply
                            .as_ref()
                            .is_some_and(|r| r.town.is_none() && r.entries.is_empty())
                    );
                    link.depot_request(Intent::Withdraw(0));
                    assert!(!link.depot_panel().pending);
                    link.depot_request(Intent::Browse {
                        box_number: 1,
                        page: 0,
                    });
                    ready(&link, "Depot box loaded").await?;
                    link.depot_request(Intent::Withdraw(0));
                    ready(&link, "Item moved").await?;
                    assert_eq!(deposit_items(&link.depot_panel()), vec![bag(), item()]);
                    assert!(
                        link.depot_panel()
                            .reply
                            .as_ref()
                            .is_some_and(|r| r.entries.is_empty())
                    );
                    link.request_logout();
                    task.await?;
                    assert_eq!(link.depot_panel(), Default::default());
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "depot scenario thread failed")?
}
