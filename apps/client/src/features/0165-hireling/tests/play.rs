use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::hireling::{CommandStatus, HirelingCommand, HirelingResponse, hireling};
use oteryn_client_ui::hireling::{Intent, panel_text};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    commands: Arc<Mutex<Vec<hireling::Request>>>,
}
impl Stepper for Peer {
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

async fn ready(link: &PlayLink) {
    while !link.hireling_panel().selected || link.hireling_panel().pending.is_some() {
        tokio::task::yield_now().await;
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
            },
            incoming,
            outbox,
        ));
        ready(&link).await;
        link.hireling_request(Intent::Refresh);
        ready(&link).await;
        link.hireling_target(7, 9, 1);
        link.hireling_request(Intent::Summon);
        ready(&link).await;
        assert!(panel_text(&link.hireling_panel()).contains("summoned"));
        link.hireling_request(Intent::Dismiss);
        ready(&link).await;
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
        task.await?;
        Ok::<(), Error>(())
    })??;
    Ok(())
}

#[test]
fn map_pick_summons_at_the_actor_position_on_the_wire() -> Result<(), Error> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            use crate::play::{PlayView, mailbox::lock};
            use oteryn_renderer::TileCoord;
            use oteryn_session::{ActorPosition, AppliedDelta, WorldSpatialObservation};

            let commands = Arc::new(Mutex::new(Vec::new()));
            let (link, incoming, outbox) = play_channel();
            outbox.push(vec![SessionEvent::WorldSpatial(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: WorldSpatialObservation {
                    content_generation: [0; 32],
                    actor_position: ActorPosition { x: 100, y: 100, floor: -7 },
                },
            })]);
            let task = tokio::spawn(run_session(
                Peer { commands: Arc::clone(&commands) },
                incoming,
                outbox,
            ));
            ready(&link).await;
            let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
            let form = crate::play::feature_0165_hireling::view::form(&link);
            (form.submit)(&mut lock(&link.mailbox), &link.commands, 3, Vec::new());
            view.primary_click(TileCoord::new(101, 100), &link)?;
            (form.submit)(&mut lock(&link.mailbox), &link.commands, 0, Vec::new());
            ready(&link).await;
            assert_eq!(
                *commands.lock().unwrap_or_else(PoisonError::into_inner),
                [hireling::Request::Summon { x: 100, y: 100, floor: -7 }]
            );
            drop(link);
            task.await?;
            Ok::<(), Error>(())
        })?
    })?.join().map_err(|_| "hireling test thread panicked")?
}
