//! The former caller-boolean case now exercises absence of the durable promotion row.
use super::*;

#[test]
fn wheel_not_promoted_refuses_from_durable_state_at_temple() -> TestResult {
    run("wheel_not_promoted", async |h| {
        eligible(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = snapshot(&h.pool).await?;
        let outcome = commit(
            h,
            &authority,
            fence(3)?,
            request(2, 0, &[(15, 9)])?,
            &ruleset(R1)?,
            AT_TEMPLE,
        )
        .await?;
        assert!(matches!(
            outcome,
            WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPromoted))
        ));
        assert_eq!(snapshot(&h.pool).await?, before);
        Ok(())
    })
}
