use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::depot::{DepotIntent, DepotReply};
use oteryn_session::SessionError;
impl Outbox {
    pub(in crate::play) fn depot_initialize(&self, selected: bool) {
        lock(&self.0).depot.selected = selected;
    }
    pub(in crate::play) fn depot_result(&self, reply: Option<DepotReply>) {
        oteryn_client_ui::depot::apply_result(&mut lock(&self.0).depot, reply);
    }
    pub(in crate::play) fn depot_publish(&self, session: &impl Stepper) {
        lock(&self.0).depot.items = session.depot_items();
        self.imbue_items_changed();
    }
    pub(in crate::play) async fn depot_container_run(
        &self,
        session: &mut impl Stepper,
        handle: oteryn_session::ItemHandle,
    ) -> Result<(), SessionError> {
        let outcome = session.depot_open_container(handle).await?;
        self.depot_publish(session);
        oteryn_client_ui::depot::apply_container(&mut lock(&self.0).depot, outcome);
        self.item_forge_container_ready(handle);
        Ok(())
    }
    pub(in crate::play) async fn depot_run(
        &self,
        session: &mut impl Stepper,
        intent: DepotIntent,
    ) -> Result<(), SessionError> {
        let reply = session.depot(intent).await?;
        self.depot_publish(session);
        self.depot_result(reply);
        Ok(())
    }
}

macro_rules! session_trait {
    ($($methods:tt)*) => {
        super::feature_local_account_achievements::protocol::session_trait! {
            $($methods)*
            fn depot_open_container(&mut self, _handle: oteryn_session::ItemHandle) -> impl std::future::Future<Output = Result<Option<oteryn_client_runtime::depot::ContainerViewOutcome>, oteryn_session::SessionError>> + Send {
                async { Ok(None) }
            }
            fn depot_items(&self) -> oteryn_client_runtime::depot::Items { Default::default() }
            fn depot(&mut self, _intent: oteryn_session::DepotIntent) -> impl std::future::Future<Output = Result<Option<oteryn_session::DepotReply>, oteryn_session::SessionError>> + Send {
                async { Ok(None) }
            }
        }
    };
}
pub(in crate::play) use session_trait;
macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        super::feature_local_account_achievements::protocol::dispatch! {
            $command, $session, $outbox, $held, $publish;
            Ok(crate::combat_input::PlayCommand::DepotContainer(handle)) => {
                match $outbox.depot_container_run(&mut $session, handle).await {
                    Ok(()) => { $publish(&mut $session, &$outbox, &mut $held); continue; }
                    Err(error) => error,
                }
            }
            Ok(crate::combat_input::PlayCommand::Depot(intent)) => {
                match $outbox.depot_run(&mut $session, intent).await {
                    Ok(()) => { $publish(&mut $session, &$outbox, &mut $held); continue; }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;
