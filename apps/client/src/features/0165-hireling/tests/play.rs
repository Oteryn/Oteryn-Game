use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::hireling::{CommandStatus, HirelingCommand, HirelingResponse, hireling};
use oteryn_client_ui::hireling::{Intent, panel_text};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

const SESSION_WAIT: Duration = Duration::from_secs(3);

struct Peer {
    position: Option<oteryn_session::ActorPosition>,
    commands: Arc<Mutex<Vec<hireling::Request>>>,
}
impl Stepper for Peer {
    fn world_position(&self) -> Option<oteryn_session::ActorPosition> {
        self.position
    }

    fn hireling_selected(&self) -> bool {
        true
    }
    async fn hireling(
        &mut self,
        request: &HirelingCommand,
    ) -> Result<HirelingResponse, SessionError> {
        let decoded =
            hireling::Request::decode(&request.0.encode()).ok_or(SessionError::SessionUnusable)?;
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(decoded);
        Ok(HirelingResponse {
            status: CommandStatus::Accepted,
            active: hireling::decode_reply(&hireling::encode_reply(!matches!(
                decoded,
                hireling::Request::Dismiss
            ))),
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

async fn ready(
    link: &PlayLink,
    task: &tokio::task::JoinHandle<()>,
    phase: &str,
) -> Result<(), Error> {
    tokio::time::timeout(SESSION_WAIT, async {
        loop {
            let panel = link.hireling_panel();
            if task.is_finished() {
                return Err(format!(
                    "hireling session ended during {phase}: selected={}, pending={:?}",
                    panel.selected, panel.pending
                )
                .into());
            }
            if panel.selected && panel.pending.is_none() {
                return Ok(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| format!("hireling session timed out during {phase}"))?
}

async fn stop(mut task: tokio::task::JoinHandle<()>) -> Result<(), Error> {
    match tokio::time::timeout(SESSION_WAIT, &mut task).await {
        Ok(result) => result.map_err(Into::into),
        Err(_) => {
            task.abort();
            Err("hireling session shutdown timed out".into())
        }
    }
}

#[test]
fn hireling_controls_use_real_codec_requests_and_apply_replies() -> Result<(), Error> {
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(async {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let (link, incoming, outbox) = play_channel();
        let task = tokio::spawn(run_session(
            Peer {
                commands: Arc::clone(&commands),
                position: None,
            },
            incoming,
            outbox,
        ));
        ready(&link, &task, "capability selection").await?;
        link.hireling_request(Intent::Refresh);
        ready(&link, &task, "refresh reply").await?;
        link.hireling_target(7, 9, 1);
        link.hireling_request(Intent::Summon);
        ready(&link, &task, "summon reply").await?;
        assert!(panel_text(&link.hireling_panel()).contains("summoned"));
        link.hireling_request(Intent::Dismiss);
        ready(&link, &task, "dismiss reply").await?;
        assert_eq!(
            *commands.lock().unwrap_or_else(PoisonError::into_inner),
            vec![
                hireling::Request::Query,
                hireling::Request::Summon {
                    x: 7,
                    y: 9,
                    floor: 1
                },
                hireling::Request::Dismiss
            ]
        );
        drop(link);
        stop(task).await?;
        Ok::<(), Error>(())
    })??;
    Ok(())
}

#[test]
fn first_map_pick_summons_at_the_admission_actor_position_without_a_map() -> Result<(), Error> {
    let builder = std::thread::Builder::new().stack_size(64 << 20);
    let thread = builder.spawn(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            use crate::play::{PlayView, mailbox::lock};
            use oteryn_renderer::TileCoord;
            use oteryn_session::ActorPosition;

            let commands = Arc::new(Mutex::new(Vec::new()));
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(
                Peer {
                    commands: Arc::clone(&commands),
                    position: Some(ActorPosition {
                        x: 100,
                        y: 100,
                        floor: -7,
                    }),
                },
                incoming,
                outbox,
            ));
            ready(&link, &task, "capability selection").await?;
            let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
            let form = crate::play::feature_0165_hireling::view::form(&link);
            (form.submit)(&mut lock(&link.mailbox), &link.commands, 3, Vec::new());
            view.primary_click(TileCoord::new(101, 100), &link)?;
            assert_eq!(link.hireling_panel().target, Some((101, 100, -7)));
            (form.submit)(&mut lock(&link.mailbox), &link.commands, 0, Vec::new());
            ready(&link, &task, "summon reply").await?;
            assert_eq!(
                *commands.lock().unwrap_or_else(PoisonError::into_inner),
                [hireling::Request::Summon {
                    x: 100,
                    y: 100,
                    floor: 7
                }]
            );
            drop(link);
            stop(task).await?;
            Ok::<(), Error>(())
        })?
    })?;
    thread.join().map_err(|_| "hireling test thread panicked")?
}
