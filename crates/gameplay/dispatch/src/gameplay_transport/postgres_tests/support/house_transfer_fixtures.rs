use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::*;

// Initial authority fixture: the composition owns these two guards before the
// transfer. The runtime readiness still comes from the native assignment API.
// Negative cases above never derive their current facts from the transfer record.
pub(super) async fn prepare_owned_session(h: &Harness) -> TestResult {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::*;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::fresh_fixture::fresh::Source::new(now)?;
    for row in &mut source.rows {
        if matches!(
            row.key,
            AdmissionAuthorityGuardKeyV1::Account { .. }
                | AdmissionAuthorityGuardKeyV1::Character(_)
        ) {
            row.source.authority =
                crate::durability::fresh_admission_composition::COMPOSITION_SOURCE_AUTHORITY.into();
            if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                security.provenance.source_authority = "platform-test".into();
            }
        }
    }
    let mut runtime = super::fresh_fixture::fresh::Source::new(now)?;
    runtime
        .rows
        .retain(|r| matches!(r.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    h.root
        .publish_runtime_readiness(
            &h.node,
            &AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    let mut claims = super::fresh_fixture::fresh::Source::new(now)?;
    claims.rows = source
        .rows
        .iter()
        .filter(|r| !matches!(r.key, AdmissionAuthorityGuardKeyV1::Runtime(_)))
        .cloned()
        .collect();
    AdmissionGuardStore::from_root(h.root.clone())
        .publish(&AdmissionAuthorityPublicationV1::prepare(&claims, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    let outcome = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&source.request()?)
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    Ok(())
}

pub(super) struct RecoverySource {
    pub(super) clock: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub(super) evidence: crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for RecoverySource {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl RecoverySource {
    pub(super) fn stop_peer(&self) { self.task.abort(); }
    pub(super) async fn start(h: &Harness) -> TestResult<Self> {
        Self::start_at(h, None).await
    }
    pub(super) async fn start_at(h: &Harness, fixed_now: Option<u64>) -> TestResult<Self> {
        use crate::durability::native_admission_source::{
            DescriptorRegistration, FreshStoreProvenance,
        };
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
        use std::sync::Arc;
        let material = rcgen::generate_simple_self_signed(vec!["source.test".into()])?;
        let certificate = material.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(material.signing_key.serialize_der());
        let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let descriptor=crate::native_admission_source::descriptor::ProducerDescriptor::new("platform-test".into(),("127.0.0.1".into(),port),"source.test".into(),"source.test".into(),vec![certificate],vec![CertificateDer::from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],PrivateKeyDer::try_from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
        let registration = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: 1,
        };
        let provenance = FreshStoreProvenance {
            namespace: "house-transfer-fixture".into(),
            authorization: "owner:test".into(),
            source_authority: "platform-test".into(),
            initialized_at: 1,
        };
        h.root
            .record_native_source_descriptor_issuance(
                "platform-test",
                registration.clone(),
                Some(provenance.clone()),
            )
            .await?;
        h.root
            .initialize_native_admission_source(&h.node, provenance, registration)
            .await?;
        let clock = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(fixed_now.unwrap_or(0)));
        let peer_clock = clock.clone();
        let task = tokio::spawn(async move {
            let mut revision = 1_u64;
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut io) = acceptor.accept(socket).await else {
                    continue;
                };
                let mut request = Vec::new();
                loop {
                    let mut byte = [0];
                    if io.read_exact(&mut byte).await.is_err() {
                        return;
                    }
                    request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 4096);
                }
                let header = String::from_utf8_lossy(&request);
                let length = header
                    .lines()
                    .find_map(|l| l.strip_prefix("Content-Length: "))
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0);
                assert!(length <= 1024);
                let mut body = vec![0; length];
                if io.read_exact(&mut body).await.is_err() {
                    return;
                }
                let Ok(input) = serde_json::from_slice::<serde_json::Value>(&body) else {
                    return;
                };
                let fixed = peer_clock.load(std::sync::atomic::Ordering::SeqCst);
                let now = if fixed != 0 { fixed } else {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|v| v.as_secs())
                        .unwrap_or(0)
                };
                revision += 1;
                let recovery = input["operation"]
                    .as_str()
                    .is_some_and(|s| s.contains("Recovery"));
                let mut reply = serde_json::json!({"version":if recovery{2}else{1},"operation":input["operation"],"result":"observed","source_authority":"platform-test","source_revision":revision.to_string(),"decision_identity":revision.to_string(),"source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0"});
                if input["operation"]
                    .as_str()
                    .is_some_and(|s| s.contains("AccountSecurity"))
                {
                    reply["account_id"] = input["account_id"].clone();
                    reply["purpose"] = "platform_security".into();
                    reply["scope"] = if recovery {
                        "existing_actor_recovery"
                    } else {
                        "fresh_admission"
                    }
                    .into();
                    reply["allowed"] = true.into();
                    reply["minimum_valid_generation"] = "1".into();
                } else {
                    use base64::Engine;
                    reply["issuer"] = if recovery {
                        "urn:oteryn:platform:game-recovery"
                    } else {
                        "urn:oteryn:platform:game-admission"
                    }
                    .into();
                    reply["profile"] = if recovery {
                        "oteryn-reauth-recovery-v1"
                    } else {
                        "oteryn-pre-admission-v1"
                    }
                    .into();
                    reply["key_purpose"] = if recovery {
                        "existing_actor_recovery"
                    } else {
                        "fresh_admission"
                    }
                    .into();
                    reply["key_id"] = if recovery {
                        "house-recovery"
                    } else {
                        "house-fresh"
                    }
                    .into();
                    reply["trusted"] = true.into();
                    reply["public_key"] = base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .encode(
                            ed25519_dalek::SigningKey::from_bytes(
                                &[if recovery { 23 } else { 31 }; 32],
                            )
                            .verifying_key()
                            .to_bytes(),
                        )
                        .into();
                }
                let body = reply.to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if io.write_all(response.as_bytes()).await.is_err() {
                    return;
                }
                let _ = io.shutdown().await;
            }
        });
        Ok(Self {
            clock,
            evidence: crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(
                descriptor,
            ),
            task,
        })
    }
    pub(super) fn token(nonce: u8) -> TestResult<String> {
        use base64::Engine;
        use ed25519_dalek::Signer;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let encode = |v: String| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        let header =
            serde_json::json!({"alg":"Ed25519","kid":"house-recovery","typ":"oteryn-recovery+jwt"});
        let payload = serde_json::json!({"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":now,"nbf":now,"exp":now+10,"jti":base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([nonce;32]),"profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"28020304-0506-7008-800a-0b0c0d0e0f28","character_id":"29020304-0506-7008-800a-0b0c0d0e0f29","world_id":"2a020304-0506-7008-800a-0b0c0d0e0f2a","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"ruleset-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"});
        let input = format!(
            "{}.{}",
            encode(header.to_string()),
            encode(payload.to_string())
        );
        let signature = ed25519_dalek::SigningKey::from_bytes(&[23; 32]).sign(input.as_bytes());
        Ok(format!(
            "{input}.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }

    /// A separately scoped fresh grant from this registered independent test
    /// producer. The native fresh-admission verifier/publication/commit remain
    /// unchanged; a recovery credential cannot be reused as a fresh grant.
    pub(super) fn fresh_token(nonce: u8) -> TestResult<String> {
        Self::fresh_token_for(nonce, id(40), id(41))
    }

    pub(super) fn fresh_token_for(
        nonce: u8,
        account: [u8; 16],
        character: [u8; 16],
    ) -> TestResult<String> {
        use base64::Engine;
        use ed25519_dalek::Signer;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let encode = |value: String| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value);
        let header =
            serde_json::json!({"alg":"Ed25519","kid":"house-fresh","typ":"oteryn-admission+jwt"});
        let payload = serde_json::json!({
            "iss":"urn:oteryn:platform:game-admission", "aud":"urn:oteryn:game:admission",
            "iat":now, "nbf":now, "exp":now+10,
            "jti":base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([nonce;32]),
            "profile":"oteryn-pre-admission-v1", "purpose":"fresh_entry",
            "attempt_ref":crate::gameplay_transport::canonical_uuid(&id(nonce)),
            "account_id":crate::gameplay_transport::canonical_uuid(&account),
            "character_id":crate::gameplay_transport::canonical_uuid(&character),
            "world_id":"2a020304-0506-7008-800a-0b0c0d0e0f2a",
            "channel_id":"2b020304-0506-7008-800a-0b0c0d0e0f2b",
            "account_security_generation":"1", "route_revision":"route-1",
            "runtime_observation_revision":"runtime-1", "scope_ownership_generation":"1",
            "protocol_major":1, "transport_profile":1, "ruleset_revision":"ruleset-1",
            "content_revision":"content-1", "map_revision":"map-1",
            "world_policy_revision":"policy-1", "offer_revision":"offer-1"
        });
        let input = format!(
            "{}.{}",
            encode(header.to_string()),
            encode(payload.to_string())
        );
        let signature = ed25519_dalek::SigningKey::from_bytes(&[31; 32]).sign(input.as_bytes());
        Ok(format!(
            "{input}.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }
}
