//! C30: carried picker -> production play task -> stash adapter -> refreshed views.
use oteryn_client::{
    combat_input::{CombatCommand, CombatReport},
    play::{PlayLink, Stepper, play_channel, run_session},
};
use oteryn_client_ui::stash::{Intent, panel_text};
use oteryn_client_ui::stash::{StashCommand, StashResponse, stash};
use oteryn_session::{
    CommandStatus, ItemEntry, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    num::{NonZeroU32, NonZeroU64},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
fn item() -> ItemEntry {
    ItemEntry {
        handle: NonZeroU64::MIN,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}
struct Peer {
    carried: bool,
    stored: bool,
    refuse: bool,
}
impl Stepper for Peer {
    fn stash_selected(&self) -> bool {
        true
    }
    fn depot_items(&self) -> oteryn_client_ui::depot::Items {
        let mut items = oteryn_client_ui::depot::Items::default();
        if self.carried {
            items.inventory.entries.push(item());
        }
        items
    }
    async fn stash(&mut self, command: &StashCommand) -> Result<StashResponse, SessionError> {
        let (intent, _filter) = stash::decode_stash_query(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        let outcome = match intent.action {
            stash::StashAction::Browse => stash::StashOutcome::Opened,
            stash::StashAction::Stow(handle) => {
                assert_eq!(handle, item().handle);
                assert!(self.carried);
                self.carried = false;
                self.stored = true;
                stash::StashOutcome::Committed
            }
            stash::StashAction::Withdraw { row, amount } => {
                assert_eq!(row, [7; 16]);
                assert_eq!(amount, 1);
                assert!(self.stored);
                if self.refuse {
                    self.refuse = false;
                    stash::StashOutcome::Overweight
                } else {
                    self.carried = true;
                    self.stored = false;
                    stash::StashOutcome::Committed
                }
            }
        };
        let reply = stash::StashReply {
            outcome,
            stowed_quantity: u32::from(matches!(intent.action, stash::StashAction::Stow(_))),
            transaction: None,
            page: intent.page,
            has_more: false,
            rows: if self.stored {
                vec![stash::StashRow {
                    handle: [7; 16],
                    definition_key: "item.apple".into(),
                    definition_revision: "r1".into(),
                    quantity: 1,
                    migration_required: false,
                }]
            } else {
                vec![]
            },
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
        Ok(Some(LogoutResult::Accepted))
    }
}
async fn ready(link: &PlayLink, carried: bool, stored: bool) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let panel = link.stash_panel();
            if panel.selected
                && !panel.pending
                && panel.carried.is_empty() != carried
                && panel.rows.is_empty() != stored
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
fn native_stash_picker_stow_refusal_retrieve_refresh_and_logout() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            carried: true,
                            stored: false,
                            refuse: true,
                        },
                        commands,
                        outbox,
                    ));
                    ready(&link, true, false).await?;
                    link.open_stash();
                    ready(&link, true, false).await?;
                    assert!(link.stash_panel().open);
                    let displayed = link.stash_panel().carried;
                    link.stash_stow_selected(&displayed, 0);
                    ready(&link, false, true).await?;
                    assert!(panel_text(&link.stash_panel()).contains("Stowed 1 object"));
                    link.stash_stow_selected(&displayed, 0);
                    assert!(!link.stash_panel().pending);
                    link.stash_request(Intent::Withdraw { row: 0, amount: 1 });
                    ready(&link, false, true).await?;
                    assert!(panel_text(&link.stash_panel()).contains("too heavy"));
                    link.stash_request(Intent::Withdraw { row: 0, amount: 1 });
                    ready(&link, true, false).await?;
                    assert_eq!(link.stash_panel().carried, vec![item()]);
                    link.stash_request(Intent::Refresh);
                    ready(&link, true, false).await?;
                    link.request_logout();
                    task.await?;
                    link.open_stash();
                    assert_eq!(link.stash_panel(), Default::default());
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "stash scenario thread failed")?
}
