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
    response_identity: (u64, u64),
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
            command_id: self.response_identity.0,
            status: exchange.status,
            server_sequence: self.response_identity.1,
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

#[tokio::test]
async fn quick_loot_restore_refusal_keeps_saved_settings() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_session::quick_loot::*;
    let (link, _, outbox) = play_channel();
    let exchange = Exchange {
        command: ServerMechanicCommand {
            capability: CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
            command_type: COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            payload: encode_quick_loot_request(&quick_loot_filter_review_request())
                .map_err(|error| format!("{error:?}"))?,
        },
        payload: encode_quick_loot_refusal(QuickLootRefusal::Unavailable),
        status: CommandStatus::Rejected,
    };
    let mut peer = Peer {
        response_identity: (1, 1),
        exchanges: Arc::new(Mutex::new(Exchanges::from([(exchange, vec![])]))),
        events: vec![],
        entities: vec![],
    };
    outbox.discovery_initialize(&peer);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let panel = &mut mailbox.discovery.loot;
        panel.filter_mode = Some(QuickLootFilterMode::Skip);
        panel.skipped.insert(4097);
        panel
            .destinations
            .push(oteryn_client_ui::quick_loot::Action::Destination {
                category: Some(QuickLootCategory::Gold),
                recipient: QuickLootRecipient::MainContainer,
                id: None,
                fallback: true,
            });
    }
    let mut expected = link.quick_loot_panel();
    expected.feedback = "The server refused the request".into();
    outbox.restore_quick_loot(&mut peer).await?;
    assert_eq!(link.quick_loot_panel(), expected);
    Ok(())
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
    scenario_with_startup(startup, (1, 1), events, entities, run)
}

fn scenario_with_startup(
    startup: Exchange,
    response_identity: (u64, u64),
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
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let runtime = ClientRuntime::new().map_err(|e| e.to_string())?;
            let queue = Arc::new(Mutex::new(Exchanges::from([(startup, vec![])])));
            let (link, rx, outbox) = play_channel();
            let task = runtime
                .spawn(run_session(
                    Peer {
                        response_identity,
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

#[cfg(bevy_shell)]
pub(super) fn restore_active_capture(
    outbox: &crate::play::Outbox,
    exchange: Exchange,
    response_identity: (u64, u64),
) -> Result<(), Box<dyn std::error::Error>> {
    let queue = Arc::new(Mutex::new(Exchanges::from([(exchange, vec![])])));
    let mut peer = Peer {
        response_identity,
        exchanges: queue.clone(),
        events: vec![],
        entities: vec![],
    };
    outbox.discovery_initialize(&peer);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(outbox.restore_quick_loot(&mut peer))?;
    assert!(
        queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    );
    Ok(())
}

#[cfg(bevy_shell)]
pub(super) fn restore_capture(
    exchange: Exchange,
    response_identity: (u64, u64),
) -> Result<oteryn_client_ui::quick_loot::Panel, Box<dyn std::error::Error>> {
    let (send, receive) = std::sync::mpsc::channel();
    scenario_with_startup(
        exchange,
        response_identity,
        vec![],
        vec![],
        move |link, _, wait| {
            wait(&|| link.quick_loot_panel().filter_mode.is_some());
            send.send(link.quick_loot_panel())
                .map_err(|error| error.to_string())
        },
    )?;
    Ok(receive.recv()?)
}
