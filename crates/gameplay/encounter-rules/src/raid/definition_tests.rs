use super::*;
use oteryn_gameplay_content::content::ProductionKey;

pub(super) fn definition(persistent: bool) -> Result<RaidDefinition, Box<dyn std::error::Error>> {
    Ok(RaidDefinition {
        policy: RaidPolicy::Random {
            mean_interval_s: 60,
            min_gap_s: 0,
        },
        despawn_after_ms: (!persistent).then_some(1_000),
        announcements: vec![Announcement {
            delay_ms: 1_000,
            text: "A raid approaches.".into(),
            kind: AnnouncementKind::Warning,
        }],
        waves: vec![Wave {
            delay_ms: 2_000,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("creature:rat")?,
                count: 1,
                area_anchor: ProductionKey::new("area:raid")?,
            }],
        }],
    })
}

// Canary reference, pinned by RAID-CONTENT-1 (not a runtime/content admission):
// https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/lua/creature/raids.cpp
// https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/lua/creature/raids.hpp
// https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/raids/raids.xml
// Every row corresponds to the task card's Reference parity table. Native
// probability/persistence/scope follow accepted BOSS_RAID0, not donor bugs.
#[test]
fn raid_reference_parity_rows() -> Result<(), Box<dyn std::error::Error>> {
    enum Row {
        Check,
        IntervalUnits,
        ZeroInterval,
        Margin,
        Sequence,
        Kinds,
        Persistent,
        Bounds,
    }
    for row in [
        Row::Check,
        Row::IntervalUnits,
        Row::ZeroInterval,
        Row::Margin,
        Row::Sequence,
        Row::Kinds,
        Row::Persistent,
        Row::Bounds,
    ] {
        let mut d = definition(false)?;
        match row {
            Row::Check => assert_eq!(CHECK_INTERVAL_MS, 60_000),
            Row::IntervalUnits => {
                // raids.xml Badger: interval2=500 minutes. Native mean is seconds.
                d.policy = RaidPolicy::Random {
                    mean_interval_s: 500 * 60,
                    min_gap_s: 60 * 60,
                };
                // Native p=60/30000: exact threshold over the 64-bit draw,
                // without the overflow/inverted formula in the pinned donor.
                assert!(super::schedule::wins_interval_roll(
                    36_893_488_147_419_103,
                    500 * 60
                ));
                assert!(!super::schedule::wins_interval_roll(
                    36_893_488_147_419_104,
                    500 * 60
                ));
                d.validate()?;
            }
            Row::ZeroInterval => {
                d.policy = RaidPolicy::Random {
                    mean_interval_s: 0,
                    min_gap_s: 0,
                };
                assert_eq!(d.validate(), Err(RaidError::InvalidDefinition));
            }
            Row::Margin => super::schedule_tests::minimum_gap_survives_reset()?,
            Row::Sequence => {
                d.announcements = [2_000, 1_000, 1_000]
                    .into_iter()
                    .enumerate()
                    .map(|(i, delay_ms)| Announcement {
                        delay_ms,
                        text: i.to_string(),
                        kind: AnnouncementKind::Advance,
                    })
                    .collect();
                let sequence = d.announcement_sequence(10_000)?;
                assert_eq!(
                    sequence
                        .iter()
                        .map(|(at, a)| (*at, a.text.as_str()))
                        .collect::<Vec<_>>(),
                    [(11_000, "1"), (11_000, "2"), (12_000, "0")]
                );
            }
            Row::Kinds => {
                d.announcements[0].kind = AnnouncementKind::Warning;
                assert_eq!(
                    d.announcement_sequence(0)?[0].1.kind,
                    AnnouncementKind::Warning
                );
                d.announcements[0].kind = AnnouncementKind::Advance;
                assert_eq!(
                    d.announcement_sequence(0)?[0].1.kind,
                    AnnouncementKind::Advance
                );
            }
            Row::Persistent => super::schedule_tests::persistent_once_per_epoch()?,
            Row::Bounds => {
                d.waves[0].entries[0].count = MAX_CREATURES;
                d.announcements
                    .resize(MAX_ANNOUNCEMENTS, d.announcements[0].clone());
                d.validate()?;
                d.waves[0].entries[0].count += 1;
                assert_eq!(d.validate(), Err(RaidError::InvalidDefinition));
                d.waves[0].entries[0].count = 1;
                d.announcements.push(d.announcements[0].clone());
                assert_eq!(d.validate(), Err(RaidError::InvalidDefinition));
            }
        }
    }
    Ok(())
}

#[test]
fn raid_definition_capacity_and_deadline_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = definition(false)?;
    d.waves.resize(MAX_WAVES, d.waves[0].clone());
    d.validate()?;
    d.waves.push(d.waves[0].clone());
    assert_eq!(d.validate(), Err(RaidError::InvalidDefinition));
    let mut d = definition(false)?;
    d.announcements[0].text = "a".repeat(MAX_ANNOUNCEMENT_BYTES);
    d.validate()?;
    d.announcements[0].text.push('a');
    assert_eq!(d.validate(), Err(RaidError::InvalidDefinition));
    let d = definition(false)?;
    assert_eq!(d.announcement_sequence(-1), Err(RaidError::InvalidTime));
    assert_eq!(d.announcement_sequence(i64::MAX), Err(RaidError::Exhausted));
    Ok(())
}
