//! The selected finite programme observes successful source actions under the same lock.
//! Source tasks, repeat limits, Premium/rank gates and boss visits retain their owner.
use super::*;

pub const PHASE: &str = "oteryn:quest-progress/chosen/killing_in_the_name_of/phase";
pub const COUNT: &str = "oteryn:quest-progress/chosen/killing_in_the_name_of/count";
pub const REPORTED: &str = "oteryn:quest-progress/chosen/killing_in_the_name_of/reported";
pub const TRACKS: [&str; 3] = [PHASE, COUNT, REPORTED];

pub(super) fn observe(
    state: &GrizzlyState,
    action: &GrizzlyAction,
    transition: &mut GrizzlyTransition,
) -> Result<(), GrizzlyRefusal> {
    let read = |key: &str| state.get(key).copied().unwrap_or(0);
    let (mut phase, mut count, mut reported) = (read(PHASE), read(COUNT), read(REPORTED));
    if !(0..=11).contains(&phase) || !(0..=650).contains(&count) || !(0..=1).contains(&reported) {
        return Err(GrizzlyRefusal::OutOfRange);
    }
    let source_after = |id| {
        let key = storage_track(id);
        transition
            .changes
            .iter()
            .find(|c| c.track == key)
            .map_or_else(|| storage_value(state, id), |c| c.after)
    };
    match (phase, action) {
        (0, GrizzlyAction::Join | GrizzlyAction::StartTask(1)) => phase = 1,
        (1 | 4 | 7, GrizzlyAction::KillCreature { task_id, .. })
            if matches!((phase, task_id), (1, 1) | (4, 4) | (7, 32)) =>
        {
            count = count.checked_add(1).ok_or(GrizzlyRefusal::OutOfRange)?;
            let required = match phase {
                1 => 300,
                4 => 150,
                _ => 650,
            };
            if count == required {
                phase += 1;
            } else if count > required {
                return Err(GrizzlyRefusal::OutOfRange);
            }
        }
        (2, GrizzlyAction::BossKill { boss_id: 1, .. }) => phase = 3,
        (3, GrizzlyAction::ReportBoss(1))
            if source_after(boss(1).ok_or(GrizzlyRefusal::InvalidBoss)?.storage) == 3 =>
        {
            phase = 4
        }
        (5, GrizzlyAction::BossKill { boss_id: 3, .. }) => phase = 6,
        (6, GrizzlyAction::ReportBoss(3))
            if source_after(boss(3).ok_or(GrizzlyRefusal::InvalidBoss)?.storage) == 3 =>
        {
            reported = 1;
            if source_after(PROMOTION_STORAGE) >= 0 {
                phase = 7;
            }
        }
        (6, GrizzlyAction::Promote) if reported == 1 && source_after(PROMOTION_STORAGE) >= 0 => {
            phase = 7
        }
        (8, GrizzlyAction::BossKill { boss_id: 19, .. }) => phase = 9,
        (9, GrizzlyAction::ReportBoss(19))
            if source_after(boss(19).ok_or(GrizzlyRefusal::InvalidBoss)?.storage) == 3 =>
        {
            phase = 10
        }
        _ => {}
    }
    if phase != read(PHASE) {
        count = 0;
        reported = 0;
    }
    for (track, after) in TRACKS.into_iter().zip([phase, count, reported]) {
        let before = read(track);
        if before != after {
            transition.changes.push(QuestTrackChange {
                track: track.into(),
                before,
                after,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_hunt_kills_do_not_pay_an_earlier_campaign_stage() -> Result<(), GrizzlyRefusal> {
        let state = GrizzlyState::from([(PHASE.into(), 1), (COUNT.into(), 299)]);
        let mut unrelated = GrizzlyTransition::default();
        observe(
            &state,
            &GrizzlyAction::KillCreature {
                task_id: 4,
                creature: "oteryn:creature.carniphila".into(),
                observed_revision: 8,
            },
            &mut unrelated,
        )?;
        assert!(unrelated.changes.is_empty());
        let mut matching = GrizzlyTransition::default();
        observe(
            &state,
            &GrizzlyAction::KillCreature {
                task_id: 1,
                creature: "oteryn:creature.crocodile".into(),
                observed_revision: 8,
            },
            &mut matching,
        )?;
        assert_eq!(
            matching.changes,
            [
                QuestTrackChange {
                    track: PHASE.into(),
                    before: 1,
                    after: 2
                },
                QuestTrackChange {
                    track: COUNT.into(),
                    before: 299,
                    after: 0
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn promotion_without_the_ordered_deathbine_report_cannot_advance() -> Result<(), GrizzlyRefusal>
    {
        let mut state = GrizzlyState::from([(PHASE.into(), 6)]);
        let mut promotion = GrizzlyTransition::default();
        promotion.changes.push(QuestTrackChange {
            track: storage_track(PROMOTION_STORAGE),
            before: -1,
            after: 0,
        });
        observe(&state, &GrizzlyAction::Promote, &mut promotion)?;
        assert!(!promotion.changes.iter().any(|c| c.track == PHASE));
        state.insert(REPORTED.into(), 1);
        observe(&state, &GrizzlyAction::Promote, &mut promotion)?;
        assert!(
            promotion
                .changes
                .iter()
                .any(|c| c.track == PHASE && c.after == 7)
        );
        Ok(())
    }
}
