use super::*;
use oteryn_client_runtime::boss_encounter::{
    bosstiary_slots::{BossSlotReply, BossSlotRequest, SlotDetails},
    catalogue::{Entry, Page, Query},
};
use oteryn_client_ui::boss_encounter::Intent;
struct SlotPeer {
    commands: Commands,
    assigned: String,
    now: u64,
    ready: u64,
    refuse_once: bool,
    removed: bool,
}
impl Stepper for SlotPeer {
    fn feature_selected(&self, id: u32) -> bool {
        id == 117
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((command.kind, command.payload.clone()));
        self.now += 1000;
        let payload = if Query::decode(&command.payload).is_some() {
            Page {
                next: None,
                points: 600,
                entries: [
                    ("oberon", "Grand Master Oberon", 60),
                    ("drume", "Drume", 5),
                    ("new_boss", "New Boss", 0),
                ]
                .into_iter()
                .map(|(key, name, kills)| Entry {
                    key: format!("oteryn:creature.{key}"),
                    name: name.into(),
                    category: 2,
                    kills,
                    thresholds: [5, 20, 60],
                    points: [10, 40, 100],
                    look: 1250,
                    object: false,
                })
                .collect(),
            }
            .encode()
        } else {
            let Some(BossSlotRequest::Detailed(inner)) = BossSlotRequest::decode(&command.payload)
            else {
                return Err(SessionError::SessionUnusable);
            };
            let mut status = 0;
            match *inner {
                BossSlotRequest::Query => {}
                BossSlotRequest::Equip { slot: 1, boss } => {
                    if self.refuse_once {
                        self.refuse_once = false;
                        status = 1;
                    } else {
                        self.assigned = boss;
                        self.ready = self.now + 490;
                    }
                }
                BossSlotRequest::Remove { slot: 1 } => {
                    self.assigned.clear();
                    self.ready = self.now + 490;
                    self.removed = true;
                }
                _ => return Err(SessionError::SessionUnusable),
            }
            BossSlotReply {
                status,
                unlocked: 1,
                bonus: 67,
                points: 600,
                remove_fee: if self.removed { 100000 } else { 0 },
                ready_at_ms: self.ready,
                bosses: [self.assigned.clone(), String::new()],
                details: Some(SlotDetails {
                    base_bonus: 67,
                    bonuses: [if self.assigned.is_empty() { 0 } else { 92 }, 0],
                    boosted: "oteryn:creature.drume".into(),
                    now: self.now,
                }),
            }
            .encode()
        }
        .ok_or(SessionError::SessionUnusable)?;
        Ok(GatedCommandResult {
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
#[test]
fn boss_slots_real_caller_selects_named_eligible_bosses_and_confirms_equipment_bonuses()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    SlotPeer {
                        commands: Arc::clone(&commands),
                        assigned: String::new(),
                        now: 0,
                        ready: 0,
                        refuse_once: true,
                        removed: false,
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, false).await?;
                for intent in [Intent::Catalogue, Intent::SlotsDetailed] {
                    link.boss_encounter_request(intent);
                    ready(&link, false).await?;
                }
                link.boss_slot_select_slot();
                link.boss_slot_select_boss(false);
                let panel = link.boss_encounter_panel();
                assert_eq!(panel.fields[0], "oteryn:creature.oberon");
                assert_eq!(panel.fields[1], "1");
                assert_eq!(
                    oteryn_client_ui::boss_encounter::slot_candidates(&panel).len(),
                    1
                );
                let text = panel_text(&panel);
                assert!(text.contains("Grand Master Oberon"));
                assert!(text.contains("Daily boosted boss: Drume — cannot be assigned"));
                assert!(text.contains("Slot 2: Locked"));
                let before = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len();
                for key in ["oteryn:creature.drume", "oteryn:creature.new_boss"] {
                    link.boss_encounter_edit(0, key.into());
                    link.boss_encounter_request(Intent::SlotEquipSelected);
                    assert!(!link.boss_encounter_panel().pending);
                }
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    before
                );
                link.boss_slot_select_boss(false);
                link.boss_encounter_request(Intent::SlotEquipSelected);
                ready(&link, false).await?;
                assert!(
                    link.boss_encounter_panel()
                        .slots
                        .as_ref()
                        .is_some_and(|s| s.status == 1 && s.bosses[0].is_empty())
                );
                link.boss_encounter_request(Intent::SlotEquipSelected);
                ready(&link, false).await?;
                let panel = link.boss_encounter_panel();
                assert!(
                    panel
                        .slots
                        .as_ref()
                        .is_some_and(|s| s.bosses[0] == "oteryn:creature.oberon")
                );
                let text = panel_text(&panel);
                assert!(text.contains("Slot 1: Grand Master Oberon | +92% equipment loot"));
                assert!(text.contains("Confirmed bonus: 67%"));
                assert!(text.contains("Removal fee: 0 gold"));
                let before = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len();
                link.boss_encounter_request(Intent::SlotRemoveSelected);
                assert!(!link.boss_encounter_panel().pending);
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    before
                );
                link.boss_encounter_request(Intent::SlotsDetailed);
                ready(&link, false).await?;
                link.boss_encounter_request(Intent::SlotRemoveSelected);
                ready(&link, false).await?;
                let panel = link.boss_encounter_panel();
                assert!(
                    panel
                        .slots
                        .as_ref()
                        .is_some_and(|s| s.bosses[0].is_empty() && s.remove_fee == 100000)
                );
                assert!(panel_text(&panel).contains("Slot 1: Empty | +0% equipment loot"));
                let sent = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                assert!(
                    sent.iter()
                        .any(|(kind, bytes)| *kind == 491 && bytes.starts_with(&[3, 1, 1]))
                );
                assert!(
                    sent.iter()
                        .any(|(kind, bytes)| *kind == 491 && *bytes == [3, 2, 1])
                );
                {
                    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                    mailbox.cancel_activity_requests();
                    assert!(mailbox.boss_encounter.slots.is_none());
                    assert!(mailbox.boss_encounter.boss_names.is_empty());
                }
                drop(link);
                task.await?;
                Ok(())
            })?
        })
        .map_err(|e| -> Error { Box::new(e) })?
        .join()
        .map_err(|_| "boss slot test thread panicked")?
}
