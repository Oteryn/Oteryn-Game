use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
type Exchange = (u32, Vec<u8>, CommandStatus, Vec<u8>);
struct Script {
    entries: Mutex<VecDeque<Exchange>>,
    release: tokio::sync::Semaphore,
}
struct Peer {
    exchanges: Arc<Script>,
    snapshot: Vec<u8>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 67
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == 0).then(|| self.snapshot.clone())
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        let (kind, payload, status, reply) = self
            .exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command.capability, 67);
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
        self.exchanges
            .release
            .acquire()
            .await
            .map_err(|_| SessionError::SessionUnusable)?
            .forget();
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status,
            payload: reply,
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
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !link.charm_progression_panel().selected || link.charm_progression_panel().pending {
            tokio::task::yield_now().await;
        }
    })
    .await
    .checked()?;
    Ok(())
}
async fn send(
    link: &PlayLink,
    exchanges: &Arc<Script>,
    intent: Intent,
    kind: u32,
    payload: Vec<u8>,
    status: CommandStatus,
    reply: Vec<u8>,
) -> Result<(), Error> {
    exchanges
        .entries
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back((kind, payload, status, reply));
    link.charm_progression_request(intent);
    assert!(link.charm_progression_panel().pending);
    // A second click before a response must not produce another command.
    link.charm_progression_request(intent);
    exchanges.release.add_permits(1);
    ready(link).await.checked()?;
    assert!(
        exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    );
    Ok(())
}
use oteryn_client_runtime::charm_progression as wire;
use oteryn_client_ui::charm_progression::Intent;
include!("play_pc01_charm_progression_scenario_1.rs");
trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[test]
fn srx2_charm_consumers_server_reply_reaches_native_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()
                .checked()?
                .block_on(async {
                    let catalogue: serde_json::Value =
                        serde_json::from_slice(include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/../../content/charms/charms-00000-00024.json"
                        )))
                        .checked()?;
                    let records = catalogue["records"].as_array().ok_or("Charm records")?;
                    let names = [
                        "adrenaline_burst",
                        "bless",
                        "carnage",
                        "cleanse",
                        "cripple",
                        "fatal_hold",
                        "numb",
                        "scavenge",
                        "void_inversion",
                    ];
                    let race = std::num::NonZeroU32::new(1).ok_or("race")?;
                    let mut charms = Vec::new();
                    for name in names {
                        let key = format!("oteryn:charm.{name}");
                        let (index, row) = records
                            .iter()
                            .enumerate()
                            .find(|(_, row)| {
                                row["definition"]["identity"]["key"].as_str() == Some(key.as_str())
                            })
                            .ok_or("Charm definition")?;
                        charms.push(wire::charm::CharmState {
                            charm: std::num::NonZeroU32::new(u32::try_from(index + 1).checked()?)
                                .ok_or("Charm index")?,
                            kind: if row["definition"]["category"].as_str() == Some("major") {
                                wire::charm::CharmKind::Major
                            } else {
                                wire::charm::CharmKind::Minor
                            },
                            unlocked_stage: 3,
                            assigned_race: Some(race),
                            next_stage_cost: 0,
                            effect_active: true,
                        });
                    }
                    let reply = wire::CharmProgressionReply {
                        outcome: wire::CharmProgressionOutcome::Query,
                        fee_gold: 0,
                        bestiary: vec![],
                        charms: wire::charm::CharmView {
                            charms,
                            charm_points_available: 0,
                            minor_charm_echoes_available: 0,
                            assignment_slot_limit: None,
                        },
                    };
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: vec![],
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await?;
                    link.charm_progression_open();
                    send(
                        &link,
                        &exchanges,
                        Intent::Query,
                        84,
                        wire::encode_charm_progression_request(
                            &wire::CharmProgressionAction::Query,
                        )
                        .checked()?,
                        CommandStatus::Accepted,
                        wire::encode_charm_progression_reply(&reply).checked()?,
                    )
                    .await?;
                    let panel = link.charm_progression_panel();
                    assert!(panel.open);
                    assert_eq!(panel.reply.as_ref(), Some(&reply));
                    let native_text = oteryn_client_ui::charm_progression::panel_text(&panel);
                    for charm in &reply.charms.charms {
                        assert!(native_text.contains(&format!("Charm {}:", charm.charm)));
                    }
                    assert_eq!(native_text.matches("active true").count(), names.len());
                    assert_eq!(native_text.matches("stage 3").count(), names.len());
                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
        })
        .checked()?
        .join()
        .map_err(|_| "Charm client play thread failed")
        .checked()?
}
