//! Actual production loop: a resource button reads, explicit confirmation writes181 once.
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{
        Stepper,
        feature_0152_item_forge::input::controller::{Action, activate},
        mailbox::lock,
        play_channel, run_session,
    },
};
use oteryn_client_runtime::{
    cui3::item_forge::{Operation, Request},
    forge_preview as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    server_mechanic::{ServerMechanicCommand, ServerMechanicResponse},
};
use oteryn_client_ui::item_forge::{
    reply::Reply,
    resource_dialog::{Content, dialog_text},
};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
fn receipt() -> serde_json::Value {
    serde_json::json!({"operation":"dust_to_slivers","succeeded":true,"applied":true,"reason":null,"bonus":0,"result":{"first_tier":null,"second_tier":null,"dust":40,"dust_limit":100,"gold_spent":0,"cores_spent":0,"slivers_spent":0,"slivers_created":3,"cores_created":0}})
}
struct Peer(Arc<AtomicUsize>);
impl Stepper for Peer {
    fn feature_selected(&self, c: u32) -> bool {
        c == 152
    }
    fn server_mechanic_selected(&self, c: u32) -> bool {
        c == 152
    }
    async fn feature(&mut self, c: &FeatureCommand) -> Result<GatedCommandResult, SessionError> {
        assert_eq!((c.capability, c.kind), (152, 6116));
        let request = wire::Request::decode(&c.payload).ok_or(SessionError::SessionUnusable)?;
        assert_eq!(
            request.selection,
            wire::Selection::Resource(Operation::DustToSlivers)
        );
        let before = wire::Resources {
            dust: 100,
            dust_limit: 100,
            ..Default::default()
        };
        let quote = wire::ResourceQuote {
            operation: Operation::DustToSlivers,
            full_resources: true,
            before,
            after_dust: 40,
            after_limit: 100,
            after_slivers: 3,
            after_cores: 0,
        };
        Ok(GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: wire::Reply {
                nonce: request.nonce,
                payload: wire::Payload::Resource(quote),
            }
            .encode()
            .ok_or(SessionError::SessionUnusable)?,
        })
    }
    async fn server_mechanic(
        &mut self,
        c: &ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, SessionError> {
        assert_eq!((c.capability, c.command_type), (152, 181));
        let request = Request::decode(&c.payload).ok_or(SessionError::SessionUnusable)?;
        let payload = if request.operation == Operation::QueryHistory {
            assert_eq!(request.occurrence, [0; 16]);
            serde_json::to_vec(&serde_json::json!([
                {"ordinal": 2, "receipt": receipt()},
                {"ordinal": 1, "receipt": receipt()}
            ]))
        } else {
            assert_eq!(request.operation, Operation::DustToSlivers);
            assert!(request.first_item.is_none() && request.second_item.is_none());
            assert_eq!(request.occurrence[6] >> 4, 7);
            assert_eq!(request.occurrence[8] & 192, 128);
            self.0.fetch_add(1, Ordering::SeqCst);
            serde_json::to_vec(&receipt())
        }
        .map_err(|_| SessionError::SessionUnusable)?;
        Ok(ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
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
async fn wait(
    link: &crate::play::PlayLink,
    check: impl Fn(&oteryn_client_ui::item_forge::ItemForgePanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !check(&link.item_forge_panel()) {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn forge_resource_actual_session_requires_confirm_and_displays_receipt_history() -> Result<(), Error>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                let count = Arc::new(AtomicUsize::new(0));
                let (link, rx, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer(count.clone()), rx, outbox));
                wait(&link, |p| p.selected && p.preview.selected).await?;
                let action = |a| activate(&mut lock(&link.mailbox), &link.commands, a);
                action(Action::Legacy(Operation::DustToSlivers));
                wait(&link, |p| p.resource_dialog.confirming()).await?;
                assert_eq!(count.load(Ordering::SeqCst), 0);
                let serial = link.item_forge_panel().resource_dialog.serial;
                let content = link
                    .item_forge_panel()
                    .resource_dialog
                    .content
                    .ok_or("confirmation")?;
                assert!(dialog_text(&content, 0).contains("100 → 40"));
                action(Action::DismissResource(serial));
                assert_eq!(count.load(Ordering::SeqCst), 0);
                action(Action::Legacy(Operation::DustToSlivers));
                wait(&link, |p| p.resource_dialog.confirming()).await?;
                let serial = link.item_forge_panel().resource_dialog.serial;
                action(Action::ConfirmResource(serial));
                action(Action::ConfirmResource(serial));
                wait(&link, |p| {
                    matches!(
                        p.resource_dialog.content,
                        Some(Content::Result(Reply::Result(_)))
                    )
                })
                .await?;
                assert_eq!(count.load(Ordering::SeqCst), 1);
                assert!(
                    dialog_text(
                        link.item_forge_panel()
                            .resource_dialog
                            .content
                            .as_ref()
                            .ok_or("receipt")?,
                        0
                    )
                    .contains("Created: 3 slivers")
                );
                action(Action::ConfirmResource(serial));
                assert_eq!(count.load(Ordering::SeqCst), 1);
                action(Action::Legacy(Operation::QueryHistory));
                wait(&link, |p| {
                    matches!(
                        p.resource_dialog.content,
                        Some(Content::Result(Reply::History(_)))
                    )
                })
                .await?;
                let content = link
                    .item_forge_panel()
                    .resource_dialog
                    .content
                    .ok_or("history")?;
                assert!(dialog_text(&content, 0).contains("History #2"));
                assert!(dialog_text(&content, 1).contains("History #1"));
                lock(&link.mailbox).cancel_activity_requests();
                assert!(link.item_forge_panel().resource_dialog.content.is_none());
                assert_eq!(count.load(Ordering::SeqCst), 1);
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })
        })?
        .join()
        .map_err(|_| "Forge resources session thread panicked")?
}

struct G20CapturePeer {
    reply: Option<GatedCommandResult>,
    paid_commands: Arc<AtomicUsize>,
}

impl Stepper for G20CapturePeer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == wire::CAPABILITY_FORGE_PREVIEW_V1
    }

    fn server_mechanic_selected(&self, capability: u32) -> bool {
        self.feature_selected(capability)
    }

    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        assert_eq!(command.capability, wire::CAPABILITY_FORGE_PREVIEW_V1);
        assert_eq!(command.kind, wire::COMMAND_TYPE_FORGE_PREVIEW);
        assert_eq!(
            wire::Request::decode(&command.payload),
            Some(wire::Request {
                nonce: 2,
                selection: wire::Selection::Resource(Operation::DustLimit),
            }),
        );
        self.reply.take().ok_or(SessionError::SessionUnusable)
    }

    async fn server_mechanic(
        &mut self,
        _: &ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, SessionError> {
        self.paid_commands.fetch_add(1, Ordering::SeqCst);
        Err(SessionError::SessionUnusable)
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

#[test]
#[ignore = "requires a live G20 capture from the shared replay runner"]
fn e2e_g20_funded_preview_replay() -> Result<(), Error> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        game_session_id: String,
        command_id: u64,
        server_sequence: u64,
        status: u32,
        payload: Vec<u8>,
    }

    let directory =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("G20 replay requires E2E_ALL_CAPTURE_DIR")?;
    let data = std::fs::read(std::path::Path::new(&directory).join("g20-funded.json"))?;
    let frame: Envelope = serde_json::from_slice(&data)?;
    assert!(!frame.game_session_id.is_empty());
    assert!(frame.command_id > 0 && frame.server_sequence > 0);
    assert_eq!(frame.status, CommandStatus::Accepted as u32);
    let result = GatedCommandResult {
        command_id: frame.command_id,
        server_sequence: frame.server_sequence,
        status: CommandStatus::Accepted,
        payload: frame.payload,
    };

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                let (link, rx, outbox) = play_channel();
                let paid_commands = Arc::new(AtomicUsize::new(0));
                let peer = G20CapturePeer {
                    reply: Some(result),
                    paid_commands: Arc::clone(&paid_commands),
                };
                let task = tokio::spawn(run_session(peer, rx, outbox));
                wait(&link, |panel| panel.selected && panel.preview.selected).await?;
                {
                    let mut mailbox = lock(&link.mailbox);
                    assert!(mailbox.item_forge.resource_dialog.content.is_none());
                    // The captured resource request followed the live Browse request.
                    mailbox.item_forge.preview.nonce = 1;
                    activate(
                        &mut mailbox,
                        &link.commands,
                        Action::Legacy(Operation::DustLimit),
                    );
                }
                wait(&link, |panel| panel.resource_dialog.confirming())
                    .await
                    .map_err(|_| "live Forge reply did not open the Dust Limit confirmation")?;
                let panel = link.item_forge_panel();
                assert!(panel.preview.pending.is_none());
                assert!(!panel.pending);
                let content = panel
                    .resource_dialog
                    .content
                    .ok_or("missing confirmation")?;
                assert_eq!(
                    content,
                    Content::Confirm {
                        operation: Operation::DustLimit,
                        quote: Some(wire::ResourceQuote {
                            operation: Operation::DustLimit,
                            full_resources: false,
                            before: wire::Resources {
                                dust: 30,
                                dust_limit: 100,
                                ..Default::default()
                            },
                            after_dust: 5,
                            after_limit: 101,
                            after_slivers: 0,
                            after_cores: 0,
                        }),
                    },
                );
                let text = dialog_text(&content, 0);
                assert!(text.contains("Raise the dust limit?"));
                assert!(text.contains("Dust: 30 → 5\nDust limit: 100 → 101"));
                drop(link);
                task.await?;
                assert_eq!(paid_commands.load(Ordering::SeqCst), 0);
                println!("G20_CLIENT_CONFIRM dust=30->5 limit=100->101");
                Ok::<(), Error>(())
            })
        })?
        .join()
        .map_err(|_| "G20 client replay thread panicked")?
}
