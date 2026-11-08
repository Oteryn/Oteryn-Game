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
        capability == 25
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == 21).then(|| self.snapshot.clone())
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
        assert_eq!(command.capability, 25);
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
        while !link.appearance_panel().selected
            || link.appearance_panel().catalogue.is_none()
            || link.appearance_panel().pending
        {
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
    link.appearance_request(intent);
    assert!(link.appearance_panel().pending);
    // A second click before a response must not produce another command.
    link.appearance_request(intent);
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
use oteryn_client_runtime::appearance as wire;
use oteryn_client_ui::appearance::Intent;
include!("play_pc01_appearance_scenario_1.rs");
trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[test]
fn nx5_item_unlock_delta_decodes_into_native_catalogue_and_feedback() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let mut snapshot = wire::Snapshot::decode(include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../crates/protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
                    )))
                    .checked()?;
                    snapshot.catalogue.outfits.truncate(1);
                    snapshot.catalogue.outfits[0].key =
                        "oteryn:content.outfit.citizen_outfits".into();
                    snapshot.catalogue.outfits[0].permitted_addons = 0;
                    snapshot.catalogue.mounts.clear();
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges,
                            snapshot: snapshot.encode().checked()?,
                        },
                        commands,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while link.appearance_panel().catalogue.is_none() {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .checked()?;
                    let draft = link.appearance_panel().fields;
                    let mut next = snapshot.catalogue;
                    next.outfits.push(wire::OutfitOption {
                        key: "oteryn:content.outfit.soil_guardian_outfits".into(),
                        look_type: 516,
                        permitted_addons: 3,
                    });
                    next.mounts
                        .push("oteryn:content.mount.singeing_steed".into());
                    // The exact server catalogue codec feeds the same event the Session emits.
                    let decoded = wire::Catalogue::decode(&next.encode().checked()?).checked()?;
                    outbox.push(vec![SessionEvent::Appearance(
                        oteryn_session::AppliedDelta {
                            server_sequence: 42,
                            base_revision: 0,
                            new_revision: 1,
                            value: oteryn_session::AppearanceUpdate::Catalogue(decoded),
                        },
                    )]);
                    let panel = link.appearance_panel();
                    let text = oteryn_client_ui::appearance::panel_text(&panel);
                    assert!(text.contains("soil_guardian_outfits; allowed addon mask: 3"));
                    assert!(text.contains("singeing_steed"));
                    assert!(
                        text.contains(
                            "Unlocked outfit: oteryn:content.outfit.soil_guardian_outfits"
                        )
                    );
                    assert!(text.contains("Unlocked mount: oteryn:content.mount.singeing_steed"));
                    assert_eq!(
                        panel.fields, draft,
                        "an unlock preserves the current selection draft"
                    );
                    // Repeated catalogue delivery is silent rather than a second success notice.
                    outbox.push(vec![SessionEvent::Appearance(
                        oteryn_session::AppliedDelta {
                            server_sequence: 43,
                            base_revision: 1,
                            new_revision: 2,
                            value: oteryn_session::AppearanceUpdate::Catalogue(next),
                        },
                    )]);
                    assert_eq!(link.appearance_panel().fields, draft);
                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()??;
            Ok(())
        })?
        .join()
        .map_err(|_| "NX-5 appearance play thread")?
}
