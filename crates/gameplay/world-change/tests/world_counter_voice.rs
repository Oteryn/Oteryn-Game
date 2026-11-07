#![allow(
    clippy::expect_used,
    reason = "focused assertions over valid authored encounter states"
)]
use oteryn_gameplay_world_change::counter::{CounterKind, CounterState, Credit, Parameters};

#[test]
fn worldchg_voice_hundred_gobbles_twenty_five_complete_owned_waves_then_mage() {
    let k = CounterKind::TheirMastersVoice;
    let p = Parameters::default();
    let mut state = CounterState::new(k, 1, false);
    for _ in 0..99 {
        state = state.credit(k, Credit::Task(0), &p, 1000).expect("gobble");
        assert_eq!(state.stage, 1);
    }
    assert_eq!(state.deadline_ms, Some(3_601_000));
    state = state
        .credit(k, Credit::Task(0), &p, 1001)
        .expect("hundredth");
    assert_eq!((state.stage, state.counts[1]), (2, 1));
    // Eligibility does not cap extraction or restart the original deadline.
    state = state
        .credit(k, Credit::Task(0), &p, 1010)
        .expect("late gobble");
    assert_eq!((state.counts[0], state.deadline_ms), (101, Some(3_601_000)));
    for wave in 1..=25 {
        let generation = state.credit_generation;
        let phase = state.phase_version;
        let first = Credit::VoiceServant { wave, ordinal: 0 };
        state = state.credit(k, first, &p, 2000).expect("first servant");
        assert_eq!(state.credit(k, first, &p, 2000).expect("duplicate"), state);
        assert!(
            state
                .credit(k, Credit::VoiceServant { wave, ordinal: 25 }, &p, 2000)
                .is_err()
        );
        for ordinal in 1..25 {
            state = state
                .credit(k, Credit::VoiceServant { wave, ordinal }, &p, 2000)
                .expect("current servant slot");
        }
        assert!(state.credit_generation > generation);
        assert!(state.phase_version > phase);
        assert!(state.credit(k, first, &p, 2001).is_err(), "old wave denied");
        if wave < 25 {
            assert_eq!(
                (state.stage, state.counts[1], state.counts[2]),
                (2, u64::from(wave) + 1, 0)
            );
        }
    }
    assert_eq!(state.stage, 3);
    state = state
        .credit(k, Credit::VoiceMage, &p, 3000)
        .expect("mage death");
    assert_eq!(state.stage, 4);
    assert_eq!(state.deadline_ms, Some(3_601_000));
    let old_generation = state.credit_generation;
    let restored = state.tick(k, 3_601_000).expect("shared timeout");
    assert_eq!(
        (restored.stage, restored.counts, restored.deadline_ms),
        (1, [0; 3], None)
    );
    assert!(restored.credit_generation > old_generation);
}

#[test]
fn worldchg_voice_timeout_before_hundred_changes_projection_and_generation() {
    let k = CounterKind::TheirMastersVoice;
    let state = CounterState::new(k, 1, false)
        .credit(k, Credit::Task(0), &Parameters::default(), 1)
        .expect("first use");
    let restored = state.tick(k, 3_600_001).expect("timeout");
    assert!(restored.phase_version > state.phase_version);
    assert!(restored.credit_generation > state.credit_generation);
    assert!(
        state
            .credit(k, Credit::Task(0), &Parameters::default(), 3_600_001)
            .is_err()
    );
}
