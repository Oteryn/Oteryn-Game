use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture(name: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(format!(
        "{}/../../../docs/contracts/fixtures/profile-b-authority-v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
}
#[test]
fn profile_b_fixtures_round_trip_byte_exact() -> TestResult {
    for name in [
        "authority-request",
        "authority",
        "changes-request",
        "changes-page",
        "changes-empty",
    ] {
        let raw = fixture(name)?;
        let value = Message::parse(&raw).ok_or("fixture rejected")?;
        assert_eq!(value.encode().ok_or("encode")?, raw);
    }
    Ok(())
}
fn answer() -> Result<Answer, Box<dyn std::error::Error>> {
    match Message::parse(&fixture("authority")?).ok_or("answer")? {
        Message::Answer(a) => Ok(a),
        _ => Err("answer type".into()),
    }
}
#[test]
fn profile_b_rejects_malformed_bounds_and_lease_over_3600() -> TestResult {
    let a = answer()?;
    let mut cases = Vec::new();
    let mut fractional = a.clone();
    fractional.entitlements[0].effective_from = Some("2023-11-14T00:00:00.5Z".into());
    cases.push(fractional);
    let mut b = a.clone();
    b.authority_revision = 0;
    cases.push(b);
    let mut b = a.clone();
    b.authority_revision = INT53_MAX + 1;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].product_version = 0;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].lifecycle_revision = INT53_MAX + 1;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].authority_valid_until = "2023-11-14T23:13:21Z".into();
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].refresh_after = b.entitlements[0].authority_valid_until.clone();
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].entitlement_id = None;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[1].effective_from = Some(a.authority_issued_at.clone());
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].lease_policy = "other.v1".into();
    cases.push(b);
    let mut b = a.clone();
    b.entitlements.push(b.entitlements[0].clone());
    cases.push(b);
    let mut b = a.clone();
    b.account_id = "019a0000-0000-4000-8000-000000000002".into();
    cases.push(b);
    for b in cases {
        assert!(Message::parse(&serde_json::to_vec(&b)?).is_none());
    }
    let raw = String::from_utf8(fixture("authority")?)?;
    for invalid in [
        raw.replacen("\"authority_revision\":7", "\"authority_revision\":7.0", 1),
        raw.replacen("\"authority_revision\":7", "\"authority_revision\":7e0", 1),
        raw.replacen("\"authority_revision\":7", "\"authority_revision\":true", 1),
        raw.replacen("\"authority_revision\":7,", "", 1),
        raw.replacen(
            "\"authority_revision\":7",
            "\"authority_revision\":7,\"authority_revision\":7",
            1,
        ),
        raw.replacen(
            "\"authority_revision\":7",
            "\"authority_revision\":7,\"unknown\":1",
            1,
        ),
        raw.replacen("\"effective_until\":null,", "", 1),
        format!("{raw}\n"),
        format!("{raw}{}", " ".repeat(AUTHORITY_BYTES)),
    ] {
        assert!(Message::parse(invalid.as_bytes()).is_none());
    }
    Ok(())
}
#[test]
fn profile_b_lease_failure_matrix_and_boundaries() -> TestResult {
    let a = answer()?;
    let e = &a.entitlements[0];
    let issued = time(&a.authority_issued_at).ok_or("time")?;
    let at = |seconds: i64, uncertainty: i64| {
        Some(GameNow {
            now_us: issued + seconds * 1_000_000,
            uncertainty_us: uncertainty,
        })
    };
    // Unavailability never refreshes a lease: outages of 30/40/60 minutes age these same facts.
    for (seconds, expected) in [
        (0, Evaluation::Active),
        (1800, Evaluation::Active),
        (2399, Evaluation::Active),
        (2400, Evaluation::Stale),
        (3599, Evaluation::Stale),
        (3600, Evaluation::Expired),
        (7200, Evaluation::Expired),
    ] {
        assert_eq!(evaluate(e, at(seconds, 0), 0), expected);
    }
    assert_eq!(evaluate(e, at(2399, 5_000_000), 0), Evaluation::Stale);
    assert_eq!(evaluate(e, at(1, 5_000_001), 0), Evaluation::None);
    assert_eq!(evaluate(e, None, 0), Evaluation::None);
    assert_eq!(
        evaluate(e, at(1, 0), e.lifecycle_revision + 1),
        Evaluation::Stale
    );
    assert_eq!(evaluate(&a.entitlements[1], at(1, 0), 4), Evaluation::None);
    let mut revoked = e.clone();
    revoked.entitlement_state = CommercialState::Revoked;
    assert_eq!(evaluate(&revoked, at(1, 0), 0), Evaluation::None);
    let mut future = e.clone();
    future.effective_from = Some("2023-11-14T22:14:20Z".into());
    future.entitlement_state = CommercialState::NotYetEffective;
    assert_eq!(evaluate(&future, at(64, 5_000_000), 0), Evaluation::None);
    assert_eq!(evaluate(&future, at(65, 5_000_000), 0), Evaluation::Active);
    let mut clipped = a.clone();
    let item = &mut clipped.entitlements[0];
    item.effective_until = Some("2023-11-14T22:23:20Z".into());
    item.authority_valid_until = "2023-11-14T22:23:20Z".into();
    item.refresh_after = "2023-11-14T22:20:00Z".into();
    assert!(clipped.valid());
    assert!(
        time(&clipped.entitlements[0].refresh_after)
            < time(&clipped.entitlements[0].authority_valid_until)
    );
    assert_eq!(
        evaluate(&clipped.entitlements[0], at(600, 0), 0),
        Evaluation::Expired
    );
    let mut active_end = e.clone();
    active_end.effective_until = Some(a.authority_issued_at.clone());
    let mut invalid_active = a.clone();
    invalid_active.entitlements[0] = active_end.clone();
    assert!(!invalid_active.valid());
    assert_eq!(evaluate(&active_end, at(0, 0), 0), Evaluation::Expired);
    let mut ended = a.clone();
    ended.entitlements[0].effective_until = Some(a.authority_issued_at.clone());
    ended.entitlements[0].entitlement_state = CommercialState::Expired;
    assert!(ended.valid());
    assert_eq!(
        evaluate(&ended.entitlements[0], at(0, 0), 0),
        Evaluation::Expired
    );
    let mut open = a.clone();
    open.entitlements[0].effective_until = None;
    assert!(open.valid());
    open.entitlements[0].authority_valid_until = "2023-11-14T23:03:20Z".into();
    open.entitlements[0].refresh_after = "2023-11-14T22:46:40Z".into();
    assert!(open.valid(), "shortened open-end lease");
    assert_eq!(
        time(&open.entitlements[0].authority_valid_until).ok_or("time")? - issued,
        3_000_000_000
    );
    assert_eq!(
        evaluate(&open.entitlements[0], at(3000, 0), 0),
        Evaluation::Expired
    );
    open.entitlements[0].authority_valid_until = "2023-11-14T23:13:21Z".into();
    open.entitlements[0].refresh_after = "2023-11-14T22:53:20Z".into();
    assert!(
        !open.valid(),
        "no configuration can authorize an overlong lease"
    );
    open.entitlements[0].authority_valid_until = a.entitlements[0].authority_valid_until.clone();
    assert_eq!(
        time(&open.entitlements[0].authority_valid_until).ok_or("time")? - issued,
        MAX_LEASE_US
    );
    Ok(())
}

#[test]
fn profile_b_request_binding_and_closed_pages() -> TestResult {
    let a = answer()?;
    let Some(Message::Request(r)) = Message::parse(&fixture("authority-request")?) else {
        return Err("request".into());
    };
    let products = a
        .entitlements
        .iter()
        .map(|e| (e.product_id.clone(), e.product_version))
        .collect::<Vec<_>>();
    assert!(a.bound_to(&r, &products));
    let mut wrong_schema = r.clone();
    wrong_schema.schema = "other.v1".into();
    assert!(!a.bound_to(&wrong_schema, &products));
    let mut wrong = r.clone();
    wrong.nonce = "0".repeat(32);
    assert!(!a.bound_to(&wrong, &products));
    let mut wrong = r.clone();
    wrong.account_id = "019a0000-0000-7000-8000-000000000003".into();
    assert!(!a.bound_to(&wrong, &products));
    let mut reversed = products.clone();
    reversed.reverse();
    assert!(!a.bound_to(&r, &reversed));
    let mut many = a.clone();
    many.entitlements.clear();
    for i in 0..17 {
        let mut e = a.entitlements[0].clone();
        e.product_id = format!("oteryn.test_{i}");
        e.entitlement_id = Some(format!("019a0000-0000-7000-8000-{i:012}"));
        many.entitlements.push(e);
        if i == 15 {
            assert!(many.valid());
        }
    }
    assert!(Message::parse(&serde_json::to_vec(&many)?).is_none());
    let oversized = format!(
        "{}{}",
        String::from_utf8(fixture("authority")?)?,
        " ".repeat(AUTHORITY_BYTES)
    );
    let decoded: Answer = serde_json::from_str(&oversized)?;
    assert!(decoded.valid());
    assert_eq!(Message::decode(oversized.as_bytes()), Err(ParseError::Size));
    let noncanonical = format!("{} ", String::from_utf8(fixture("authority")?)?);
    assert_eq!(
        Message::decode(noncanonical.as_bytes()),
        Err(ParseError::NonCanonical)
    );
    let Some(Message::ChangesRequest(request)) = Message::parse(&fixture("changes-request")?)
    else {
        return Err("request".into());
    };
    for bad in ["v1:0", "v1:01"] {
        let mut r = request.clone();
        r.after = Some(bad.into());
        assert!(Message::parse(&serde_json::to_vec(&r)?).is_none());
    }
    for limit in [0, 501] {
        let mut r = request.clone();
        r.limit = limit;
        assert!(Message::parse(&serde_json::to_vec(&r)?).is_none());
    }
    let Some(Message::Page(mut page)) = Message::parse(&fixture("changes-page")?) else {
        return Err("page".into());
    };
    page.next_cursor = None;
    assert!(Message::parse(&serde_json::to_vec(&page)?).is_none());
    for name in ["authority", "changes-page"] {
        let value: serde_json::Value = serde_json::from_slice(&fixture(name)?)?;
        assert!(
            Message::parse(&serde_json::to_vec(&value)?).is_none(),
            "sorted member order must be rejected"
        );
    }
    for which in 0..5 {
        let mut b = a.clone();
        let fractional = |s: &str| s.replace('Z', ".5Z");
        match which {
            0 => b.authority_issued_at = fractional(&b.authority_issued_at),
            1 => {
                b.entitlements[0].effective_from =
                    b.entitlements[0].effective_from.as_deref().map(fractional)
            }
            2 => {
                b.entitlements[0].effective_until =
                    b.entitlements[0].effective_until.as_deref().map(fractional)
            }
            3 => {
                b.entitlements[0].authority_valid_until =
                    fractional(&b.entitlements[0].authority_valid_until)
            }
            _ => b.entitlements[0].refresh_after = fractional(&b.entitlements[0].refresh_after),
        }
        assert!(Message::parse(&serde_json::to_vec(&b)?).is_none());
    }
    Ok(())
}

fn run(future: impl std::future::Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future)
}
type Captured = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;
async fn peer(
    body: Vec<u8>,
    status: u16,
    retry: Option<u64>,
    authority: bool,
) -> Result<(transport::Client, Captured), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let client =
        transport::Client::local(&format!("http://{}", listener.local_addr()?), authority)?;
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output = captured.clone();
    tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else {
            return;
        };
        let mut raw = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let Ok(n) = stream.read(&mut chunk).await else {
                return;
            };
            if n == 0 {
                return;
            }
            raw.extend_from_slice(&chunk[..n]);
            if let Some(end) = raw.windows(4).position(|v| v == b"\r\n\r\n") {
                let length = String::from_utf8_lossy(&raw[..end])
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if raw.len() >= end + 4 + length {
                    break;
                }
            }
        }
        if let Ok(mut out) = output.lock() {
            *out = raw;
        }
        let retry = retry
            .map(|n| format!("Retry-After: {n}\r\n"))
            .unwrap_or_default();
        let header = format!(
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{retry}Connection: close\r\n\r\n",
            body.len()
        );
        if stream.write_all(header.as_bytes()).await.is_ok() {
            let _ = stream.write_all(&body).await;
        }
    });
    Ok((client, captured))
}
#[test]
fn profile_b_clients_require_expected_variant_and_requested_page_limit() -> TestResult {
    run(async {
        let a = answer()?;
        let products = a
            .entitlements
            .iter()
            .map(|e| (e.product_id.clone(), e.product_version))
            .collect::<Vec<_>>();
        let Some(Message::Request(request)) = Message::parse(&fixture("authority-request")?) else {
            return Err("request".into());
        };
        let (client, captured) = peer(fixture("authority")?, 200, None, true).await?;
        assert_eq!(
            client
                .read_authority(&request, &products)
                .await
                .map_err(|e| format!("{e:?}"))?,
            fixture("authority")?
        );
        {
            let captured = captured.lock().map_err(|_| "mutex")?;
            assert!(
                captured
                    .starts_with(format!("POST {} HTTP/1.1", transport::AUTHORITY_PATH).as_bytes())
            );
            assert!(captured.ends_with(&fixture("authority-request")?));
        }
        let (client, captured) = peer(fixture("changes-page")?, 200, None, false).await?;
        assert_eq!(
            client
                .read_changes(None, 200)
                .await
                .map_err(|e| format!("{e:?}"))?,
            fixture("changes-page")?
        );
        assert!(
            captured
                .lock()
                .map_err(|_| "mutex")?
                .ends_with(&fixture("changes-request")?)
        );
        let (client, _) = peer(fixture("changes-page")?, 200, None, true).await?;
        assert_eq!(
            client.read_authority(&request, &products).await,
            Err(crate::platform_streams::enforcement::transport::Failure::Defect)
        );
        let (client, _) = peer(fixture("authority")?, 200, None, false).await?;
        assert_eq!(
            client.read_changes(None, 200).await,
            Err(crate::platform_streams::enforcement::transport::Failure::Defect)
        );
        let (client, _) = peer(fixture("changes-page")?, 200, None, false).await?;
        assert_eq!(
            client.read_changes(None, 1).await,
            Err(crate::platform_streams::enforcement::transport::Failure::Defect)
        );
        Ok(())
    })
}
#[test]
fn profile_b_transport_failure_statuses_and_backoff() -> TestResult {
    use super::runtime::retry_delay as backoff;
    use crate::platform_streams::enforcement::transport::Failure;
    run(async {
        for (status, retry, expected) in [
            (400, None, Failure::Defect),
            (401, None, Failure::Configuration),
            (404, None, Failure::Missing),
            (
                429,
                Some(90),
                Failure::Retry(Some(std::time::Duration::from_secs(60))),
            ),
            (503, None, Failure::Retry(None)),
        ] {
            let (client, _) = peer(vec![], status, retry, false).await?;
            assert_eq!(client.read_changes(None, 200).await, Err(expected));
        }
        assert_eq!(backoff(0, None).as_secs(), 1);
        assert_eq!(backoff(8, None).as_secs(), 60);
        assert_eq!(
            backoff(0, Some(std::time::Duration::from_secs(90))).as_secs(),
            60
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let client =
            transport::Client::local(&format!("http://{}", listener.local_addr()?), false)?;
        drop(listener);
        assert_eq!(
            client.read_changes(None, 200).await,
            Err(Failure::Retry(None)),
            "transport failure is no fresh authority"
        );
        Ok(())
    })
}
#[test]
fn profile_b_config_default_off_strict_flags_and_products() -> TestResult {
    use crate::premium::client::ClientConfigError;
    use transport::*;
    assert!(Clients::from_lookup(|_| None)?.is_none());
    for flag in [AUTHORITY_ENABLED_VAR, CHANGES_ENABLED_VAR] {
        for invalid in ["false", "TRUE", ""] {
            assert!(
                matches!(Clients::from_lookup(|key|(key==flag).then(||invalid.into())),Err(ClientConfigError::Invalid(k)) if k==flag)
            );
        }
        let own = if flag == AUTHORITY_ENABLED_VAR {
            AUTHORITY_IDENTITY_VAR
        } else {
            CHANGES_IDENTITY_VAR
        };
        assert!(
            matches!(Clients::from_lookup(|key|(key==flag).then(||"true".into())),Err(ClientConfigError::Invalid(k)) if k==own)
        );
    }
    assert!(products("[]")?.is_empty());
    for invalid in [
        "[\"oteryn.premium_time@1\"]",
        "[\"oteryn.example_active@01\"]",
        "[\"oteryn.example_active@0\"]",
        "[\"oteryn.example_active@9007199254740992\"]",
        "[\"oteryn.example_active@1\",\"oteryn.example_active@1\"]",
    ] {
        assert!(products(invalid).is_err());
    }
    let values = (0..17)
        .map(|i| format!("oteryn.test_{i}@1"))
        .collect::<Vec<_>>();
    assert!(products(&serde_json::to_string(&values)?).is_err());
    let dir = std::env::temp_dir().join(format!("p4c-identities-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let first = dir.join("first.pem");
    let second = dir.join("second.pem");
    std::fs::write(
        &first,
        "-----BEGIN CERTIFICATE-----\nYQ==\n-----END CERTIFICATE-----\n",
    )?;
    std::fs::write(
        &second,
        "-----BEGIN CERTIFICATE-----\nYg==\n-----END CERTIFICATE-----\n",
    )?;
    for other in [
        CHANGES_IDENTITY_VAR,
        crate::platform_streams::enforcement::transport::RESULTS_IDENTITY_VAR,
        crate::platform_streams::SECURITY_REVOCATION_IDENTITY_VAR,
    ] {
        let values = std::collections::HashMap::from([
            (AUTHORITY_ENABLED_VAR, std::ffi::OsString::from("true")),
            (CHANGES_ENABLED_VAR, std::ffi::OsString::from("true")),
            (AUTHORITY_IDENTITY_VAR, first.clone().into_os_string()),
            (
                CHANGES_IDENTITY_VAR,
                if other == CHANGES_IDENTITY_VAR {
                    first.clone()
                } else {
                    second.clone()
                }
                .into_os_string(),
            ),
            (other, first.clone().into_os_string()),
        ]);
        let expected = if other == CHANGES_IDENTITY_VAR {
            CHANGES_IDENTITY_VAR
        } else {
            AUTHORITY_IDENTITY_VAR
        };
        assert!(
            matches!(Clients::from_lookup(|key|values.get(key).cloned()),Err(ClientConfigError::Invalid(k)) if k==expected),
            "purpose identities overlap"
        );
    }
    let values = std::collections::HashMap::from([
        (CHANGES_ENABLED_VAR, std::ffi::OsString::from("true")),
        (AUTHORITY_IDENTITY_VAR, first.clone().into_os_string()),
        (CHANGES_IDENTITY_VAR, first.clone().into_os_string()),
    ]);
    assert!(
        matches!(
            Clients::from_lookup(|key| values.get(key).cloned()),
            Err(ClientConfigError::Invalid(CHANGES_IDENTITY_VAR))
        ),
        "disabled B1 identity still participates"
    );
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
