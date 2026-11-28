use super::{HostOwner, keys};
use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, run_session};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome, battle_instinct_status as wire,
};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(serde::Deserialize)]
struct Capture {
    run: String,
    session: String,
    four: [u64; 2],
    four_status: u32,
    four_payload: Vec<u8>,
    five: [u64; 2],
    five_status: u32,
    five_payload: Vec<u8>,
    rejected: [u64; 2],
    rejected_status: u32,
    rejected_payload: Vec<u8>,
}

fn validated_capture(bytes: &[u8], expected: &str, run: &str) -> Result<Capture, Error> {
    let capture: Capture = serde_json::from_slice(bytes)?;
    let digest = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let [four, five, rejected] = [capture.four[1], capture.five[1], capture.rejected[1]];
    let binding = format!(
        "run={} session={} sequences={four},{five},{rejected} digest={digest}",
        capture.run, capture.session,
    );
    if capture.run != run || binding != expected {
        return Err("C11 capture does not match the live run evidence".into());
    }
    if capture.four[0] == 0
        || four == 0
        || !(four < five && five < rejected)
        || !(capture.four[0] < capture.five[0] && capture.five[0] < capture.rejected[0])
        || capture.four_status != CommandStatus::Accepted as u32
        || capture.five_status != CommandStatus::Accepted as u32
        || capture.rejected_status != CommandStatus::Rejected as u32
        || !capture.rejected_payload.is_empty()
    {
        return Err("C11 capture has invalid reply order or refusal payload".into());
    }
    Ok(capture)
}

#[test]
fn c11_capture_rejects_swapped_runs_and_mutated_bytes() -> Result<(), Error> {
    let bytes = br#"{"run":"run-one","session":"session-one","four":[1,4],"four_status":1,"four_payload":[1],"five":[2,5],"five_status":1,"five_payload":[2],"rejected":[3,6],"rejected_status":2,"rejected_payload":[]}"#;
    let digest = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let binding = format!("run=run-one session=session-one sequences=4,5,6 digest={digest}");
    validated_capture(bytes, &binding, "run-one")?;
    assert!(validated_capture(bytes, &binding, "run-two").is_err());
    assert!(
        validated_capture(
            bytes,
            &binding.replace("session-one", "session-two"),
            "run-one"
        )
        .is_err()
    );
    assert!(validated_capture(bytes, &binding.replace("4,5,6", "4,5,7"), "run-one").is_err());
    let mut changed = bytes.to_vec();
    changed.push(b' ');
    assert!(validated_capture(&changed, &binding, "run-one").is_err());
    Ok(())
}

struct CapturedPeer {
    replies: VecDeque<GatedCommandResult>,
    delivered: Arc<AtomicUsize>,
}

impl Stepper for CapturedPeer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1
    }

    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        assert_eq!(
            (command.capability, command.kind, command.payload.as_slice()),
            (
                wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
                wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
                &[][..]
            )
        );
        let reply = self
            .replies
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        self.delivered.fetch_add(1, Ordering::Relaxed);
        Ok(reply)
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }

    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}

#[test]
#[ignore = "requires a live C11 run capture and its independently logged binding"]
fn c11_live_native_replay() -> Result<(), Error> {
    let dir = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C11 capture directory missing")?;
    let bytes = std::fs::read(Path::new(&dir).join("c11-capture.json"))?;
    let run = std::env::var("C11_CAPTURE_RUN")?;
    let binding = std::env::var("C11_CAPTURE_EVIDENCE")?;
    let capture = validated_capture(&bytes, &binding, &run)?;
    assert!(validated_capture(&bytes, &binding, "wrong-run").is_err());
    assert!(validated_capture(&bytes, "wrong-binding", &run).is_err());
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async move {
                let (mut app, commands, outbox) =
                    super::super::super::tests::app().map_err(|error| error.to_string())?;
                let responses = [
                    (capture.four, capture.four_payload),
                    (capture.five, capture.five_payload),
                ];
                let mut results = VecDeque::from(responses.map(
                    |([command_id, server_sequence], payload)| GatedCommandResult {
                        command_id,
                        server_sequence,
                        status: CommandStatus::Accepted,
                        payload,
                    },
                ));
                let [command_id, server_sequence] = capture.rejected;
                results.push_back(GatedCommandResult {
                    command_id,
                    server_sequence,
                    status: CommandStatus::Rejected,
                    payload: capture.rejected_payload,
                });
                // A client-side decoder fault, separate from the live refusal evidence.
                results.push_back(GatedCommandResult {
                    command_id: command_id.checked_add(1).ok_or("C11 command id overflow")?,
                    server_sequence: server_sequence.checked_add(1).ok_or("C11 sequence overflow")?,
                    status: CommandStatus::Accepted,
                    payload: vec![2],
                });
                let delivered = Arc::new(AtomicUsize::new(0));
                let peer = CapturedPeer {
                    replies: results,
                    delivered: Arc::clone(&delivered),
                };
                let task = tokio::spawn(run_session(peer, commands, outbox));
                let link = |app: &bevy::prelude::App| {
                    // The app's actual link is used for both input and HUD projection.
                    app.world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .map(|play| play.link.battle_instinct_status_panel())
                };
                let input = NormalizedInputEvent::Key {
                    code: KeyCode::new(5)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control)
                        .with(Modifier::Alt)
                        .with(Modifier::Shift),
                    repeat: false,
                };
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link(&app).is_some_and(|panel| panel.selected) {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link(&app).is_some_and(|panel| !panel.open && panel.snapshot.is_none()));
                assert!(
                    !keys(&mut app)
                        .map_err(|error| error.to_string())?
                        .iter()
                        .any(|key| key == "Battle Instinct")
                );
                let owner = app.world().non_send::<HostOwner>();
                assert!(
                    owner
                        .play
                        .as_ref()
                        .ok_or("native play link missing")?
                        .link
                        .battle_instinct_status_input(std::slice::from_ref(&input))
                );
                // The first HUD projection queues its normal refresh, which consumes the five-actor reply.
                for (melee, shielding, label, state) in
                    [(0, 0, "Instinct ready", "Eligible"), (1, 6, "Instinct", "Active")]
                {
                    let expected = wire::BattleInstinctStatus {
                        active: true,
                        melee,
                        shielding,
                    };
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while !link(&app).is_some_and(|panel| {
                            panel.open
                                && !panel.pending
                                && panel.error.is_none()
                                && panel.snapshot == Some(expected)
                        }) {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    assert_eq!(
                        oteryn_client_ui::battle_instinct_status::panel_text(
                            &link(&app).ok_or("native play link missing")?,
                        ),
                        format!("Battle Instinct: {state}\r\nMelee: +{melee}\r\nShielding: +{shielding}\r\n")
                    );
                    app.update();
                    assert!(
                        keys(&mut app)
                            .map_err(|error| error.to_string())?
                            .iter()
                            .any(|key| key == "Battle Instinct")
                    );
                    let mut rows = app.world_mut().query::<&super::super::StatusRow>();
                    assert!(rows.iter(app.world()).any(|row| {
                        row.0
                            .iter()
                            .any(|icon| icon.key == "Battle Instinct" && icon.label == label)
                    }));
                }
                for (count, failure) in [(3, "live refusal"), (4, "accepted malformed payload")] {
                    app.world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .ok_or("native play link missing")?
                        .link
                        .battle_instinct_status_request();
                    let panel = tokio::time::timeout(Duration::from_secs(3), async {
                        loop {
                            if let Some(panel) = link(&app)
                                && !panel.pending
                                && delivered.load(Ordering::Relaxed) == count
                            {
                                break panel;
                            }
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    assert_eq!(
                        panel.snapshot,
                        Some(wire::BattleInstinctStatus { active: true, melee: 1, shielding: 6 }),
                        "{failure}"
                    );
                    assert_eq!(
                        oteryn_client_ui::battle_instinct_status::panel_text(&panel),
                        "Battle Instinct could not be refreshed. Try Refresh.",
                        "{failure}"
                    );
                }
                let owner = app.world().non_send::<HostOwner>();
                owner
                    .play
                    .as_ref()
                    .ok_or("native play link missing")?
                    .link
                    .request_logout();
                task.await?;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "C11 replay worker panicked")?
}
