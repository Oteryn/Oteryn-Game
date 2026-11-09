use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::Mailbox, mailbox::lock},
};
use oteryn_client_runtime::{
    charm_progression::catalogue as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::charm_progression::Panel;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn bestiary_catalogue_request(&self, action: usize) {
        submit(&mut lock(&self.mailbox), &self.commands, action);
    }
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
) {
    let panel = &mut mailbox.properties.charm_progression;
    if !panel.selected || panel.pending {
        return;
    }
    let mut request = wire::Request {
        after: 0,
        race: 0,
        category: panel.catalogue.category,
    };
    match action {
        0 => {}
        1 => {
            let Some(next) = panel.catalogue.page.next else {
                panel.feedback = "End of creature catalogue".into();
                return;
            };
            request.after = next;
        }
        2 | 3 => {
            let Some(race) = panel.fields[1]
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|n| (1..=1024).contains(n))
            else {
                panel.feedback = "Select a creature race from the catalogue".into();
                return;
            };
            request.race = race;
            if action == 3 {
                let Some(next) = panel.catalogue.next_loot else {
                    panel.feedback = "End of creature loot".into();
                    return;
                };
                request.after = next;
            }
        }
        4 | 5 => {
            let count = panel.catalogue.page.categories.len() + 1;
            let index = usize::from(request.category);
            request.category = (if action == 4 {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            }) as u8;
        }
        6 | 7 => {
            let cards = &panel.catalogue.page.cards;
            if cards.is_empty() {
                return;
            }
            let selected = panel.fields[1].parse::<u16>().ok();
            let index = cards
                .iter()
                .position(|c| Some(c.race) == selected)
                .unwrap_or(0);
            let index = if action == 6 {
                (index + cards.len() - 1) % cards.len()
            } else {
                (index + 1) % cards.len()
            };
            panel.fields[1] = cards[index].race.to_string();
            panel.catalogue.detail.clear();
            panel.catalogue.next_loot = None;
            return;
        }
        _ => return,
    }
    let Ok(payload) = wire::encode_request(request) else {
        panel.feedback = "Invalid creature catalogue request".into();
        return;
    };
    if !panel.begin(84) {
        return;
    }
    panel.catalogue.pending = Some(request);
    if commands
        .send(PlayCommand::Feature(FeatureCommand {
            capability: 67,
            kind: 84,
            payload,
        }))
        .is_err()
    {
        panel.catalogue.pending = None;
        panel.failed(tr!("The session has ended"));
    }
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) -> bool {
    if command.payload.first() != Some(&255) {
        return panel.catalogue.pending.is_some();
    }
    let Ok(request) = wire::decode_request(&command.payload) else {
        return true;
    };
    if !panel.pending || panel.catalogue.pending != Some(request) {
        return true;
    }
    panel.pending = false;
    panel.catalogue.pending = None;
    if !crate::play::feature_0115_mail::protocol::accepted(result) {
        panel.feedback = "The server refused the creature catalogue request".into();
        return true;
    }
    let Ok(page) = wire::decode_page(&result.payload) else {
        panel.feedback = "Invalid creature catalogue reply".into();
        return true;
    };
    if page.next.is_some_and(|next| next <= request.after)
        || usize::from(request.category) > page.categories.len()
    {
        panel.feedback = "Invalid creature catalogue cursor or category".into();
        return true;
    }
    let confirmed_names: Vec<_> = page
        .cards
        .iter()
        .map(|card| (card.race, card.name.clone()))
        .collect();
    if request.race == 0 {
        if page
            .next
            .is_some_and(|next| page.cards.last().is_none_or(|card| next != card.race))
        {
            panel.feedback = "Invalid creature catalogue page boundary".into();
            return true;
        }
        if page
            .cards
            .first()
            .is_some_and(|card| card.race <= request.after)
        {
            panel.feedback = "Invalid creature catalogue order".into();
            return true;
        }
        panel.catalogue.category = request.category;
        panel.catalogue.detail.clear();
        panel.catalogue.next_loot = None;
        if let Some(card) = page.cards.first() {
            panel.fields[1] = card.race.to_string();
        }
        panel.catalogue.page = page;
    } else {
        let [card] = page.cards.as_slice() else {
            panel.feedback = "Creature detail unavailable".into();
            return true;
        };
        if card.race != request.race {
            panel.feedback = "Mismatched creature detail".into();
            return true;
        }
        panel.catalogue.detail = format!("{} [{}]\r\n{}", card.name, card.category, card.detail);
        panel.catalogue.next_loot = page.next;
    }
    panel.catalogue.names.extend(confirmed_names);
    panel.feedback = "Creature catalogue updated".into();
    true
}

#[cfg(test)]
#[path = "../tests/catalogue.rs"]
mod tests;
