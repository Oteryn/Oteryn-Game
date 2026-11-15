use crate::play::{Stepper, play_channel};
use oteryn_client_runtime::world_interaction::{Action, Command, Target};
use oteryn_session::{SessionError, SessionEvent};

struct LookPeer;
impl Stepper for LookPeer {
    async fn world_context(&mut self, _: &Command) -> Result<String, SessionError> {
        Ok("You see a sword.\nIt weighs 35.00 oz.".into())
    }
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn serve(&mut self, _: std::time::Duration) -> Result<(), SessionError> {
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<oteryn_session::LogoutResult>, SessionError> {
        Ok(None)
    }
}

#[tokio::test]
async fn otc_textmsg_look_handler_retains_full_description_and_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let command = Command {
        target: Target::Item(oteryn_session::ItemHandle::MIN),
        action: Action::Look,
        selection: None,
    };
    outbox.world_context_run(&mut LookPeer, &command).await?;
    let first = link.look_message().ok_or("look receipt")?;
    assert_eq!(first.text, "You see a sword.\nIt weighs 35.00 oz.");
    outbox.world_context_run(&mut LookPeer, &command).await?;
    let second = link.look_message().ok_or("second receipt")?;
    assert!(second.receipt > first.receipt);
    assert!(link.world_context_feedback().is_some());
    assert!(
        link.chat_panel()
            .console
            .notices
            .text(Default::default(), Default::default())
            .contains(&first.text)
    );
    outbox
        .world_context_run(
            &mut LookPeer,
            &Command {
                action: Action::Use,
                ..command
            },
        )
        .await?;
    assert_eq!(
        link.look_message().ok_or("held look")?.receipt,
        second.receipt
    );
    Ok(())
}
