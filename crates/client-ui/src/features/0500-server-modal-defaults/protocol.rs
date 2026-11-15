pub fn keyboard_button(panel: &crate::server_modal::Panel, escape: bool) -> Option<u8> {
    crate::server_modal::keyboard_button(panel, escape)
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
