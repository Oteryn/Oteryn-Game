use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn weapon_proficiency_tick(&self, now: std::time::Instant) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) =
            oteryn_client_ui::weapon_proficiency::tick(&mut mailbox.weapon_proficiency, now)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.weapon_proficiency.pending = false;
            mailbox.weapon_proficiency.feedback = Some(tr!("The session has ended"));
        }
    }
    /// Both the advancement icon and the HUD progress bar use this exact weapon identity.
    pub fn weapon_proficiency_open_track(&self, advancement: bool) {
        {
            let mut mailbox = lock(&self.mailbox);
            let p = &mut mailbox.weapon_proficiency;
            if p.pending {
                return;
            }
            let definition = if advancement {
                p.advancements.first().copied()
            } else {
                p.progress_entry().map(|entry| entry.definition)
            };
            let row = definition.and_then(|definition| {
                p.catalogue
                    .as_ref()?
                    .entries
                    .iter()
                    .position(|entry| entry.definition == definition)
            });
            let Some(row) = row else {
                return;
            };
            if !p.select_weapon(row) {
                return;
            }
            let weapon = p.weapon;
            p.advancements.retain(|definition| *definition != weapon);
        }
        lock(&self.mailbox).weapon_proficiency.open = true;
        self.weapon_proficiency_request(Intent::State);
    }
    pub fn weapon_proficiency_select_weapon(&self, row: usize) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.weapon_proficiency.select_weapon(row)
            && let Some(command) = request(&mut mailbox.weapon_proficiency, Intent::State)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.weapon_proficiency.pending = false;
            mailbox.weapon_proficiency.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn weapon_proficiency_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.weapon_proficiency, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.weapon_proficiency.pending = false;
            mailbox.weapon_proficiency.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn weapon_proficiency_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::weapon_proficiency::open_key(events) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.weapon_proficiency;
        panel.open = true;
        // Restart the catalogue walk without invalidating an in-flight page reply.
        if !panel.pending {
            panel.catalogue_offset = 0;
        }
        self.weapon_proficiency_request(Intent::Catalogue);
        true
    }
}

impl PlayLink {
    /// Recheck the displayed model under its owner lock before selecting a perk.
    pub fn weapon_proficiency_action_bound(
        &self,
        expected: &oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel,
        action: super::view::ProficiencyAction,
    ) -> bool {
        use super::view::ProficiencyAction;
        let mut mailbox = lock(&self.mailbox);
        let p = &mut mailbox.weapon_proficiency;
        if !p.open || p != expected {
            return false;
        }
        if matches!(action, ProficiencyAction::Close) {
            p.open = false;
            return true;
        }
        if !p.selected || p.pending {
            return false;
        }
        let intent = match action {
            ProficiencyAction::Close => return false,
            ProficiencyAction::Slot(slot) => {
                if !(1..=2).contains(&slot) {
                    return false;
                }
                p.clear_confirmation = None;
                p.slot = slot;
                p.shaping_choice = 0;
                return true;
            }
            ProficiencyAction::Offer(choice) => {
                let valid =
                    current_slot(p).is_some_and(|slot| usize::from(choice) < slot.offers.len());
                if valid {
                    p.shaping_choice = choice;
                }
                return valid;
            }
            ProficiencyAction::Shape(intent @ (Intent::Clear | Intent::CancelClear)) => {
                // Both only open or close the local confirmation; no command is sent yet.
                request(p, intent);
                return true;
            }
            ProficiencyAction::Shape(Intent::ConfirmClear) => Intent::ConfirmClear,
            ProficiencyAction::Modify(level) => {
                let Some(perk) = p
                    .weapon_entry()
                    .and_then(|entry| entry.levels.get(level))
                    .and_then(|row| row.selected)
                else {
                    p.feedback = Some(tr!("Apply the selected perk before modifying it"));
                    return false;
                };
                let Ok(level) = u8::try_from(level) else {
                    return false;
                };
                if !committed_revision(p) {
                    return false;
                }
                p.level = level;
                p.perk = perk;
                p.clear_perk = false;
                Intent::Modify
            }
            ProficiencyAction::Shape(
                intent @ (Intent::RankUp
                | Intent::OrbRank
                | Intent::ReshapeOffer
                | Intent::ReshapeChoose
                | Intent::DeclineOffer),
            ) => {
                if !committed_revision(p) {
                    return false;
                }
                if intent == Intent::ReshapeChoose
                    && current_slot(p)
                        .is_none_or(|slot| usize::from(p.shaping_choice) >= slot.offers.len())
                {
                    p.feedback = Some(tr!("Select one of the server's current reshape offers"));
                    return false;
                }
                intent
            }
            ProficiencyAction::Shape(_) => return false,
            ProficiencyAction::Refresh => {
                p.catalogue_offset = 0;
                Intent::Catalogue
            }
            ProficiencyAction::Weapon(row) => {
                if !p.select_weapon(row) {
                    return false;
                }
                if !p.state_selected {
                    return true;
                }
                Intent::State
            }
            ProficiencyAction::Perk(level, perk) => {
                let Some(entry) = p.weapon_entry() else {
                    return false;
                };
                let Some(data) = entry.levels.get(level) else {
                    return false;
                };
                let (Ok(level_index), Ok(perk_index)) = (u8::try_from(level), u8::try_from(perk))
                else {
                    return false;
                };
                if entry
                    .thresholds
                    .get(level)
                    .is_none_or(|xp| entry.experience < *xp)
                    || perk >= data.perks.len()
                    || data.selected == Some(perk_index)
                {
                    return false;
                }
                p.level = level_index;
                p.perk = perk_index;
                p.clear_perk = false;
                Intent::SelectPerk
            }
        };
        let Some(command) = request(p, intent) else {
            return false;
        };
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            p.pending = false;
            p.feedback = Some(tr!("The session has ended"));
            return false;
        }
        true
    }
}

/// The committed shaping slot the player selected, as the server last reported it.
fn current_slot(
    p: &oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel,
) -> Option<&oteryn_client_runtime::weapon_proficiency_state::Slot> {
    p.shaping
        .as_ref()
        .filter(|state| state.definition == p.weapon)?
        .slots
        .get(usize::from(p.slot).checked_sub(1)?)?
        .as_ref()
}

/// Bind a slot change to the committed shaping revision; a stale or missing one refuses it.
fn committed_revision(
    p: &mut oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel,
) -> bool {
    let Some(revision) = p
        .shaping
        .as_ref()
        .filter(|state| state.definition == p.weapon && state.revision != 0)
        .map(|state| state.revision)
    else {
        p.feedback = Some(tr!(
            "Refresh committed shaping state before changing a slot"
        ));
        return false;
    };
    p.revision = revision;
    true
}
