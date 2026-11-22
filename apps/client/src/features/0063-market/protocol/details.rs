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
        let mut mailbox = lock(&self.0);
        let mailbox = &mut *mailbox;
        let applied = details::apply(&mut mailbox.market, command, result.status, &result.payload);
        #[cfg(bevy_shell)]
        if applied && let Some(stats) = &mailbox.market.details.statistics {
            mailbox
                .screen_loot
                .market_price(stats.definition, stats.average);
        }
        if applied
            && let Some((id, price)) = mailbox
                .market
                .details
                .statistics
                .as_ref()
                .and_then(|s| Some((s.definition, s.average?)))
        {
            mailbox.refresh_loot_quote(id, price, oteryn_client_ui::analyser::PriceSource::Market);
        }
    }
}

#[cfg(all(test, bevy_shell))]
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

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_market_response(
        &self,
        response: &oteryn_client_runtime::market::MarketResponse,
    ) {
        self.market_result(response);
    }
}
