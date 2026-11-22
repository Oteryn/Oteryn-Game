// MP connection scheduling and server event observations.
use std::time::Duration;

use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::{
    analyser::{AnalyserFact, AnalyserImpact},
    world_spatial::ActorPosition,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(in super::super) async fn while_servicing_liveness<T>(
    step: impl std::future::Future<Output = T>,
    mut pump: impl AsyncFnMut() -> TestResult,
) -> TestResult<T> {
    let (finished, mut step_finished) = tokio::sync::oneshot::channel::<()>();
    let run_step = async move {
        let result = step.await;
        drop(finished);
        TestResult::Ok(result)
    };
    let service = async {
        let mut slice = tokio::time::interval(Duration::from_millis(250));
        slice.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                _ = &mut step_finished => return Ok(()),
                _ = slice.tick() => pump().await?,
            }
        }
    };
    // Finish any in-flight liveness read before handing the peers back to the caller.
    let (result, ()) = tokio::try_join!(run_step, service)?;
    Ok(result)
}

pub(super) async fn prepare_mp09_profiles(
    mp01_profiles_confirmed: bool,
    provision: impl std::future::Future<Output = TestResult<String>>,
) -> TestResult<String> {
    if mp01_profiles_confirmed {
        return Ok("reused confirmed MP01 knight, druid and paladin profiles".into());
    }
    provision.await
}

pub(super) fn is_temple_arrival(event: &SessionEvent, after: u64, temple: ActorPosition) -> bool {
    let (sequence, position) = match event {
        SessionEvent::WorldSpatial(delta) => (delta.server_sequence, delta.value.actor_position),
        SessionEvent::WorldSpatialEntities(delta) => {
            (delta.server_sequence, delta.value.actor_position)
        }
        _ => return false,
    };
    sequence > after && position == temple
}

pub(super) fn analyser_fact(
    event: &SessionEvent,
    after: u64,
    matches_fact: impl Fn(&AnalyserFact) -> bool,
) -> bool {
    match event {
        SessionEvent::AnalyserFacts(delta) if delta.server_sequence > after => {
            delta.value.iter().any(matches_fact)
        }
        SessionEvent::AnalyserAttributedFacts(delta) if delta.server_sequence > after => {
            delta.value.iter().any(|fact| matches_fact(&fact.fact))
        }
        _ => false,
    }
}

pub(super) fn committed_swing(event: &SessionEvent, after: u64) -> bool {
    analyser_fact(event, after, |fact| {
        matches!(fact, AnalyserFact::Impact(AnalyserImpact::Damage { .. }))
    })
}
