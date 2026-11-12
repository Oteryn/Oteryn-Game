//! Contract protocol 1 through shipping Game transport and portal-rs TLS routes.
#[path = "support/device_capture.rs"]
mod device_capture;
#[path = "support/devices.rs"]
mod devices;
mod support;
use devices::*;
use oteryn_client::{device_login::DeviceLogin, play::MiscellaneousPreferences};
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::{
    device_sessions::{DeviceSession, now},
    native_login::{DeviceRefusal, NativeLoginError},
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
    let (mut capture, client) = device_capture::Capture::start(&t).await?;
    let http = http(&t)?;
    let token = t.oauth(0).await?;
    let before_enrollment = device_state(&t)?;
    let enrolled = client
        .enroll_device(&token, true, None, CancellationToken::new())
        .await;
    let enrollment_proof = capture.request().await?;
    enrollment_proof.require(true)?;
    let first = match enrolled {
        Ok(session) => session,
        Err(NativeLoginError::DeviceRefused(refusal)) => {
            blocked(&t, true, refusal, &before_enrollment, &enrollment_proof)?;
            return Ok(());
        }
        Err(_) => return Err("enrollment_transport_or_payload_failed".into()),
    };
    persisted(&t, &first, &t.input.accounts[0].account_id)?;
    // An empty snapshot for a fresh account is published through the real LCFA
    // mTLS handler. No projection, ticket or family rows are inserted by tests.
    empty_projection(&t, &t.input.accounts[0].account_id).await?;
    capture.expect_device(&first.device_credential)?;
    let before_rotation = device_state(&t)?;
    let rotated = client
        .device_characters(&first, CancellationToken::new())
        .await;
    let rotation_proof = capture.request().await?;
    rotation_proof.require(false)?;
    let (second, characters) = match rotated {
        Ok(value) => value,
        Err(NativeLoginError::DeviceRefused(refusal)) => {
            blocked(&t, false, refusal, &before_rotation, &rotation_proof)?;
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
            Err(NativeLoginError::DeviceRefused(DeviceRefusal {
                status: 401,
                error: "device_authorization_unavailable"
            }))
        ),
        "wrong_client_accepted",
    )?;
    persisted(&t, &third, &t.input.accounts[0].account_id)?;
    let (status, body) = post(&http, origin, "/tickets", &format!("OterynDevice {}", third.device_credential.expose()), json!({"protocol_version":1,"client_id":t.input.client,"account_id":t.input.accounts[1].account_id})).await?;
    check(
        status == 400 && body == json!({"status":400,"error":"Invalid request"}),
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
            Err(NativeLoginError::DeviceRefused(DeviceRefusal {
                status: 401,
                error: "device_authorization_unavailable"
            }))
        ),
        "old_credential_replayed",
    )?;
    revoked(&t, &fourth)?;
    check(
        matches!(
            client
                .device_ticket(&fourth, CancellationToken::new())
                .await,
            Err(NativeLoginError::DeviceRefused(DeviceRefusal {
                status: 401,
                error: "device_authorization_unavailable"
            }))
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
            Err(NativeLoginError::DeviceRefused(DeviceRefusal {
                status: 401,
                error: "device_authorization_unavailable"
            }))
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
            Err(NativeLoginError::DeviceRefused(DeviceRefusal {
                status: 401,
                error: "device_authorization_unavailable"
            }))
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
            Err(NativeLoginError::DeviceRefused(DeviceRefusal {
                status: 503,
                error: "Temporarily unavailable"
            }))
        ),
        "disabled_oauth_accepted",
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
        enrollment_gap(400, &json!({"status":400,"error":"Invalid request"})),
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

#[test]
fn protocol_blocker_rejects_every_state_change() -> TestResult {
    for (enrollment, refusal) in [
        (
            true,
            DeviceRefusal {
                status: 400,
                error: "Invalid request",
            },
        ),
        (
            false,
            DeviceRefusal {
                status: 401,
                error: "device_authorization_unavailable",
            },
        ),
    ] {
        check(
            contract_gap(enrollment, refusal, "unchanged", "unchanged"),
            "valid_signature_refused",
        )?;
        // Includes the review's case: revoked family, unconsumed credential.
        check(
            !contract_gap(enrollment, refusal, "unchanged", "changed"),
            "mutated_family_marked_blocked",
        )?;
        for status in [400, 401, 429, 503] {
            if status != refusal.status {
                check(
                    !contract_gap(
                        enrollment,
                        DeviceRefusal { status, ..refusal },
                        "unchanged",
                        "unchanged",
                    ),
                    "wrong_status_marked_blocked",
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn blocker_requires_byte_exact_shipping_request() -> TestResult {
    use device_capture::{RequestMeta, contract_request};
    use oteryn_platform_client::device_sessions::DeviceCredential;
    let client = "019a5f4a-1000-7000-8000-000000000001";
    let base = "/api/v1/game-auth/device-sessions";
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/contracts/fixtures/remembered-device-v1/enroll-response.json"
    )))?;
    let issued = DeviceCredential::parse(text(&fixture, "device_credential")?)
        .map_err(|_| "credential_fixture_invalid")?;
    let authorization = format!("OterynDevice {}", issued.expose());
    for (enrollment, path, authorization, body) in [
        (
            true,
            base.to_owned(),
            "Bearer transport-test",
            br#"{"protocol_version":1,"remember_device":true}"#.as_slice(),
        ),
        (
            false,
            format!("{base}/native-characters"),
            authorization.as_str(),
            br#"{"protocol_version":1,"client_id":"019a5f4a-1000-7000-8000-000000000001"}"#,
        ),
    ] {
        let meta = RequestMeta {
            method: "POST",
            path: &path,
            content_type: "application/json",
            content_encoding: None,
            authorization,
        };
        check(
            contract_request(&meta, body, enrollment, client, Some(&issued))?,
            "contract_request_refused",
        )?;
        for malformed in [
            br#"{"protocol_version":1,"remember_devic":true}"#.as_slice(),
            br#"{"protocol_version":1,"remember_device":"true"}"#,
            br#"{"protocol_version":1,"remember_device":true"#,
            br#"{"protocol_version":1,"clien_id":"019a5f4a-1000-7000-8000-000000000001"}"#,
            br#"{"protocol_version":1,"client_id":7}"#,
            br#"{"client_id":"019a5f4a-1000-7000-8000-000000000001","protocol_version":1}"#,
            br#"{"protocol_version":1,"client_id":"#,
        ] {
            check(
                !contract_request(&meta, malformed, enrollment, client, Some(&issued))?,
                "malformed_request_marked_blocked",
            )?;
        }
        for bad in [
            RequestMeta {
                method: "GET",
                ..meta
            },
            RequestMeta {
                content_type: "text/plain",
                ..meta
            },
            RequestMeta {
                content_type: "application/json; charset=utf-8",
                ..meta
            },
            RequestMeta {
                content_encoding: Some("gzip"),
                ..meta
            },
            RequestMeta {
                authorization: "Basic invalid",
                ..meta
            },
            RequestMeta {
                path: "/wrong",
                ..meta
            },
        ] {
            check(
                !contract_request(&bad, body, enrollment, client, Some(&issued))?,
                "wrong_request_metadata_marked_blocked",
            )?;
        }
        if !enrollment {
            let malformed = RequestMeta {
                authorization: "OterynDevice malformed",
                ..meta
            };
            check(
                !contract_request(&malformed, body, false, client, Some(&issued))?,
                "malformed_credential_marked_blocked",
            )?;
            check(
                !contract_request(&meta, body, false, client, None)?,
                "missing_issued_credential_marked_blocked",
            )?;
            let mut other = issued.expose().as_bytes().to_vec();
            other[5] = if other[5] == b'A' { b'B' } else { b'A' };
            let other = DeviceCredential::parse(std::str::from_utf8(&other)?)
                .map_err(|_| "credential_mutation_invalid")?;
            check(
                !contract_request(&meta, body, false, client, Some(&other))?,
                "wrong_issued_credential_marked_blocked",
            )?;
        }
    }
    Ok(())
}
