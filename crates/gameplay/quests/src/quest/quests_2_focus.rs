//! FocusModule preemption for the closed Crystal batch-2 NPC callbacks only.
//!
//! zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data/npclib/npc_system/modules.lua:5-8,286-310,361-387;
//! data/npclib/npc_system/npc_handler.lua:446-451.
//! This is a text predicate, not a conversation or durable-state owner. Earlier
//! source KeywordHandler registrations must be considered before calling it.

/// Literal FocusModule.messageMatcher, including its whole-message veto when
/// any occurrence touches Lua's `[%w+]` class on either side. `%w` is ASCII
/// alphanumeric here; '+' belongs to the class and '_' does not.
pub(super) fn intercepts(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    let bytes = message.as_bytes();
    let touches_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'+';
    ["hi", "hello", "bye", "farewell", "trade", "offers"]
        .iter()
        .any(|word| {
            let mut found = false;
            for (start, _) in message.match_indices(*word) {
                let end = start + word.len();
                if (start > 0 && touches_word(bytes[start - 1]))
                    || (end < bytes.len() && touches_word(bytes[end]))
                {
                    return false;
                }
                found = true;
            }
            found
        })
}

#[cfg(test)]
mod tests {
    use super::intercepts;

    #[test]
    fn source_focus_literal_boundaries_and_whole_word_veto() {
        for message in [
            "HI",
            "yes hello",
            "_hi_",
            "bye",
            "farewell",
            "trade",
            "offers",
        ] {
            assert!(intercepts(message), "source FocusModule match: {message}");
        }
        for message in ["high", "+hi", "hi+", "hi xhi", "hi hi+", "offers2", "hail"] {
            assert!(!intercepts(message), "source FocusModule veto: {message}");
        }
        assert!(
            intercepts("hi xhi hello"),
            "a vetoed hi does not veto hello"
        );
    }
}
