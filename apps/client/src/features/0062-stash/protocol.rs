use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::{
    raw_feature::FeatureCommand,
    stash::{StashCommand, StashResponse},
};
use oteryn_client_ui::stash::{
    self as ui, Batch, BatchKind, Intent, container_batch::Request, stash,
};
use oteryn_session::SessionError;

#[path = "reverse.rs"]
mod reverse;
impl Outbox {
    pub(in crate::play) fn stash_publish(&self, session: &impl Stepper) {
        let sources = oteryn_client_ui::depot::DepotPanel {
            items: session.depot_items(),
            ..Default::default()
        };
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.stash;
        panel.carried = oteryn_client_ui::depot::deposit_items(&sources);
        panel.actions_selected = session.feature_selected(stash::CAPABILITY_STASH_ACTIONS_V1);
        panel.container_selected = session.feature_selected(BatchKind::Stash.capability());
        panel.managed_selected = session.feature_selected(BatchKind::ManagedMove.capability());
    }
    pub(in crate::play) fn stash_initialize(&self, selected: bool) {
        lock(&self.0).stash.selected = selected;
    }
    pub(in crate::play) fn stash_result(&self, response: &StashResponse) {
        oteryn_client_ui::stash::apply_result(&mut lock(&self.0).stash, response);
    }
    pub(in crate::play) async fn stash_extension(
        &self,
        session: &mut impl Stepper,
        command: &FeatureCommand,
    ) -> Result<(), SessionError> {
        let result = session.feature(command).await?;
        let (refresh, followup) = {
            let mut mailbox = lock(&self.0);
            let panel = &mut mailbox.stash;
            if command.kind == stash::COMMAND_TYPE_STASH_ROW_ACTION {
                let intent = stash::decode_stash_row_intent(&command.payload)
                    .map_err(|_| SessionError::SessionUnusable)?;
                let reference = ui::apply_row_action(panel, &result);
                if intent.action.changes_loot_list() && reference.is_some() {
                    panel.feedback = Some("The active loot list was updated.".into());
                }
                let refresh = (reference.is_none() || intent.action.changes_loot_list())
                    .then(|| prepare_refresh(panel, Intent::Refresh))
                    .flatten();
                let followup = reference
                    .filter(|_| !intent.action.changes_loot_list())
                    .map(|reference| (intent.action, reference.get()));
                (refresh, followup)
            } else {
                let request =
                    Request::decode(&command.payload).ok_or(SessionError::SessionUnusable)?;
                if command.capability == BatchKind::ManagedMove.capability() {
                    panel.managed_batch.apply(request, &result);
                    return Ok(());
                }
                panel.pending = false;
                panel.container_batch.apply(request, &result);
                if !panel.open && matches!(panel.container_batch, Batch::Quote { .. }) {
                    panel.container_batch = Batch::Idle;
                }
                let intent = if matches!(panel.container_batch, Batch::Committed(_)) {
                    Intent::Filter
                } else {
                    Intent::Refresh
                };
                let refresh = matches!(request, Request::Confirm { .. })
                    .then(|| prepare_refresh(panel, intent))
                    .flatten();
                (refresh, None)
            }
        };
        if let Some((command, feedback)) = refresh {
            let response = self.stash_execute(session, &command).await?;
            let mut mailbox = lock(&self.0);
            ui::apply_result(&mut mailbox.stash, &response);
            if feedback.is_some() && mailbox.stash.feedback.is_none() {
                mailbox.stash.feedback = feedback;
            }
        }
        if let Some((action, reference)) = followup {
            self.stash_row_followup(session, action, reference).await?;
        }
        Ok(())
    }

    async fn stash_row_followup(
        &self,
        session: &mut impl Stepper,
        action: stash::StashRowAction,
        definition: u32,
    ) -> Result<(), SessionError> {
        match action {
            stash::StashRowAction::Inspect => {
                use oteryn_client_ui::inspection as inspect;
                let command = {
                    let mut mailbox = lock(&self.0);
                    let panel = &mut mailbox.discovery.inspection;
                    let command = inspect::request(panel, inspect::Request::Catalogue(definition));
                    if command.is_some() {
                        panel.open = true;
                        mailbox.stash.open = false;
                    }
                    command
                };
                if let Some(command) = command {
                    let result = session.server_mechanic(&command).await?;
                    self.discovery_result(command.capability, &result);
                }
            }
            stash::StashRowAction::Market => {
                use oteryn_client_ui::market;
                let command = {
                    let mut mailbox = lock(&self.0);
                    if !mailbox.market.selected || mailbox.market.pending.is_some() {
                        return Ok(());
                    }
                    mailbox.market.definition = definition;
                    let command = market::request(&mut mailbox.market, market::Intent::Browse);
                    if command.is_some() {
                        mailbox.market.open = true;
                        mailbox.stash.return_from_market = true;
                        mailbox.stash.open = false;
                    }
                    command
                };
                if let Some(command) = command {
                    self.market_result(&session.market(&command).await?);
                }
            }
            stash::StashRowAction::LootAdd | stash::StashRowAction::LootRemove => {
                return Err(SessionError::SessionUnusable);
            }
        }
        Ok(())
    }
}

fn prepare_refresh(
    panel: &mut ui::StashPanel,
    intent: Intent,
) -> Option<(StashCommand, Option<String>)> {
    let feedback = panel.feedback.take();
    Some((ui::request(panel, intent)?, feedback))
}

impl Mailbox {
    pub(in crate::play) fn stash_end(&mut self) {
        self.stash = oteryn_client_ui::stash::StashPanel {
            sort: self.stash.sort,
            sort_edit: self.stash.sort_edit,
            ..Default::default()
        };
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_stash_initialize(&self, selected: bool) {
        self.stash_initialize(selected);
    }
    pub(crate) fn bevy_stash_result(&self, response: &StashResponse) {
        self.stash_result(response);
    }
}
