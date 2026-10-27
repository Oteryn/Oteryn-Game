use super::*;

const CURRENT: ProcessGeneration = ProcessGeneration::new(7);
const STALE: ProcessGeneration = ProcessGeneration::new(6);

fn configured() -> SurfaceState {
    let mut state = SurfaceState::new(CURRENT);
    assert_eq!(
        state.apply(SurfaceEvent::Resize {
            generation: CURRENT,
            width: 960,
            height: 540,
        }),
        Ok(SurfaceDecision::Configure(SurfaceSize::new(960, 540)))
    );
    assert_eq!(
        state.apply(SurfaceEvent::Configured {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::None)
    );
    state
}

#[test]
fn non_zero_resize_configures_and_zero_size_suspends() {
    let mut state = SurfaceState::new(CURRENT);
    assert_eq!(
        state.apply(SurfaceEvent::Resize {
            generation: CURRENT,
            width: 640,
            height: 480,
        }),
        Ok(SurfaceDecision::Configure(SurfaceSize::new(640, 480)))
    );
    assert_eq!(state.phase(), SurfacePhase::Lost);
    assert_eq!(
        state.apply(SurfaceEvent::Configured {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::None)
    );
    assert_eq!(state.phase(), SurfacePhase::Configured);
    assert_eq!(
        state.apply(SurfaceEvent::Resize {
            generation: CURRENT,
            width: 0,
            height: 480,
        }),
        Ok(SurfaceDecision::Suspend)
    );
    assert_eq!(state.phase(), SurfacePhase::Suspended);
}

#[test]
fn stale_generation_is_transactional() {
    let mut state = configured();
    let before = state.clone();
    assert_eq!(
        state.apply(SurfaceEvent::Resize {
            generation: STALE,
            width: 1,
            height: 1,
        }),
        Err(RendererError::StaleGeneration {
            expected: CURRENT,
            received: STALE,
        })
    );
    assert_eq!(state, before);
}

#[test]
fn timeout_and_occlusion_skip_without_mutation() {
    let mut state = configured();
    let before = state.clone();
    assert_eq!(
        state.apply(SurfaceEvent::Timeout {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::SkipTimeout)
    );
    assert_eq!(state, before);
    assert_eq!(
        state.apply(SurfaceEvent::Occluded {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::SkipOccluded)
    );
    assert_eq!(state, before);
}

#[test]
fn outdated_and_lost_request_bounded_recovery() {
    let mut state = configured();
    assert_eq!(
        state.apply(SurfaceEvent::Outdated {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::Reconfigure(SurfaceSize::new(960, 540)))
    );
    assert_eq!(state.phase(), SurfacePhase::Lost);
    assert_eq!(state.reconfigure_attempts(), 1);
    assert_eq!(
        state.apply(SurfaceEvent::Configured {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::None)
    );
    assert_eq!(
        state.apply(SurfaceEvent::Lost {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::Recreate(SurfaceSize::new(960, 540)))
    );
    assert_eq!(state.reconfigure_attempts(), 2);
}

#[test]
fn recovery_limit_failure_does_not_mutate() {
    let mut state = configured();
    for _ in 0..MAX_RECONFIGURE_ATTEMPTS {
        assert!(
            state
                .apply(SurfaceEvent::Outdated {
                    generation: CURRENT,
                })
                .is_ok()
        );
        if state.reconfigure_attempts() < MAX_RECONFIGURE_ATTEMPTS {
            assert_eq!(
                state.apply(SurfaceEvent::Configured {
                    generation: CURRENT,
                }),
                Ok(SurfaceDecision::None)
            );
        }
    }
    let before = state.clone();
    assert_eq!(
        state.apply(SurfaceEvent::Resume {
            generation: CURRENT,
        }),
        Err(RendererError::ReconfigureLimitExceeded {
            max_attempts: MAX_RECONFIGURE_ATTEMPTS,
        })
    );
    assert_eq!(state, before);
}

#[test]
fn successful_present_resets_recovery_and_counts_frames() {
    let mut state = configured();
    assert_eq!(
        state.apply(SurfaceEvent::Presented {
            generation: CURRENT,
            suboptimal: false,
        }),
        Ok(SurfaceDecision::Present)
    );
    assert_eq!(state.presented_frames(), 1);
    assert_eq!(state.reconfigure_attempts(), 0);
}

#[test]
fn suboptimal_frame_presents_then_requests_reconfiguration() {
    let mut state = configured();
    assert_eq!(
        state.apply(SurfaceEvent::Presented {
            generation: CURRENT,
            suboptimal: true,
        }),
        Ok(SurfaceDecision::PresentAndReconfigure(SurfaceSize::new(
            960, 540
        )))
    );
    assert_eq!(state.phase(), SurfacePhase::Lost);
    assert_eq!(state.presented_frames(), 1);
}

#[test]
fn suspend_resume_uses_latest_non_zero_size() {
    let mut state = configured();
    assert_eq!(
        state.apply(SurfaceEvent::Suspend {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::Suspend)
    );
    assert_eq!(
        state.apply(SurfaceEvent::Resume {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::Reconfigure(SurfaceSize::new(960, 540)))
    );
}

#[test]
fn presented_frame_overflow_fails_without_mutation() {
    let mut state = configured();
    state.presented_frames = u64::MAX;
    let before = state.clone();
    assert_eq!(
        state.apply(SurfaceEvent::Presented {
            generation: CURRENT,
            suboptimal: false,
        }),
        Err(RendererError::CounterOverflow {
            counter: RendererCounter::PresentedFrames,
        })
    );
    assert_eq!(state, before);
}

#[test]
fn close_is_idempotent_and_rejects_presentation() {
    let mut state = configured();
    assert_eq!(
        state.apply(SurfaceEvent::Close {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::Close)
    );
    assert_eq!(
        state.apply(SurfaceEvent::Close {
            generation: CURRENT,
        }),
        Ok(SurfaceDecision::None)
    );
    assert_eq!(
        state.apply(SurfaceEvent::Presented {
            generation: CURRENT,
            suboptimal: false,
        }),
        Err(RendererError::InvalidTransition {
            phase: SurfacePhase::Closing,
            event: SurfaceEventKind::Presented,
        })
    );
}
