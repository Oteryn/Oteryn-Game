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
