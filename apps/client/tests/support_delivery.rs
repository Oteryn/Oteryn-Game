use oteryn_client::{
    about::review_survey_at,
    crash_report::{CrashReport, review_pending_at},
    play::play_channel,
};
#[test]
fn support_consent_reaches_real_play_queue() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("oteryn-support-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let crash = dir.join("pending.txt");
    let approved = dir.join("approved-crash.txt");
    CrashReport::capture(Some("private/path/client.rs"), 42).save(&crash, true)?;
    let (link, mut queue, _) = play_channel();
    link.queue_support_at(&dir);
    assert!(queue.try_recv().is_err());
    assert!(!review_pending_at(&crash, &approved, |_| false)?);
    link.queue_support_at(&dir);
    assert!(queue.try_recv().is_err());
    CrashReport::capture(Some("private/path/client.rs"), 42).save(&crash, true)?;
    assert!(review_pending_at(&crash, &approved, |preview| {
        assert!(!preview.contains("private/path"));
        true
    })?);
    assert!(review_survey_at(&dir.join("approved-survey.txt"), |_| {
        true
    })?);
    link.queue_support_at(&dir);
    assert!(queue.try_recv().is_ok());
    assert!(queue.try_recv().is_ok());
    assert!(queue.try_recv().is_err());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

use oteryn_client::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, run_session},
};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::time::Duration;
struct Peer {
    selected: bool,
    status: CommandStatus,
    reply: Vec<u8>,
    sent: tokio::sync::mpsc::UnboundedSender<(u32, Vec<u8>)>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        self.selected && capability == 462
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        self.sent
            .send((command.kind, command.payload.clone()))
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: self.status,
            payload: self.reply.clone(),
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
        Ok(Some(LogoutResult::Accepted))
    }
}
#[test]
fn support_play_delivery_acknowledges_only_durable_success() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            oteryn_client_runtime::ClientRuntime::new()
                .map_err(|e| format!("{e:?}"))?
                .block_on(async {
                    for (index, selected, status, reply, removed, kind) in [
                        (0, true, CommandStatus::Accepted, vec![1], true, 6144),
                        (5, true, CommandStatus::Accepted, vec![1], true, 6145),
                        (1, true, CommandStatus::DuplicateReplay, vec![1], true, 6145),
                        (2, true, CommandStatus::Accepted, vec![0], false, 6145),
                        (3, true, CommandStatus::Rejected, vec![1], false, 6145),
                        (4, false, CommandStatus::Accepted, vec![1], false, 6145),
                    ] {
                        let dir = std::env::temp_dir().join(format!(
                            "oteryn-support-play-{}-{index}",
                            std::process::id()
                        ));
                        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                        let path = dir.join(if kind == 6144 {
                            "approved-crash.txt"
                        } else {
                            "approved-survey.txt"
                        });
                        if kind == 6144 {
                            let pending = dir.join("pending-crash.txt");
                            CrashReport::capture(Some("client.rs"), 42)
                                .save(&pending, true)
                                .map_err(|e| e.to_string())?;
                            assert!(
                                review_pending_at(&pending, &path, |_| true)
                                    .map_err(|e| e.to_string())?
                            );
                        } else {
                            assert!(review_survey_at(&path, |_| true).map_err(|e| e.to_string())?);
                        }
                        let preview = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                        let (link, commands, outbox) = play_channel();
                        link.queue_support_at(&dir);
                        let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
                        let task = tokio::spawn(run_session(
                            Peer {
                                selected,
                                status,
                                reply,
                                sent,
                            },
                            commands,
                            outbox,
                        ));
                        if selected {
                            let (sent_kind, payload) =
                                tokio::time::timeout(Duration::from_secs(3), received.recv())
                                    .await
                                    .map_err(|e| e.to_string())?
                                    .ok_or("no command")?;
                            assert_eq!(sent_kind, kind);
                            assert_eq!(&payload[1..], preview.as_bytes());
                            assert_eq!(payload[0], 1);
                            if removed {
                                tokio::time::timeout(Duration::from_secs(3), async {
                                    while path.exists() {
                                        tokio::task::yield_now().await;
                                    }
                                })
                                .await
                                .map_err(|e| e.to_string())?;
                            }
                        }
                        link.request_logout();
                        tokio::time::timeout(Duration::from_secs(3), task)
                            .await
                            .map_err(|e| e.to_string())?
                            .map_err(|e| e.to_string())?;
                        assert_eq!(path.exists(), !removed);
                        assert!(received.try_recv().is_err());
                        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                })
                .map_err(|e| format!("{e:?}"))?
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "support play test panicked".to_string())?
}
