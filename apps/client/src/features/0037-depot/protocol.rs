use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::depot::{DepotReply, DepotRequest};
use oteryn_session::SessionError;
impl Outbox {
    pub(in crate::play) fn depot_initialize(&self, selected: bool, cross_town_selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.depot.selected = selected;
        mailbox.depot.cross_town_selected = selected && cross_town_selected;
    }
    pub(in crate::play) fn depot_result(&self, reply: Option<DepotReply>) {
        oteryn_client_ui::depot::apply_result(&mut lock(&self.0).depot, reply);
    }
    pub(in crate::play) fn depot_publish(&self, session: &impl Stepper) {
        lock(&self.0).depot.replace_items(session.depot_items());
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
        request: DepotRequest,
    ) -> Result<(), SessionError> {
        let reply = session.depot(request).await?;
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
            fn depot(&mut self, _request: oteryn_session::DepotRequest) -> impl std::future::Future<Output = Result<Option<oteryn_session::DepotReply>, oteryn_session::SessionError>> + Send {
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
            Ok(crate::combat_input::PlayCommand::Depot(request)) => {
                match $outbox.depot_run(&mut $session, request).await {
                    Ok(()) => { $publish(&mut $session, &$outbox, &mut $held); continue; }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_depot_initialize(&self, selected: bool, cross_town_selected: bool) {
        self.depot_initialize(selected, cross_town_selected);
    }
    pub(crate) fn bevy_depot_result(&self, reply: Option<DepotReply>) {
        self.depot_result(reply);
    }
    pub(crate) fn bevy_depot_items(&self, items: oteryn_client_runtime::depot::Items) {
        lock(&self.0).depot.replace_items(items);
    }
}
