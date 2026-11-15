//! Slow successful Platform I/O must leave unrelated live casts authoritative.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, id, runtime};
use oteryn_gameplay_social::{
    cast::{
        Settings,
        access::CastAccessClient,
        service::{CastService, Operation, Reply},
    },
    spectate::{
        TargetFacts,
        spectate_test_support::{facts, observer},
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn s17_cast_service_renews_privacy_during_serial_platform_io() -> TestResult {
    run(0)
}

#[test]
fn s17_cast_service_applies_withdrawal_during_announcements() -> TestResult {
    run(1)
}

#[test]
fn s17_cast_service_applies_withdrawal_during_preferences() -> TestResult {
    run(2)
}

fn run(scenario: u8) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|e| e.to_string())?
                .block_on(case(admin, scenario))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "cast service I/O test thread panicked")?
        .map_err(Into::into)
}

async fn case(admin: String, scenario: u8) -> TestResult {
    let h = Harness::create(admin, &format!("s17_service_io_{scenario}"), true).await?;
    let result = async {
        let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])?;
        let certificate_pem = pem("CERTIFICATE", certified.cert.der());
        let identity = format!("{certificate_pem}{}", pem("PRIVATE KEY", &certified.signing_key.serialize_der()));
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![certified.cert.der().clone()], rustls::pki_types::PrivateKeyDer::Pkcs8(
                rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der())))?;
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = format!("https://{}/", listener.local_addr()?);
        let announce_delay = Arc::new(AtomicU64::new(0));
        let preference_delay = Arc::new(AtomicU64::new(0));
        let io_started = Arc::new(tokio::sync::Notify::new());
        let started = io_started.clone();
        let (ad, pd) = (announce_delay.clone(), preference_delay.clone());
        let server = tokio::spawn(async move {
            while let Ok((tcp, _)) = listener.accept().await {
                let (acceptor, ad, pd, started) = (acceptor.clone(), ad.clone(), pd.clone(), started.clone());
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(tcp).await else { return; };
                    let mut bytes = Vec::new();
                    let request = loop {
                        let mut buf = [0; 4096];
                        let Ok(n) = tls.read(&mut buf).await else { return; };
                        if n == 0 { return; }
                        bytes.extend_from_slice(&buf[..n]);
                        if bytes.len() > 8192 { return; }
                        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let header = String::from_utf8_lossy(&bytes[..at]);
                            let length = header.lines().find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().ok()).flatten()
                            }).unwrap_or(0);
                            if bytes.len() >= at + 4 + length {
                                break serde_json::from_slice::<serde_json::Value>(&bytes[at + 4..at + 4 + length]);
                            }
                        }
                    };
                    let Ok(request) = request else { return; };
                    let preferences = request["operation"] == "preferences";
                    let delay = if preferences { pd.load(Ordering::SeqCst) } else if request["operation"] == "announce" { ad.load(Ordering::SeqCst) } else { 0 };
                    if delay > 0 { started.notify_one(); }
                    // This is the scripted peer's response latency, not a test readiness wait.
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    let body = if preferences { r#"{"delaySeconds":0,"viewerCap":50,"chatEnabled":true,"passwordRequired":false,"accessRevision":1,"description":""}"# } else { "{}" };
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    let _ = tls.write_all(response.as_bytes()).await;
                    let _ = tls.shutdown().await;
                });
            }
        });
        let access = Arc::new(CastAccessClient::new(&endpoint, *h.node.fact().node_id().as_bytes(), identity.as_bytes(), certificate_pem.as_bytes())?);
        let service = CastService::spawn(h.root.clone(), h.node.clone(), 32, access)?;
        let mut states = Vec::new();
        for i in 1..=11 {
            let Reply::State(state) = service.apply(Operation::Start(target(i)?, Settings::default(), id(i))).await? else { return Err("missing start state".into()); };
            states.push(state);
        }
        if scenario == 0 {
        service.apply(Operation::Refresh(target(2)?)).await?;
        announce_delay.store(100, Ordering::SeqCst);
        service.apply(Operation::Update(target(1)?, states[0].revision, Settings::default())).await?;
        announce_delay.store(0, Ordering::SeqCst);
        assert!(matches!(service.apply(Operation::State(target(2)?.authority.current_game_session_id())).await, Ok(Reply::State(_))), "serial successful announcements expired an unrelated cast's privacy lease");
        service.apply(Operation::Refresh(target(2)?)).await?;
        preference_delay.store(1200, Ordering::SeqCst);
        service.apply(Operation::Start(target(12)?, Settings::default(), id(12))).await?;
        preference_delay.store(0, Ordering::SeqCst);
        assert!(matches!(service.apply(Operation::State(target(2)?.authority.current_game_session_id())).await, Ok(Reply::State(_))), "slow successful preferences expired an unrelated cast's privacy lease");
        } else {
            withdrawal_case(&h, &service, states[0], scenario,
                (&announce_delay, &preference_delay), &io_started).await?;
        }
        service.apply(Operation::Shutdown).await?;
        server.abort();
        Ok(())
    }.await;
    h.cleanup().await?;
    result
}

fn target(marker: u8) -> TestResult<TargetFacts> {
    use oteryn_gameplay_foundation::foundation::{
        ChannelId, CharacterLease, CharacterWorldEligibilityClaimV1, ConnectionGeneration,
        FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionState,
        RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
    };
    let base = facts(1, 1, 1);
    let native = FreshAdmissionFacts::new(
        [marker; 32],
        oteryn_gameplay_foundation::foundation::CharacterId::decode(
            &crate::bestiary_postgres_harness::id(marker),
        )
        .map_err(|e| format!("{e:?}"))?,
        WorldId::decode(&id(crate::bestiary_postgres_harness::WORLD))
            .map_err(|e| format!("{e:?}"))?,
        ChannelId::decode(&id(crate::bestiary_postgres_harness::CHANNEL))
            .map_err(|e| format!("{e:?}"))?,
        1,
        1,
    )
    .map_err(|e| format!("{e:?}"))?;
    let transport = base
        .authority
        .current_transport()
        .ok_or("missing transport")?;
    let commit = FreshAdmissionCommit::from_facts(observer(marker).session, native, transport)
        .map_err(|e| format!("{e:?}"))?;
    let authority = GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Active,
        ConnectionGeneration::new(1).map_err(|e| format!("{e:?}"))?,
        Some(transport),
        CharacterLease::new(commit.character_id(), 1).map_err(|e| format!("{e:?}"))?,
        Some(CharacterWorldEligibilityClaimV1::new(
            commit.character_id(),
            commit.world_id(),
        )),
        RuntimeScopeRefV1::channel(commit.world_id(), commit.channel_id()),
        ScopeOwnershipGeneration::new(1).map_err(|e| format!("{e:?}"))?,
    )
    .map_err(|e| format!("{e:?}"))?;
    Ok(TargetFacts { authority, ..base })
}

fn pem(label: &str, der: &[u8]) -> String {
    use base64::Engine;
    let text = base64::engine::general_purpose::STANDARD.encode(der);
    let mut result = format!("-----BEGIN {label}-----\n");
    for line in text.as_bytes().chunks(64) {
        result.push_str(&String::from_utf8_lossy(line));
        result.push('\n');
    }
    result.push_str(&format!("-----END {label}-----\n"));
    result
}

async fn withdrawal_case(
    h: &Harness,
    service: &CastService,
    state: oteryn_gameplay_social::cast::State,
    scenario: u8,
    (announce_delay, preference_delay): (&AtomicU64, &AtomicU64),
    io_started: &tokio::sync::Notify,
) -> TestResult {
    use crate::bestiary_postgres_harness::{debug, fence};
    use crate::durability::cast_consent::{CastConsent, CastConsentOutcome};
    use oteryn_gameplay_social::cast::{Content, Scene, test_support::admission};
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let fence = fence(1)?;
    let on = CastConsent {
        allowed: true,
        revision: 1,
    };
    assert_eq!(
        write_consent(h, &authority, fence, (0, true)).await?,
        CastConsentOutcome::Written(on)
    );
    service
        .apply(Operation::Consent(fence.character_id, on))
        .await?;
    let Reply::Observer(viewer) = service
        .apply(Operation::Join(admission(state, id(99), id(99), 1)))
        .await?
    else {
        return Err("missing viewer".into());
    };
    let Reply::Capture(permit) = service.apply(Operation::Capture(target(1)?)).await? else {
        return Err("missing capture permit".into());
    };
    service
        .apply(Operation::Publish(
            permit,
            target(1)?,
            vec![fence.character_id],
            Scene {
                tiles: vec![],
                actors: vec![],
            },
            vec![(fence.character_id, on)],
        ))
        .await?;
    service.apply(Operation::Refresh(target(2)?)).await?;
    if scenario == 1 {
        announce_delay.store(100, Ordering::SeqCst);
    } else {
        preference_delay.store(1600, Ordering::SeqCst);
    }
    let worker = service.clone();
    let operation = Operation::Start(target(12)?, Settings::default(), id(12));
    let applied = tokio::spawn(async move { worker.apply(operation).await });
    tokio::time::timeout(std::time::Duration::from_secs(3), io_started.notified()).await?;
    let withdrawn = write_consent(h, &authority, fence, (1, false)).await;
    assert!(matches!(applied.await?, Ok(Reply::State(_))));
    announce_delay.store(0, Ordering::SeqCst);
    preference_delay.store(0, Ordering::SeqCst);
    assert!(
        matches!(
            service
                .apply(Operation::State(
                    target(2)?.authority.current_game_session_id()
                ))
                .await,
            Ok(Reply::State(_))
        ),
        "a withdrawal during successful external I/O expired an unrelated cast"
    );
    assert_eq!(
        withdrawn?,
        CastConsentOutcome::Written(CastConsent {
            allowed: false,
            revision: 2
        })
    );
    let delivered = Arc::new(std::sync::Mutex::new(Vec::new()));
    let output = delivered.clone();
    service
        .apply(Operation::Flush(
            viewer,
            Box::new(move |_, content| match output.lock() {
                Ok(mut lines) => {
                    lines.push(content.clone());
                    true
                }
                Err(_) => false,
            }),
        ))
        .await?;
    let delivered = delivered.lock().map_err(|_| "poisoned output")?;
    assert!(delivered.iter().any(|c| matches!(c, Content::Clear)));
    assert!(delivered.iter().all(|c| !matches!(c, Content::Scene(_))));
    Ok(())
}

async fn write_consent(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    change: (i64, bool),
) -> TestResult<crate::durability::cast_consent::CastConsentOutcome> {
    use crate::durability::{DurabilityError, character_progression::CharacterProgressionError};
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        match h
            .root
            .character_cast_consent(authority, &h.node, fence, Some(change))
            .await
        {
            Ok(value) => return Ok(value),
            Err(CharacterProgressionError::Unavailable(DurabilityError::RootUnavailable))
                if tokio::time::Instant::now() < deadline =>
            {
                let now = tokio::time::Instant::now();
                h.root
                    .wait_holder_retry(
                        now + std::time::Duration::from_millis(2),
                        (now + std::time::Duration::from_millis(50)).min(deadline),
                    )
                    .await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}
