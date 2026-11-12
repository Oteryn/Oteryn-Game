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
    inventory: Option<oteryn_session::CharacterInventory>,
}
impl Stepper for Peer {
    fn inventory(&self) -> Option<oteryn_session::CharacterInventory> { self.inventory.clone() }
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 115 || (capability == 4 && self.inventory.is_some())
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
        assert_eq!(command.capability, 115);
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
        while !link.mail_panel().selected || link.mail_panel().pending {
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
    link.mail_request(intent);
    assert!(link.mail_panel().pending);
    // A second click before a response must not produce another command.
    link.mail_request(intent);
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
#[test]
fn cp01_mail_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, mut incoming, outbox) = play_channel();
                    link.mail_request(Intent::Write);
                    assert!(!link.mail_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            inventory: None,
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(16).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.mail_input(&[event]));
                    assert!(link.mail_panel().open);

                    link.mail_edit(0, "7".into());
                    link.mail_edit(1, "9".into());
                    link.mail_edit(2, "Alice\nThais\nHello 📨".into());
                    let write = wire::encode_intent(&wire::MailIntent::WriteText {
                        item_handle: 7,
                        text: "Alice\nThais\nHello 📨".into(),
                    })
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        Intent::Write,
                        124,
                        write.clone(),
                        CommandStatus::Accepted,
                        vec![2],
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.mail_panel().feedback, "Item text saved");
                    assert_eq!(link.mail_panel().fields[2], "Alice\nThais\nHello 📨");
                    let post = wire::encode_intent(&wire::MailIntent::Post {
                        item_handle: 7,
                        mailbox_handle: 9,
                    })
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        Intent::Post,
                        124,
                        post.clone(),
                        CommandStatus::DuplicateReplay,
                        vec![1],
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.mail_panel().feedback, "Mail posted");
                    for code in 3..=11 {
                        send(
                            &link,
                            &exchanges,
                            Intent::Post,
                            124,
                            post.clone(),
                            CommandStatus::Accepted,
                            vec![code],
                        )
                        .await
                        .checked()?;
                        assert_ne!(link.mail_panel().feedback, "Mail posted");
                        assert!(!link.mail_panel().feedback.contains("invalid"));
                    }
                    send(
                        &link,
                        &exchanges,
                        Intent::Write,
                        124,
                        write.clone(),
                        CommandStatus::Rejected,
                        vec![2],
                    )
                    .await
                    .checked()?;
                    assert!(link.mail_panel().feedback.contains("refused"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Write,
                        124,
                        write,
                        CommandStatus::Accepted,
                        vec![2, 0],
                    )
                    .await
                    .checked()?;
                    assert!(link.mail_panel().feedback.contains("invalid"));
                    link.mail_edit(0, "0".into());
                    link.mail_request(Intent::Post);
                    assert!(!link.mail_panel().pending);
                    assert!(link.mail_panel().feedback.contains("valid item"));

                    drop(link);
                    task.await.checked()?;
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

use oteryn_client_ui::mail::Intent;
use oteryn_session::property_services::mail as wire;

trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[test]
fn srx1_native_writing_selected_item_utf8_save_and_write_once_refusal() -> Result<(), Error> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), Error> {
        oteryn_client_runtime::ClientRuntime::new().checked()?.block_on(async {
            let exchanges = Arc::new(Script { entries: Mutex::new(VecDeque::new()), release: tokio::sync::Semaphore::new(0) });
            let (link, incoming, outbox) = play_channel();
            let item = oteryn_session::ItemEntry { handle: 7.try_into()?, item_definition_ref: 1.try_into()?, count: 1.try_into()?, sub_type: 0 };
            let task = tokio::spawn(run_session(Peer { exchanges: Arc::clone(&exchanges), inventory: Some(oteryn_session::CharacterInventory { main_backpack: Some(oteryn_session::ItemEntry { handle: 1.try_into()?, ..item }), entries: vec![item], equipment: vec![] }) }, incoming, outbox));
            ready(&link).await?;
            // Mail readiness is published before the initial inventory snapshot.
            tokio::time::timeout(Duration::from_secs(3), async {
                while link.inventory_panel().inventory.is_none() {
                    tokio::task::yield_now().await;
                }
            }).await.checked()?;
            assert!(link.inventory_select(item.handle));
            let event = oteryn_input_actions::NormalizedInputEvent::Key {
                code: oteryn_input_actions::KeyCode::new(16).checked()?,
                state: oteryn_input_actions::ButtonState::Pressed,
                modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control).with(oteryn_input_actions::Modifier::Alt), repeat: false,
            };
            assert!(link.mail_input(&[event]));
            assert!(link.mail_panel().open);
            assert_eq!(link.mail_panel().fields[0], "7", "native writer uses the selected server handle");
            link.mail_edit(2, "One inscription: żółw 🐢".into());
            let write = wire::encode_intent(&wire::MailIntent::WriteText { item_handle: 7, text: "One inscription: żółw 🐢".into() }).checked()?;
            send(&link, &exchanges, Intent::Write, 124, write.clone(), CommandStatus::Accepted, vec![wire::MailOutcome::TextWritten as u8]).await?;
            assert_eq!(link.mail_panel().feedback, "Item text saved");
            send(&link, &exchanges, Intent::Write, 124, write, CommandStatus::Accepted, vec![wire::MailOutcome::NotWritable as u8]).await?;
            assert_eq!(link.mail_panel().feedback, "This item cannot be written on");
            assert_eq!(link.mail_panel().fields[2], "One inscription: żółw 🐢");
            assert!(oteryn_client_ui::mail::panel_text(&link.mail_panel()).contains("cannot be written"));
            drop(link); task.await?; Ok::<(), Error>(())
        }).checked()??;
        Ok(())
    }).checked()?.join().map_err(|_| "writing scenario thread failed").checked()?
}

fn e2e_e09_capture() -> Result<([Vec<u8>; 3], String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            ["e09-write.bin", "e09-post.bin", "e09-unknown.bin"]
                .map(|name| std::fs::read(root.join(name)))
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .map_err(|_| "capture count")?,
            std::fs::read_to_string(root.join("e09-expected.txt"))?,
        ));
    }
    Ok((
        [
            vec![wire::MailOutcome::TextWritten as u8],
            vec![wire::MailOutcome::Posted as u8],
            vec![wire::MailOutcome::UnknownRecipient as u8],
        ],
        "write_payload=2\npost_payload=1\nunknown_payload=4\nwrite=Item text saved\npost=Mail posted\nunknown=Unknown recipient\n".into(),
    ))
}

fn e2e_e09_value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .ok_or_else(|| format!("expected.txt lacks {key}").into())
}

/// E09 replay: the Mail panel `apply` in `protocol.rs` consumes the raw MAIL outcome payloads the
/// server sent for write, post and unknown-recipient refusal.
#[test]
fn e2e_e09_real_mail_outcomes_replay_into_mail_panel() -> Result<(), Error> {
    let ([written, posted, unknown], expected) = e2e_e09_capture()?;
    assert_eq!(
        [written[0], posted[0], unknown[0]],
        [
            e2e_e09_value(&expected, "write_payload")?.parse::<u8>()?,
            e2e_e09_value(&expected, "post_payload")?.parse::<u8>()?,
            e2e_e09_value(&expected, "unknown_payload")?.parse::<u8>()?
        ]
    );
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), Error> {
        oteryn_client_runtime::ClientRuntime::new().checked()?.block_on(async {
            let exchanges = Arc::new(Script { entries: Mutex::new(VecDeque::new()), release: tokio::sync::Semaphore::new(0) });
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(Peer { exchanges: Arc::clone(&exchanges), inventory: None }, incoming, outbox));
            ready(&link).await?;
            let address = "Alice\nThais".to_string();
            link.mail_edit(0, "7".into());
            link.mail_edit(1, "9".into());
            link.mail_edit(2, address.clone());
            let write = wire::encode_intent(&wire::MailIntent::WriteText { item_handle: 7, text: address.clone() }).checked()?;
            let post = wire::encode_intent(&wire::MailIntent::Post { item_handle: 7, mailbox_handle: 9 }).checked()?;
            send(&link, &exchanges, Intent::Write, 124, write.clone(), CommandStatus::Accepted, written.clone()).await?;
            assert_eq!(link.mail_panel().feedback, e2e_e09_value(&expected, "write")?);
            send(&link, &exchanges, Intent::Post, 124, post.clone(), CommandStatus::Accepted, posted.clone()).await?;
            assert_eq!(link.mail_panel().feedback, e2e_e09_value(&expected, "post")?);
            send(&link, &exchanges, Intent::Post, 124, post.clone(), CommandStatus::Accepted, unknown).await?;
            assert_eq!(link.mail_panel().feedback, e2e_e09_value(&expected, "unknown")?);
            assert!(oteryn_client_ui::mail::panel_text(&link.mail_panel()).ends_with("Unknown recipient"));
            // A post outcome answering a write command is invalid and keeps the form unchanged.
            let fields = link.mail_panel().fields;
            send(&link, &exchanges, Intent::Write, 124, write, CommandStatus::Accepted, posted).await?;
            assert_eq!(link.mail_panel().feedback, "The server returned an invalid mail reply");
            assert_eq!(link.mail_panel().fields, fields);
            assert_eq!(fields, ["7".to_string(), "9".to_string(), address]);
            drop(link); task.await?; Ok::<(), Error>(())
        }).checked()??;
        Ok(())
    }).checked()?.join().map_err(|_| "e09 scenario thread failed").checked()?
}
