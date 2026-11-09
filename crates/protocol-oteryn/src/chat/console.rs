//! Bounded, read-only current sender-level lookup. CHAT_V1 lines remain unchanged.
pub const COMMAND: u32 = 736;
pub const MAX_NAMES: usize = 32;
pub fn encode_names(names: &[String]) -> Option<Vec<u8>> {
    if names.len() > MAX_NAMES {
        return None;
    }
    let mut out = vec![names.len() as u8];
    for name in names {
        if name.is_empty() || name.len() > 120 || name.contains('\0') {
            return None;
        }
        out.push(name.len() as u8);
        out.extend_from_slice(name.as_bytes());
    }
    Some(out)
}
pub fn decode_names(bytes: &[u8]) -> Option<Vec<String>> {
    let (&count, mut rest) = bytes.split_first()?;
    if usize::from(count) > MAX_NAMES {
        return None;
    }
    let mut names = Vec::new();
    for _ in 0..count {
        let (&length, tail) = rest.split_first()?;
        let (name, tail) = tail.split_at_checked(usize::from(length))?;
        let name = std::str::from_utf8(name).ok()?;
        if name.is_empty() || name.len() > 120 || name.contains('\0') {
            return None;
        }
        names.push(name.to_owned());
        rest = tail;
    }
    rest.is_empty().then_some(names)
}
pub fn encode_levels(levels: &[Option<u32>]) -> Option<Vec<u8>> {
    if levels.len() > MAX_NAMES || levels.contains(&Some(0)) {
        return None;
    }
    let mut out = vec![levels.len() as u8];
    for level in levels {
        out.extend_from_slice(&level.unwrap_or(0).to_le_bytes());
    }
    Some(out)
}
pub fn decode_levels(bytes: &[u8]) -> Option<Vec<Option<u32>>> {
    let (&count, rest) = bytes.split_first()?;
    if usize::from(count) > MAX_NAMES || rest.len() != usize::from(count) * 4 {
        return None;
    }
    Some(
        rest.as_chunks::<4>()
            .0
            .iter()
            .map(|v| {
                let level = u32::from_le_bytes(*v);
                (level != 0).then_some(level)
            })
            .collect(),
    )
}
