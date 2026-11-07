use super::*;

#[test]
fn planned_world_reset_loss_requires_matching_world_and_sealed_binding() -> TestResult {
    let mut source = owner()?;
    let world_id = source.0.session.commit().world_id();
    source.0.cause = ControlLossCauseV1::PlannedWorldReset {
        world_id,
        reset_epoch: 0,
        retirement_binding: [7; 32],
        target_digest: [9; 32],
    };
    let session = source.0.session.current_game_session_id();
    let authorization = ControlLossAuthorizationV1::authorize(&source, session, 100)
        .map_err(|_| "planned reset authorization")?;
    let effect = authorization
        .validate_final(&source, 100)
        .map_err(|_| "planned reset final")?;
    assert_eq!(
        effect.successor().session_state(),
        GameSessionState::Reconnectable
    );
    assert_eq!(
        effect.successor().current_original_grace_deadline(),
        Some(120)
    );
    for cause in [
        ControlLossCauseV1::PlannedWorldReset {
            world_id: WorldId::decode(source.0.session.commit().character_id().as_bytes())
                .map_err(|_| "other world")?,
            reset_epoch: 0,
            retirement_binding: [7; 32],
            target_digest: [9; 32],
        },
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch: 0,
            retirement_binding: [0; 32],
            target_digest: [9; 32],
        },
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch: u64::MAX,
            retirement_binding: [7; 32],
            target_digest: [9; 32],
        },
    ] {
        let mut changed = source.clone();
        changed.0.cause = cause;
        assert!(ControlLossAuthorizationV1::authorize(&changed, session, 100).is_err());
        assert!(authorization.validate_final(&changed, 100).is_err());
    }
    for cause in [
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch: 1,
            retirement_binding: [7; 32],
            target_digest: [9; 32],
        },
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch: 0,
            retirement_binding: [8; 32],
            target_digest: [9; 32],
        },
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch: 0,
            retirement_binding: [7; 32],
            target_digest: [8; 32],
        },
    ] {
        let mut changed = source.clone();
        changed.0.cause = cause;
        assert!(ControlLossAuthorizationV1::authorize(&changed, session, 100).is_ok());
        assert!(authorization.validate_final(&changed, 100).is_err());
    }
    Ok(())
}
