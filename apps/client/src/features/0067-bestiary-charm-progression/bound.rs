//! Atomic Bevy bindings. Native entry points retain their existing behavior.
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::charm_progression::{Intent, Panel};
use std::sync::{Arc, Mutex, Weak};
#[derive(Clone)]
pub(crate) struct BestiaryBinding {
    session: Weak<Mutex<Mailbox>>,
    pub panel: Panel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BestiaryControl {
    Request(Intent),
    Catalogue(usize),
    CharmNames,
    RaceNames,
    SelectCharm(u32),
    SelectRace(u32),
    Close,
}
impl PlayLink {
    pub(crate) fn bestiary_binding(&self) -> BestiaryBinding {
        BestiaryBinding {
            session: Arc::downgrade(&self.mailbox),
            panel: self.charm_progression_panel(),
        }
    }
    pub(crate) fn bestiary_binding_current(&self, expected: &BestiaryBinding) -> bool {
        self.bestiary_binding_matches(expected, &lock(&self.mailbox))
    }
    fn bestiary_binding_matches(&self, expected: &BestiaryBinding, m: &Mailbox) -> bool {
        Weak::ptr_eq(&expected.session, &Arc::downgrade(&self.mailbox))
            && !self.commands.is_closed()
            && !m.server_modal.is_active()
            && m.properties.charm_progression == expected.panel
            && expected.panel.open
            && expected.panel.selected
            && !expected.panel.pending
    }
    /// Revalidate the exact session, displayed state, availability and modal under its lock.
    pub(crate) fn bestiary_dispatch_bound(
        &self,
        expected: &BestiaryBinding,
        control: BestiaryControl,
        confirmed: bool,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        // Closing presentation preserves every in-flight operation and receipt.
        if control == BestiaryControl::Close {
            if !expected
                .session
                .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
                || self.commands.is_closed()
                || m.quest_shutdown
                || m.custom_hotkeys_ended
            {
                return false;
            }
            let open = m.properties.charm_progression.open;
            m.properties.charm_progression.open = false;
            return open;
        }

        if !self.bestiary_binding_matches(expected, &m) {
            return false;
        }
        let p = &mut m.properties.charm_progression;
        match control {
            BestiaryControl::Close => {
                p.open = false;
            }
            BestiaryControl::SelectCharm(charm) => return p.select_confirmed(charm, None),
            BestiaryControl::SelectRace(race) => {
                if !p
                    .reply
                    .as_ref()
                    .is_some_and(|r| r.bestiary.iter().any(|r| r.race.get() == race))
                    && !p
                        .catalogue
                        .page
                        .cards
                        .iter()
                        .any(|r| u32::from(r.race) == race)
                {
                    return false;
                }
                p.fields[1] = race.to_string();
                p.catalogue.detail.clear();
                p.catalogue.detail_card = None;
                p.catalogue.next_loot = None;
            }
            BestiaryControl::Request(intent) => {
                if matches!(intent, Intent::Unlock | Intent::Remove | Intent::ResetAll)
                    && !confirmed
                {
                    return false;
                }
                if intent != Intent::Query {
                    let Some(charm) = p.reply.as_ref().and_then(|r| {
                        r.charms
                            .charms
                            .iter()
                            .find(|c| c.charm.to_string() == p.fields[0])
                    }) else {
                        return false;
                    };
                    if intent == Intent::Assign
                        && (charm.unlocked_stage == 0
                            || !p
                                .assignment_choices()
                                .iter()
                                .any(|(id, _)| id.to_string() == p.fields[1]))
                    {
                        return false;
                    }
                    if intent == Intent::Remove && charm.assigned_race.is_none() {
                        return false;
                    }
                }
                if super::super::protocol::command(p, intent).is_err() {
                    return false;
                }
                super::super::input::submit(&mut m, &self.commands, intent);
                return m.properties.charm_progression.pending;
            }
            BestiaryControl::Catalogue(action) => {
                if action > 5 {
                    return false;
                }
                super::super::input::catalogue::submit(&mut m, &self.commands, action);
            }
            BestiaryControl::CharmNames => {
                let after = if p.selector.attempted {
                    let Some(next) = p.selector.next else {
                        return false;
                    };
                    next
                } else {
                    0
                };
                super::super::input::selector::request(&mut m, &self.commands, after);
            }
            BestiaryControl::RaceNames => {
                return super::super::input::selector::request_races(&mut m, &self.commands);
            }
        }
        true
    }
}
