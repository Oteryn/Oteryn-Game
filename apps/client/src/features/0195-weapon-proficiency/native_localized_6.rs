use super::*;
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
