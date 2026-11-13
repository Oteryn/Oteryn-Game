//! P1-a: revocation application and reconnect freshness at real durable owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::platform_streams::{
    PageCounts, PageOutcome, PlatformStream, SecurityRevocation, StreamPage,
    security_revocation_read_fresh,
};
use crate::foundation::GameSessionState;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FreshAdmissionAuthority, GraceExpiryResult, ResumeAttempt,
};

#[test]
fn revocation_freshness_uses_injected_clock_and_preserves_disabled_behavior() {
    assert!(security_revocation_read_fresh(false, None, 100_000));
    assert!(!security_revocation_read_fresh(true, None, 100_000));
    assert!(security_revocation_read_fresh(true, Some(71_000), 100_000));
    assert!(!security_revocation_read_fresh(true, Some(69_000), 100_000));
    assert!(!security_revocation_read_fresh(
        true,
        Some(100_001),
        100_000
    ));
}

#[test]
fn revocation_generation_cause_replay_and_terminal_reconnect_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new().name("security-revocation".into()).stack_size(64<<20).spawn(move || {
        let result = run(async |admin| {
            for (generation, cause, terminal) in [(2,1,true),(1,3,true),(1,1,false)] {
                let h = Harness::create_with_admission_seed(admin.clone(), "security_revocation", true, false).await?;
                let result: TestResult = async {
                    prepare_current_session(&h).await?;
                    let fixed_now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
                    let _clock=super::preadmission::FrozenAdmissionClock::install(&h,fixed_now).await?;
                    let mut connection=h.root.try_acquire_ready()?;
                    sqlx::query("SET ROLE oteryn_game_runtime").execute(&mut *connection).await?;
                    let returned=connection.return_to_pool_observed_until(std::time::Instant::now()+std::time::Duration::from_secs(5)).await;
                    assert!(matches!(returned,sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle));
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    let deps = Dependencies::new()?;
                    let mut owner = deps.owner(&h,&authority);
                    std::sync::Arc::get_mut(&mut owner.states).ok_or("exclusive states")?.security_revocation_stream_enabled=true;
                    let session = deps.admitted.game_session_id;
                    assert!(!h.root.security_session_revoked(session).await.map_err(debug)?);
                    // A failed read never calls apply and cannot terminate current play.
                    assert_eq!(FreshAdmissionStore::from_root(h.root.clone()).current_session(session).await?.session_state(),GameSessionState::Active);
                    if cause==3 {
                        owner.states.command_ingress.admit(session);
                        assert_eq!(owner.lose_control(deps.admitted,std::time::Duration::ZERO,None).await,crate::gameplay_transport::connection::ControlLossResult::Recorded);
                let lost=owner.states.lost.lock().map_err(|_|"lost lock")?.get(&session).copied().ok_or("published loss")?;
                assert!(owner.states.command_ingress.resumable(&lost),"unrelated command continuity must permit recovery");

                    } else if generation==2 {
                        // A session that predates migration 2827 is backfilled from its actual signed admission receipt.
                        sqlx::query("UPDATE game_durability_reconnect_sessions SET admission_security_generation=NULL WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).execute(&h.pool).await?;
                    }
                    let item = SecurityRevocation { revocation_id:id(91), account_id:id(40), native_security_generation:generation, cause, effective_at_ms:1_700_000_000_000 };
                    let page = StreamPage::SecurityRevocations(vec![item]);
                    assert_eq!(h.root.apply_platform_stream_page(None,page.clone(),Some("v1:1".into())).await.map_err(debug)?,PageOutcome::Advanced(PageCounts { applied:1, ..Default::default() }));
                    assert_eq!(h.root.security_session_revoked(session).await.map_err(debug)?,terminal);
                    // Lost cursor: re-request the original page and verify idempotency by ID.
                    sqlx::query("UPDATE game_platform_stream_cursors SET stream_cursor=NULL WHERE stream=3").execute(&h.pool).await?;
                    assert_eq!(h.root.apply_platform_stream_page(None,page,Some("v1:1".into())).await.map_err(debug)?,PageOutcome::Advanced(PageCounts { replayed:1, ..Default::default() }));
                    let applied:i64=sqlx::query_scalar("SELECT count(*) FROM game_security_revocations").fetch_one(&h.pool).await?;
                    assert_eq!(applied,1);
                    assert_eq!(h.root.platform_stream_cursor(PlatformStream::SecurityRevocations).await.map_err(debug)?.as_deref(),Some("v1:1"));
                    if terminal {
                        let held = super::house_transfer::RecoverySource::token(97)?;
                        let captured=owner.resume(ResumeAttempt { game_session_id:session,recovery_material:held.as_bytes(),last_applied_server_sequence:0,transport:crate::foundation::AuthenticatedTransportRefV1::decode(&[97;16]).map_err(debug)?,supported_capabilities:&[] }).await;
                        assert!(matches!(captured,Err(AdmissionRefusal::Rejected)),"captured session refuses before terminal release");
                        for candidate in [false,true] {
                            let error=if candidate {
                                sqlx::query("UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref=$2 WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).bind([1u8;8].as_slice()).execute(&h.pool).await.err()
                            } else {
                                sqlx::query("UPDATE game_durability_reconnect_sessions SET current_transport_ref=$2 WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).bind([97u8;16].as_slice()).execute(&h.pool).await.err()
                            }.ok_or("revoked proof must not acquire controller")?;
                            assert_eq!(error.as_database_error().map(|e|e.message()),Some("security revoked reconnect"));
                        }
                        assert_eq!(owner.release_staff_session(deps.admitted).await,GraceExpiryResult::Released);
                        assert_eq!(FreshAdmissionStore::from_root(h.root.clone()).current_session(session).await?.session_state(),GameSessionState::Terminal);
                        let held = super::house_transfer::RecoverySource::token(97)?;
                        let refusal=owner.resume(ResumeAttempt { game_session_id:session, recovery_material:held.as_bytes(), last_applied_server_sequence:0, transport:crate::foundation::AuthenticatedTransportRefV1::decode(&[97;16]).map_err(debug)?, supported_capabilities:&[] }).await;
                        assert!(matches!(refusal,Err(AdmissionRefusal::Rejected)));
                        use crate::durability::admission_authority_guards::AdmissionGuardStore;
                        use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
                        let now=fixed_now;
                        let mut next=super::fresh_fixture::fresh::Source::new(now)?;
                        next.security_generation=2;
                        let store=FreshAdmissionStore::from_root(h.root.clone());
                        next.current.character_lease_generation=store.current_session(session).await?.current_character_lease().generation();
                        let keys=next.rows.iter().map(|row|row.key.clone()).collect::<Vec<_>>();
                        next.rows=AdmissionGuardStore::from_root(h.root.clone()).load(&keys).await?.into_iter().collect::<Option<Vec<_>>>().ok_or("released authority")?;
                        let request=next.request_with_nonce(crate::foundation::GameSessionId::decode(&id(120)).map_err(debug)?,crate::foundation::AuthenticatedTransportRefV1::decode(&[120;16]).map_err(debug)?,Some((&crate::gameplay_transport::canonical_uuid(&id(40)),&crate::gameplay_transport::canonical_uuid(&id(41)))),now,Some([120;32]))?;
                        assert!(matches!(store.commit(&request).await?,FreshAdmissionDurableOutcomeV1::Committed(_)));
                        // Old duplicate must not capture this newer, independently authorized session.
                        let cursor=h.root.platform_stream_cursor(PlatformStream::SecurityRevocations).await.map_err(debug)?;
                        h.root.apply_platform_stream_page(cursor,StreamPage::SecurityRevocations(vec![SecurityRevocation { revocation_id:id(91), account_id:id(40), native_security_generation:generation,cause,effective_at_ms:1_700_000_000_000 }]),Some("v1:1".into())).await.map_err(debug)?;
                        assert_eq!(store.current_session(crate::foundation::GameSessionId::decode(&id(120)).map_err(debug)?).await?.session_state(),GameSessionState::Active);
                        h.root.apply_platform_stream_page(Some("v1:1".into()),StreamPage::SecurityRevocations(vec![SecurityRevocation { revocation_id:id(92), account_id:id(40),native_security_generation:generation,cause:1,effective_at_ms:1_700_000_000_001 }]),Some("v1:2".into())).await.map_err(debug)?;
                        assert!(!h.root.security_session_revoked(crate::foundation::GameSessionId::decode(&id(120)).map_err(debug)?).await.map_err(debug)?);

                    } else {
                        assert_eq!(owner.release_staff_session(deps.admitted).await,GraceExpiryResult::NotApplicable);
                        assert_eq!(FreshAdmissionStore::from_root(h.root.clone()).current_session(session).await?.session_state(),GameSessionState::Active);
                    }
                    Ok(())
                }.await;
                let cleanup=h.cleanup().await;
                result?;cleanup?;
            }
            Ok(())
        }).map_err(|e|e.to_string());
        let _=send.send(result);
    })?;
    receive
        .recv_timeout(std::time::Duration::from_secs(120))
        .map_err(|e| {
            format!("revocation scenario watchdog (admission/apply/release/cleanup): {e}")
        })?
        .map_err(Into::into)
}

#[test]
fn revocation_freshness_real_resume_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("revocation-freshness".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "revocation_freshness", true, false)
                        .await?;
                let result: TestResult = async {
                    prepare_current_session(&h).await?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                    )
                    .fetch_one(&h.pool)
                    .await?;
                    let _clock =
                        super::preadmission::FrozenAdmissionClock::install(&h, now).await?;
                    let source = super::house_transfer::RecoverySource::start_at(
                        &h,
                        Some(u64::try_from(now)?),
                    )
                    .await?;
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let deps = Dependencies::new()?;
                    let mut owner = deps.owner(&h, &authority);
                    owner.evidence = &source.evidence;
                    let session = deps.admitted.game_session_id;
                    // Native connection entry owns this command ledger; the fixture admitted durably above.
                    owner.states.command_ingress.admit(session);
                    let disabled_credential = fixed_recovery_token(now, 98)?;
                    assert_eq!(
                        h.root
                            .security_revocation_last_read_ms()
                            .await
                            .map_err(debug)?,
                        None
                    );
                    assert_eq!(
                        owner
                            .lose_control(deps.admitted, std::time::Duration::ZERO, None)
                            .await,
                        crate::gameplay_transport::connection::ControlLossResult::Recorded
                    );
                    let current = owner
                        .resume(ResumeAttempt {
                            game_session_id: session,
                            recovery_material: disabled_credential.as_bytes(),
                            last_applied_server_sequence: 0,
                            transport: crate::foundation::AuthenticatedTransportRefV1::decode(
                                &[98; 16],
                            )
                            .map_err(debug)?,
                            supported_capabilities: &[],
                        })
                        .await
                        .map_err(|e| format!("disabled-stream reconnect: {e:?}"))?;
                    assert_eq!(
                        h.root
                            .security_revocation_last_read_ms()
                            .await
                            .map_err(debug)?,
                        None
                    );
                    let credential = fixed_recovery_token(now, 99)?;
                    let attempt = || -> TestResult<ResumeAttempt<'_>> {
                        Ok(ResumeAttempt {
                            game_session_id: session,
                            recovery_material: credential.as_bytes(),
                            last_applied_server_sequence: 0,
                            transport: crate::foundation::AuthenticatedTransportRefV1::decode(
                                &[99; 16],
                            )
                            .map_err(debug)?,
                            supported_capabilities: &[],
                        })
                    };
                    std::sync::Arc::get_mut(&mut owner.states)
                        .ok_or("exclusive states")?
                        .security_revocation_stream_enabled = true;
                    assert!(
                        matches!(
                            owner.resume(attempt()?).await,
                            Err(AdmissionRefusal::Rejected)
                        ),
                        "no successful read"
                    );
                    assert!(
                        owner.staff_refresh(current).await,
                        "an unavailable revocation stream does not terminate current play"
                    );
                    assert_eq!(
                        FreshAdmissionStore::from_root(h.root.clone())
                            .current_session(session)
                            .await?
                            .session_state(),
                        GameSessionState::Active
                    );

                    assert_eq!(
                        owner
                            .lose_control(current, std::time::Duration::ZERO, None)
                            .await,
                        crate::gameplay_transport::connection::ControlLossResult::Recorded
                    );
                    sqlx::query("UPDATE preadm_clock.instant SET value=$1")
                        .bind(now - 31)
                        .execute(&h.pool)
                        .await?;
                    assert_eq!(
                        h.root
                            .apply_platform_stream_page(
                                None,
                                StreamPage::SecurityRevocations(Vec::new()),
                                None
                            )
                            .await
                            .map_err(debug)?,
                        PageOutcome::Advanced(PageCounts::default())
                    );
                    sqlx::query("UPDATE preadm_clock.instant SET value=$1")
                        .bind(now)
                        .execute(&h.pool)
                        .await?;
                    assert!(
                        matches!(
                            owner.resume(attempt()?).await,
                            Err(AdmissionRefusal::Rejected)
                        ),
                        "31-second-old read"
                    );
                    // A stream outage does not change the lost session or revoke current play.
                    assert_eq!(
                        FreshAdmissionStore::from_root(h.root.clone())
                            .current_session(session)
                            .await?
                            .session_state(),
                        GameSessionState::Reconnectable
                    );
                    sqlx::query("UPDATE preadm_clock.instant SET value=$1")
                        .bind(now - 29)
                        .execute(&h.pool)
                        .await?;
                    assert_eq!(
                        h.root
                            .apply_platform_stream_page(
                                None,
                                StreamPage::SecurityRevocations(Vec::new()),
                                None
                            )
                            .await
                            .map_err(debug)?,
                        PageOutcome::Advanced(PageCounts::default())
                    );
                    sqlx::query("UPDATE preadm_clock.instant SET value=$1")
                        .bind(now)
                        .execute(&h.pool)
                        .await?;
                    let resumed = owner
                        .resume(attempt()?)
                        .await
                        .map_err(|e| format!("29-second fresh reconnect: {e:?}"))?;
                    assert_eq!(resumed.game_session_id, session);
                    assert_eq!(
                        FreshAdmissionStore::from_root(h.root.clone())
                            .current_session(session)
                            .await?
                            .session_state(),
                        GameSessionState::Active
                    );
                    Ok(())
                }
                .await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|e| e.to_string());
            let _ = send.send(result);
        })?;
    receive
        .recv_timeout(std::time::Duration::from_secs(120))
        .map_err(|e| format!("revocation freshness watchdog (setup/loss/resume/cleanup): {e}"))?
        .map_err(Into::into)
}
fn fixed_recovery_token(now: i64, nonce: u8) -> TestResult<String> {
    use base64::Engine as _;
    use ed25519_dalek::Signer as _;
    let token = super::house_transfer::RecoverySource::token(nonce)?;
    let mut parts = token.split('.');
    let header = parts.next().ok_or("header")?;
    let payload = parts.next().ok_or("payload")?;
    let codec = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut claims: serde_json::Value = serde_json::from_slice(&codec.decode(payload)?)?;
    claims["iat"] = now.into();
    claims["nbf"] = now.into();
    claims["exp"] = (now + 30).into();
    let input = format!("{header}.{}", codec.encode(serde_json::to_vec(&claims)?));
    let signature = ed25519_dalek::SigningKey::from_bytes(&[23; 32]).sign(input.as_bytes());
    Ok(format!("{input}.{}", codec.encode(signature.to_bytes())))
}
