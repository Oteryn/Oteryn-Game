#[test]
fn srv7_native_dead_house_retries_transient_refusal_before_temple_resume()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::combat_input::{CombatCommand, CombatReport};
            use crate::play::{PlayView, Stepper, play_channel, run_session};
            use oteryn_client_runtime::{house_transfer as wire, raw_feature::FeatureCommand};
            use oteryn_session::{
                ActorVitals, CommandStatus, GatedCommandResult, LogoutResult, SessionError,
                SessionEvent, StepDirection, StepOutcome,
            };
            use std::sync::{
                Arc,
                atomic::{AtomicBool, AtomicUsize, Ordering},
            };
            struct DeadHouse {
                scope: wire::HouseTransferAck,
                scope_payload: Vec<u8>,
                done: Arc<AtomicBool>,
                attempts: Arc<AtomicUsize>,
            }
            impl Stepper for DeadHouse {
                fn feature_selected(&self, capability: u32) -> bool {
                    capability == 86
                }
                fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
                    (domain == wire::STATE_DOMAIN_HOUSE_SCOPE && !self.done.load(Ordering::SeqCst))
                        .then(|| self.scope_payload.clone())
                }
                fn own_house_vitals_current(&self) -> bool {
                    true
                }
                fn actor_vitals(&self) -> Option<ActorVitals> {
                    Some(ActorVitals {
                        health: if self.done.load(Ordering::SeqCst) {
                            150
                        } else {
                            0
                        },
                        max_health: 150,
                        ..Default::default()
                    })
                }
                async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(
                    &mut self,
                    duration: std::time::Duration,
                ) -> Result<(), SessionError> {
                    tokio::time::sleep(duration).await;
                    Ok(())
                }
                fn drain(&mut self) -> Vec<SessionEvent> {
                    Vec::new()
                }
                async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
                    Ok(None)
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    assert_eq!((command.capability, command.kind), (86, 102));
                    let intent = wire::decode_house_transfer_intent(&command.payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert!(intent.exit);
                    assert_eq!(Some(intent.house.as_str()), self.scope.house.as_deref());
                    if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                        return Ok(GatedCommandResult {
                            command_id: 1,
                            server_sequence: 1,
                            status: CommandStatus::Rejected,
                            payload: Vec::new(),
                        });
                    }
                    let mut temple = self.scope.clone();
                    temple.house = None;
                    temple.position = [32369, 32241, 7];
                    temple.connection_generation += 1;
                    temple.lease_generation += 1;
                    temple.session[15] += 1;
                    let payload = wire::encode_house_transfer_ack(&temple)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    self.done.store(true, Ordering::SeqCst);
                    Ok(GatedCommandResult {
                        command_id: 2,
                        server_sequence: 2,
                        status: CommandStatus::Accepted,
                        payload,
                    })
                }
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let mut id = [0; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id[15] = 1;
                let scope = wire::HouseTransferAck {
                    transfer: id,
                    session: id,
                    world: id,
                    channel: id,
                    house: Some("oteryn:content.house.test".into()),
                    scope_generation: 2,
                    connection_generation: 1,
                    lease_generation: 1,
                    selection: vec![86],
                    position: [100, 200, 7],
                };
                let scope_payload = wire::encode_house_transfer_ack(&scope)
                    .map_err(|e| format!("House snapshot fixture: {e:?}"))?;
                let done = Arc::new(AtomicBool::new(false));
                let attempts = Arc::new(AtomicUsize::new(0));
                let started = std::time::Instant::now();
                let (link, commands, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    DeadHouse {
                        scope,
                        scope_payload,
                        done: done.clone(),
                        attempts: attempts.clone(),
                    },
                    commands,
                    outbox,
                ));
                let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    loop {
                        // Editable fields are never authority for the internal death exit.
                        link.house_transfer_edit(0, "oteryn:content.house.forged".into());
                        assert!(view.tick(&link).is_ok());
                        if done.load(Ordering::SeqCst) && !view.death.dead {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .map_err(|_| "native dead House never issued the existing exit/Temple handoff")?;
                assert_eq!(
                    attempts.load(Ordering::SeqCst),
                    2,
                    "one original refusal then one ordinary exit retry"
                );
                assert!(
                    started.elapsed() >= std::time::Duration::from_millis(900),
                    "native retry must back off rather than busy-loop"
                );
                assert_eq!(link.house_transfer_panel().feedback, "Transfer committed");
                assert!(
                    link.house_transfer_panel()
                        .summary
                        .contains("32369,32241,7")
                );
                assert!(!view.hud_status().contains("You are dead"));
                task.abort();
                let _ = task.await;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            })
        })?
        .join()
        .map_err(|_| "native House death thread panicked")?
}
