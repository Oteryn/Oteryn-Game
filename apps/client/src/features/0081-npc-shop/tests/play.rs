use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::npc_shop::{
    NpcIntentDisposition, NpcShopCommand, NpcShopResponse, NpcTradeSide, npc_shop,
};
use oteryn_client_ui::npc_shop::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, EntityRef, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
struct Peer;
impl Stepper for Peer {
    fn npc_shop_selected(&self) -> bool {
        true
    }
    async fn npc_shop(
        &mut self,
        command: &NpcShopCommand,
    ) -> Result<NpcShopResponse, SessionError> {
        let bytes = npc_shop::encode_npc_shop_intent(&command.0)
            .map_err(|_| SessionError::SessionUnusable)?;
        let intent =
            npc_shop::decode_npc_shop_intent(&bytes).map_err(|_| SessionError::SessionUnusable)?;
        let result = match intent.action {
            npc_shop::NpcShopAction::Look => npc_shop::NpcShopResult {
                disposition: NpcIntentDisposition::Ok,
                look_text: "A polished shield".into(),
                sold_units: 0,
                total_gold_units: 0,
            },
            npc_shop::NpcShopAction::SellAll => npc_shop::NpcShopResult {
                disposition: NpcIntentDisposition::Ok,
                look_text: String::new(),
                sold_units: 3,
                total_gold_units: 36,
            },
        };
        let reply =
            npc_shop::encode_npc_shop_result(&result).map_err(|_| SessionError::SessionUnusable)?;
        Ok(NpcShopResponse {
            status: CommandStatus::Accepted,
            result: Some(
                npc_shop::decode_npc_shop_result(&reply)
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
fn offer() -> npc_shop::NpcShopIntent {
    npc_shop::NpcShopIntent {
        npc_actor: EntityRef {
            identity: [9; 16],
            generation: 2,
        },
        catalogue_revision: 7,
        offer_index: 4,
        side: NpcTradeSide::Sell,
        action: npc_shop::NpcShopAction::Look,
        expected_unit_price: 12,
    }
}
#[test]
fn npc_shop_actions_reach_the_session_and_render_real_encoded_replies()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer, incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.npc_shop_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.npc_shop_request(Intent::Look(offer()));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !panel_text(&link.npc_shop_panel()).contains("polished") {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.npc_shop_request(Intent::SellAll(offer()));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !panel_text(&link.npc_shop_panel()).contains("3 units") {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                drop(link);
                task.await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })?
        })?
        .join()
        .map_err(|_| "NPC shop UI test thread panicked")??;
    Ok(())
}

#[test]
fn closed_server_shop_retains_inline_refusal_and_removes_tradeable_quotes() {
    use oteryn_session::{AppliedDelta, npc_service};
    let (link, _, outbox) = play_channel();
    let update = |value| {
        SessionEvent::NpcTradeWindow(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value,
        })
    };
    outbox.push(vec![update(npc_service::NpcTradeWindow {
        open: Some((offer().npc_actor, 7)),
        offers: vec![npc_service::NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Sell,
            count: 1,
            unit_price: 12,
        }],
    })]);
    crate::play::mailbox::lock(&outbox.0)
        .npc_shop
        .live
        .result(NpcIntentDisposition::NotInRange);
    outbox.push(vec![update(Default::default())]);
    let panel = link.npc_shop_panel();
    assert!(panel.open);
    assert!(panel.live.quotes().is_empty());
    assert!(panel.live.text().contains("NotInRange"));
}

struct HouseViewsPeer {
    payload: Vec<u8>,
    selected: bool,
}
impl Stepper for HouseViewsPeer {
    fn npc_shop_selected(&self) -> bool {
        true
    }
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 3 | 14) || (capability == 446 && self.selected)
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == 14).then(|| self.payload.clone())
    }
    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
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
#[test]
fn npc_shop_retains_twenty_fifth_container_source_only_when_negotiated()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::container_tree::{
        ContainerView, ContainerViews, encode_container_views_with_house_capacity,
    };
    let views = ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: 1_u64.try_into()?,
            parent_handle: None,
            capacity: 25,
            entries: (2_u64..=26)
                .map(|id| {
                    Ok(oteryn_session::ItemEntry {
                        handle: id.try_into()?,
                        item_definition_ref: 2854_u32.try_into()?,
                        count: 1_u32.try_into()?,
                        sub_type: 0,
                    })
                })
                .collect::<Result<_, Box<dyn std::error::Error>>>()?,
            browse: None,
        }],
    };
    let payload = encode_container_views_with_house_capacity(&views, false, true)
        .map_err(|e| format!("{e:?}"))?;
    for selected in [false, true] {
        let (link, _, outbox) = play_channel();
        outbox.npc_shop_initialize(&HouseViewsPeer {
            payload: payload.clone(),
            selected,
        });
        let panel = link.npc_shop_panel();
        if selected {
            assert_eq!(panel.live.containers, views);
        } else {
            assert!(panel.live.containers.views.is_empty());
        }
    }
    Ok(())
}

include!("options.rs");
