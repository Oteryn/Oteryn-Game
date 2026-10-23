//! A receipt walk after the original turn must retain its committed activity clock.
use super::*;
#[test]
fn party_xp_spell_death_preserves_original_precise_clock_on_replay()
-> Result<(), Box<dyn std::error::Error>> {
    let mut f = fixture();
    let queue = KillSettlementQueue::default();
    let rewards = rewards();
    let receipt = hit(&mut f, 1, 20);
    assert_eq!(
        f.runtime
            .committed_spell_batch_time(&receipt)
            .expect("retained time")
            .expect("precise clock")
            .get(),
        100_999
    );
    let sink = KillSink {
        activity: &[],
        queue: &queue,
        rewards: &rewards,
    };
    record_batch_kills(&mut f.runtime, sink, &receipt, 999_999);
    let taken = match queue.take(f.session, 1) {
        crate::gameplay_transport::kill_reward::TakeOutcome::Taken(t) => t,
        _ => return Err("real spell kill queued".into()),
    };
    let entry = taken.entry().expect("entry");
    assert_eq!(entry.facts.death_at_ms, 100);
    assert_eq!(
        entry.activity_observed_at.expect("frozen clock").get(),
        100_999
    );
    taken.requeue();
    record_batch_kills(&mut f.runtime, sink, &receipt, 1_999_999);
    let taken = match queue.take(f.session, 1) {
        crate::gameplay_transport::kill_reward::TakeOutcome::Taken(t) => t,
        _ => return Err("same kill retained".into()),
    };
    assert_eq!(taken.entry().expect("entry").facts.death_at_ms, 100);
    Ok(())
}
