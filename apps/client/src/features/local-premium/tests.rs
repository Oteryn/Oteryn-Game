use super::*;
use crate::account_services::test_server::{
    MockPlatform, TestResult, premium::snapshot, test_thread,
};

#[test]
fn premium_panel_fetches_status_features_expiry_and_handles_all_entitlement_states() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let mut view = View::default();
        for (revision, state) in [
            PremiumState::Active,
            PremiumState::Expired,
            PremiumState::Revoked,
            PremiumState::None,
        ]
        .into_iter()
        .enumerate()
        {
            let mut status = snapshot(state);
            status.authority_revision = revision as u64 + 1;
            mock.authority.lock().map_err(|_| "lock")?.premium_reply =
                Some(serde_json::to_value(status)?);
            mock.runtime.block_on(refresh(
                &mut view,
                &client,
                &token,
                CancellationToken::new(),
            ));
            assert!(view.fresh());
            assert_eq!(view.status.as_ref().map(|s| s.state), Some(state));
            let text = view.status_text();
            assert!(text.contains("Private channels"));
            assert!(text.contains("Create a private conversation"));
            assert!(text.contains(if state == PremiumState::None {
                "No Premium expiry"
            } else {
                "Expiry:"
            }));
            assert!(text.contains(if state == PremiumState::Active {
                "[available]"
            } else {
                "[not confirmed available]"
            }));
        }
        let log = &mock.authority.lock().map_err(|_| "lock")?.requests;
        assert_eq!(
            log.iter()
                .filter(|r| r.starts_with("GET /api/v1/native-account/premium "))
                .count(),
            4
        );
        assert!(
            !log.iter()
                .any(|r| r.starts_with("POST /api/v1/native-account/premium"))
        );
        Ok(())
    })
}

#[test]
fn premium_panel_rejects_malformed_and_regressing_snapshots_without_confirming_cached_features()
-> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let mut view = View::default();
        mock.runtime.block_on(refresh(
            &mut view,
            &client,
            &token,
            CancellationToken::new(),
        ));
        let original = view.status.clone();
        let valid = serde_json::to_value(snapshot(PremiumState::Active))?;
        let mut missing = valid.clone();
        missing
            .as_object_mut()
            .ok_or("object")?
            .remove("expires_at_unix");
        let mut duplicate = valid.clone();
        duplicate["features"]
            .as_array_mut()
            .ok_or("array")?
            .push(valid["features"][0].clone());
        let mut inactive = valid.clone();
        inactive["state"] = serde_json::json!("none");
        inactive["expires_at_unix"] = serde_json::Value::Null;
        for bad in [
            missing,
            duplicate,
            inactive,
            serde_json::json!({"version":2}),
        ] {
            mock.authority.lock().map_err(|_| "lock")?.premium_reply = Some(bad);
            mock.runtime.block_on(refresh(
                &mut view,
                &client,
                &token,
                CancellationToken::new(),
            ));
            assert!(!view.fresh());
            assert_eq!(view.status, original);
            assert!(!view.status_text().contains("[available]"));
        }
        let mut changed = snapshot(PremiumState::Active);
        changed.features[0].title = "Conflicting same revision".into();
        mock.authority.lock().map_err(|_| "lock")?.premium_reply =
            Some(serde_json::to_value(changed)?);
        mock.runtime.block_on(refresh(
            &mut view,
            &client,
            &token,
            CancellationToken::new(),
        ));
        assert!(!view.fresh());
        assert_eq!(view.status, original);
        assert!(view.feedback.contains("Inconsistent"));
        view.fresh_until = Some(Instant::now() + Duration::from_secs(60));
        view.requested_at = Instant::now().checked_sub(Duration::from_secs(86401));
        assert!(view.status_text().contains("period has ended"));
        assert!(!view.status_text().contains("[available]"));
        Ok(())
    })
}
