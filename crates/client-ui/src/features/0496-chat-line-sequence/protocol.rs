//! Capability 496 `CHAT_LINE_SEQUENCE_V1`: every line carries a sequence that only grows within
//! a session, so an incoming private message opens its Conversation tab once, and a tab the
//! player closed is not reopened by the same line. Without the capability no tab opens by
//! itself.
use oteryn_client_runtime::chat::ChatLine;

/// The senders of the private messages newer than `seen`, each once, in arrival order. Own
/// echoes (a recipient is set) open nothing. `seen` moves to the highest sequence of the log.
pub fn new_senders(
    lines: &[ChatLine],
    sequences: &[Option<u64>],
    recipients: &[Option<String>],
    seen: &mut u64,
) -> Vec<String> {
    let mut senders: Vec<String> = Vec::new();
    let before = *seen;
    for (index, line) in lines.iter().enumerate() {
        let Some(sequence) = sequences.get(index).copied().flatten() else {
            continue;
        };
        if sequence <= before {
            continue;
        }
        *seen = (*seen).max(sequence);
        let own_echo = recipients.get(index).is_some_and(Option::is_some);
        if let ChatLine::Private { speaker_name, .. } = line
            && !own_echo
            && !senders
                .iter()
                .any(|name| name.eq_ignore_ascii_case(speaker_name))
        {
            senders.push(speaker_name.clone());
        }
    }
    senders
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
