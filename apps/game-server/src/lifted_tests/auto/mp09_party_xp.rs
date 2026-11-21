//! Focused MP01 and MP09 qualification harness checks.
//! Crystal 201189f8 src/creatures/players/grouping/party.cpp:548-594:
//! range <=30 and level >=ceil(highest/1.5); config.lua.dist:302 sets 1.5.
//! These tests do not certify wire E2E.
use std::time::Duration;

use oteryn_dev_client::SessionEvent;
use oteryn_gameplay_social_rules::{
    shared_experience::SharedExperienceActivityObservation,
    shared_experience_split::{
        SharedExperienceMember, SharedExperiencePool, shared_experience_share,
    },
};
use oteryn_protocol_oteryn::{
    analyser::{AnalyserFact, AnalyserImpact},
    world_spatial::ActorPosition,
};
use oteryn_simulation_determinism::SemanticTimeMicros;

mod observations {
    include!("../../qualification/client_phases/mp/observations.rs");
}

use observations::{committed_swing, is_temple_arrival, while_servicing_liveness};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn mp09_reuses_confirmed_mp01_profiles_without_provision_logout() {
    use std::future::{Future, ready};
    use std::task::{Context, Poll, Waker};

    let mut context = Context::from_waker(Waker::noop());
    let mut reused = std::pin::pin!(observations::prepare_mp09_profiles(
        true,
        ready(Err("logout-after-provision refused".into())),
    ));
    assert!(matches!(reused.as_mut().poll(&mut context), Poll::Ready(Ok(_))));

    let mut standalone = std::pin::pin!(observations::prepare_mp09_profiles(
        false,
        ready(Ok("standalone provisioning completed".into())),
    ));
    assert!(matches!(
        standalone.as_mut().poll(&mut context),
        Poll::Ready(Ok(detail)) if detail == "standalone provisioning completed"
    ));
}


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

#[tokio::test(start_paused = true)]
async fn mp01_relog_admission_progresses_during_an_in_flight_liveness_read() -> TestResult {
    let read_started = tokio::sync::Notify::new();
    let admission_finished = tokio::sync::Notify::new();
    let step = async {
        read_started.notified().await;
        admission_finished.notify_one();
    };
    tokio::time::timeout(
        Duration::from_secs(1),
        while_servicing_liveness(step, async || {
            read_started.notify_one();
            admission_finished.notified().await;
            Ok(())
        }),
    )
    .await?
}

#[test]
fn mp01_pvp_requires_a_fresh_temple_arrival_event() {
    let temple = ActorPosition {
        x: 100,
        y: 200,
        floor: 7,
    };
    let spatial = |sequence, x| {
        SessionEvent::WorldSpatial(oteryn_dev_client::AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence,
            new_revision: sequence + 1,
            value: oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation {
                content_generation: [1; 32],
                actor_position: ActorPosition {
                    x,
                    y: 200,
                    floor: 7,
                },
            },
        })
    };
    assert!(!is_temple_arrival(&spatial(20, 100), 20, temple));
    assert!(!is_temple_arrival(&spatial(21, 101), 20, temple));
    assert!(is_temple_arrival(&spatial(21, 100), 20, temple));
    let entities = SessionEvent::WorldSpatialEntities(oteryn_dev_client::AppliedDelta {
        server_sequence: 21,
        base_revision: 20,
        new_revision: 21,
        value: oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesDelta {
            content_generation: [1; 32],
            actor_position: temple,
            enter: Vec::new(),
            update: Vec::new(),
            leave: Vec::new(),
        },
    });
    assert!(is_temple_arrival(&entities, 20, temple));
}

#[test]
fn mp09_counts_only_fresh_committed_melee_damage() -> TestResult {
    use oteryn_dev_client::AppliedDelta;
    use oteryn_protocol_oteryn::analyser::AttributedFact;
    let value = std::num::NonZeroU32::new(3).ok_or("fixture damage")?;
    let damage = AnalyserFact::Impact(AnalyserImpact::Damage {
        value,
        element: oteryn_protocol_oteryn::damage_element::DamageElement::Physical,
    });
    let event = SessionEvent::AnalyserAttributedFacts(AppliedDelta {
        server_sequence: 21,
        base_revision: 1,
        new_revision: 2,
        value: vec![AttributedFact {
            fact: damage,
            attacker: None,
        }],
    });
    assert!(committed_swing(&event, 20));
    assert!(!committed_swing(&event, 21));
    let healing = SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 22,
        base_revision: 2,
        new_revision: 3,
        value: vec![AnalyserFact::Impact(AnalyserImpact::Healing(value))],
    });
    assert!(!committed_swing(&healing, 20));
    Ok(())
}
