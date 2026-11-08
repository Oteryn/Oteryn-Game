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
        capability == 97
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
        assert_eq!(command.capability, 97);
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
        while !link.character_info_panel().selected || link.character_info_panel().pending {
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
    link.character_info_request(intent);
    assert!(link.character_info_panel().pending);
    let draft = link.character_info_panel().fields;
    link.character_info_edit(0, "changed".into());
    assert_eq!(link.character_info_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.character_info_request(intent);
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
use oteryn_client_ui::character_info::Intent;

#[test]
fn pc03_character_info_scenario() -> Result<(), Error> {
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
                    link.character_info_request(Intent::Query);
                    assert!(!link.character_info_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(13).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.character_info_input(&[event]));
                    assert!(link.character_info_panel().open);
                    let fields = ["0", "50", "", ""];
                    for (i, value) in fields.iter().enumerate() {
                        link.character_info_edit(i, (*value).into());
                    }
                    let intent = Intent::Query;
                    let payload = vec![0, 0, 0, 50];
                    let reply = {
                        let mut bytes = vec![0];
                        bytes.extend_from_slice(&42i64.to_le_bytes());
                        bytes.resize(69, 0);
                        bytes.extend_from_slice(&[0, 0]);
                        bytes
                    };
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    assert!(!link.character_info_panel().summary.is_empty());
                    link.character_info_edit(2, "11111111-1111-1111-1111-111111111111".into());
                    let mut visible = vec![1];
                    visible.extend_from_slice(&[0x11; 16]);
                    visible.extend_from_slice(&[0, 0, 50]);
                    send(
                        &link,
                        &exchanges,
                        Intent::Visible,
                        113,
                        visible,
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    let mut look = vec![4];
                    look.extend_from_slice(&[0x11; 16]);
                    send(
                        &link,
                        &exchanges,
                        Intent::Look,
                        113,
                        look,
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    link.character_info_edit(3, "1".into());
                    send(
                        &link,
                        &exchanges,
                        Intent::Unlock,
                        113,
                        vec![2, 1],
                        CommandStatus::Accepted,
                        vec![2, 1],
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.character_info_panel()
                            .summary
                            .contains("Adept Armorer")
                    );
                    let mut selected = vec![3];
                    selected.extend_from_slice(&1u64.to_le_bytes());
                    send(
                        &link,
                        &exchanges,
                        Intent::Select,
                        113,
                        vec![3, 1],
                        CommandStatus::Accepted,
                        selected.clone(),
                    )
                    .await
                    .checked()?;
                    link.character_info_edit(3, "0".into());
                    send(
                        &link,
                        &exchanges,
                        Intent::Select,
                        113,
                        vec![3, 0],
                        CommandStatus::Accepted,
                        selected,
                    )
                    .await
                    .checked()?;
                    assert!(link.character_info_panel().summary.contains("None"));
                    link.character_info_edit(1, "51".into());
                    link.character_info_request(Intent::Query);
                    assert!(!link.character_info_panel().pending);
                    assert!(link.character_info_panel().feedback.contains("1-50"));
                    link.character_info_edit(1, "50".into());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    let reconnect_payload = payload.clone();
                    let reconnect_reply = reply.clone();
                    let summary = link.character_info_panel().summary;
                    assert!(!summary.is_empty());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.character_info_panel().feedback.contains("refused"));
                    assert_eq!(link.character_info_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.character_info_panel().feedback.contains("Invalid"));
                    assert_eq!(link.character_info_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload,
                        CommandStatus::DuplicateReplay,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.character_info_panel().summary, summary);
                    // An unexpected transport end while a request is in flight resets every session field.
                    link.character_info_request(intent);
                    assert!(link.character_info_panel().pending);
                    task.await.checked()?;
                    let ended = link.character_info_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    link.character_info_request(intent);
                    assert!(!link.character_info_panel().pending);
                    let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnected = PlayLink {
                        commands,
                        mailbox: Arc::clone(&link.mailbox),
                    };
                    let reconnect = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        receiver,
                        outbox,
                    ));
                    ready(&reconnected).await.checked()?;
                    let fresh = reconnected.character_info_panel();
                    assert!(!fresh.open && !fresh.pending);
                    assert!(fresh.summary.is_empty() && fresh.fields.iter().all(String::is_empty));
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.character_info_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        113,
                        reconnect_payload,
                        CommandStatus::Accepted,
                        reconnect_reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.character_info_panel().summary.is_empty());
                    drop(reconnected);
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
#[test]
fn srv6_title_sources_decode_and_reach_native_play_panel() -> Result<(), Error> {
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
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await?;
                    link.character_info_open();
                    for name in [
                        "Aspiring Huntsman",
                        "Competent Beastslayer",
                        "Feared Bountyhunter",
                        "Chompmeister",
                        "Dedicated Entrepreneur",
                        "Hero of Bounac",
                        "Planegazer",
                        "Time Traveller",
                        "Proconsul of Iksupan",
                        "Robinson Crusoe",
                    ] {
                        let title =
                            oteryn_client_runtime::character_info_panel::CharacterTitle::ALL
                                .into_iter()
                                .find(|title| title.name() == name)
                                .ok_or("missing title")?;
                        link.character_info_edit(3, title.id().to_string());
                        send(
                            &link,
                            &exchanges,
                            Intent::Unlock,
                            113,
                            vec![2, title.id()],
                            CommandStatus::Accepted,
                            vec![2, title.id()],
                        )
                        .await?;
                        let panel = link.character_info_panel();
                        assert!(panel.open);
                        assert!(
                            oteryn_client_ui::character_info::panel_text(&panel).contains(name)
                        );
                    }
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
        .map_err(|_| "title source play thread failed")
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

#[test]
fn title_selector_receives_self_ownership_and_selected_title_from_the_command_adapter()
-> Result<(), Error> {
    use crate::play::feature_0097_character_info::protocol::{apply, command};
    use oteryn_client_ui::character_info::{Intent, Panel, TitleFilter, title_rows};
    let mut p = Panel {
        selected: true,
        fields: ["0".into(), "1".into(), String::new(), "1".into()],
        ..Default::default()
    };
    let query = command(&p, Intent::Query).map_err(|e| -> Error { e.into() })?;
    assert!(p.begin(query.kind));
    let mut payload = vec![0; 72];
    payload[69] = 1;
    payload[70] = 1;
    apply(
        &mut p,
        &query,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
        },
    );
    p.title_filter = TitleFilter::Earned;
    assert_eq!(title_rows(&p).len(), 1);
    let select = command(&p, Intent::Select).map_err(|e| -> Error { e.into() })?;
    assert_eq!(select.payload, [3, 1]);
    assert!(p.begin(select.kind));
    let mut payload = vec![3];
    payload.extend_from_slice(&2u64.to_le_bytes());
    apply(
        &mut p,
        &select,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload,
        },
    );
    assert!(title_rows(&p)[0].selected);
    Ok(())
}
