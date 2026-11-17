//! Carried/ground container adapter. Depot browsing is not covered.
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::container_tree::{ContainerViewOutcome, ContainerViews};
use oteryn_session::SessionEvent;

impl Outbox {
    #[cfg(test)]
    pub(in crate::play) fn container_initialize(&self, views: Option<ContainerViews>) {
        self.container_initialize_with_house_capacity(views, false);
    }
    pub(in crate::play) fn container_initialize_with_house_capacity(
        &self,
        views: Option<ContainerViews>,
        selected: bool,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.containers.house_capacity_v2 = selected;
        oteryn_client_ui::house_container_capacity::project(&mut mailbox.containers, views);
    }
    pub(in crate::play) fn container_events(&self, events: &[SessionEvent]) {
        let mut mailbox = lock(&self.0);
        for event in events {
            if let SessionEvent::ContainerViews(delta) = event {
                oteryn_client_ui::house_container_capacity::project(
                    &mut mailbox.containers,
                    Some(delta.value.clone()),
                );
            }
        }
    }
    pub(in crate::play) fn container_result(&self, outcome: Option<ContainerViewOutcome>) {
        oteryn_client_ui::container_tree::protocol::apply_result(
            &mut lock(&self.0).containers,
            outcome,
        );
    }
    pub(in crate::play) fn container_end(&self) {
        lock(&self.0).containers.end();
    }
}
impl PlayLink {
    pub fn container_panel(&self) -> oteryn_client_ui::container_tree::ContainerPanel {
        lock(&self.mailbox).containers.clone()
    }
}

macro_rules! session_trait {
    ($($methods:tt)*) => {
        super::feature_0004_item_view_move::protocol::session_trait! {
            $($methods)*
            fn container_views(&self) -> Option<oteryn_client_runtime::container_tree::ContainerViews> { None }
            fn container_view(&mut self, _intent: oteryn_client_runtime::container_tree::ContainerViewIntent) -> impl std::future::Future<Output = Result<Option<oteryn_client_runtime::container_tree::ContainerViewOutcome>, oteryn_session::SessionError>> + Send {
                async { Err(oteryn_session::SessionError::CapabilityNotSelected { capability: 14 }) }
            }
        }
    };
}
pub(in crate::play) use session_trait;

macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        super::feature_0004_item_view_move::protocol::dispatch! {
            $command, $session, $outbox, $held, $publish;
            Ok(crate::combat_input::PlayCommand::ContainerView(intent)) => {
                match $session.container_view(intent).await {
                    Ok(outcome) => {
                        $outbox.container_result(outcome);
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(crate::combat_input::PlayCommand::MoveContents(requests)) => {
                let mut failure = None;
                for request in requests {
                    match $session.move_item(request).await {
                        Ok(response) => {
                            let more = $outbox.container_bulk_result(response);
                            $publish(&mut $session, &$outbox, &mut $held);
                            if !more { break; }
                        }
                        Err(error) => { failure = Some(error); break; }
                    }
                }
                if let Some(error) = failure { error } else { continue; }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;
