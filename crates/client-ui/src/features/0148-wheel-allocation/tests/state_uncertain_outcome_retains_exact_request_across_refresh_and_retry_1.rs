#[test]
fn uncertain_outcome_retains_exact_request_across_refresh_and_retry() -> Result<(), Error> {
    let mut panel = panel();
    let command = request(&mut panel, Intent::Reveal, occurrence(1)).ok_or("reveal")?;
    let (wheel, atelier) = current(5);
    let expired = response(
        424,
        CommandStatus::DuplicateOutcomeExpired,
        vec![],
        &wheel,
        &atelier,
    )?;
    assert!(apply_result(&mut panel, &expired));
    assert_eq!(panel.retry.as_ref(), Some(&command));
    assert!(request(&mut panel, Intent::Reveal, occurrence(2)).is_none());
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    assert!(apply_result(
        &mut panel,
        &response(
            161,
            CommandStatus::Accepted,
            wire(wheel::encode_wheel_reply(&wheel))?,
            &wheel,
            &atelier
        )?
    ));
    assert_eq!(panel.retry.as_ref(), Some(&command));
    let retry = request(&mut panel, Intent::Retry, occurrence(99)).ok_or("retry")?;
    assert_eq!(retry, command);
    assert!(apply_result(
        &mut panel,
        &response(
            424,
            CommandStatus::DuplicateReplay,
            wire(atelier::encode_reply(&atelier))?,
            &wheel,
            &atelier
        )?
    ));
    assert!(panel.retry.is_none());
    assert!(panel.ready());
    Ok(())
}
