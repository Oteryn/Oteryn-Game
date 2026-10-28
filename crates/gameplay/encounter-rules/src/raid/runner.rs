use super::{AnnouncementKind, RaidDefinition, RaidError, WaveEntry};

/// Durable position of one channel raid run. The cursor is the number of
/// actions already committed, rather than an in-memory timer position. This
/// position may only be reused by the same live ownership generation. A new
/// generation must mark the old run LOST, never resume it (BOSS-RAID-0 §4.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RaidRunCursor {
    pub committed_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaidRunAction<'a> {
    Announce {
        text: &'a str,
        kind: AnnouncementKind,
    },
    SpawnWave {
        wave: u16,
        entries: &'a [WaveEntry],
        /// Marks the last scheduled wave, without asserting that its creatures
        /// are bosses or granting boss reward eligibility.
        boss_step: bool,
    },
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidRunStep<'a> {
    pub ordinal: u32,
    pub due_at_ms: i64,
    pub action: RaidRunAction<'a>,
}

impl RaidRunCursor {
    /// Returns only the next uncommitted due action. The caller durably commits
    /// `ordinal + 1` with the action's idempotency key before asking again. This
    /// pure planner does not authorize delivery, spawning or generation recovery.
    pub fn next_due<'a>(
        self,
        definition: &'a RaidDefinition,
        fired_at_ms: i64,
        now_ms: i64,
    ) -> Result<Option<RaidRunStep<'a>>, RaidError> {
        definition.validate()?;
        if fired_at_ms < 0 || now_ms < 0 {
            return Err(RaidError::InvalidTime);
        }

        let mut steps =
            Vec::with_capacity(definition.announcements.len() + definition.waves.len() + 1);
        for (source, announcement) in definition.announcements.iter().enumerate() {
            let due = fired_at_ms
                .checked_add(i64::from(announcement.delay_ms))
                .ok_or(RaidError::Exhausted)?;
            steps.push((
                due,
                0_u8,
                source,
                RaidRunAction::Announce {
                    text: &announcement.text,
                    kind: announcement.kind,
                },
            ));
        }
        let last_wave = definition
            .waves
            .iter()
            .enumerate()
            .max_by_key(|(source, wave)| (wave.delay_ms, *source))
            .map(|(source, _)| source)
            .ok_or(RaidError::InvalidDefinition)?;
        for (source, wave) in definition.waves.iter().enumerate() {
            let due = fired_at_ms
                .checked_add(i64::from(wave.delay_ms))
                .ok_or(RaidError::Exhausted)?;
            steps.push((
                due,
                1_u8,
                source,
                RaidRunAction::SpawnWave {
                    wave: u16::try_from(source).map_err(|_| RaidError::Exhausted)?,
                    entries: &wave.entries,
                    boss_step: source == last_wave,
                },
            ));
        }
        let complete_at = definition.deadline(fired_at_ms)?;
        steps.push((complete_at, 2_u8, usize::MAX, RaidRunAction::Complete));
        steps.sort_by_key(|(due, class, source, _)| (*due, *class, *source));

        let ordinal = usize::try_from(self.committed_steps).map_err(|_| RaidError::Exhausted)?;
        if ordinal > steps.len() {
            return Err(RaidError::InvalidState);
        }
        let Some((due_at_ms, _, _, action)) = steps.into_iter().nth(ordinal) else {
            return Ok(None);
        };
        if due_at_ms > now_ms {
            return Ok(None);
        }
        Ok(Some(RaidRunStep {
            ordinal: self.committed_steps,
            due_at_ms,
            action,
        }))
    }

    pub fn commit(self, step: &RaidRunStep<'_>) -> Result<Self, RaidError> {
        if step.ordinal != self.committed_steps {
            return Err(RaidError::InvalidState);
        }
        Ok(Self {
            committed_steps: self
                .committed_steps
                .checked_add(1)
                .ok_or(RaidError::Exhausted)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raid::{Announcement, RaidPolicy, Wave};
    use oteryn_gameplay_content::content::ProductionKey;

    fn fixture() -> Result<RaidDefinition, Box<dyn std::error::Error>> {
        let entry = |creature| -> Result<WaveEntry, Box<dyn std::error::Error>> {
            Ok(WaveEntry {
                creature: ProductionKey::new(creature)?,
                count: 1,
                area_anchor: ProductionKey::new("area:raid")?,
            })
        };
        Ok(RaidDefinition {
            policy: RaidPolicy::Random {
                mean_interval_s: 60,
                min_gap_s: 0,
            },
            despawn_after_ms: Some(1_000),
            announcements: vec![Announcement {
                delay_ms: 0,
                text: "Raid!".into(),
                kind: AnnouncementKind::Warning,
            }],
            waves: vec![
                Wave {
                    delay_ms: 10,
                    entries: vec![entry("creature:rat")?],
                },
                Wave {
                    delay_ms: 20,
                    entries: vec![entry("creature:boss")?],
                },
            ],
        })
    }

    #[test]
    fn fixture_runs_announce_waves_boss_complete() -> Result<(), Box<dyn std::error::Error>> {
        let definition = fixture()?;
        let mut cursor = RaidRunCursor::default();
        let mut observed = Vec::new();
        while let Some(step) = cursor.next_due(&definition, 100, 2_000)? {
            observed.push(match step.action {
                RaidRunAction::Announce { .. } => "announce",
                RaidRunAction::SpawnWave {
                    boss_step: false, ..
                } => "wave",
                RaidRunAction::SpawnWave {
                    boss_step: true, ..
                } => "boss",
                RaidRunAction::Complete => "complete",
            });
            cursor = cursor.commit(&step)?;
        }
        assert_eq!(observed, ["announce", "wave", "boss", "complete"]);
        Ok(())
    }

    #[test]
    fn same_generation_reload_does_not_repeat_spawns() -> Result<(), Box<dyn std::error::Error>> {
        // Reload within the same live generation only; a new generation loses
        // the run instead. This fixture does not qualify process restart.
        let definition = fixture()?;
        let mut durable = RaidRunCursor::default();
        for _ in 0..2 {
            let step = durable.next_due(&definition, 100, 2_000)?.ok_or("due")?;
            durable = durable.commit(&step)?;
        }
        let restarted = durable;
        let step = restarted
            .next_due(&definition, 100, 2_000)?
            .ok_or("boss due")?;
        assert!(matches!(
            step.action,
            RaidRunAction::SpawnWave {
                wave: 1,
                boss_step: true,
                ..
            }
        ));
        assert_eq!(step.ordinal, 2);
        Ok(())
    }

    #[test]
    fn unsorted_waves_mark_the_last_executed_wave() -> Result<(), Box<dyn std::error::Error>> {
        let mut definition = fixture()?;
        definition.waves.swap(0, 1);
        let mut cursor = RaidRunCursor { committed_steps: 1 };
        let early = cursor
            .next_due(&definition, 100, 110)?
            .ok_or("first wave")?;
        assert!(matches!(
            early.action,
            RaidRunAction::SpawnWave {
                wave: 1,
                boss_step: false,
                ..
            }
        ));
        cursor = cursor.commit(&early)?;
        assert!(cursor.next_due(&definition, 100, 119)?.is_none());
        let late = cursor.next_due(&definition, 100, 120)?.ok_or("last wave")?;
        assert!(matches!(
            late.action,
            RaidRunAction::SpawnWave {
                wave: 0,
                boss_step: true,
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn equal_delays_are_stable_and_completion_is_last() -> Result<(), Box<dyn std::error::Error>> {
        let mut definition = fixture()?;
        definition.announcements[0].delay_ms = 20;
        definition.waves[0].delay_ms = 20;
        definition.despawn_after_ms = None;
        let mut cursor = RaidRunCursor::default();
        assert!(cursor.next_due(&definition, 100, 119)?.is_none());
        for ordinal in 0..4 {
            let step = cursor.next_due(&definition, 100, 120)?.ok_or("due")?;
            assert_eq!(step.ordinal, ordinal);
            assert_eq!(step.due_at_ms, 120);
            match ordinal {
                0 => assert!(matches!(step.action, RaidRunAction::Announce { .. })),
                1 => assert!(matches!(
                    step.action,
                    RaidRunAction::SpawnWave {
                        wave: 0,
                        boss_step: false,
                        ..
                    }
                )),
                2 => assert!(matches!(
                    step.action,
                    RaidRunAction::SpawnWave {
                        wave: 1,
                        boss_step: true,
                        ..
                    }
                )),
                _ => assert!(matches!(step.action, RaidRunAction::Complete)),
            }
            cursor = cursor.commit(&step)?;
            assert_eq!(cursor.commit(&step), Err(RaidError::InvalidState));
        }
        assert!(cursor.next_due(&definition, 100, 120)?.is_none());
        Ok(())
    }

    #[test]
    fn invalid_cursor_and_time_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
        let definition = fixture()?;
        assert_eq!(
            RaidRunCursor { committed_steps: 5 }.next_due(&definition, 100, 2_000),
            Err(RaidError::InvalidState)
        );
        let cursor = RaidRunCursor::default();
        assert_eq!(
            cursor.next_due(&definition, -1, 0),
            Err(RaidError::InvalidTime)
        );
        assert_eq!(
            cursor.next_due(&definition, 0, -1),
            Err(RaidError::InvalidTime)
        );
        assert_eq!(
            cursor.next_due(&definition, i64::MAX, i64::MAX),
            Err(RaidError::Exhausted)
        );
        let mut invalid = definition;
        invalid.waves.clear();
        assert_eq!(
            cursor.next_due(&invalid, 0, 0),
            Err(RaidError::InvalidDefinition)
        );
        Ok(())
    }
}
