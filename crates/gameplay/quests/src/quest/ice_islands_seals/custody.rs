//! Derive paint consumption from the exact three-track plan held by the writer.
use super::{MISSION04, QUESTLINE, Snapshot, TARGETS, paint};
use crate::quest::{QuestRefusal, QuestTrackChange};

/// The source removes one vial only on completion. Refuse partial, duplicate or
/// forged changes rather than treating an unverifiable plan as a reusable use.
pub fn consumes_paint(seal: u16, changes: &[QuestTrackChange]) -> Result<bool, QuestRefusal> {
    if changes.len() != 3 {
        return Err(QuestRefusal::NotSupported);
    }
    let read = |track: &str| {
        let mut found = changes.iter().filter(|change| change.track == track);
        let first = found.next().ok_or(QuestRefusal::NotSupported)?;
        if found.next().is_some() {
            return Err(QuestRefusal::NotSupported);
        }
        Ok(first)
    };
    let questline = read(QUESTLINE)?;
    let targets = read(TARGETS)?;
    let mission = read(MISSION04)?;
    let next = paint(
        seal,
        Snapshot {
            questline: questline.before,
            targets: targets.before,
            mission04: mission.before,
        },
    )?;
    if (questline.after, targets.after, mission.after)
        != (next.questline, next.targets, next.mission04)
    {
        return Err(QuestRefusal::StageMismatch);
    }
    Ok(next.questline == 9)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changes(before: Snapshot, after: Snapshot) -> Vec<QuestTrackChange> {
        [QUESTLINE, TARGETS, MISSION04]
            .into_iter()
            .zip([before.questline, before.targets, before.mission04])
            .zip([after.questline, after.targets, after.mission04])
            .map(|((track, before), after)| QuestTrackChange {
                track: track.into(),
                before,
                after,
            })
            .collect()
    }

    #[test]
    fn paint_is_consumed_exactly_on_the_third_distinct_seal() -> Result<(), QuestRefusal> {
        let mut before = Snapshot {
            questline: 8,
            targets: 0,
            mission04: 1,
        };
        for (index, seal) in [0, 63, 197].into_iter().enumerate() {
            let after = paint(seal, before)?;
            assert_eq!(consumes_paint(seal, &changes(before, after))?, index == 2);
            before = after;
        }
        Ok(())
    }

    #[test]
    fn malformed_or_forged_plans_never_choose_a_consumption() -> Result<(), QuestRefusal> {
        let before = Snapshot {
            questline: 8,
            targets: 0,
            mission04: 1,
        };
        let after = paint(0, before)?;
        let correct = changes(before, after);
        assert!(consumes_paint(0, &correct[..2]).is_err());
        let mut extra = correct.clone();
        extra.push(correct[0].clone());
        assert!(consumes_paint(0, &extra).is_err());
        let mut duplicate = correct.clone();
        duplicate[2] = correct[0].clone();
        assert!(consumes_paint(0, &duplicate).is_err());
        for index in 0..3 {
            let mut forged = correct.clone();
            forged[index].after += 1;
            assert!(consumes_paint(0, &forged).is_err());
        }
        assert!(consumes_paint(1, &correct).is_err());
        Ok(())
    }
}
