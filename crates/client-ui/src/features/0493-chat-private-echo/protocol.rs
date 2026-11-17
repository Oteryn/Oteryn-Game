//! Capability 493 `CHAT_PRIVATE_ECHO_V1`: the sender's own private message comes back as a line
//! with the name it was sent to, so the Conversation tab shows both sides. Without the
//! capability the lines carry no recipient and nothing here applies.
use oteryn_client_runtime::chat::ChatLine;

/// An own echo as the player reads it.
#[must_use]
pub fn echo_text(recipient: &str, message: &str) -> String {
    lfmt!(
        "To {recipient}: {message}",
        recipient = recipient,
        message = message
    )
}

/// Whether a private line belongs in the Conversation tab of `peer`: an incoming message from
/// the peer, or an own echo sent to the peer.
#[must_use]
pub fn in_conversation(peer: &str, line: &ChatLine, recipient: Option<&str>) -> bool {
    match (line, recipient) {
        (ChatLine::Private { .. }, Some(recipient)) => recipient.eq_ignore_ascii_case(peer),
        (ChatLine::Private { speaker_name, .. }, None) => speaker_name.eq_ignore_ascii_case(peer),
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
