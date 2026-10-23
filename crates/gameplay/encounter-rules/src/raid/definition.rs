use super::{
    CalendarSelector, MAX_ANNOUNCEMENT_BYTES, MAX_ANNOUNCEMENTS, MAX_CREATURES, MAX_WAVES,
    RaidError,
};
use oteryn_gameplay_content::content::ProductionKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidPolicy {
    Random {
        mean_interval_s: u32,
        min_gap_s: u32,
    },
    Fixed {
        selector: CalendarSelector,
        offset_after_reset_ms: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnouncementKind {
    Warning,
    Advance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub delay_ms: u32,
    pub text: String,
    pub kind: AnnouncementKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveEntry {
    pub creature: ProductionKey,
    pub count: u16,
    pub area_anchor: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    pub delay_ms: u32,
    pub entries: Vec<WaveEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidDefinition {
    pub policy: RaidPolicy,
    pub despawn_after_ms: Option<u32>,
    pub announcements: Vec<Announcement>,
    pub waves: Vec<Wave>,
}

impl RaidDefinition {
    pub fn validate(&self) -> Result<(), RaidError> {
        match self.policy {
            RaidPolicy::Random {
                mean_interval_s: 0, ..
            } => return Err(RaidError::InvalidDefinition),
            RaidPolicy::Fixed { selector, .. } => selector.validate()?,
            _ => {}
        }
        if self.waves.is_empty()
            || self.waves.len() > MAX_WAVES
            || self.announcements.len() > MAX_ANNOUNCEMENTS
            || self.despawn_after_ms == Some(0)
        {
            return Err(RaidError::InvalidDefinition);
        }
        let mut creatures = 0_u16;
        for wave in &self.waves {
            if wave.entries.is_empty() || wave.entries.len() > usize::from(MAX_CREATURES) {
                return Err(RaidError::InvalidDefinition);
            }
            for entry in &wave.entries {
                creatures = creatures
                    .checked_add(entry.count)
                    .ok_or(RaidError::InvalidDefinition)?;
                if entry.count == 0 || creatures > MAX_CREATURES {
                    return Err(RaidError::InvalidDefinition);
                }
            }
        }
        if self.announcements.iter().any(|a| {
            a.text.is_empty() || a.text.len() > MAX_ANNOUNCEMENT_BYTES || a.text.contains('\0')
        }) {
            return Err(RaidError::InvalidDefinition);
        }
        Ok(())
    }

    /// Absolute Oteryn deadlines (BOSS_RAID0 §4.3), sorted stably by delay.
    /// Canary source delays are milliseconds; its minimum scheduler tick is
    /// normalized by the content importer, not by a runtime clock here.
    pub fn announcement_sequence(
        &self,
        fired_at_ms: i64,
    ) -> Result<Vec<(i64, &Announcement)>, RaidError> {
        self.validate()?;
        if fired_at_ms < 0 {
            return Err(RaidError::InvalidTime);
        }
        let mut sequence = self
            .announcements
            .iter()
            .map(|a| {
                fired_at_ms
                    .checked_add(i64::from(a.delay_ms))
                    .map(|at| (at, a))
                    .ok_or(RaidError::Exhausted)
            })
            .collect::<Result<Vec<_>, _>>()?;
        sequence.sort_by_key(|(at, _)| *at);
        Ok(sequence)
    }

    pub(super) fn deadline(&self, fired_at_ms: i64) -> Result<i64, RaidError> {
        let last_wave = self.waves.iter().map(|w| w.delay_ms).max().unwrap_or(0);
        let last_announcement = self
            .announcement_sequence(fired_at_ms)?
            .last()
            .map(|(at, _)| *at)
            .unwrap_or(fired_at_ms);
        let deadline = fired_at_ms
            .checked_add(i64::from(last_wave))
            .and_then(|at| at.checked_add(i64::from(self.despawn_after_ms.unwrap_or(0))))
            .ok_or(RaidError::Exhausted)?;
        // A malformed definition cannot close before its final announcement.
        if last_announcement > deadline {
            return Err(RaidError::InvalidDefinition);
        }
        Ok(deadline)
    }
}
