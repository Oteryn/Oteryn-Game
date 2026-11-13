//! The native browser and portable tests use the same command queue and result caller.
use crate::combat_input::PlayCommand;
use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::market::details::{self, Query};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn market_query(&self, query: Query) -> bool {
        enqueue(&mut lock(&self.mailbox), &self.commands, query)
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    query: Query,
) -> bool {
    let Some(command) = details::request(&mut mailbox.market, query) else {
        return false;
    };
    if commands.send(PlayCommand::Feature(command)).is_err() {
        mailbox.market.details.pending = None;
        mailbox.market.details.feedback = Some("The session has ended");
        return false;
    }
    true
}
impl Outbox {
    pub(in crate::play) fn market_details_end(&self) {
        lock(&self.0).market.details = Default::default();
    }
    pub(in crate::play) fn market_details_result(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        details::apply(
            &mut lock(&self.0).market,
            command,
            result.status,
            &result.payload,
        );
    }
}

#[cfg(windows)]
pub(in crate::play) fn enqueue_book(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: oteryn_client_ui::market::Intent,
) -> bool {
    use oteryn_client_ui::market::Intent;
    if mailbox.market.pending.is_some() {
        return false;
    }
    let page = match intent {
        Intent::Browse => 0,
        Intent::NextPage => {
            if !mailbox.market.details.book_has_next {
                return false;
            }
            mailbox.market.page.saturating_add(1)
        }
        Intent::PreviousPage => mailbox.market.page.saturating_sub(1),
        _ => mailbox.market.page,
    };
    let definition = mailbox.market.definition;
    enqueue(mailbox, commands, Query::Book { definition, page })
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_market_initialize(&self, selected: bool) {
        self.market_initialize(selected);
    }
    pub(crate) fn bevy_market_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        self.market_details_result(command, result);
    }
}
