use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Inspection is unavailable in this session".into();
    }
    let mut text = format!("Inspection\r\n{}", panel.feedback);
    if let Some(reply) = &panel.reply {
        for item in &reply.items {
            text.push_str(&format!(
                "\r\nSlot {}: {} ×{} (tier {})",
                item.slot, item.name, item.count, item.tier
            ));
            for (key, value) in &item.attributes {
                text.push_str(&format!("\r\n  {key}: {value}"));
            }
        }
        text.push_str(&format!(
            "\r\nPermission requests: {}",
            reply.requests.len()
        ));
    }
    text
}
