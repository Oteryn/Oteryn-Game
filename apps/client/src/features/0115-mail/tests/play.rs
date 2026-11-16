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
            let item = oteryn_session::ItemEntry { appearance_id: 0, handle: 7.try_into()?, item_definition_ref: 1.try_into()?, count: 1.try_into()?, sub_type: 0 };
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

#[test]
fn rejected_unicode_edits_never_submit_stored_values() -> Result<(), Error> {
    use crate::play::mailbox::lock;
    let (link, mut incoming, _) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.properties.mail.selected = true;
        mailbox.properties.house_transfer.selected = true;
        mailbox.properties.guildhall.selected = true;
        mailbox.boss_encounter.selected = true;
    }
    link.mail_edit(0, "7".into());
    link.mail_edit(2, "previous inscription".into());
    link.mail_edit(2, "ż".repeat(4000));
    link.mail_request(Intent::Write);
    assert!(incoming.try_recv().is_err(), "submitted stale inscription");
    assert!(!link.mail_panel().feedback.is_empty());
    link.house_transfer_edit(0, "oteryn:content.house.test".into());
    link.house_transfer_edit(0, "界".repeat(256));
    link.house_transfer_request(oteryn_client_ui::house_transfer::Intent::Enter);
    assert!(incoming.try_recv().is_err(), "submitted stale house");
    link.guildhall_edit(0, "00000000-0000-7000-8000-000000000001".into());
    link.guildhall_edit(1, "00000000-0000-7000-8000-000000000002".into());
    link.guildhall_edit(2, "1".into());
    link.guildhall_edit(0, "界".repeat(256));
    link.guildhall_request(oteryn_client_ui::guildhall::Intent::Transfer);
    assert!(incoming.try_recv().is_err(), "submitted stale guild");
    link.boss_encounter_edit(0, "oteryn:creature.test".into());
    link.boss_encounter_edit(0, "界".repeat(256));
    link.boss_encounter_request(oteryn_client_ui::boss_encounter::Intent::Enter);
    assert!(incoming.try_recv().is_err(), "submitted stale boss");
    Ok(())
}

#[test]
fn writing_drafts_follow_item_and_session_identity() -> Result<(), Error> {
    use crate::play::mailbox::lock;
    let (link, _, outbox) = play_channel();
    let selected = Peer {
        exchanges: Arc::new(Script {
            entries: Mutex::new(VecDeque::new()),
            release: tokio::sync::Semaphore::new(0),
        }),
        inventory: None,
    };
    outbox.properties_initialize(&selected);
    link.mail_edit(0, "7".into());
    link.mail_edit(2, "letter A".into());
    link.mail_edit(0, "7".into());
    assert_eq!(link.mail_panel().fields[2], "letter A");
    link.mail_edit(0, "8".into());
    assert!(
        link.mail_panel().fields[2].is_empty(),
        "letter B inherited A's draft"
    );
    link.mail_edit(2, "letter B".into());
    outbox.properties_initialize(&selected);
    assert!(link.mail_panel().fields.iter().all(String::is_empty));
    link.mail_edit(0, "7".into());
    link.mail_edit(2, "another session".into());
    lock(&link.mailbox).properties.end();
    assert!(link.mail_panel().fields.iter().all(String::is_empty));
    Ok(())
}

#[cfg(bevy_shell)]
impl crate::play::Outbox {
    /// Admits the property capabilities the Bevy item and furniture menus drive.
    pub(crate) fn bevy_property_items_selected(&self) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        mailbox.properties.mail.selected = true;
        mailbox.properties.timed_item.selected = true;
        mailbox.properties.house_furniture_rotation.selected = true;
        mailbox.properties.house_furniture_wrap.selected = true;
    }
    /// Admits capability 511, the writing window's item text read.
    pub(crate) fn bevy_item_inscription_selected(&self) {
        crate::play::mailbox::lock(&self.0)
            .properties
            .mail_inscription
            .selected = true;
    }
    /// Applies a server reply through the production property reducer.
    pub(crate) fn bevy_properties_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        self.properties_result(command, result);
    }
}

/// Selects the writing window and, with `inscription`, capability 511.
fn writing_window(mail: bool) -> (PlayLink, tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>, crate::play::Outbox) {
    let (link, incoming, outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.properties.mail.selected = mail;
        mailbox.properties.mail_inscription.selected = true;
    }
    (link, incoming, outbox)
}

fn inscription_reply(text: &str) -> Result<GatedCommandResult, Error> {
    let payload = oteryn_client_ui::item_inscription::Reply { text: text.into(), max_length: 4000, writable: true }
        .encode()
        .ok_or("inscription reply")?;
    Ok(GatedCommandResult { command_id: 1, server_sequence: 1, status: CommandStatus::Accepted, payload })
}

fn sent_feature(incoming: &mut tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>) -> Result<FeatureCommand, Error> {
    match incoming.try_recv()? {
        crate::combat_input::PlayCommand::Feature(command) => Ok(command),
        other => Err(format!("unexpected command {other:?}").into()),
    }
}

#[test]
fn inscription_read_waits_for_the_mail_capability() -> Result<(), Error> {
    use oteryn_client_ui::item_inscription::Known;
    let (link, mut incoming, _outbox) = writing_window(false);
    link.mail_write_item(7.try_into()?);
    assert!(incoming.try_recv().is_err());
    for _ in 0..2 {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        crate::play::feature_0115_mail::input::tick(&mut mailbox, &link.commands);
        assert_eq!(mailbox.properties.mail_inscription.known, Known::Unread);
    }
    assert!(incoming.try_recv().is_err());
    Ok(())
}

#[test]
fn inscription_read_keeps_typed_text_and_asks_for_the_dropped_write_again() -> Result<(), Error> {
    let (link, mut incoming, outbox) = writing_window(true);
    link.mail_edit(0, "7".into());
    link.mail_edit(2, "typed letter".into());
    link.mail_request(Intent::Write);
    let read = sent_feature(&mut incoming)?;
    outbox.properties_result(&read, &inscription_reply("old text")?);
    assert_eq!(link.mail_panel().fields[2], "typed letter");
    assert_eq!(link.mail_panel().feedback, "The item's text was read; press Write again to replace it");
    link.mail_request(Intent::Write);
    let write = sent_feature(&mut incoming)?;
    assert_eq!(
        wire::decode_intent(&write.payload).checked()?,
        wire::MailIntent::WriteText { item_handle: 7, text: "typed letter".into() }
    );
    Ok(())
}

#[test]
fn inscription_read_at_the_form_byte_bound_fills_the_editor_and_beyond_blocks_write() -> Result<(), Error> {
    let (link, mut incoming, outbox) = writing_window(true);
    link.mail_write_item(7.try_into()?);
    let read = sent_feature(&mut incoming)?;
    let bound = "ż".repeat(3998);
    outbox.properties_result(&read, &inscription_reply(&bound)?);
    assert_eq!(link.mail_panel().fields[2], bound);
    link.mail_request(Intent::Write);
    assert!(sent_feature(&mut incoming).is_ok(), "a read text at the bound is written");

    let (link, mut incoming, outbox) = writing_window(true);
    link.mail_write_item(8.try_into()?);
    let read = sent_feature(&mut incoming)?;
    outbox.properties_result(&read, &inscription_reply(&"ż".repeat(3999))?);
    assert!(link.mail_panel().fields[2].is_empty());
    assert_eq!(link.mail_panel().feedback, "The letter text is too long");
    link.mail_edit(2, "replacement".into());
    link.mail_request(Intent::Write);
    assert!(incoming.try_recv().is_err(), "an unshown text is never replaced");
    assert!(!link.mail_panel().feedback.is_empty());
    Ok(())
}
