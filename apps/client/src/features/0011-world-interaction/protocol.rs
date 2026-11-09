use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::world_interaction::Command;
impl Outbox {
    pub(in crate::play) fn targeting_initialize(&self, enabled: bool) {
        lock(&self.0).world_context.use_with = enabled;
    }

    pub(in crate::play) fn world_context_initialize(&self, look: bool, bed: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.world_context.look = look;
        mailbox.world_context.bed = bed;
    }
    pub(in crate::play) async fn world_context_run(
        &self,
        session: &mut impl Stepper,
        command: &Command,
    ) -> Result<(), oteryn_session::SessionError> {
        let result = session.world_context(command).await;
        let mut mailbox = lock(&self.0);
        mailbox.world_context.pending = false;
        match result {
            Ok(text) => {
                mailbox
                    .world_context
                    .set_feedback(text, std::time::Instant::now());
                Ok(())
            }
            Err(error) => {
                mailbox
                    .world_context
                    .set_feedback(tr!("Session unavailable").into(), std::time::Instant::now());
                Err(error)
            }
        }
    }
}

macro_rules! session_trait {
    ($($methods:tt)*) => {
        super::feature_0037_depot::protocol::session_trait! {
            $($methods)*
            fn targeting_source_current(&self, source: oteryn_session::ItemHandle) -> bool {
                self.inventory().is_some_and(|inventory| {
                    inventory.main_backpack.iter().chain(&inventory.entries)
                        .any(|item| item.handle == source)
                })
            }
            fn world_context(
                &mut self,
                _command: &oteryn_client_runtime::world_interaction::Command,
            ) -> impl std::future::Future<
                Output = Result<String, oteryn_session::SessionError>,
            > + Send {
                async {
                    Err(oteryn_session::SessionError::CapabilityNotSelected { capability: 28 })
                }
            }
        }
    };
}
pub(in crate::play) use session_trait;

macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        super::feature_0051_staff::protocol::dispatch! {
            $command, $session, $outbox, $held, $publish;
            Ok(crate::combat_input::PlayCommand::WorldContext(command)) => {
                match $outbox.world_context_run(&mut $session, &command).await {
                    Ok(()) => {
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;
