use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_runtime::{
    charm_progression::catalogue as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::charm_progression::Panel;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn charm_catalogue_request(&self, next: bool) {
        submit(&mut lock(&self.mailbox), &self.commands, next);
    }
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    next: bool,
) {
    let panel = &mut mailbox.properties.charm_progression;
    if !panel.selected || panel.pending {
        return;
    }
    let after = if next {
        let Some(after) = panel.charm_browser.next else {
            panel.feedback = "All charm names are loaded".into();
            return;
        };
        after
    } else {
        0
    };
    let Ok(payload) = wire::encode_charm_request(after) else {
        return;
    };
    if !panel.begin(84) {
        return;
    }
    panel.charm_browser.pending = Some(after);
    if commands
        .send(PlayCommand::Feature(FeatureCommand {
            capability: 67,
            kind: 84,
            payload,
        }))
        .is_err()
    {
        panel.charm_browser.pending = None;
        panel.failed("The session has ended");
    }
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) -> bool {
    if command.payload.first() != Some(&254) {
        return panel.charm_browser.pending.is_some();
    }
    let Ok(after) = wire::decode_charm_request(&command.payload) else {
        return true;
    };
    if !panel.pending
        || panel.charm_browser.pending != Some(after)
        || (command.capability, command.kind) != (67, 84)
    {
        return true;
    }
    panel.pending = false;
    panel.charm_browser.pending = None;
    if !crate::play::feature_0115_mail::protocol::accepted(result) {
        panel.feedback = "The server refused the charm catalogue".into();
        return true;
    }
    let Ok(page) = wire::decode_page(&result.payload) else {
        panel.feedback = "Invalid charm catalogue".into();
        return true;
    };
    let valid = page.cards.iter().all(|c| {
        c.race > after
            && u32::from(c.race) <= oteryn_client_runtime::charm_progression::charm::MAX_CHARM
            && matches!(c.category.as_str(), "Major" | "Minor")
    }) && page
        .next
        .is_none_or(|n| page.cards.last().is_some_and(|c| c.race == n && n > after));
    if !valid {
        panel.feedback = "Invalid charm catalogue cursor".into();
        return true;
    }
    let mut cards = if after == 0 {
        std::collections::BTreeMap::new()
    } else {
        panel.charm_browser.cards.clone()
    };
    for card in page.cards {
        if cards.contains_key(&card.race)
            || cards
                .values()
                .any(|c: &wire::Card| c.name.eq_ignore_ascii_case(&card.name))
        {
            panel.feedback = "Repeated charm catalogue entry".into();
            return true;
        }
        cards.insert(card.race, card);
    }
    if panel.fields[0].is_empty()
        && let Some(id) = cards.keys().next()
    {
        panel.fields[0] = id.to_string();
    }
    panel.charm_browser.cards = cards;
    panel.charm_browser.next = page.next;
    panel.charm_browser.loaded = true;
    panel.feedback = "Charm names and descriptions updated".into();
    true
}

#[cfg(test)]
#[path = "../tests/charm_browser.rs"]
mod tests;
