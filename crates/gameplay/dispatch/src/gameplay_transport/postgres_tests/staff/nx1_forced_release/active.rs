// NX-1 focused staff release case; included in the original test module.
#[test]
fn nx1_staff_signed_kick_saves_in_fight_and_releases_exact_session_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "forced release panicked")??;
    eprintln!(
        "NX-1 forced staff release PostgreSQL duration: {:?}",
        began.elapsed()
    );
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let signing = Signing::new()?;
        let h =
            Harness::create_with_admission_seed(admin, "nx1_forced_release", true, false).await?;
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
        assert_eq!(
            owner.release_staff_session(deps.admitted).await,
            GraceExpiryResult::NotApplicable,
            "no signed staff decision cannot force a release"
        );
        let issuer = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?)
            .await?;
        let issuer_fence =
            crate::gameplay_transport::item_fence_of(issuer).ok_or("Channel issuer fence")?;
        authorized_kick(&signing, &h, &authority, issuer_fence, 96).await?;
        {
            let runtime = owner.runtime.lock().await;
            owner.states.attack.lock().await.record_hit_taken(
                &runtime,
                deps.actor,
                deps.admitted.game_session_id,
                owner.owner_now(),
            );
        }
        let mut stale = deps.admitted;
        stale.continuity.connection_generation += 1;
        assert_eq!(
            owner.release_staff_session(stale).await,
            GraceExpiryResult::NotApplicable
        );
        assert_eq!(
            owner.release_staff_session(deps.admitted).await,
            GraceExpiryResult::Released
        );
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert_eq!(
            store
                .current_session(deps.admitted.game_session_id)
                .await?
                .session_state(),
            GameSessionState::Terminal
        );
        let saves:i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(saves, 1, "forced release saves the owned position");
        let claims:i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions WHERE account_id=encode($1,'hex')::uuid AND session_state<>3").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(claims, 0, "account can obtain a fresh admission");
        let bans: i64 = sqlx::query_scalar("SELECT count(*) FROM game_staff_bans")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(bans, 0, "kick does not create an account ban");
        // Re-read released account authority and commit a genuine new admission.
        use super::super::fresh_fixture::fresh::Source;
        use crate::durability::admission_authority_guards::AdmissionGuardStore;
        use crate::foundation::CharacterId;
        use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
        use crate::gameplay_transport::canonical_uuid;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut next = Source::appearance_observer(
            now,
            canonical_uuid(&id(40)),
            CharacterId::decode(&id(41)).map_err(debug)?,
        )?;
        next.current.character_lease_generation = store
            .current_session(deps.admitted.game_session_id)
            .await?
            .current_character_lease()
            .generation();
        let keys: Vec<_> = next.rows.iter().map(|row| row.key.clone()).collect();
        next.rows = AdmissionGuardStore::from_root(h.root.clone())
            .load(&keys)
            .await?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("released authority missing")?;
        let fresh = next
            .request_with_nonce(
                crate::foundation::GameSessionId::decode(&id(120)).map_err(debug)?,
                crate::foundation::AuthenticatedTransportRefV1::decode(&[120; 16])
                    .map_err(debug)?,
                Some((&canonical_uuid(&id(40)), &canonical_uuid(&id(41)))),
                now,
                Some([120; 32]),
            )
            .map_err(|error| format!("NX-1 fresh request after saved staff release: {error}"))?;
        assert!(matches!(
            store.commit(&fresh).await.map_err(|error| format!(
                "NX-1 fresh commit after saved staff release: {error:?}"
            ))?,
            FreshAdmissionDurableOutcomeV1::Committed(_)
        ));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
