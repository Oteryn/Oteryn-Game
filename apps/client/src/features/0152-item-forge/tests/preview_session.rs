//! Regression for negotiated capabilities travelling through the real production session loop.
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{
        Stepper,
        feature_0152_item_forge::input::controller::{Action, activate},
        mailbox::lock,
        play_channel, run_session,
    },
};
use oteryn_client_runtime::{
    cui3::item_forge::Operation,
    forge_preview as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::{
    num::{NonZeroU32, NonZeroU64},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer;
fn card(handle: u64) -> Result<wire::Item, SessionError> {
    Ok(wire::Item {
        handle: NonZeroU64::new(handle).ok_or(SessionError::SessionUnusable)?,
        definition_ref: 1,
        appearance: 3264,
        name: "Native practice sword".into(),
        classification: 1,
        tier: 0,
        maximum_tier: 1,
        container: false,
    })
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 37 | 152)
    }
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        capability == 152
    }
    fn depot_items(&self) -> oteryn_client_runtime::depot::Items {
        let mut items = oteryn_client_runtime::depot::Items::default();
        for handle in [2, 3] {
            if let Ok(card) = card(handle) {
                items
                    .inventory
                    .entries
                    .push(oteryn_client_runtime::depot::ItemEntry {
                        handle: card.handle,
                        item_definition_ref: NonZeroU32::MIN,
                        count: NonZeroU32::MIN,
                        sub_type: 0,
                    });
            }
        }
        items
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        if command.capability != 152 || command.kind != 6116 {
            return Err(SessionError::SessionUnusable);
        }
        let request =
            wire::Request::decode(&command.payload).ok_or(SessionError::SessionUnusable)?;
        let resources = wire::Resources {
            carried_gold: 25000,
            dust: 100,
            dust_limit: 100,
            ..Default::default()
        };
        let payload = match request.selection {
            wire::Selection::Browse { after } => wire::Payload::Page(wire::Page {
                after,
                next: None,
                items: vec![card(2)?, card(3)?],
                resources,
            }),
            selection => wire::Payload::Quote(wire::Quote {
                selection,
                first: card(2)?,
                second: card(3)?,
                resources,
                cost: wire::Cost {
                    gold: 25000,
                    dust: 100,
                    cores: 0,
                },
                target_tier: 1,
                success_percent: 50,
                protection_percent: 0,
            }),
        };
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::Reply {
                nonce: request.nonce,
                payload,
            }
            .encode()
            .ok_or(SessionError::SessionUnusable)?,
        })
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
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
fn forge_preview_negotiated_reply_reaches_actual_run_session() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, commands, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer, commands, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.item_forge_panel().preview.live.len() != 2 {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.item_forge_panel().preview.selected);
                activate(&mut lock(&link.mailbox), &link.commands, Action::Refresh);
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.item_forge_panel().preview.items.len() != 2 {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(
                    link.item_forge_panel().preview.items[0].name,
                    "Native practice sword"
                );
                for action in [
                    Action::Operation(Operation::Fusion),
                    Action::First(2),
                    Action::Second(3),
                    Action::Quote,
                ] {
                    activate(&mut lock(&link.mailbox), &link.commands, action);
                }
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.item_forge_panel().verified_quote() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(
                    oteryn_client_ui::item_forge::selected_pair_text(&link.item_forge_panel())
                        .contains("25000 gold")
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Forge session test thread panicked")?
}
