use super::*;
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
