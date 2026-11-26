use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::{
    ClientRuntime,
    server_mechanic::{ServerMechanicCommand, ServerMechanicResponse},
};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex};
type Exchanges = std::collections::VecDeque<(Exchange, Vec<SessionEvent>)>;
#[derive(Clone)]
pub(in crate::play::tests) struct Exchange {
    pub command: ServerMechanicCommand,
    pub payload: Vec<u8>,
    pub status: CommandStatus,
}
struct Peer {
    exchanges: Arc<Mutex<Exchanges>>,
    events: Vec<SessionEvent>,
    entities: Vec<oteryn_session::WorldSpatialEntity>,
}
impl Stepper for Peer {
    fn server_mechanic_selected(&self, _: u32) -> bool {
        true
    }
    async fn server_mechanic(
        &mut self,
        command: &ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, SessionError> {
        let mut queue = self
            .exchanges
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (exchange, events) = queue.pop_front().ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command, &exchange.command);
        self.events.extend(events);
        Ok(ServerMechanicResponse {
            command_id: 1,
            status: exchange.status,
            server_sequence: 1,
            payload: exchange.payload,
        })
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn serve(&mut self, _: std::time::Duration) -> Result<(), SessionError> {
        tokio::task::yield_now().await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }
    fn entities_changed(
        &self,
        held: &[oteryn_session::WorldSpatialEntity],
    ) -> Option<Vec<oteryn_session::WorldSpatialEntity>> {
        (self.entities != held).then(|| self.entities.clone())
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}
pub(in crate::play::tests) fn scenario(
    run: impl FnOnce(
        &crate::play::PlayLink,
        &mut dyn FnMut(Exchange),
        &mut dyn FnMut(&dyn Fn() -> bool),
    ) -> Result<(), String>
    + Send
    + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    scenario_with_events(vec![], vec![], |link, push, wait| {
        run(link, &mut |exchange| push(exchange, vec![]), wait)
    })
}

pub(in crate::play::tests) fn scenario_with_events(
    events: Vec<SessionEvent>,
    entities: Vec<oteryn_session::WorldSpatialEntity>,
    run: impl FnOnce(
        &crate::play::PlayLink,
        &mut dyn FnMut(Exchange, Vec<SessionEvent>),
        &mut dyn FnMut(&dyn Fn() -> bool),
    ) -> Result<(), String>
    + Send
    + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::quick_loot::*;
    let startup = Exchange {
        command: ServerMechanicCommand {
            capability: CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
            command_type: COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            payload: encode_quick_loot_request(&quick_loot_filter_review_request())
                .map_err(|error| format!("{error:?}"))?,
        },
        payload: encode_quick_loot_destinations(&QuickLootDestinations {
            loot: vec![QuickLootDestination {
                category: None,
                recipient: QuickLootRecipient::MainContainer,
                recipient_id: None,
            }],
            obtain: vec![],
            fallback_to_main: true,
            filters: Some(QuickLootFilters {
                mode: QuickLootFilterMode::Accept,
                skipped: vec![4],
                accepted: vec![7],
            }),
        })
        .map_err(|error| format!("{error:?}"))?,
        status: CommandStatus::Accepted,
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let runtime = ClientRuntime::new().map_err(|e| e.to_string())?;
            let queue = Arc::new(Mutex::new(Exchanges::from([(startup, vec![])])));
            let (link, rx, outbox) = play_channel();
            let task = runtime
                .spawn(run_session(
                    Peer {
                        exchanges: queue.clone(),
                        events,
                        entities,
                    },
                    rx,
                    outbox,
                ))
                .map_err(|e| e.to_string())?;
            let mut push = |exchange, events| {
                queue
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push_back((exchange, events))
            };
            let mut wait = |condition: &dyn Fn() -> bool| {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !condition() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "panel did not reach expected state"
                    );
                    std::thread::yield_now();
                }
            };
            wait(&|| link.discovery_panel().loot.selected);
            run(&link, &mut push, &mut wait)?;
            assert!(
                queue
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .is_empty()
            );
            drop(link);
            task.abort();
            Ok(())
        })?
        .join()
        .map_err(|_| "scenario thread failed")??;
    Ok(())
}
