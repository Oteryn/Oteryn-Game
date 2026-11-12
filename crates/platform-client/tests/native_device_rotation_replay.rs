//! Contract protocol 1 through shipping Game transport and portal-rs TLS routes.
#[path = "support/devices.rs"]
mod devices;
mod support;
use devices::*;
use oteryn_client::{device_login::DeviceLogin, play::MiscellaneousPreferences};
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::{
    device_sessions::{DeviceSession, now},
    native_login::NativeLoginError,
};
use serde_json::json;
use support::*;

fn successor(previous: &DeviceSession, next: &DeviceSession) -> TestResult {
    previous
        .successor(next, now().map_err(|_| "clock_failed")?)
        .map_err(|_| "invalid_rotation")?;
    check(
        !format!("{next:?}").contains(next.device_credential.expose()),
        "credential_debug_leaked",
    )?;
    let restored = DeviceSession::from_vault(
        &next.vault_bytes().map_err(|_| "vault_serialize_failed")?,
        now().map_err(|_| "clock_failed")?,
    )
    .map_err(|_| "vault_restore_failed")?;
    check(
        restored.device_credential == next.device_credential
            && restored.family_id == next.family_id
            && restored.absolute_expires_at == next.absolute_expires_at,
        "vault_round_trip_failed",
    )
}
async fn journey(t: Topology) -> TestResult {
    t.no_admissions()?;
    let origin = &t.input.native_origin;
    let client = native(&t, origin, &t.input.client)?;
    let http = http(&t)?;
    let token = t.oauth(0).await?;
    let enrolled = client
        .enroll_device(&token, true, None, CancellationToken::new())
        .await;
    let first = match enrolled {
        Ok(session) => session,
        Err(NativeLoginError::TokenRefused) => {
            // This is a second real OAuth journey, not a manufactured token or
            // injected success response. Only exact live HTTP refusal is BLOCKED.
            let bearer = t.bearer(0).await?;
            let (status, body) = post(
                &http,
                origin,
                "",
                &format!("Bearer {bearer}"),
                json!({"protocol_version":1,"remember_device":true}),
            )
            .await?;
            check(
                enrollment_gap(status, &body),
                "unexpected_enrollment_refusal",
            )?;
            check(
                sql(
                    &t.database,
                    "no_enrolled_families",
                    "SELECT count(*) FROM device_sessions;",
                )? == "0",
                "refused_enrollment_created_family",
            )?;
            t.no_admissions()?;
            gaps(true)?;
            return Ok(());
        }
        Err(_) => return Err("enrollment_transport_or_payload_failed".into()),
    };
    persisted(&t, &first, &t.input.accounts[0].account_id)?;
    // An empty snapshot for a fresh account is published through the real LCFA
    // mTLS handler. No projection, ticket or family rows are inserted by tests.
    empty_projection(&t, &t.input.accounts[0].account_id).await?;
    let rotated = client
        .device_characters(&first, CancellationToken::new())
        .await;
    let (second, characters) = match rotated {
        Ok(value) => value,
        Err(NativeLoginError::TokenRefused) => {
            let (status, body) = post(
                &http,
                origin,
                "/native-characters",
                &format!("OterynDevice {}", first.device_credential.expose()),
                json!({"protocol_version":1,"client_id":t.input.client}),
            )
            .await?;
            check(rotation_gap(status, &body), "unexpected_rotation_refusal")?;
            check(
                sql(
                    &t.database,
                    "rotation_not_consumed",
                    &format!(
                        "SELECT count(*) FROM device_session_credentials WHERE family_id='{}' AND consumed_at IS NULL;",
                        first.family_id
                    ),
                )? == "1",
                "refused_rotation_consumed_credential",
            )?;
            gaps(false)?;
            return Ok(());
        }
        Err(_) => return Err("rotation_transport_or_payload_failed".into()),
    };
    check(characters.is_empty(), "empty_account_received_characters")?;
    successor(&first, &second)?;
    persisted(&t, &second, &t.input.accounts[0].account_id)?;
    let (third, _ticket) = client
        .device_ticket(&second, CancellationToken::new())
        .await
        .map_err(|_| "device_ticket_refused")?;
    successor(&second, &third)?;
    check(
        sql(
            &t.database,
            "native_ticket_owner",
            &format!(
                "SELECT count(*) FROM game_login_tickets WHERE device_family_id='{}' AND account_id='{}' AND audience='oteryn-native-game-gateway' AND used_at IS NULL;",
                third.family_id, t.input.accounts[0].account_id
            ),
        )? == "1",
        "native_ticket_wrong_account",
    )?;
    check(
        sql(
            &t.database,
            "native_ticket_cross_account",
            &format!(
                "SELECT count(*) FROM game_login_tickets WHERE device_family_id='{}' AND account_id<>'{}';",
                third.family_id, t.input.accounts[0].account_id
            ),
        )? == "0",
        "native_ticket_crossed_account",
    )?;

    // Unknown client and account override must refuse without revoking A.
    let wrong_client = oteryn_platform_client::native_login::AttemptRef::generate()
        .map_err(|_| "client_id_generation_failed")?;
    let wrong = native(&t, origin, wrong_client.as_str())?;
    check(
        matches!(
            wrong
                .device_characters(&third, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "wrong_client_accepted",
    )?;
    persisted(&t, &third, &t.input.accounts[0].account_id)?;
    let (status, body) = post(&http, origin, "/tickets", &format!("OterynDevice {}", third.device_credential.expose()), json!({"protocol_version":1,"client_id":t.input.client,"account_id":t.input.accounts[1].account_id})).await?;
    check(
        status == 400 && body == json!({"error":"invalid_request"}),
        "wrong_account_override_accepted",
    )?;
    persisted(&t, &third, &t.input.accounts[0].account_id)?;
    let (fourth, _) = client
        .device_characters(&third, CancellationToken::new())
        .await
        .map_err(|_| "negative_cases_revoked_valid_family")?;
    successor(&third, &fourth)?;

    // A known predecessor is security-terminal for the whole family, including
    // the newest successor. No auto-retry of a predecessor is performed.
    check(
        matches!(
            client
                .device_characters(&first, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "old_credential_replayed",
    )?;
    revoked(&t, &fourth)?;
    check(
        matches!(
            client
                .device_ticket(&fourth, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "replay_left_successor_usable",
    )?;

    // Independently enroll B and revoke through the real endpoint.
    let b_token = t.oauth(1).await?;
    let b = client
        .enroll_device(&b_token, true, None, CancellationToken::new())
        .await
        .map_err(|_| "account_b_enrollment_refused")?;
    persisted(&t, &b, &t.input.accounts[1].account_id)?;
    let (b_next, _ticket) = client
        .device_ticket(&b, CancellationToken::new())
        .await
        .map_err(|_| "account_b_ticket_refused")?;
    successor(&b, &b_next)?;
    check(
        sql(
            &t.database,
            "account_b_ticket",
            &format!(
                "SELECT count(*) FROM game_login_tickets WHERE device_family_id='{}' AND account_id='{}';",
                b_next.family_id, t.input.accounts[1].account_id
            ),
        )? == "1",
        "account_b_ticket_wrong_owner",
    )?;
    client
        .revoke_device(&b_next.device_credential, CancellationToken::new())
        .await
        .map_err(|_| "device_revoke_refused")?;
    revoked(&t, &b_next)?;
    check(
        matches!(
            client
                .device_characters(&b_next, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "revoked_credential_accepted",
    )?;

    let token = t.oauth(0).await?;
    let generation = client
        .enroll_device(&token, true, None, CancellationToken::new())
        .await
        .map_err(|_| "generation_enrollment_failed")?;
    generation_change(&t, &t.input.accounts[0].account_id)?;
    check(
        matches!(
            client
                .device_characters(&generation, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "generation_change_accepted",
    )?;
    revoked(&t, &generation)?;

    // Real binary, same database/TLS identity, OAuth configuration disabled.
    let disabled = Portal::start(&t, true).await?;
    let disabled_client = native(&t, &disabled.origin, &t.input.client)?;
    let token = t.oauth(0).await?;
    check(
        matches!(
            disabled_client
                .enroll_device(&token, true, None, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "disabled_oauth_accepted",
    )?;
    let bearer = t.bearer(0).await?;
    let (status, body) = post(
        &http,
        &disabled.origin,
        "",
        &format!("Bearer {bearer}"),
        json!({"protocol_version":1,"remember_device":true}),
    )
    .await?;
    check(
        status == 503 && body == json!({"error":"device_authorization_unavailable"}),
        "disabled_oauth_wrong_refusal",
    )?;
    drop(disabled);

    // Exercise shipping DeviceLogin, DeviceAttempt and its actual platform vault
    // boundary. On Linux the production vault is unavailable: session-class
    // secrets stay in process memory, exactly as the shipping client requires.
    let mut portal = Portal::start(&t, false).await?;
    let offline_client = native(&t, &portal.origin, &t.input.client)?;
    let lifecycle = DeviceLogin::configured(&portal.origin, &t.input.client, true);
    let options = MiscellaneousPreferences {
        stay_logged_in: true,
        remember_device: false,
        ..Default::default()
    };
    let mut attempt = lifecycle
        .begin(&offline_client, options, CancellationToken::new())
        .await
        .map_err(|_| "lifecycle_begin_failed")?
        .ok_or("lifecycle_disabled")?;
    let token = t.oauth(0).await?;
    attempt
        .enroll(&offline_client, &token, CancellationToken::new())
        .await
        .map_err(|_| "lifecycle_enrollment_failed")?;
    check(attempt.has_session(), "lifecycle_did_not_store_session")?;
    drop(attempt);
    portal.stop().await?;
    lifecycle.sign_out(&offline_client).await;
    let attempt = lifecycle
        .begin(&offline_client, options, CancellationToken::new())
        .await
        .map_err(|_| "offline_begin_failed")?
        .ok_or("lifecycle_disabled")?;
    check(!attempt.has_session(), "offline_signout_kept_local_session")?;
    t.no_admissions()?;
    println!(
        "NODE_BOOT_EVIDENCE flow=native_device_rotation result=pass rotations=4 replay_refused=1 signout_offline=1"
    );
    Ok(())
}
#[test]
fn native_device_rotation_replay() -> TestResult {
    run("native_device_rotation", 120, journey)
}
#[test]
fn blocker_requires_exact_http_status_and_error() -> TestResult {
    check(
        enrollment_gap(400, &json!({"error":"invalid_request"})),
        "enrollment_gap_not_detected",
    )?;
    check(
        rotation_gap(401, &json!({"error":"device_authorization_unavailable"})),
        "rotation_gap_not_detected",
    )?;
    for (status, body) in [
        (503, json!({"error":"invalid_request"})),
        (400, json!({"error":"unavailable"})),
        (400, json!({"error":"invalid_request","extra":true})),
    ] {
        check(
            !enrollment_gap(status, &body),
            "unrelated_failure_marked_blocked",
        )?;
    }
    check(
        !rotation_gap(503, &json!({"error":"device_authorization_unavailable"})),
        "disabled_service_marked_protocol_gap",
    )
}
