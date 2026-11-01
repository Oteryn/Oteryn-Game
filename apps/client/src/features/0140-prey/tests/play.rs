use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::prey::{PreyResponse, prey};
use oteryn_client_ui::prey::Intent;
use oteryn_session::{CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    reply: prey::Reply,
    commands: Arc<Mutex<Vec<prey::Request>>>,
}

impl Stepper for Peer {
    fn prey_selected(&self) -> bool { true }
    async fn prey(&mut self, request: &prey::Request) -> Result<PreyResponse, SessionError> {
        let bad = || SessionError::SessionUnusable;
        let payload = prey::encode_request(request).map_err(|_| bad())?;
        let request = prey::decode_request(&payload).map_err(|_| bad())?;
        self.commands.lock().unwrap_or_else(PoisonError::into_inner).push(request.clone());
        self.reply.offered_today.clear();
        match &request {
            prey::Request::Query => {}
            prey::Request::SetTier { slot, tier } => self.reply.slots[usize::from(*slot - 1)].chosen_tier = *tier,
            prey::Request::OfferedToday { .. } => self.reply.offered_today.push(prey::OfferedCreature {
                creature: "oteryn:creature.dragon".into(), price_gold: 45000, occupied_slot: 0,
            }),
            prey::Request::PickAgain { slot, creature, expected_price_gold } => {
                assert_eq!(*expected_price_gold, 45000);
                let state = &mut self.reply.slots[usize::from(*slot - 1)];
                state.selected.clone_from(creature);
                state.state = 2;
                state.hunting_seconds = 7200;
                state.selection_epoch = 12;
            }
            _ => return Err(bad()),
        }
        let payload = prey::encode_reply(&self.reply).map_err(|_| bad())?;
        Ok(PreyResponse { request, status: CommandStatus::Accepted, current: Some(payload.clone()), payload })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await; Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> { Vec::new() }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> { Err(SessionError::SessionUnusable) }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> { Err(SessionError::SessionUnusable) }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> { Ok(None) }
}

async fn ready(link: &PlayLink, predicate: impl Fn(&oteryn_client_ui::prey::PreyPanel) -> bool) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.prey_panel();
            if panel.selected && panel.pending.is_none() && predicate(&panel) { break; }
            tokio::task::yield_now().await;
        }
    }).await?;
    Ok(())
}

#[test]
fn prey_controls_reach_session_queue_and_apply_confirmed_difficulty_and_paid_pick() -> Result<(), Error> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let commands = Arc::new(Mutex::new(Vec::new()));
            let peer = Peer {
                commands: Arc::clone(&commands),
                reply: prey::Reply {
                    status: 0, wildcard_balance: 4,
                    slots: vec![prey::Slot {
                        slot: 1, state: 1, creatures: vec!["oteryn:creature.rat".into()],
                        selected: String::new(), bonus: 0, grade: 0, percent: 0,
                        hunting_seconds: 0, option: 0, free_reroll_at: 0, chosen_tier: 1,
                        allowed_tier: 3, selection_epoch: 0,
                    }],
                    list_size: 12, expansion_owned: true, offered_today: Vec::new(), offered_next: String::new(),
                },
            };
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, |_| true).await?;
            link.prey_request(Intent::Refresh);
            ready(&link, |panel| panel.confirmed.is_some()).await?;
            let before = link.prey_panel().slot(1).ok_or("slot")?.creatures.clone();
            link.prey_request(Intent::SetTier { slot: 1, tier: 3 });
            ready(&link, |panel| panel.slot(1).is_some_and(|slot| slot.chosen_tier == 3)).await?;
            assert_eq!(link.prey_panel().slot(1).ok_or("slot")?.creatures, before);
            link.prey_request(Intent::OfferedToday { slot: 1, after: String::new() });
            ready(&link, |panel| panel.offered_today.len() == 1).await?;
            let confirmation = crate::play::mailbox::lock(&link.mailbox).prey.confirm_offer(0).ok_or("quote")?;
            assert_eq!(confirmation.price_gold, 45000);
            assert_eq!(commands.lock().unwrap_or_else(PoisonError::into_inner).len(), 3);
            link.prey_request(Intent::ConfirmPickAgain(confirmation));
            ready(&link, |panel| panel.slot(1).is_some_and(|slot| slot.selected == "oteryn:creature.dragon")).await?;
            let recorded = commands.lock().unwrap_or_else(PoisonError::into_inner).clone();
            assert_eq!(recorded, vec![
                prey::Request::Query,
                prey::Request::SetTier { slot: 1, tier: 3 },
                prey::Request::OfferedToday { slot: 1, after: String::new() },
                prey::Request::PickAgain { slot: 1, creature: "oteryn:creature.dragon".into(), expected_price_gold: 45000 },
            ]);
            assert_eq!(link.prey_panel().confirmed.as_ref().ok_or("reply")?.list_size, 12);
            drop(link);
            task.await?;
            Ok::<(), Error>(())
        })?
    })?.join().map_err(|_| "Prey UI test thread panicked")?
}
