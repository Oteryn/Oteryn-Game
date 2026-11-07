use super::*;

#[test]
fn reset_drain_exit_is_retryable_and_retains_the_observed_reset()
-> Result<(), Box<dyn std::error::Error>> {
    let state = ResetWatch::default();
    assert!(classify_exit(&state, None).is_ok());
    assert!(matches!(
        classify_exit(
            &state,
            Some(crate::gameplay_transport::GameplayServeError::InvalidLimits)
        ),
        Err(BootError::Serve)
    ));
    state.requested.store(true, Ordering::Release);
    let error = classify_exit(&state, None)
        .err()
        .ok_or("a completed reset drain must restart")?;
    assert_eq!(error.exit_code(), 17);
    assert!(matches!(error, BootError::Readiness("World reset drain")));
    Ok(())
}
