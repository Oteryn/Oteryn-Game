use super::Panel;
use oteryn_client_runtime::cast::Message;
// Crystal protocolgame.cpp:8043: only a committed public map becomes a picture.
pub fn apply(
    panel: &mut Panel,
    message: Message,
    now: std::time::Instant,
) -> Result<(), &'static str> {
    if let Err(error) = panel.picture.apply(message, now) {
        panel.picture.clear();
        panel.picture.ended = true;
        panel.feedback = error.into();
        return Err(error);
    }
    Ok(())
}
