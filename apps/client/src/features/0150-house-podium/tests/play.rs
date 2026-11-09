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
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 150
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
        assert_eq!(command.capability, 150);
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
        while !link.house_podium_panel().selected || link.house_podium_panel().pending {
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
    link.house_podium_request(intent);
    assert!(link.house_podium_panel().pending);
    // A second click before a response must not produce another command.
    link.house_podium_request(intent);
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
fn cp01_house_podium_scenario() -> Result<(), Error> {
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
                    link.house_podium_request(Intent::Refresh);
                    assert!(!link.house_podium_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(19).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.house_podium_input(&[event]));
                    assert!(link.house_podium_panel().open);

                    link.house_podium_edit(0, ID.into());
                    link.house_podium_request(Intent::Set);
                    assert!(!link.house_podium_panel().pending);
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        174,
                        ITEM.to_vec(),
                        CommandStatus::Accepted,
                        vec![0],
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.house_podium_panel().revision, Some(0));
                    for (i, value) in [(1, "1"), (2, "2"), (3, "citizen")] {
                        link.house_podium_edit(i, value.into());
                    }
                    for (kind, looks, object) in [
                        (1, [128, 136], false),
                        (2, [368, 368], false),
                        (3, [2122, 2122], true),
                    ] {
                        assert!(link.house_podium_choose_display(kind, 1));
                        let preview = wire::preview::Request {
                            display_kind: kind,
                            direction: 1,
                            key: "citizen".into(),
                        };
                        let artwork = wire::preview::Artwork {
                            object,
                            direction: 1,
                            looks,
                        };
                        send(
                            &link,
                            &exchanges,
                            Intent::Preview,
                            wire::preview::COMMAND_PREVIEW,
                            preview.encode().ok_or("preview request")?,
                            CommandStatus::Accepted,
                            wire::preview::Artwork::encode(Some(artwork)).ok_or("preview reply")?,
                        )
                        .await?;
                        assert_eq!(link.house_podium_panel().artwork, Some(artwork));
                        assert_eq!(
                            link.house_podium_panel().revision,
                            Some(0),
                            "preview cannot mutate the podium"
                        );
                    }
                    assert!(link.house_podium_choose_display(1, 2));
                    assert!(link.house_podium_panel().artwork.is_none());
                    let set = wire::SetRequest {
                        item: ITEM,
                        expected_revision: 0,
                        display_kind: 1,
                        direction: 2,
                        key: "citizen".into(),
                    }
                    .encode()
                    .ok_or("set encode")
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        Intent::Set,
                        175,
                        set,
                        CommandStatus::Accepted,
                        wire::SetResult::Accepted { revision: 7 }.encode(),
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.house_podium_panel().revision, Some(7));
                    let projection = wire::Projection {
                        podium_kind: 1,
                        display_kind: 1,
                        direction: 2,
                        revision: 7,
                        key: "citizen".into(),
                    };
                    send(
                        &link,
                        &exchanges,
                        Intent::Projection,
                        176,
                        ITEM.to_vec(),
                        CommandStatus::DuplicateReplay,
                        wire::Projection::encode(Some(&projection))
                            .ok_or("projection encode")
                            .checked()?,
                    )
                    .await
                    .checked()?;
                    assert!(link.house_podium_panel().projection.contains("citizen"));
                    assert!(link.house_podium_panel().projection.contains("facing 2"));
                    link.house_podium_edit(0, "00000000-0000-7000-8000-000000000002".into());
                    link.house_podium_request(Intent::Set);
                    assert!(!link.house_podium_panel().pending);
                    assert!(link.house_podium_panel().feedback.contains("Refresh"));
                    link.house_podium_edit(0, ID.into());
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        174,
                        ITEM.to_vec(),
                        CommandStatus::Accepted,
                        wire::Projection::encode(Some(&projection))
                            .ok_or("projection encode")
                            .checked()?,
                    )
                    .await
                    .checked()?;
                    let set = wire::SetRequest {
                        item: ITEM,
                        expected_revision: 7,
                        display_kind: 1,
                        direction: 2,
                        key: "citizen".into(),
                    }
                    .encode()
                    .ok_or("set encode")
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        Intent::Set,
                        175,
                        set,
                        CommandStatus::Accepted,
                        wire::SetResult::Refused(wire::SetRefusal::Changed).encode(),
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.house_podium_panel().revision, None);
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        174,
                        ITEM.to_vec(),
                        CommandStatus::Accepted,
                        vec![1],
                    )
                    .await
                    .checked()?;
                    assert!(link.house_podium_panel().feedback.contains("Invalid"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Projection,
                        176,
                        ITEM.to_vec(),
                        CommandStatus::Rejected,
                        vec![0],
                    )
                    .await
                    .checked()?;
                    assert!(link.house_podium_panel().feedback.contains("refused"));

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

use oteryn_client_ui::house_podium::Intent;
use oteryn_session::property_services::house_podium as wire;
const ID: &str = "00000000-0000-7000-8000-000000000001";
const ITEM: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];

trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[test]
fn podium_creature_selection_uses_the_existing_wire_and_creature_preview()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, _) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let p = &mut mailbox.properties.house_podium;
        p.selected = true;
        p.fields = [
            ID.into(),
            "3".into(),
            "2".into(),
            "oteryn:creature.dragon".into(),
        ];
        p.loaded_item = ID.into();
        p.revision = Some(38);
    }
    link.house_podium_request(Intent::Set);
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("podium command missing".into());
    };
    let request = wire::SetRequest::decode(&command.payload).ok_or("podium wire")?;
    assert_eq!((command.capability, command.kind), (150, 175));
    assert_eq!(request.display_kind, 3);
    assert_eq!(request.key, "oteryn:creature.dragon");
    assert!(
        oteryn_client_ui::house_podium::panel_text(&link.house_podium_panel())
            .contains("Preview: creature oteryn:creature.dragon facing south")
    );
    Ok(())
}

#[test]
fn podium_creature_selector_updates_the_real_panel_without_sending()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = crate::play::play_channel();
    link.house_podium_edit(3, "oteryn:creature.test".into());
    assert!(link.house_podium_choose_display(3, 2));
    let panel = link.house_podium_panel();
    assert_eq!(panel.fields[1], "3");
    assert_eq!(panel.fields[2], "2");
    assert!(oteryn_client_ui::house_podium::panel_text(&panel).contains("creature"));
    assert!(!link.house_podium_choose_display(4, 0));
    assert!(commands.try_recv().is_err());
    Ok(())
}
