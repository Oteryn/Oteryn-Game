//! Boundary checks supporting MP09's real-wire distance journey.
//! Crystal 201189f8 src/creatures/players/grouping/party.cpp:548-594:
//! range <=30 and level >=ceil(highest/1.5); config.lua.dist:302 sets 1.5.
//! These tests do not certify wire E2E.
use oteryn_gameplay_social_rules::{
    shared_experience::SharedExperienceActivityObservation,
    shared_experience_split::{
        SharedExperienceMember, SharedExperiencePool, shared_experience_share,
    },
};
use oteryn_simulation_determinism::SemanticTimeMicros;

#[test]
fn mp09_one_distant_or_underlevel_member_disables_the_whole_share() {
    let mut members = [1, 2, 3].map(|id| SharedExperienceMember {
        character: [id; 16],
        channel: [1; 16],
        level: 100,
        x: 0,
        y: 0,
        z: 7,
        vocation: id,
        activity: Some(SharedExperienceActivityObservation {
            original_event_at: SemanticTimeMicros::from_micros(1_000_000),
            observed_at: SemanticTimeMicros::from_micros(2_000_000),
        }),
    });
    let share = |members: &[SharedExperienceMember]| {
        shared_experience_share(
            true,
            [1; 16],
            [1; 16],
            false,
            members,
            SharedExperiencePool {
                creature_xp: 700,
                party_xp: 700,
                summon_xp: 0,
            },
        )
    };
    assert!(matches!(share(&members), Ok(Some(_))));
    members[2].x = 30;
    assert!(matches!(share(&members), Ok(Some(_))));
    members[2].x = 31;
    assert_eq!(share(&members), Ok(None));
    members[2].x = 0;
    members[2].level = 66;
    assert_eq!(share(&members), Ok(None));
    members[2].level = 67;
    assert!(matches!(share(&members), Ok(Some(_))));
    members[2].activity = None;
    assert_eq!(share(&members), Ok(None));
}
