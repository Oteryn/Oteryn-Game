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
#[test]
fn pc01_appearance_scenario() -> Result<(), Error> {
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
                    snapshot.catalogue.palette = vec![[0, 0, 0], [255, 255, 255]];
                    snapshot.catalogue.outfits[0].permitted_addons = 3;
                    snapshot.catalogue.mounts = vec!["oteryn:mount.horse".into()];
                    let bytes = snapshot.encode().checked()?;
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, mut incoming, outbox) = play_channel();
                    link.appearance_request(Intent::Apply);
                    assert!(!link.appearance_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: bytes.clone(),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(4).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.appearance_input(&[event]));
                    assert!(link.appearance_panel().open);
                    let catalogue = link
                        .appearance_panel()
                        .catalogue
                        .ok_or("catalogue missing")?;
                    let outfit = catalogue.outfits.first().ok_or("outfit missing")?;
                    for (i, value) in [
                        (0, outfit.key.clone()),
                        (1, "1".into()),
                        (2, "oteryn:mount.horse".into()),
                        (3, "0,1,0,1".into()),
                        (4, "r1".into()),
                    ] {
                        link.appearance_edit(i, value);
                    }
                    let expected = wire::Change {
                        context: catalogue.context,
                        expected_root_revision: catalogue.root_revision,
                        prior: catalogue.selected.clone(),
                        selection: wire::Selection {
                            outfit_key: outfit.key.clone(),
                            colours: [0, 1, 0, 1],
                            addons: 1,
                            mount_key: Some("oteryn:mount.horse".into()),
                        },
                        content_revision: "r1".into(),
                    };
                    send(
                        &link,
                        &exchanges,
                        Intent::Apply,
                        25,
                        expected.encode().checked()?,
                        CommandStatus::Accepted,
                        wire::ChangeResult::Committed(2).encode().checked()?,
                    )
                    .await
                    .checked()?;
                    assert!(link.appearance_panel().feedback.contains("committed"));
                    // A result is not a catalogue; the next request uses the pushed revision/prior.
                    let mut next = catalogue.clone();
                    next.root_revision = 2;
                    next.selected = Some(wire::Selected {
                        selection: expected.selection.clone(),
                        revision: 2,
                        provenance: "r1".into(),
                    });
                    outbox.push(vec![SessionEvent::Appearance(
                        oteryn_session::AppliedDelta {
                            server_sequence: 2,
                            base_revision: 1,
                            new_revision: 2,
                            value: oteryn_session::AppearanceUpdate::Catalogue(next.clone()),
                        },
                    )]);
                    assert_eq!(link.appearance_panel().confirmed, next.selected);
                    let expected = wire::Change {
                        expected_root_revision: 2,
                        prior: next.selected,
                        ..expected
                    };
                    let payload = expected.encode().checked()?;
                    let intent = Intent::Apply;
                    let refused = wire::ChangeResult::Rejected.encode().checked()?;

                    send(
                        &link,
                        &exchanges,
                        intent,
                        25,
                        payload.clone(),
                        CommandStatus::Accepted,
                        refused,
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.appearance_panel()
                            .feedback
                            .to_lowercase()
                            .contains("refused")
                    );
                    send(
                        &link,
                        &exchanges,
                        intent,
                        25,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.appearance_panel()
                            .feedback
                            .to_lowercase()
                            .contains("refused")
                    );
                    send(
                        &link,
                        &exchanges,
                        intent,
                        25,
                        payload,
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.appearance_panel().feedback.contains("Invalid"));
                    link.appearance_edit(0, "invalid".into());
                    link.appearance_request(Intent::Apply);
                    assert!(!link.appearance_panel().pending);
                    // Ending with a request in flight clears pending, selection, drafts and confirmed state.
                    link.appearance_edit(0, outfit.key.clone());
                    link.appearance_request(Intent::Apply);
                    assert!(link.appearance_panel().pending);
                    task.await.checked()?;
                    let ended = link.appearance_panel();
                    assert!(!ended.pending);
                    assert!(!ended.selected);
                    assert!(!ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    // Re-admit on this mailbox with a fresh peer. No old selection/confirmation survives.
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnect = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: bytes,
                        },
                        receiver,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    assert!(!link.appearance_panel().open);
                    assert!(!link.appearance_panel().pending);
                    assert_eq!(
                        link.appearance_panel().confirmed,
                        snapshot.catalogue.selected
                    );
                    drop(sender);
                    reconnect.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })
        .checked()?
        .join()
        .map_err(|_| "scenario thread failed")
        .checked()?
}
trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[path = "play_additional.rs"]
mod additional;
