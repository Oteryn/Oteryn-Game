use super::SpellInfo;
pub fn spell_text(info: &SpellInfo) -> String {
    let mut text = format!(
        "\r\n{}\r\nBase spell cooldown: {} ms",
        info.description
            .as_deref()
            .unwrap_or("No description available."),
        info.cooldown_ms
    );
    for (group, duration) in &info.group_cooldowns_ms {
        text.push_str(&format!("\r\nBase group {group} cooldown: {duration} ms"));
    }
    text
}
