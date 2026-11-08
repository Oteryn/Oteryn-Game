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

include!("play_pc03_character_info_scenario_1.rs");
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
