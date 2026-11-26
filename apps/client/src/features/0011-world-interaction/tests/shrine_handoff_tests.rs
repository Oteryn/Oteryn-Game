use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel};
use oteryn_client_runtime::{
    imbue::{ImbueResponse, imbuement},
    world_interaction::{Action, Command, Response, Target, world_object::WorldObjectTarget},
};
use oteryn_session::{
    CommandStatus, ItemHandle, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;

struct ShrinePeer(Option<imbuement::ImbuementWindow>);

impl Stepper for ShrinePeer {
    async fn world_context(&mut self, _: &Command, _: bool) -> Result<String, SessionError> {
        Ok(if self.0.is_some() {
            "Imbuement shrine opened".into()
        } else {
            "Use: requirements not met".into()
        })
    }

    async fn world_context_response(
        &mut self,
        command: &Command,
        raw_wire: bool,
    ) -> Result<Response, SessionError> {
        assert!(matches!(
            (&command.target, command.action),
            (Target::Object(_), Action::Use)
        ));
        match self.0.take() {
            Some(window) => Ok(Response::Imbuement(ImbueResponse {
                status: CommandStatus::Accepted,
                reply: Some(imbuement::ImbuementReply {
                    result: imbuement::ImbuementResult::Ok,
                    window: Some(window),
                }),
                success: Vec::new(),
            })),
            None => self
                .world_context(command, raw_wire)
                .await
                .map(Response::Feedback),
        }
    }

    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        Err(SessionError::SessionUnusable)
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }

    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Err(SessionError::SessionUnusable)
    }
}

#[tokio::test]
async fn imbuement_shrine_use_opens_panel_through_world_context_handoff() -> Result<(), SessionError>
{
    let (link, _, outbox) = play_channel();
    outbox.imbue_initialize(true);
    let command = Command {
        action: Action::Use,
        target: Target::Object(WorldObjectTarget {
            placement: vec![1; 40],
            expected_revision: 1,
        }),
        selection: None,
    };

    outbox
        .world_context_run(&mut ShrinePeer(None), &command)
        .await?;
    assert!(!link.imbue_panel().open);
    assert!(link.imbue_panel().window.is_none());
    assert_eq!(
        link.world_context_feedback().as_deref(),
        Some("Use: requirements not met")
    );

    // Initial shrine window from gameplay_transport/imbuement.rs:352.
    let window = imbuement::ImbuementWindow {
        slot_count: 0,
        applied: Vec::new(),
        offers: Vec::new(),
        clear_gold: 15_000,
    };
    link.imbue_item(ItemHandle::MIN);
    outbox
        .world_context_run(&mut ShrinePeer(Some(window.clone())), &command)
        .await?;
    let panel = link.imbue_panel();
    assert!(panel.open);
    assert_eq!(panel.window, Some(window));
    assert_eq!(panel.item, None);
    Ok(())
}
