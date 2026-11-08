use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Team discovery is unavailable in this session".into();
    }
    let tab = if panel.applicants {
        "Assemble team"
    } else {
        "Join team"
    };
    let mut text = format!("Social — {tab}\r\n{}", panel.feedback);
    match &panel.result {
        Some(TeamFinderResult::Browse { listings, .. }) => {
            text.push_str(&format!(
                "\r\n{} of {} teams on this page{}",
                panel.visible_indices().len(),
                listings.len(),
                if panel.next().is_some() {
                    "; more teams available"
                } else {
                    ""
                }
            ));
            if panel.visible_indices().is_empty() {
                text.push_str("\r\nNo teams match. Adjust filters or browse the next page.");
            }
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
            listing,
            occupied_slots,
            ..
        }) => {
            if let Some(listing) = listing {
                text.push_str(&format!(
                    "\r\nAdvertisement: level {}–{}, vocations {}, {} slots, target {:?}, start {}",
                    listing.min_level,
                    listing.max_level,
                    listing.vocation_mask,
                    listing.slots,
                    listing.target,
                    listing.timestamp
                ));
            } else {
                text.push_str("\r\nNo active advertisement. Publish one to assemble a team.");
            }
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
