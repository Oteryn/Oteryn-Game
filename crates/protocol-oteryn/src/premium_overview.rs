//! Read-only account observation carried by character-info operation 5 (capability 97).
//! No new capability or entitlement-grant operation is introduced.
pub const OPERATION: u8 = 5;
pub const MAX_BYTES: usize = 16_384;
pub const MAX_FEATURES: usize = 16;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Unavailable = 0,
    None = 1,
    Active = 2,
    Expired = 3,
    Revoked = 4,
    Scheduled = 5,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    pub title: String,
    pub description: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overview {
    pub state: State,
    pub authority_revision: u64,
    pub observed_at_us: i64,
    /// Actual subscription end, never the shorter Game evidence lease deadline.
    pub expires_at_us: Option<i64>,
    /// Conservative remaining validity of enabled indicators, at most one minute.
    pub valid_for_us: u64,
    pub features: Vec<Feature>,
}
impl Overview {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.observed_at_us < 0
            || self.valid_for_us > 60_000_000
            || self.features.len() > MAX_FEATURES
            || self.expires_at_us.is_some_and(|t| t <= 0)
            || (self.state == State::Unavailable
                && (self.authority_revision != 0
                    || self.expires_at_us.is_some()
                    || self.valid_for_us != 0))
            || (self.state != State::Unavailable && self.authority_revision == 0)
            || (self.state == State::None && self.expires_at_us.is_some())
            || (matches!(
                self.state,
                State::Active | State::Expired | State::Scheduled
            ) && self.expires_at_us.is_none())
            || (self.valid_for_us > 0
                && (self.state != State::Active
                    || self.expires_at_us.is_none_or(|t| {
                        t <= self.observed_at_us
                            || u64::try_from(t - self.observed_at_us)
                                .is_ok_and(|left| left < self.valid_for_us)
                    })))
        {
            return None;
        }
        let mut out = vec![1, self.state as u8];
        out.extend(self.authority_revision.to_le_bytes());
        out.extend(self.observed_at_us.to_le_bytes());
        out.extend(self.expires_at_us.unwrap_or(0).to_le_bytes());
        out.extend(self.valid_for_us.to_le_bytes());
        out.push(u8::try_from(self.features.len()).ok()?);
        let mut names = std::collections::BTreeSet::new();
        for f in &self.features {
            if !names.insert(&f.title)
                || (f.enabled && (self.state != State::Active || self.valid_for_us == 0))
            {
                return None;
            }
            out.push(u8::from(f.enabled));
            for (text, max) in [(&f.title, 80), (&f.description, 512)] {
                if text.is_empty() || text.len() > max || text.chars().any(char::is_control) {
                    return None;
                }
                out.extend(u16::try_from(text.len()).ok()?.to_le_bytes());
                out.extend(text.as_bytes());
            }
        }
        (out.len() <= MAX_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_BYTES {
            return None;
        }
        let mut r = Reader(bytes);
        if r.byte()? != 1 {
            return None;
        }
        let state = match r.byte()? {
            0 => State::Unavailable,
            1 => State::None,
            2 => State::Active,
            3 => State::Expired,
            4 => State::Revoked,
            5 => State::Scheduled,
            _ => return None,
        };
        let authority_revision = u64::from_le_bytes(r.take(8)?.try_into().ok()?);
        let observed_at_us = i64::from_le_bytes(r.take(8)?.try_into().ok()?);
        let expiry = i64::from_le_bytes(r.take(8)?.try_into().ok()?);
        let valid_for_us = u64::from_le_bytes(r.take(8)?.try_into().ok()?);
        let count = usize::from(r.byte()?);
        if count > MAX_FEATURES {
            return None;
        }
        let mut features = Vec::with_capacity(count);
        for _ in 0..count {
            let enabled = match r.byte()? {
                0 => false,
                1 => true,
                _ => return None,
            };
            features.push(Feature {
                title: r.text()?,
                description: r.text()?,
                enabled,
            });
        }
        let value = Self {
            state,
            authority_revision,
            observed_at_us,
            expires_at_us: (expiry != 0).then_some(expiry),
            valid_for_us,
            features,
        };
        (r.0.is_empty() && value.encode()?.as_slice() == bytes).then_some(value)
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(n)?;
        self.0 = tail;
        Some(head)
    }
    fn byte(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }
    fn text(&mut self) -> Option<String> {
        let n = usize::from(u16::from_le_bytes(self.take(2)?.try_into().ok()?));
        Some(std::str::from_utf8(self.take(n)?).ok()?.to_owned())
    }
}

#[cfg(test)]
#[path = "premium_overview_tests.rs"]
mod tests;
