//! Native fresh-entry configuration and grant custody at the real handshake.
use super::{
    TestResult,
    fixture::{debug, id},
};
use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
use crate::gameplay_transport::connection::FreshAdmissionAttempt;

#[test]
fn preadm_attempt_debug_never_exposes_grant_material() -> TestResult {
    let token = super::house_transfer::RecoverySource::fresh_token(91)?;
    let attempt = FreshAdmissionAttempt {
        character_id: CharacterId::decode(&id(41)).map_err(debug)?,
        admission_material: token.as_bytes(),
        game_session_id: GameSessionId::decode(&id(91)).map_err(debug)?,
        transport: AuthenticatedTransportRefV1::decode(&[91; 16]).map_err(debug)?,
        supported_capabilities: &[],
    };
    let description = format!("{attempt:?}");
    assert!(
        description.contains("[REDACTED]"),
        "fresh grant debug must be redacted"
    );
    assert!(!description.contains(&token));
    assert!(!description.contains(&format!("{:?}", token.as_bytes())));
    Ok(())
}

#[test]
fn preadm_disabled_is_unconsumed_and_enabled_handshake_has_one_concurrent_winner() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    use std::sync::{Arc, Mutex, mpsc};
    let stage = Arc::new(Mutex::new("setup"));
    let observed = stage.clone();
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::Builder::new().name("preadm-handshake".into()).stack_size(64 << 20)
        .spawn(move || {
            let result = super::fixture::run(async move |admin| {
                use crate::gameplay_transport::connection::{admit_frame, AdmissionRefusal, ConnectionEnd, FirstEntryOutcome};
                use tokio::io::AsyncReadExt;
                let h = super::fixture::Harness::create_with_admission_seed(admin, "preadm_handshake", true, false).await?;
                let result: TestResult = async {
                    let source = super::house_transfer::RecoverySource::start(&h).await?;
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
                    let mut runtime = super::fresh_fixture::fresh::Source::new(i64::try_from(now)?)?;
                    runtime.rows.retain(|row| matches!(row.key, crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
                    let publication = crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(&runtime, i64::try_from(now)?).map_err(debug)?;
                    h.root.publish_runtime_readiness(&h.node, &publication).await.map_err(debug)?;
                    let mut connection = h.root.try_acquire_ready()?;
                    sqlx::query("SET ROLE oteryn_game_runtime").execute(&mut *connection).await?;
                    let returned = connection.return_to_pool_observed_until(std::time::Instant::now() + std::time::Duration::from_secs(5)).await;
                    assert!(matches!(returned, sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle));
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    let world = super::fixture::fence(1)?.runtime_scope.world_id();
                    let deps = super::Dependencies::with_content_and_capacity(
                        super::ChannelContentPin::test(world),
                        crate::content::qualify_native_entry_room(world).map_err(debug)?,
                        crate::spell::cast::v1_spell_book().map_err(debug)?,
                        4,
                    )?;
                    let mut owner = deps.owner(&h, &authority);
                    owner.evidence = &source.evidence;
                    Arc::get_mut(&mut owner.states).ok_or("exclusive fixture state")?.native_pre_admission_disabled = true;
                    let token = race_grant()?;
                    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(&oteryn_protocol_oteryn::ClientBootstrapValue {
                        schema_revision: 1, character_id: CharacterId::decode(&id(41)).map_err(debug)?,
                        admission_material: token.as_bytes(), client_build_id: "preadm-contract", supported_capabilities: &[],
                    }).map_err(debug)?;
                    *stage.lock().map_err(|_| "stage lock")? = "disabled handshake";
                    let (mut server, client) = tokio::io::duplex(1 << 20);
                    assert!(matches!(admit_frame(&mut server, &frame, &owner, &Identifiers(91)).await,
                        Err(ConnectionEnd::AdmissionRefused(AdmissionRefusal::Unavailable))));
                    drop(server); drop(client);
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts").fetch_one(&h.pool).await?;
                    assert_eq!(count, 0, "disabled entry must not consume the grant");
                    Arc::get_mut(&mut owner.states).ok_or("exclusive fixture state")?.native_pre_admission_disabled = false;
                    *stage.lock().map_err(|_| "stage lock")? = "concurrent enabled handshakes";
                    let (mut server_a, mut client_a) = tokio::io::duplex(1 << 20);
                    let (mut server_b, mut client_b) = tokio::io::duplex(1 << 20);
                    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(40), async {
                        tokio::join!(admit_frame(&mut server_a, &frame, &owner, &Identifiers(91)),
                                     admit_frame(&mut server_b, &frame, &owner, &Identifiers(92)))
                    }).await?;
                    drop(server_a); drop(server_b);
                    let mut bytes_a = Vec::new(); let mut bytes_b = Vec::new();
                    client_a.read_to_end(&mut bytes_a).await?;
                    client_b.read_to_end(&mut bytes_b).await?;
                    let (admitted, accepted) = match (a, b) {
                        (Ok(admitted), Err(_)) => (admitted, bytes_a),
                        (Err(_), Ok(admitted)) => (admitted, bytes_b),
                        _ => return Err("one grant must have exactly one successful handshake".into()),
                    };
                    assert!(matches!(admitted.first_entry, FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled), "the winner must be positioned for gameplay");
                    let length = usize::try_from(u32::from_be_bytes(accepted.get(..4).ok_or("acceptance length")?.try_into()?))?;
                    let envelope = crate::foundation::decode_wire_envelope(accepted.get(4..4+length).ok_or("acceptance body")?).map_err(debug)?;
                    assert_eq!(envelope.message_type(), crate::foundation::MessageType::ServerAccepted);
                    assert!(!accepted.windows(token.len()).any(|bytes| bytes == token.as_bytes()), "the response may not echo the grant");
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts").fetch_one(&h.pool).await?;
                    assert_eq!(count, 1, "concurrent connections must consume once durably");
                    assert_eq!(owner.logout_session(admitted).await,
                        crate::gameplay_transport::connection::LogoutReply::Result(oteryn_protocol_oteryn::logout::LogoutResult::Accepted));
                    let payload = token.split('.').nth(1).ok_or("grant payload")?;
                    use base64::Engine as _;
                    let claims: serde_json::Value = serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?)?;
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
                    assert!(now < claims["exp"].as_u64().ok_or("grant expiry")?, "the replay case must test an unexpired consumed grant");
                    *stage.lock().map_err(|_| "stage lock")? = "consumed grant replay";
                    let (mut server, client) = tokio::io::duplex(1 << 20);
                    assert!(admit_frame(&mut server, &frame, &owner, &Identifiers(93)).await.is_err());
                    drop(server); drop(client);
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts").fetch_one(&h.pool).await?;
                    assert_eq!(count, 1);
                    Ok(())
                }.await;
                *stage.lock().map_err(|_| "stage lock")? = "cleanup";
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            }).map_err(|error| error.to_string());
            let _ = send.send(result);
        })?;
    receive
        .recv_timeout(std::time::Duration::from_secs(120))
        .map_err(|error| {
            format!(
                "preadm watchdog at {}: {error}",
                observed.lock().map(|s| *s).unwrap_or("stage lock")
            )
        })?
        .map_err(Into::into)
}

struct Identifiers(u8);
impl crate::gameplay_transport::connection::ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(self.0)).ok()
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}

// The same fixture issuer and exact profile, with its accepted 30-second TTL.
fn race_grant() -> TestResult<String> {
    use base64::Engine as _;
    use ed25519_dalek::Signer as _;
    let token = super::house_transfer::RecoverySource::fresh_token(91)?;
    let mut parts = token.split('.');
    let header = parts.next().ok_or("grant header")?;
    let payload = parts.next().ok_or("grant payload")?;
    let codec = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut claims: serde_json::Value = serde_json::from_slice(&codec.decode(payload)?)?;
    claims["exp"] = serde_json::Value::from(claims["iat"].as_u64().ok_or("grant issuance")? + 30);
    let input = format!("{header}.{}", codec.encode(serde_json::to_vec(&claims)?));
    let signature = ed25519_dalek::SigningKey::from_bytes(&[31; 32]).sign(input.as_bytes());
    Ok(format!("{input}.{}", codec.encode(signature.to_bytes())))
}
