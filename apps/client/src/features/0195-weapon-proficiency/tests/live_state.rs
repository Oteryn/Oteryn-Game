use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_runtime::weapon_proficiency::{
    ModificationOperation, ModificationReply, ModificationRequest, ModificationResult,
    ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
};
use oteryn_client_runtime::weapon_proficiency_state::State;
use oteryn_client_ui::weapon_proficiency::Intent;
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// A named catalogue wire fixture, delivered through the same Stepper boundary as the server.
fn catalogue() -> ProficiencyCataloguePage {
    ProficiencyCataloguePage {
        offset: 0,
        total: 1,
        entries: vec![ProficiencyCatalogueEntry {
            definition: 258,
            weapon: "oteryn:item.test_sword".into(),
            name: "Test sword".into(),
            experience: 90_000,
            thresholds: [1; 9],
            levels: vec![ProficiencyCatalogueLevel {
                selected: Some(0),
                perks: vec!["Attack +1".into(), "Defence +2".into()],
            }],
        }],
    }
}

struct Peer {
    paid: Arc<Mutex<Option<ModificationRequest>>>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 195 | 448)
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        let payload = match (command.capability, command.kind) {
            (195, 660) => {
                assert_eq!(command.payload, 0u32.to_be_bytes());
                catalogue().encode().ok_or(SessionError::SessionUnusable)?
            }
            (448, 732) => {
                assert_eq!(command.payload, 258u32.to_be_bytes());
                State {
                    definition: 258,
                    revision: 123,
                    slots: [None, None],
                    modify_dust: [250, 1000],
                    rank_dust: [25; 10],
                    reshape_dust: 100,
                    clear_dust: 10,
                    orbs: 1,
                }
                .encode()
                .ok_or(SessionError::SessionUnusable)?
            }
            (195, 379) => {
                let paid = ModificationRequest::decode(command.kind, &command.payload)
                    .ok_or(SessionError::SessionUnusable)?;
                assert_eq!(paid.definition.get(), 258);
                assert_eq!(paid.slot, 1);
                assert_eq!(paid.operation, ModificationOperation::Modify { level: 0 });
                assert_eq!(paid.expected_revision, 123);
                *self
                    .paid
                    .lock()
                    .map_err(|_| SessionError::SessionUnusable)? = Some(paid);
                ModificationReply {
                    result: ModificationResult::Accepted,
                    slot: None,
                }
                .encode()
            }
            _ => return Err(SessionError::SessionUnusable),
        };
        Ok(GatedCommandResult {
            command_id: u64::from(command.kind),
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        })
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
        Ok(None)
    }
}

#[test]
fn proficiency_session_routes_state_from_native_selection_into_exact_paid_revision()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let paid = Arc::new(Mutex::new(None));
                let (link, commands, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        paid: Arc::clone(&paid),
                    },
                    commands,
                    outbox,
                ));
                tokio::time::timeout(Duration::from_secs(2), async {
                    while !link.weapon_proficiency_panel().state_selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.weapon_proficiency_request(Intent::Catalogue);
                tokio::time::timeout(Duration::from_secs(2), async {
                    while link.weapon_proficiency_panel().catalogue.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(link.weapon_proficiency_panel().catalogue, Some(catalogue()));
                // This production entry calls select_weapon, shared with native field-zero dropdown.
                link.weapon_proficiency_select_weapon(0);
                tokio::time::timeout(Duration::from_secs(2), async {
                    while link.weapon_proficiency_panel().shaping.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(link.weapon_proficiency_panel().revision, 123);
                link.weapon_proficiency_request(Intent::Modify);
                tokio::time::timeout(Duration::from_secs(2), async {
                    loop {
                        let panel = link.weapon_proficiency_panel();
                        if !panel.pending && panel.state_refresh_needed {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(paid.lock().map_err(|_| "paid command lock")?.is_some());
                let panel = link.weapon_proficiency_panel();
                assert_eq!(panel.revision, 0);
                assert!(panel.shaping.is_none());
                assert!(!panel.mutation_uncertain);
                drop(link);
                task.await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })
        })?
        .join()
        .map_err(|_| "proficiency session thread panicked")?
}
