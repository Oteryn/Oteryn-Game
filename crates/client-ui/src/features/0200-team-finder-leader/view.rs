use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return tr!("Leader advertisement is unavailable in this session").into();
    }
    let mut text = format!("Assemble team — Your advertisement\r\n{}", panel.feedback);
    if let Some(listing) = panel.confirmed {
        text.push_str(&format!(
            "\r\nLevel {}–{}; vocations {}; {} slots, {} initially free\r\nTarget: {:?}; start: {}; include party members: {}",
            listing.min_level, listing.max_level, listing.vocation_mask,
            listing.slots, listing.free_slots, listing.target, listing.timestamp,
            if listing.party_members_join { "yes" } else { "no" }
        ));
    } else {
        text.push_str("\r\nNo advertisement confirmed in this session.");
    }
    text
}
