use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Team discovery is unavailable in this session".into();
    }
    let mut text = format!("Team Finder\r\n{}", panel.feedback);
    match &panel.result {
        Some(TeamFinderResult::Browse { listings, .. }) => {
            for index in panel.visible_indices() {
                let row = &listings[index];
                text.push_str(&format!(
                    "\r\n{}: level {}–{}, {} free, {:?}",
                    row.name,
                    row.listing.min_level,
                    row.listing.max_level,
                    row.listing.slots.saturating_sub(row.occupied_slots),
                    row.status
                ));
            }
        }
        Some(TeamFinderResult::Applicants {
            members,
            occupied_slots,
            ..
        }) => {
            text.push_str(&format!("\r\nOccupied: {occupied_slots}"));
            for row in members {
                text.push_str(&format!(
                    "\r\n{}: level {}, vocation {}, {:?}",
                    row.name, row.level, row.vocation, row.status
                ));
            }
        }
        None => {}
    }
    text
}
