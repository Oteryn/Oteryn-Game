//! Canary `src/utils/tools.cpp:1392-1415,1601` and
//! `data/scripts/actions/doors/key_door.lua:32`: preserve the server's refusal text.
use crate::combat_input::{CombatCommand, CombatReport, PlayCommand};
use crate::input::StepDir;
use crate::play::{PlayLink, PlayView, Stepper, play_channel, run_session};
use oteryn_client_runtime::world_interaction::{Action, Command, Target};
use oteryn_session::refusal_detail::RefusalDetail;
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepDisposition,
    StepOutcome,
};
use std::collections::VecDeque;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    steps: VecDeque<Option<RefusalDetail>>,
    uses: VecDeque<String>,
    detail: Option<RefusalDetail>,
}

impl Stepper for Peer {
    fn refusal_detail(&self) -> Option<RefusalDetail> {
        self.detail
    }

    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        self.detail = self
            .steps
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        Ok(StepOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: if self.detail.is_some() {
                StepDisposition::Blocked
            } else {
                StepDisposition::Moved
            },
            result_server_sequence: 1,
            world_spatial_delta: None,
            world_object_overlay_delta: None,
            gate_message_id: None,
        })
    }

    async fn world_context(&mut self, command: &Command) -> Result<String, SessionError> {
        assert_eq!(command.action, Action::Use);
        self.uses.pop_front().ok_or(SessionError::SessionUnusable)
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }

    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}

async fn rendered(
    view: &mut PlayView,
    link: &PlayLink,
    ready: impl Fn(&PlayView) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            view.tick(link).map_err(|_| "native tick failed")?;
            if ready(view) {
                return Ok::<(), Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await?
}

#[test]
fn clhud_step_and_use_driver_present_every_detail_and_clear_success() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()
                .map_err(|error| format!("{error:?}"))?
                .block_on(async {
                    let (link, commands, outbox) = play_channel();
                    let mut steps: VecDeque<_> = RefusalDetail::ALL.into_iter().map(Some).collect();
                    steps.push_back(None);
                    let mut uses: VecDeque<_> = RefusalDetail::ALL
                        .into_iter()
                        .map(|detail| detail.reference_message().to_owned())
                        .collect();
                    uses.push_back("Use: completed".into());
                    let task = tokio::spawn(run_session(
                        Peer {
                            steps,
                            uses,
                            detail: None,
                        },
                        commands,
                        outbox,
                    ));
                    let mut view = crate::play::tests::view()?;
                    for detail in RefusalDetail::ALL {
                        link.request(StepDir::North);
                        rendered(&mut view, &link, |view| {
                            view.combat_status().contains(detail.reference_message())
                        })
                        .await?;
                    }
                    link.request(StepDir::North);
                    rendered(&mut view, &link, |view| {
                        !RefusalDetail::ALL
                            .iter()
                            .any(|detail| view.combat_status().contains(detail.reference_message()))
                    })
                    .await?;
                    for expected in RefusalDetail::ALL
                        .into_iter()
                        .map(RefusalDetail::reference_message)
                        .chain(["Use: completed"])
                    {
                        link.commands.send(PlayCommand::WorldContext(Command {
                            target: Target::Item(oteryn_session::ItemHandle::MIN),
                            action: Action::Use,
                            selection: None,
                        }))?;
                        rendered(&mut view, &link, |view| {
                            view.hud_status().contains(expected)
                        })
                        .await?;
                        assert_eq!(link.world_context_feedback().as_deref(), Some(expected));
                    }
                    assert!(!view.hud_status().contains("It is locked."));
                    link.request_logout();
                    tokio::time::timeout(Duration::from_secs(3), task).await??;
                    Ok::<(), Error>(())
                })
                .map_err(|error| format!("{error:?}"))?
        })?
        .join()
        .map_err(|_| "refusal detail driver thread failed")?
}
