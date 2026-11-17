use super::protocol::ServedChannel;
fn identity(bytes: &[u8; 16]) -> String {
    let mut text = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        use std::fmt::Write;
        let _ = write!(text, "{byte:02x}");
    }
    text
}
pub fn channel_text(channel: ServedChannel) -> String {
    // oteryn-choice: the contract supplies UUIDs, so display both with explicit labels.
    lfmt!(
        "World: {} | Channel: {} | Premium-only: {}",
        identity(&channel.world_id),
        identity(&channel.channel_id),
        channel.premium_only
    )
}
