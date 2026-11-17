// NX-1 focused staff release case; included in the original test module.
#[test]
fn nx1_staff_captured_kick_rejects_lost_resume_before_generation_mutation_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| lost_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "lost kick fixture panicked")??;
    eprintln!(
        "NX-1 captured lost kick PostgreSQL duration: {:?}",
        began.elapsed()
    );
    Ok(())
}
fn lost_scenario() -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, ControlLossEpochRefV1, GameSessionId};
    use crate::gameplay_transport::connection::{
        AdmissionRefusal, ControlLossResult, ResumeAttempt,
    };
    run(async |admin| {
        let signing = Signing::new()?;
        let h = Harness::create_with_admission_seed(admin, "nx1_lost_kick", true, false).await?;
        prepare_current_session(&h).await?;
        h.seed_pvp_target().await?;
        crate::gameplay_transport::monk_save::postgres_tests::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
        signing.bootstrap(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        super::super::track(&owner, &deps)?;
        owner
            .states
            .command_ingress
            .admit(deps.admitted.game_session_id);
        assert_eq!(
            owner
                .lose_control(deps.admitted, std::time::Duration::ZERO, None)
                .await,
            ControlLossResult::Recorded
        );
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let before = store.current_session(deps.admitted.game_session_id).await?;
        let fence = crate::gameplay_transport::item_fence_of(before).ok_or("lost fence")?;
        let epoch = before.current_control_loss_epoch().ok_or("lost epoch")?;
        let account = crate::domain::AccountId::from_bytes(id(40)).map_err(debug)?;
        assert!(
            !h.root
                .staff_lost_session_kicked(&authority, &h.node, fence, account, epoch)
                .await?
        );
        let issuer = store
            .current_session(GameSessionId::decode(&id(110)).map_err(debug)?)
            .await?;
        authorized_kick(
            &signing,
            &h,
            &authority,
            crate::gameplay_transport::item_fence_of(issuer).ok_or("issuer")?,
            98,
        )
        .await?;
        assert!(
            h.root
                .staff_lost_session_kicked(&authority, &h.node, fence, account, epoch)
                .await?
        );
        assert!(
            h.root
                .staff_lost_session_kicked(
                    &authority,
                    &h.node,
                    fence,
                    account,
                    ControlLossEpochRefV1::new(epoch.get() + 1).map_err(debug)?
                )
                .await
                .is_err()
        );
        let token = super::super::house_transfer::RecoverySource::token(2)?;
        assert!(matches!(
            owner
                .resume(ResumeAttempt {
                    game_session_id: deps.admitted.game_session_id,
                    recovery_material: token.as_bytes(),
                    transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
                    last_applied_server_sequence: deps.admitted.continuity.server_sequence,
                    supported_capabilities: &[],
                })
                .await,
            Err(AdmissionRefusal::Rejected)
        ));
        let after = store.current_session(deps.admitted.game_session_id).await?;
        assert_eq!(after.session_state(), GameSessionState::Reconnectable);
        assert_eq!(
            after.current_connection_generation(),
            before.current_connection_generation()
        );
        assert_eq!(
            after.current_control_loss_epoch(),
            before.current_control_loss_epoch()
        );
        assert_eq!(
            after.current_original_grace_deadline(),
            before.current_original_grace_deadline()
        );
        assert!(after.current_transport().is_none());
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        assert!(
            after
                .current_original_grace_deadline()
                .is_some_and(|deadline| deadline > now)
        );
        assert_eq!(
            owner.expire_control_loss(deps.admitted).await,
            GraceExpiryResult::Released,
            "autonomous grace owner observes signed kick before the original deadline"
        );
        assert_eq!(
            store
                .current_session(deps.admitted.game_session_id)
                .await?
                .session_state(),
            GameSessionState::Terminal
        );
        let saves:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            saves, 1,
            "lost kick also saves owned position before terminal release"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
