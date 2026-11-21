//! Self-only blessing details, selected through capability 503 and command 388.
use super::protection::Forecast;

pub const CAPABILITY_BLESSINGS_DIALOG_V1: u32 = 503;
pub const SELECTOR: u8 = 134;
pub const MAX_HISTORY: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub at_unix_ms: u64,
    pub acquired: bool,
    pub blessings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub forecast: Forecast,
    pub blessings: Vec<String>,
    /// Reduction from the normal, unblessed experience penalty, before XP capping.
    pub experience_protection_percent: u8,
    /// The accepted death writer preserves all skill and magic progress.
    pub skill_protection_percent: u8,
    /// Newest twenty recorded acquisitions/consumptions, ordered by Character revision.
    pub history: Vec<HistoryEntry>,
}

fn keys(out: &mut Vec<u8>, values: &[String]) -> Option<()> {
    if values.len() > 8 {
        return None;
    }
    out.push(values.len() as u8);
    let mut previous = "";
    for key in values {
        if key.is_empty()
            || key.len() > 128
            || key.as_str() <= previous
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        {
            return None;
        }
        previous = key;
        out.push(key.len() as u8);
        out.extend_from_slice(key.as_bytes());
    }
    Some(())
}
fn take<'a>(tail: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    let (value, rest) = tail.split_at_checked(n)?;
    *tail = rest;
    Some(value)
}
fn read_keys(tail: &mut &[u8]) -> Option<Vec<String>> {
    let count = take(tail, 1)?[0];
    if count > 8 {
        return None;
    }
    let mut values = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let len = usize::from(take(tail, 1)?[0]);
        values.push(std::str::from_utf8(take(tail, len)?).ok()?.to_owned());
    }
    Some(values)
}
impl Snapshot {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.history.len() > MAX_HISTORY
            || self.experience_protection_percent > 100
            || self.skill_protection_percent > 100
        {
            return None;
        }
        let forecast = self.forecast.encode()?;
        let mut out = vec![SELECTOR, forecast.len() as u8];
        out.extend_from_slice(&forecast);
        out.extend_from_slice(&[
            self.experience_protection_percent,
            self.skill_protection_percent,
        ]);
        keys(&mut out, &self.blessings)?;
        out.push(self.history.len() as u8);
        for row in &self.history {
            if row.at_unix_ms > self.forecast.observed_at_unix_ms
                || row.at_unix_ms > i64::MAX as u64
                || row.blessings.is_empty()
            {
                return None;
            }
            out.extend_from_slice(&row.at_unix_ms.to_le_bytes());
            out.push(u8::from(row.acquired));
            keys(&mut out, &row.blessings)?;
        }
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 24576 {
            return None;
        }
        let mut tail = bytes;
        if take(&mut tail, 1)? != [SELECTOR] {
            return None;
        }
        let len = usize::from(take(&mut tail, 1)?[0]);
        let forecast = Forecast::decode(take(&mut tail, len)?)?;
        let experience_protection_percent = take(&mut tail, 1)?[0];
        let skill_protection_percent = take(&mut tail, 1)?[0];
        let blessings = read_keys(&mut tail)?;
        let count = take(&mut tail, 1)?[0];
        if usize::from(count) > MAX_HISTORY {
            return None;
        }
        let mut history = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let at_unix_ms = u64::from_le_bytes(take(&mut tail, 8)?.try_into().ok()?);
            let acquired = match take(&mut tail, 1)?[0] {
                0 => false,
                1 => true,
                _ => return None,
            };
            history.push(HistoryEntry {
                at_unix_ms,
                acquired,
                blessings: read_keys(&mut tail)?,
            });
        }
        let value = Self {
            forecast,
            blessings,
            experience_protection_percent,
            skill_protection_percent,
            history,
        };
        if !tail.is_empty() || value.encode()? != bytes {
            return None;
        }
        Some(value)
    }
}
