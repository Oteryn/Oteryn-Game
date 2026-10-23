use super::*;
use oteryn_simulation_determinism::SemanticTimeMicros;
fn member(id: u8) -> SharedExperienceMember {
    SharedExperienceMember {
        character: [id; 16],
        channel: [1; 16],
        level: 100,
        x: 0,
        y: 0,
        vocation: 1,
        activity: Some(SharedExperienceActivityObservation {
            original_event_at: SemanticTimeMicros::from_micros(0),
            observed_at: SemanticTimeMicros::from_micros(120_000_000),
        }),
    }
}
fn pool(xp: i64, summon_xp: i64) -> SharedExperiencePool {
    SharedExperiencePool {
        creature_xp: xp,
        party_xp: xp,
        summon_xp,
    }
}
fn split(
    members: &[SharedExperienceMember],
    xp: i64,
) -> Result<Option<i64>, InvalidSharedExperience> {
    shared_experience_share(true, [1; 16], [1; 16], false, members, pool(xp, 0))
}
#[test]
fn shared_experience_exact_boundaries_and_every_failed_participant_disable() {
    let mut members = [member(1), member(2)];
    assert_eq!(split(&members, 101), Ok(Some(61)));
    members[1].level = 66;
    assert_eq!(split(&members, 101), Ok(None));
    members[1].level = 67;
    members[1].x = 30;
    members[1].y = -30;
    assert_eq!(split(&members, 101), Ok(Some(61)));
    members[1].x = 31;
    assert_eq!(split(&members, 101), Ok(None));
    members[1].x = 0;
    members[1].activity = Some(SharedExperienceActivityObservation {
        original_event_at: SemanticTimeMicros::from_micros(0),
        observed_at: SemanticTimeMicros::from_micros(120_000_001),
    });
    assert_eq!(split(&members, 101), Ok(None));
    members[1].activity = None;
    assert_eq!(split(&members, 101), Ok(None));
    members[1].channel = [2; 16];
    assert_eq!(split(&members, 101), Ok(Some(122)));
    assert_eq!(
        shared_experience_share(true, [1; 16], [1; 16], true, &members, pool(101, 0)),
        Ok(None)
    );
    assert_eq!(
        shared_experience_share(false, [1; 16], [1; 16], false, &members, pool(101, 0)),
        Ok(None)
    );
    assert_eq!(
        shared_experience_share(true, [3; 16], [1; 16], false, &members, pool(101, 0)),
        Ok(None)
    );
}
#[test]
fn shared_experience_vocation_bonus_ceiling_and_summon_deduction() {
    for (count, percent) in [(1, 120), (2, 130), (3, 160), (4, 200), (5, 200)] {
        let members: Vec<_> = (1..=count)
            .map(|id| {
                let mut m = member(id);
                m.vocation = id;
                m
            })
            .collect();
        let pool = 101_i128 * percent;
        let denominator = 100 * i128::from(count);
        let expected = pool / denominator + i128::from(pool % denominator != 0);
        assert_eq!(split(&members, 101), Ok(Some(expected as i64)));
    }
    assert_eq!(split(&[member(1), member(2)], 19), Ok(Some(10)));
    assert_eq!(
        shared_experience_share(
            true,
            [1; 16],
            [1; 16],
            false,
            &[member(1), member(2)],
            pool(100, 50)
        ),
        Ok(Some(35))
    );
    assert_eq!(
        shared_experience_share(true, [1; 16], [1; 16], false, &[member(1)], pool(100, 101)),
        Err(InvalidSharedExperience)
    );
}
#[test]
fn shared_experience_bound_and_wide_integer_oracle() {
    let members: Vec<_> = (1..=50).map(member).collect();
    for xp in [0, 1, 19, 20, 101, i64::MAX - 1, i64::MAX] {
        let pool = i128::from(xp) * if xp < 20 { 100 } else { 120 };
        assert_eq!(
            split(&members, xp),
            Ok(Some((pool / 5000 + i128::from(pool % 5000 != 0)) as i64))
        );
    }
    let mut excess = members;
    excess.push(member(51));
    assert_eq!(split(&excess, 1), Err(InvalidSharedExperience));
    assert_eq!(split(&[], 1), Err(InvalidSharedExperience));
    assert_eq!(
        split(&[member(1), member(1)], 1),
        Err(InvalidSharedExperience)
    );
    assert_eq!(split(&[member(1)], i64::MAX), Err(InvalidSharedExperience));
}

#[test]
fn shared_experience_damage_pool_handles_aggregated_wide_integer_products() {
    let mut inputs = vec![(u64::MAX, true); 50];
    assert_eq!(damage_experience_pool(i64::MAX, &inputs), Ok(i64::MAX));
    for input in &mut inputs[..25] {
        input.1 = false;
    }
    assert_eq!(damage_experience_pool(i64::MAX, &inputs), Ok(i64::MAX / 2));
    assert_eq!(damage_experience_pool(5, &[(5, false), (15, true)]), Ok(3));
    assert_eq!(damage_experience_pool(5, &[]), Ok(5));
    assert_eq!(
        damage_experience_pool(-1, &[]),
        Err(InvalidSharedExperience)
    );
    inputs.push((1, true));
    assert_eq!(
        damage_experience_pool(1, &inputs),
        Err(InvalidSharedExperience)
    );
}
