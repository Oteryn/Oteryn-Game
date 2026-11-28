use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};

pub(super) const TRANSFER_TEST_HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

// Initial authority fixture: the composition owns these two guards before the
// transfer. The runtime readiness still comes from the native assignment API.
// Negative cases above never derive their current facts from the transfer record.
pub(super) async fn prepare_owned_session(h: &Harness) -> TestResult {
    prepare_owned_session_request(h).await.map(|_| ())
}

pub(super) async fn prepare_owned_session_request(
    h: &Harness,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
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
    let request = source.request()?;
    let outcome = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&request)
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    Ok(request)
}

pub(super) async fn create_other_house_owner(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    evidence: &crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
) -> TestResult<[u8; 16]> {
    use crate::character_bootstrap_intent as bootstrap;
    use crate::gameplay_transport::canonical_uuid;

    let account = canonical_uuid(&id(120));
    evidence.refresh_account(&h.root, &h.node, &account).await.map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let operation = id(80);
    let source = serde_json::json!({
        "contract_version": bootstrap::CONTRACT_VERSION,
        "variant": bootstrap::VARIANT,
        "issuer_authority": bootstrap::ISSUER_AUTHORITY,
        "issuer_decision_id": canonical_uuid(&id(90)),
        "source_revision": "2",
        "operation_id": canonical_uuid(&operation),
        "operation": bootstrap::OPERATION,
        "account_id": account,
        "target_world_id": canonical_uuid(&id(42)),
        "requested_name": "Other House Owner",
        "requested_sex": "Male",
        "requested_vocation": "Knight",
        "interpretation_context": {
            "profile_revision": "profile-1", "ruleset_revision": "ruleset-1",
            "content_revision": "content-1", "starter_template_revision": "starter-1"
        },
        "issued_at_source": now.to_string(),
        "expires_at_source": (now+120).to_string(),
        "audience": bootstrap::AUDIENCE,
    });
    let intent = bootstrap::decode_producer_response(&serde_json::to_vec(&source)?, operation)
        .map_err(debug)?;
    let created = h.root.bootstrap_character(authority, &h.node, &intent).await.map_err(debug)?;
    Ok(*created.character_id.as_bytes())
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

pub(super) async fn resume_same_session(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: crate::foundation::GameSessionId,
    token: &str,
    transport: u8,
    supported: &[u32],
) -> TestResult<AdmittedSession> {
    let mut body = Vec::new();
    bytes(&mut body, 1, source.as_bytes());
    bytes(&mut body, 2, token.as_bytes());
    body.extend([24, 0]);
    scalar(&mut body, 4, 1);
    scalar(&mut body, 5, 1);
    scalar(&mut body, 6, 1);
    for id in supported {
        scalar(&mut body, 7, u64::from(*id));
    }
    bytes(&mut body, 8, b"resume-busy-holder-test");
    let mut frame = vec![8, MessageType::ClientResume as u8];
    bytes(&mut frame, 4, &body);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        owner,
        &RecoveryIdentifiers(transport),
    )
    .await
    .map_err(|e| format!("same-session resume {e:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let n = u32::from_be_bytes(output.get(..4).ok_or("no reply")?.try_into()?) as usize;
    let envelope = decode_wire_envelope(&output[4..4 + n]).map_err(debug)?;
    assert_eq!(envelope.message_type(), MessageType::ServerResumeAccepted);
    Ok(admitted)
}

pub(super) struct RecoveryIdentifiers(pub(super) u8);
impl crate::gameplay_transport::connection::ConnectionIdentifiers for RecoveryIdentifiers {
    fn game_session_id(&self) -> Option<crate::foundation::GameSessionId> {
        None
    }
    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}

pub(super) fn scalar(out: &mut Vec<u8>, tag: u8, mut value: u64) {
    out.push(tag << 3);
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
pub(super) fn bytes(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag << 3 | 2);
    let mut n = value.len();
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(value);
}

pub(super) async fn prepare_transfer_scene(h: &Harness) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryMovementCells,
    Dependencies,
)> {
    let base = Dependencies::new()?;
    let (controller, native_pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        base.admitted.world_id,
        "content-1",
    )?;
    let room = native_pin.qualified_room().clone();
    let world = transfer_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        cells.scope().generation_digest,
        native_pin.identity().client_artifact_digest(),
        native_pin.frame_binding().digest(),
        [4; 32],
        (32309, 32805, 7),
    );
    // House recovery reads equipment under the same current Content activation
    // as the runtime; publishing only an in-memory pin cannot satisfy that read.
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(base.admitted.world_id.as_bytes().as_slice())
        .bind(base.admitted.channel_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(base.admitted.world_id.as_bytes().as_slice())
        .bind(base.admitted.channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice())
        .bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool)
        .await?;
    let deps = Dependencies::with_content(pin, room, base.spells)?;
    Ok((controller, cells, deps))
}

pub(super) async fn initialize_transfer_source(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    generation_digest: [u8; 32],
) -> TestResult<AdmittedSession> {
    initialize_transfer_source_for_owner(h, owner, deps, generation_digest, id(41)).await
}

pub(super) async fn initialize_transfer_source_for_owner(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    generation_digest: [u8; 32],
    house_owner: [u8; 16],
) -> TestResult<AdmittedSession> {
    use crate::durability::house_scope_handoff::{
        HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
    };
    use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
    use crate::foundation::MovementLocalPosition;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;

    deps.initialize(owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!(
        "../../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(TRANSFER_TEST_HOUSE)
    .bind(house_owner.as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,1").bind(id(42).as_slice()).bind(TRANSFER_TEST_HOUSE).execute(&h.pool).await?;
    assert!(matches!(
        h.root
            .assign_house_scope(HouseScopeAssignmentRequest {
                operation_key: OperationKey::from_bytes([82; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                actor_role: "oteryn_test_admin".into(),
                command: HouseScopeAssignmentCommand::Assign {
                    house: HouseId::new(deps.admitted.world_id, TRANSFER_TEST_HOUSE).ok_or("house")?,
                    target: h.node.fact()
                }
            })
            .await
            .map_err(debug)?,
        HouseScopeAssignmentOutcome::Committed(_)
    ));
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[86]).ok_or("selection")?;
    // Recovery reads the selection admitted by the real Channel login owner;
    // the House reader deliberately cannot initialize missing Character state.
    assert!(owner.admit_hazard(admitted).await);
    let admission = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    let equipment = h
        .root
        .initialize_admission_equipment(
            owner.character,
            &h.node,
            &admitted.item_fence.ok_or("source equipment fence")?,
            generation_digest,
            admission.operation(),
        )
        .await
        .map_err(debug)?;
    assert!(equipment.items.is_empty());
    Ok(admitted)
}

fn transfer_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    for tile in &mut sector.tiles {
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    entrance.x = 32309;
    entrance.y = 32805;
    sector.tiles.push(entrance);
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        fixture::door_items,
    )
    .map_err(debug)
}
