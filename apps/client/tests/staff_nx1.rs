//! Real native session driver: imported signatures remain off-host authority.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    calls: Arc<AtomicUsize>,
    tools: bool,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 51 || (self.tools && capability == 330)
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            index < if self.tools { 4 } else { 3 },
            "duplicate submission"
        );
        assert_eq!(
            (command.capability, command.kind),
            (
                if self.tools { 330 } else { 51 },
                if self.tools {
                    if index == 1 { 561 } else { 560 }
                } else if index == 1 {
                    61
                } else {
                    60
                }
            )
        );
        let envelope = oteryn_session::property_services::staff::decode_envelope(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        assert_eq!(envelope.principal, "operator");
        assert_eq!(envelope.canonical, b" signed request ");
        assert_eq!(envelope.signature, [1, 2]);
        // Refusal, receipt acknowledgement, then a malformed reply.
        let mut payload = vec![
            8,
            if index == 1 {
                3
            } else if index == 3 {
                1
            } else {
                2
            },
            16,
            2,
            26,
            32,
        ];
        payload.extend([1; 32]);
        payload.extend([32, 0, 40, if index == 1 || index == 3 { 1 } else { 2 }]);
        if self.tools {
            let result = oteryn_session::property_services::staff::decode_result(&payload)
                .map_err(|_| SessionError::SessionUnusable)?;
            payload = oteryn_session::property_services::staff_tools::encode_reply(
                &oteryn_session::property_services::staff_tools::StaffToolsReply {
                    result,
                    info: if index == 3 {
                        "Character level 42; position (100,100,7); group God".into()
                    } else {
                        String::new()
                    },
                },
            )
            .map_err(|_| SessionError::SessionUnusable)?;
        }
        if index == 2 {
            payload.push(0);
        }
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
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
fn nx1_staff_native_driver_decodes_refusal_and_clears_authority_on_end() -> Result<(), Error> {
    scenario(false)
}
#[test]
fn nx1_full_staff_native_driver_negotiates_tools_and_shows_inspection() -> Result<(), Error> {
    scenario(true)
}
fn scenario(tools: bool) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()
                .map_err(|e| format!("{e:?}"))?
                .block_on(async {
                    let (link, incoming, outbox) = play_channel();
                    link.staff_submit(false);
                    assert!(!link.staff_panel().pending);
                    let calls = Arc::new(AtomicUsize::new(0));
                    let task = tokio::spawn(run_session(
                        Peer {
                            calls: Arc::clone(&calls),
                            tools,
                        },
                        incoming,
                        outbox,
                    ));
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while !link.staff_panel().selected {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    link.staff_edit(2, "not hex".into());
                    link.staff_submit(false);
                    assert!(!link.staff_panel().pending);
                    assert_eq!(calls.load(Ordering::SeqCst), 0);
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(39)
                            .map_err(|e| format!("{e:?}"))?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.mail_input(&[event]));
                    assert!(link.staff_panel().open);
                    for (i, value) in ["operator", " signed request ", "0102"]
                        .into_iter()
                        .enumerate()
                    {
                        link.staff_edit(i, value.into());
                    }
                    link.staff_submit(false);
                    assert!(link.staff_panel().pending);
                    link.staff_edit(0, "changed".into());
                    assert_eq!(link.staff_panel().fields[0], "operator");
                    link.staff_submit(false);
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while link.staff_panel().pending {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    assert!(link.staff_panel().feedback.contains("revoked"));
                    assert!(link.staff_panel().summary.contains("2"));
                    assert_eq!(calls.load(Ordering::SeqCst), 1);
                    assert!(link.staff_panel().fields.iter().all(String::is_empty));
                    for (receipt, expected) in
                        [(true, "receipt stored"), (false, "Invalid staff reply")]
                    {
                        for (i, value) in ["operator", " signed request ", "0102"]
                            .into_iter()
                            .enumerate()
                        {
                            link.staff_edit(i, value.into());
                        }
                        link.staff_submit(receipt);
                        tokio::time::timeout(Duration::from_secs(3), async {
                            while link.staff_panel().pending {
                                tokio::task::yield_now().await;
                            }
                        })
                        .await?;
                        assert!(link.staff_panel().feedback.contains(expected));
                    }
                    assert_eq!(calls.load(Ordering::SeqCst), 3);
                    if tools {
                        for (i, value) in ["operator", " signed request ", "0102"]
                            .into_iter()
                            .enumerate()
                        {
                            link.staff_edit(i, value.into());
                        }
                        link.staff_submit(false);
                        tokio::time::timeout(Duration::from_secs(3), async {
                            while link.staff_panel().pending {
                                tokio::task::yield_now().await;
                            }
                        })
                        .await?;
                        assert!(link.staff_panel().feedback.contains("level 42"));
                        assert!(link.staff_panel().feedback.contains("position (100,100,7)"));
                        assert_eq!(calls.load(Ordering::SeqCst), 4);
                    }
                    link.request_logout();
                    tokio::time::timeout(Duration::from_secs(3), task).await??;
                    assert!(!link.staff_panel().selected);
                    assert!(!link.staff_panel().open);
                    assert!(link.staff_panel().fields.iter().all(String::is_empty));
                    Ok(())
                })
                .map_err(|e| format!("{e:?}"))?
        })?
        .join()
        .map_err(|_| "native staff test panicked")?
}
