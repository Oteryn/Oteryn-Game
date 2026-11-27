//! Committed charm damage lines belong in the Server Log.
use oteryn_client_runtime::chat::ChatLine;

/// The Game owner sends this exact damage-line shape through system chat.
pub fn damage_line(line: &ChatLine) -> Option<&str> {
    let ChatLine::System { text } = line else {
        return None;
    };
    let (damage, charm) = text.strip_suffix(" charm)")?.rsplit_once(" (")?;
    let (target, loss) = damage
        .strip_suffix(" due to your attack.")?
        .rsplit_once(" loses ")?;
    let (value, unit) = loss.split_once(' ')?;
    let value: std::num::NonZeroU64 = value.parse().ok()?;
    let expected_unit = if value.get() == 1 {
        "hitpoint"
    } else {
        "hitpoints"
    };
    (!target.is_empty()
        && unit == expected_unit
        && !charm.is_empty()
        && charm
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b' '))
    .then_some(text)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
