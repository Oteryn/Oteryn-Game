use super::{NpcKeywordMode, NpcRuleError};

/// The migrated corpus only uses literal keywords and the donor's `password*`
/// trailing Lua repetition. Refuse any unimplemented Lua pattern at admission.
pub(super) fn validate(keyword: &str, mode: NpcKeywordMode) -> Result<(), NpcRuleError> {
    if keyword.is_empty() || keyword.len() > 256 {
        return Err(NpcRuleError::Invalid("invalid keyword length".into()));
    }
    if mode != NpcKeywordMode::MsgContains {
        return Ok(());
    }
    let pattern = keyword.as_bytes();
    for (index, byte) in pattern.iter().enumerate() {
        if b"^$()%.[]+-?*".contains(byte)
            && !(*byte == b'*'
                && index + 1 == pattern.len()
                && index >= 1
                && pattern[index - 1].is_ascii_alphanumeric())
        {
            return Err(NpcRuleError::Invalid(format!(
                "unimplemented Lua keyword pattern: {keyword}"
            )));
        }
    }
    Ok(())
}

pub(super) fn matches(
    message: &str,
    keyword: &str,
    mode: NpcKeywordMode,
) -> Result<bool, NpcRuleError> {
    validate(keyword, mode)?;
    let message = message.to_ascii_lowercase();
    let keyword = keyword.to_ascii_lowercase();
    if message == keyword {
        return Ok(true);
    }
    match mode {
        NpcKeywordMode::Exact => Ok(false),
        NpcKeywordMode::Substring => Ok(message.contains(&keyword)),
        NpcKeywordMode::MsgContains => {
            // string.find searches the whole message twice. A later standalone
            // occurrence does not override an earlier word-prefixed occurrence.
            // Lua %w is letters/digits, and does not contain underscore.
            let pattern = keyword.as_bytes();
            let required = if pattern.last() == Some(&b'*') {
                &pattern[..pattern.len() - 2]
            } else {
                pattern
            };
            let bytes = message.as_bytes();
            let mut found = false;
            for start in 0..=bytes.len().saturating_sub(required.len()) {
                if bytes.get(start..start + required.len()) == Some(required) {
                    if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
                        return Ok(false);
                    }
                    found = true;
                }
            }
            Ok(found)
        }
    }
}
