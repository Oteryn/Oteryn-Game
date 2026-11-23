use super::*;
use gameplay_transport::fresh_evidence::FreshEvidenceSource;
use native_admission_source::descriptor::{Operation, ProducerDescriptor};
use native_admission_source::{CHARACTER_BOOTSTRAP_INTENT_ISSUER, TransientCapacity};
use production_server::node::serve::bootstrap_one;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[test]
fn operator_bootstrap_preserves_the_projection_portfolio_cap() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let db = Database::create(admin, "bootstrap_portfolio_cap").await?;
            let result = exercise(&db).await;
            db.cleanup().await?;
            result
        })
}

async fn exercise(db: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&db.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&db.url).await?;
    let retained = fence_parent().join(format!("bootstrap-cap-{}", db.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    let seal = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&seal)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    initialize_s2(&root, &node).await?;
    assign_world(&pool, &root, &node, 90, 95).await?;
    configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?;
    for tag in 100..164 {
        allow(&root, &node, 31).await?;
        root.bootstrap_character(&authority, &node, &intent(tag, 31, i64::from(tag))?)
            .await
            .map_err(|e| format!("bootstrap {tag}: {e:?}"))?;
    }
    let (intents, evidence, source) = bootstrap_source(Wire::new(164, 31, 164)?).await?;
    let capacity = TransientCapacity::new();
    let rejected = bootstrap_one(
        &root,
        &authority,
        &node,
        &evidence,
        &intents,
        &capacity,
        id(164),
    )
    .await;
    tokio::time::timeout(std::time::Duration::from_secs(2), source).await???;
    assert_eq!(rejected, "rejected");
    let snapshot = root
        .next_account_characters_snapshot(&authority)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("missing account projection")?;
    assert_eq!(snapshot.account_id, uuid(id(31)));
    assert_eq!(snapshot.characters.len(), 64);
    assert!(
        root.reconcile_character_bootstrap(&authority, id(164))
            .await
            .map_err(|e| format!("{e:?}"))?
            .is_none()
    );
    assert_eq!(
        bootstrap_one(
            &root,
            &authority,
            &node,
            &evidence,
            &intents,
            &capacity,
            id(163)
        )
        .await,
        "committed"
    );
    drop(authority);
    drop(seal);
    drop(recovery);
    std::fs::remove_dir_all(&retained)?;
    eprintln!("PLATFORM_PG_EXECUTED: operator bootstrap portfolio cap");
    Ok(())
}

// Local mTLS peer for the two authenticated Platform reads made by bootstrap_one.
async fn bootstrap_source(
    intent: Wire,
) -> TestResult<(
    ProducerDescriptor,
    FreshEvidenceSource,
    tokio::task::JoinHandle<Result<(), String>>,
)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let server = rcgen::generate_simple_self_signed(vec!["source.test".into()])?;
    let mut client_roots = rustls::RootCertStore::empty();
    client_roots.add(CertificateDer::from(
        include_bytes!("../spell-save-tls-fixture/ca.der").to_vec(),
    ))?;
    let provider = std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let client_verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        std::sync::Arc::new(client_roots),
        provider.clone(),
    )
    .build()?;
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(
            vec![server.cert.der().clone()],
            PrivatePkcs8KeyDer::from(server.signing_key.serialize_der()).into(),
        )?;
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(tls));
    let descriptor = |issuer: &str| {
        ProducerDescriptor::new(
            issuer.into(),
            ("127.0.0.1".into(), port),
            "source.test".into(),
            "source.test".into(),
            vec![server.cert.der().clone()],
            vec![CertificateDer::from(
                include_bytes!("../spell-save-tls-fixture/client.der").to_vec(),
            )],
            PrivateKeyDer::try_from(
                include_bytes!("../spell-save-tls-fixture/client-key.der").to_vec(),
            )
            .map_err(|_| native_admission_source::SourceError::InvalidDescriptor)?,
        )
    };
    let intents = descriptor(CHARACTER_BOOTSTRAP_INTENT_ISSUER)?;
    let evidence = FreshEvidenceSource::new(descriptor("platform")?);
    let task = tokio::spawn(async move {
        async {
            let account_id = uuid(id(intent.account));
            let now = now_secs()?;
            let revision = S2_REVISION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let observation = format!(
                r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"{revision}","decision_identity":"{revision}","source_observed_at":"{now}","clock_uncertainty_seconds":"0","account_id":"{account_id}","purpose":"platform_security","scope":"fresh_admission","allowed":true,"minimum_valid_generation":"1"}}"#
            );
            for (op, response) in [
                (Operation::ReadCharacterBootstrapIntentV1, intent.json()),
                (Operation::ReadAccountSecurityV1, observation),
            ] {
                let (tcp, _) = listener.accept().await?;
                let mut tls = BufReader::new(acceptor.accept(tcp).await?);
                let mut line = String::new();
                tls.read_line(&mut line).await?;
                assert_eq!(line, format!("POST {} HTTP/1.1\r\n", op.path()));
                let mut len = None;
                loop {
                    line.clear();
                    if tls.read_line(&mut line).await? == 0 {
                        return Err("incomplete fixture request".into());
                    }
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("Content-Length: ") {
                        len = Some(value.trim().parse::<usize>()?);
                    }
                }
                let mut request = vec![0; len.filter(|len| *len <= 1024).ok_or("fixture request bound")?];
                tls.read_exact(&mut request).await?;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                    response.len(),
                );
                tls.get_mut().write_all(response.as_bytes()).await?;
                tls.get_mut().shutdown().await?;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await
        .map_err(|error| error.to_string())
    });
    Ok((intents, evidence, task))
}
