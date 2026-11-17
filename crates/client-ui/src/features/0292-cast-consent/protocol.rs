use crate::cast_control::{Message, Panel};
// Accepted S17 difference from Canary livestream.cpp:486: bystanders default off.
pub fn request(panel: &Panel, allowed: Option<bool>) -> Result<Message, &'static str> {
    panel.command(match allowed {
        None => 2,
        Some(true) => 3,
        Some(false) => 4,
    })
}
pub fn apply(panel: &mut Panel, payload: &[u8]) -> Result<(), &'static str> {
    let message =
        Message::decode(payload).map_err(|_| tr!("Invalid character consent response"))?;
    if !matches!(message, Message::ConsentState { .. }) {
        return Err(tr!("Invalid character consent response"));
    }
    panel.apply(message)
}
