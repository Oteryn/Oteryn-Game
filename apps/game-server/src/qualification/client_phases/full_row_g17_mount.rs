// Keep the module at item scope so its ordinary unit tests are discoverable.
#[path = "full_row_g17.rs"]
mod full_row_g17;
#[cfg(test)]
pub(crate) use full_row_g17::validate_offer_source as g17_validate_offer_source;
