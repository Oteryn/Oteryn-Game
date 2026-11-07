//! NX-1: imported off-host authorization reaches the real signed staff owner.
use super::*;

#[test]
fn nx1_imported_staff_signature_preserves_bytes_and_replays_once_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "NX-1 staff PostgreSQL thread panicked")??;
    eprintln!("NX-1 staff PostgreSQL duration: {:?}", began.elapsed());
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let signing = Signing::new()?;
        let h = Harness::create_with_admission_seed(admin, "nx1_native_staff", true, false).await?;
        prepare_current_session(&h).await?;
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
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[wire::CAPABILITY_STAFF_V1],
        )
        .ok_or("staff negotiation")?;
        admitted.item_fence = Some(
            owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("current fence")?
                .item_fence(),
        );
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        crate::gameplay_transport::staff::test_clock::set(Some((now, now)));
        let request = signing.request(
            2,
            &signing.config.newest_activation_sha256,
            91,
            now,
            rules::Action::Inspect {
                character_id: hex(&id(41)),
            },
        );
        let signed = signing.sign(&canonical(&request)?, false)?;
        let mut altered = signed.clone();
        altered.canonical.insert(0, b' ');
        assert!(submit(&owner, admitted, &altered, false).await?.is_none());
        assert_eq!(counts(&h).await?, (0, 0, 0));
        let first = submit(&owner, admitted, &signed, false)
            .await?
            .ok_or("signed inspect result")?;
        let result = wire::decode_result(&first)?;
        assert_eq!(result.code, wire::StaffCode::Ok);
        assert_eq!(result.control_seq, 2);
        assert_eq!(result.disposition, 1);
        assert_eq!(result.mute_until_ms, 0);
        assert_eq!(counts(&h).await?, (1, 1, 0));
        let replay = submit(&owner, admitted, &signed, false)
            .await?
            .ok_or("replay result")?;
        assert_eq!(replay, first);
        assert_eq!(counts(&h).await?, (1, 1, 0));
        crate::gameplay_transport::staff::test_clock::set(None);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
