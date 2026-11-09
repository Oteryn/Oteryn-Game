use super::*;
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, play_channel, run_session},
};
use oteryn_session::{
    AppliedDelta, EntityRef, LogoutResult, NpcIntentDisposition, SessionError, SessionEvent,
    StepDirection, StepOutcome,
    npc_service::{self, NpcConversation, NpcConversationState, NpcTalkIntent},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    events: Vec<SessionEvent>,
    log: Arc<Mutex<Vec<NpcTalkIntent>>>,
}
impl Stepper for Peer {
    fn feature_selected(&self, id: u32) -> bool {
        id == 3
    }
    async fn npc_talk(
        &mut self,
        intent: &NpcTalkIntent,
    ) -> Result<NpcIntentDisposition, SessionError> {
        let bytes = npc_service::encode_npc_talk_intent(intent)
            .map_err(|_| SessionError::SessionUnusable)?;
        let intent = npc_service::decode_npc_talk_intent(&bytes)
            .map_err(|_| SessionError::SessionUnusable)?;
        let line = if intent.text == "yes" {
            "Server: exchange accepted"
        } else {
            "Server: confirm the quoted token cost with {yes} or {no}"
        };
        let conversation = NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines: vec![line.into()],
        };
        let bytes = npc_service::encode_npc_conversation(&conversation)
            .map_err(|_| SessionError::SessionUnusable)?;
        let value = npc_service::decode_npc_conversation(&bytes)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.events
            .push(SessionEvent::NpcConversation(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value,
            }));
        self.log
            .lock()
            .map_err(|_| SessionError::SessionUnusable)?
            .push(intent);
        Ok(NpcIntentDisposition::Ok)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
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
        while !link.npc_dialogue_panel().selected || link.npc_dialogue_panel().pending {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn memorial_tabs_and_explicit_exchange_cross_the_real_npc_session_caller() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let log = Arc::new(Mutex::new(Vec::new()));
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            events: vec![],
                            log: log.clone(),
                        },
                        commands,
                        outbox,
                    ));
                    ready(&link).await?;
                    link.memorial_action(MemorialAction::Ask);
                    assert!(log.lock().map_err(|_| "log")?.is_empty());
                    let npc = EntityRef {
                        identity: [7; 16],
                        generation: 2,
                    };
                    link.talk_to_npc(npc, "hi");
                    ready(&link).await?;
                    link.memorial_action(MemorialAction::Tokens);
                    link.memorial_action(MemorialAction::Ask);
                    link.memorial_action(MemorialAction::Yes); // A second click before the quote cannot confirm it.
                    ready(&link).await?;
                    assert_eq!(log.lock().map_err(|_| "log")?.len(), 2);
                    assert!(
                        link.npc_dialogue_panel()
                            .text()
                            .contains("quoted token cost")
                    );
                    assert!(link.appearance_panel().confirmed.is_none());
                    link.memorial_action(MemorialAction::Yes);
                    ready(&link).await?;
                    let entries = log.lock().map_err(|_| "log")?;
                    assert_eq!(
                        entries.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
                        vec!["hi", "basic soil guardian outfit", "yes"]
                    );
                    assert!(entries.iter().all(|e| e.npc_actor == npc));
                    drop(entries);
                    assert!(
                        link.npc_dialogue_panel()
                            .text()
                            .contains("exchange accepted")
                    );
                    assert!(link.appearance_panel().confirmed.is_none());
                    task.abort();
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "memorial session thread")?
}
