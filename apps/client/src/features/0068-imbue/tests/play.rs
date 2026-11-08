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
    details: bool,
}

impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        self.details && matches!(capability, 37 | 447)
    }
    fn depot_items(&self) -> oteryn_client_runtime::depot::Items {
        let mut items = oteryn_client_runtime::depot::Items::default();
        {
            items
                .inventory
                .entries
                .push(oteryn_client_runtime::depot::ItemEntry {
                    handle: NonZeroU64::MIN.saturating_add(41),
                    item_definition_ref: std::num::NonZeroU32::MIN,
                    count: std::num::NonZeroU32::MIN,
                    sub_type: 0,
                });
        }
        items
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        use oteryn_client_runtime::imbuement_details as wire;
        if !self.details
            || command.capability != wire::CAPABILITY_IMBUEMENT_DETAILS_V1
            || command.kind != wire::COMMAND_TYPE_IMBUEMENT_DETAILS
        {
            return Err(SessionError::SessionUnusable);
        }
        let item =
            wire::decode_query(&command.payload).map_err(|_| SessionError::SessionUnusable)?;
        let reply = wire::Reply {
            item,
            quotes: vec![wire::Quote {
                family: 3,
                tier: 2,
                gold: 400,
                requires_premium: true,
                recipe_available: true,
                success_percent: 100,
                materials: vec![wire::Material {
                    definition_key: "oteryn:item.tibia.i11466".into(),
                    quantity: 25,
                }],
            }],
        };
        Ok(oteryn_session::GatedCommandResult {
            command_id: 9,
            server_sequence: 9,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&reply).map_err(|_| SessionError::SessionUnusable)?,
        })
    }

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
                    details: false,
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, &requests, 0).await?;
                link.imbue_item(NonZeroU64::new(42).ok_or("item")?);
                link.open_imbue();
                ready(&link, &requests, 1).await?;
                assert!(panel_text(&link.imbue_panel()).contains("400 gold (premium)"));
                assert!(link.imbue_slot(1));
                link.imbue_request(Intent::Apply);
                ready(&link, &requests, 2).await?;
                assert!(panel_text(&link.imbue_panel()).contains("materials are missing"));
                assert!(link.imbue_slot(0));
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
                            slot: 1,
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

include!(
    "play_imbuement_selected_slot_drives_live_command_and_item_change_discards_old_quote_1.rs"
);

#[test]
fn imbuement_activity_handoff_discards_old_item_and_material_reply() -> Result<(), Error> {
    use oteryn_client_runtime::{imbuement_details as wire, raw_feature::GatedCommandResult};
    let (link, _receiver, _outbox) = play_channel();
    let item = NonZeroU64::new(7).ok_or("item")?;
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.imbue.selected = true;
    mailbox.imbue.details_selected = true;
    mailbox.imbue.item = Some(item);
    mailbox.imbue.window = Some(imbuement::ImbuementWindow {
        slot_count: 1,
        applied: vec![],
        offers: vec![],
        clear_gold: 15000,
    });
    let command = oteryn_client_ui::imbue::details::request(&mut mailbox.imbue).ok_or("query")?;
    mailbox.cancel_activity_requests();
    assert!(mailbox.imbue.selected && mailbox.imbue.details_selected);
    assert!(mailbox.imbue.item.is_none() && mailbox.imbue.window.is_none());
    assert!(mailbox.imbue.details_pending.is_none());
    oteryn_client_ui::imbue::details::apply(
        &mut mailbox.imbue,
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
            payload: wire::encode_reply(&wire::Reply {
                item,
                quotes: vec![],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(mailbox.imbue.details.is_none());
    drop(mailbox);
    Ok(())
}

/// Exercises the production live picker and the actual session-loop Feature reply route.
#[test]
fn imbuement_live_picker_and_material_reply_reach_run_session() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let requests = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        requests: requests.clone(),
                        details: true,
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, &requests, 0).await?;
                tokio::time::timeout(Duration::from_secs(3), async {
                    while crate::play::feature_0068_imbue::input::carried_items(
                        &crate::play::mailbox::lock(&link.mailbox),
                    )
                    .is_empty()
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(!link.imbue_carried_item(NonZeroU64::new(999).ok_or("unknown item")?));
                assert!(
                    requests
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_empty()
                );
                assert!(link.imbue_carried_item(NonZeroU64::new(42).ok_or("carried item")?));
                ready(&link, &requests, 1).await?;
                assert_eq!(
                    requests.lock().unwrap_or_else(PoisonError::into_inner)[0]
                        .item
                        .get(),
                    42
                );
                assert!(link.imbue_panel().details_selected);
                link.imbue_details_request();
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.imbue_panel().details.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.imbue_slot(1));
                let panel = link.imbue_panel();
                assert!(panel.details_pending.is_none());
                assert!(panel.can_apply());
                let text = oteryn_client_ui::imbue::selected_detail_text(&panel);
                assert!(text.contains("25"));
                assert!(text.contains("i11466"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Imbuement live route test thread panicked")?
}
