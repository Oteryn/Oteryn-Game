//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::weapon_proficiency::{Intent, WeaponProficiencyPanel, request};

pub(in crate::play) fn action_clicked(id: usize, notification: usize) -> bool {
    (matches!(id, 101..=114) || id == 116) && notification == 0 // Win32 BN_CLICKED
}

pub(in crate::play) fn display(p: &WeaponProficiencyPanel) -> String {
    format!(
        "{}\r\n{}\r\n{}",
        if p.pending {
            tr!("Waiting for the server")
        } else if !p.selected {
            tr!("Feature unavailable in this session")
        } else {
            tr!("Ready")
        },
        p.feedback.unwrap_or(""),
        oteryn_client_ui::weapon_proficiency::panel_text(p).replace('\n', "\r\n")
    )
}

pub(in crate::play) fn submit(
    p: &mut WeaponProficiencyPanel,
    id: usize,
    values: [&str; 6],
    clear: bool,
) -> Option<FeatureCommand> {
    if p.pending || !p.selected {
        return None;
    }
    if let Some(intent) = match id {
        112 => Some(Intent::Browse),
        113 => Some(Intent::PreviousPage),
        114 => Some(Intent::NextPage),
        _ => None,
    } {
        return request(p, intent);
    }
    if matches!(id, 109..=111) {
        p.catalogue_offset = match id {
            109 => 0,
            110 => p.catalogue.as_ref().map_or(0, |page| {
                let next = page.offset.saturating_add(page.entries.len() as u32);
                if next < page.total { next } else { page.offset }
            }),
            _ => p.catalogue_offset.saturating_sub(8),
        };
        return request(p, Intent::Catalogue);
    }
    let intent = match id {
        101 => Intent::SelectPerk,
        102 => Intent::Modify,
        103 => Intent::RankUp,
        104 => Intent::OrbRank,
        105 => Intent::ReshapeOffer,
        106 => Intent::ReshapeChoose,
        107 => Intent::DeclineOffer,
        108 => Intent::Clear,
        _ => return None,
    };
    let parsed = (|| {
        Some((
            values[0].trim().parse::<u32>().ok()?,
            values[1].trim().parse::<u8>().ok()?,
            values[2].trim().parse::<u8>().ok()?,
            values[3].trim().parse::<u8>().ok()?,
            values[4].trim().parse::<u64>().ok()?,
            values[5].trim().parse::<u8>().ok()?,
        ))
    })();
    let Some((weapon, level, perk, slot, revision, choice)) = parsed else {
        p.feedback = Some(tr!("Enter valid numeric proficiency fields"));
        return None;
    };
    if level > 6 || (!clear && perk == u8::MAX) || choice > 2 {
        p.feedback = Some(tr!("Enter level 0-6, perk 0-254 and choice 0-2"));
        return None;
    }
    p.weapon = weapon;
    p.level = level;
    p.perk = perk;
    p.slot = slot;
    p.revision = revision;
    p.shaping_choice = choice;
    p.clear_perk = clear;
    request(p, intent)
}

pub(in crate::play) fn choose(p: &mut WeaponProficiencyPanel, field: usize, row: usize) -> bool {
    if p.pending || !p.selected {
        return false;
    }
    match field {
        0 => p.select_weapon(row),
        1 => p.choose_level(row),
        2 => {
            let valid = p
                .weapon_entry()
                .and_then(|entry| entry.levels.get(usize::from(p.level)))
                .is_some_and(|level| row < level.perks.len());
            if valid {
                p.perk = row as u8;
            }
            valid
        }
        3 if row < 2 => {
            p.slot = row as u8 + 1;
            p.shaping_choice = 0;
            true
        }
        5 => {
            let valid = p
                .shaping
                .as_ref()
                .and_then(|state| state.slots.get(usize::from(p.slot).wrapping_sub(1)))
                .and_then(Option::as_ref)
                .is_some_and(|slot| row < slot.offers.len());
            if valid {
                p.shaping_choice = row as u8;
            }
            valid
        }
        _ => false,
    }
}

pub(in crate::play) fn visual_submit(
    p: &mut WeaponProficiencyPanel,
    id: usize,
    clear: bool,
) -> Option<FeatureCommand> {
    if p.pending || !p.selected {
        return None;
    }
    if id == 116 {
        return request(p, Intent::State);
    }
    if matches!(id, 109..=114) {
        return submit(p, id, [""; 6], clear);
    }
    let entry = p.weapon_entry()?;
    let level = entry.levels.get(usize::from(p.level))?;
    if !clear && usize::from(p.perk) >= level.perks.len() {
        return None;
    }
    if id == 102 && level.selected != Some(p.perk) {
        p.feedback = Some(tr!("Apply the selected perk before modifying it"));
        return None;
    }
    if id != 101 {
        let Some(state) = p
            .shaping
            .as_ref()
            .filter(|state| state.definition == p.weapon && state.revision != 0)
        else {
            p.feedback = Some(tr!(
                "Refresh committed shaping state before changing a slot"
            ));
            return None;
        };
        p.revision = state.revision;
        if id == 106
            && state
                .slots
                .get(usize::from(p.slot).wrapping_sub(1))
                .and_then(Option::as_ref)
                .is_none_or(|slot| usize::from(p.shaping_choice) >= slot.offers.len())
        {
            p.feedback = Some(tr!("Select one of the server's current reshape offers"));
            return None;
        }
    }
    let values = [
        p.weapon.to_string(),
        p.level.to_string(),
        p.perk.to_string(),
        p.slot.to_string(),
        p.revision.to_string(),
        p.shaping_choice.to_string(),
    ];
    submit(p, id, values.each_ref().map(String::as_str), clear)
}

#[cfg(windows)]
pub(in crate::play) use window::open_window;
#[cfg(windows)]
#[path = "native_window.rs"]
mod window;
