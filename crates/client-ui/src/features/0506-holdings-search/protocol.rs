pub fn display_name<'a>(
    items: &'a crate::cyclopedia_character::ItemSummary,
    row: &'a oteryn_client_runtime::cyclopedia_character::items::Row,
) -> &'a str {
    items.display_name(row)
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
