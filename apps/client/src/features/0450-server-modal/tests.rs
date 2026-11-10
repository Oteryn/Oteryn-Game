use super::*;
use crate::combat_input::{CombatCommand, CombatReport};
use oteryn_client_runtime::{
    raw_feature::FeatureCommand,
    server_modal::{Dialog, Image, Reply},
};
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionEvent, StepDirection, StepOutcome,
};
struct Peer {
    answers: usize,
}
impl Stepper for Peer {
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        assert_eq!(command.capability, 450);
        let reply = match command.kind {
            736 => Reply::Dialog(Dialog {
                id: 73,
                title: "Choose".into(),
                message: "Server-defined options".into(),
                choices: vec![(9, "Nine".into())],
                buttons: vec![(4, "Accept".into()), (8, "Cancel".into())],
                cancel_button: Some(8),
                image: Some(Image {
                    width: 1,
                    height: 1,
                    rgba: vec![255, 0, 0, 255],
                }),
            }),
            737 => {
                self.answers += 1;
                Reply::Answered(
                    Answer::decode(&command.payload).map_err(|_| SessionError::SessionUnusable)?,
                )
            }
            _ => return Err(SessionError::SessionUnusable),
        };
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply.encode().map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
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
#[test]
fn server_modal_session_caller_delivers_inline_image_and_exact_answer_ack()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut peer = Peer { answers: 0 };
            let mut driver = Driver::new(true);
            assert_eq!(driver.poll(&mut peer).await?, None);
            let dialog = driver
                .panel
                .active
                .as_ref()
                .ok_or("missing native descriptor")?;
            assert_eq!(
                dialog.image.as_ref().ok_or("image")?.rgba,
                vec![255, 0, 0, 255]
            );
            let answer = Answer {
                dialog_id: 73,
                button_id: 4,
                choice_id: Some(9),
            };
            let (tx, rx) = oneshot::channel();
            driver.sender.try_send((
                Answer {
                    choice_id: Some(10),
                    ..answer
                },
                tx,
            ))?;
            assert_eq!(driver.poll(&mut peer).await?, None);
            assert!(!rx.await?);
            assert_eq!(peer.answers, 0);
            let (tx, rx) = oneshot::channel();
            driver.sender.try_send((answer, tx))?;
            assert_eq!(driver.poll(&mut peer).await?, Some(answer));
            assert!(rx.await?);
            assert_eq!(peer.answers, 1);
            assert!(driver.panel.active.is_none());
            let sender = driver.sender.clone();
            drop(driver);
            assert!(sender.is_closed());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
