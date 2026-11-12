//! Capability 487 `CYCLOPEDIA_CHARACTER_VOCATION_V1`: the General Stats reply ends with the
//! character's vocation id, which names the exact title the promotion view shows.

/// Vocation titles by id `1..=11`, in the spellbook details vocation order.
const VOCATION_TITLES: [&str; 11] = [
    "None",
    "Druid",
    "Elder Druid",
    "Sorcerer",
    "Master Sorcerer",
    "Knight",
    "Elite Knight",
    "Paladin",
    "Royal Paladin",
    "Monk",
    "Exalted Monk",
];

/// The title of vocation `id`; `None` outside `1..=11`.
#[must_use]
pub fn vocation_title(id: u8) -> Option<&'static str> {
    let index = usize::from(id.checked_sub(1)?);
    VOCATION_TITLES.get(index).copied()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
