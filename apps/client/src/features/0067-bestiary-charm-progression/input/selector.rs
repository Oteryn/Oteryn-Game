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
    pub fn charm_selector_request(&self, intent: oteryn_client_ui::charm_progression::Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn charm_race_names_request(&self) {
        request_races(&mut lock(&self.mailbox), &self.commands);
    }
    pub fn charm_selector_catalogue_request(&self, after: u8) {
        request(&mut lock(&self.mailbox), &self.commands, after);
    }
    pub fn charm_selector_select(&self, charm: u32, race: Option<u32>) -> bool {
        lock(&self.mailbox)
            .properties
            .charm_progression
            .select_confirmed(charm, race)
    }
}
pub(in crate::play) fn request(
    m: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    after: u8,
) {
    let p = &mut m.properties.charm_progression;
    if after > 32 || !p.begin(84) {
        return;
    }
    p.selector.pending = Some(after);
    p.selector.attempted = true;
    p.selector.next = None;
    if commands
        .send(PlayCommand::Feature(FeatureCommand {
            capability: 67,
            kind: 84,
            payload: vec![255, 2, after],
        }))
        .is_err()
    {
        p.selector.pending = None;
        p.failed("The session has ended");
    }
}
pub(in crate::play) fn apply(
    p: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) -> bool {
    if command.payload.starts_with(&[255, 3]) {
        return apply_races(p, command, result);
    }
    if !command.payload.starts_with(&[255, 2]) {
        return p.selector.pending.is_some() || p.selector.race_pending.is_some();
    }
    let [255, 2, after] = command.payload.as_slice() else {
        return true;
    };
    if !p.pending || p.selector.pending != Some(*after) {
        return true;
    }
    p.pending = false;
    p.selector.pending = None;
    if !crate::play::feature_0115_mail::protocol::accepted(result) {
        p.feedback = "The server refused charm descriptions".into();
        return true;
    }
    let Ok(page) = wire::decode_page(&result.payload) else {
        p.feedback = "Invalid charm descriptions".into();
        return true;
    };
    if page
        .cards
        .iter()
        .any(|c| c.race <= u16::from(*after) || c.race > 32)
        || page
            .next
            .is_some_and(|n| n > 32 || page.cards.last().is_none_or(|c| c.race != n))
        || (*after != 0
            && p.selector
                .cards
                .last()
                .is_none_or(|c| c.race != u16::from(*after)))
    {
        p.feedback = "Invalid charm catalogue cursor".into();
        return true;
    }
    if *after == 0 {
        p.selector.cards.clear();
    }
    p.selector.cards.extend(page.cards);
    p.selector.next = page.next.and_then(|n| n.try_into().ok());
    p.feedback = "Charm descriptions updated".into();
    true
}

#[cfg(test)]
#[path = "../tests/selector.rs"]
mod tests;

pub(in crate::play) fn request_races(
    m: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) -> bool {
    let p = &mut m.properties.charm_progression;
    if !p.selected || p.pending || p.selector.race_failed {
        return false;
    }
    let ids: Vec<u16> = p
        .reply
        .iter()
        .flat_map(|r| &r.bestiary)
        .filter(|r| {
            r.kill_count > 0
                && !p
                    .selector
                    .races
                    .iter()
                    .any(|c| u32::from(c.race) == r.race.get())
        })
        .filter_map(|r| r.race.get().try_into().ok())
        .take(5)
        .collect();
    if ids.is_empty() || !p.begin(84) {
        return false;
    }
    let mut payload = vec![255, 3];
    for id in &ids {
        payload.extend(id.to_le_bytes());
    }
    p.selector.race_pending = Some(ids);
    if commands
        .send(PlayCommand::Feature(FeatureCommand {
            capability: 67,
            kind: 84,
            payload,
        }))
        .is_err()
    {
        p.selector.race_pending = None;
        p.failed("The session has ended");
        p.selector.race_failed = true;
    }
    true
}
fn apply_races(p: &mut Panel, command: &FeatureCommand, result: &GatedCommandResult) -> bool {
    let Some(ids) = p.selector.race_pending.as_ref() else {
        return true;
    };
    let mut expected = vec![255, 3];
    for id in ids {
        expected.extend(id.to_le_bytes());
    }
    if !p.pending || command.payload != expected {
        return true;
    }
    p.pending = false;
    let ids = p.selector.race_pending.take().unwrap_or_default();
    let page = wire::decode_page(&result.payload)
        .ok()
        .filter(|page| page.next.is_none() && page.cards.iter().map(|c| c.race).eq(ids));
    if crate::play::feature_0115_mail::protocol::accepted(result)
        && let Some(page) = page
    {
        p.selector.races.extend(page.cards);
    } else {
        p.selector.race_failed = true;
        p.feedback = "Creature names unavailable".into();
    }
    true
}

pub(in crate::play) fn submit(
    m: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: oteryn_client_ui::charm_progression::Intent,
) {
    let p = &mut m.properties.charm_progression;
    if p.selected && !p.pending && intent == oteryn_client_ui::charm_progression::Intent::Assign {
        p.fields[1] = p
            .selector
            .selected_race
            .map_or_else(String::new, |race| race.to_string());
    }
    super::submit(m, commands, intent);
}
