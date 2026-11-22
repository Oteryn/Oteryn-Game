//! The qualification predicates consume the same TLS/native-session outcomes as live journeys.
use super::*;
use oteryn_protocol_oteryn::{self as native, actor_spell};

#[derive(Clone, Copy)]
enum Check {
    C04,
    ProtectionZone,
    Party,
}

#[test]
fn cast_status_regression_c04_uses_the_native_rejected_status() -> TestResult {
    run_transport(Check::C04)
}

#[test]
fn cast_status_regression_pz_probe_keeps_rejection_and_paid_cast_distinct() -> TestResult {
    run_transport(Check::ProtectionZone)
}

#[test]
fn cast_status_regression_party_keeps_rejection_and_cooldown_distinct() -> TestResult {
    run_transport(Check::Party)
}

fn run_transport(check: Check) -> TestResult {
    let thread = std::thread::Builder::new()
        .name("cast-status-tls".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(async {
                    tokio::time::timeout(Duration::from_secs(20), exercise(check))
                        .await
                        .map_err(|_| "CAST status TLS fixture deadline")?
                }))
            })();
            result.map_err(|error| error.to_string())
        })?;
    match thread.join() {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

async fn read(stream: &mut ServerStream) -> TestResult<Vec<u8>> {
    let size = stream.read_u32().await?;
    if size == 0 || size > native::MAX_WIRE_FRAME_BYTES {
        return Err("CAST frame outside native size bound".into());
    }
    let mut bytes = vec![0; usize::try_from(size)?];
    stream.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn send(stream: &mut ServerStream, bytes: &[u8]) -> TestResult {
    stream.write_all(&framed(bytes)).await?;
    stream.flush().await?;
    Ok(())
}

async fn exercise(check: Check) -> TestResult {
    let before = actor_spell::ActorVitals {
        health: 150,
        max_health: 150,
        mana: 55,
        max_mana: 100,
        soul: 100,
        harmony: 0,
        serene: false,
    };
    let after = actor_spell::ActorVitals { mana: 45, ..before };
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    config.alpn_protocols = vec![native::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::task::spawn_local(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        let bytes = read(&mut stream).await?;
        native::decode_wire_envelope(&bytes)?.client_bootstrap()?;
        send(&mut stream, &native::encode_server_accepted(&native::ServerAcceptedValue {
            game_session_id: native::GameSessionId::decode(&v7(1, 1))?,
            world_id: native::WorldId::decode(&v7(2, 2))?,
            channel_id: native::ChannelId::decode(&v7(3, 3))?,
            connection_generation: 1,
            current_server_sequence: 40,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?).await?;
        let spatial = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [0x11; 32],
            actor_position: ActorPosition { x: 0, y: 0, floor: 7 },
        });
        let overlay = native::world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("CAST overlay: {error:?}"))?;
        let vitals = actor_spell::encode_actor_vitals(&before)
            .map_err(|error| format!("CAST vitals snapshot: {error:?}"))?;
        for frame in native::encode_single_chunk_snapshot(1, 1, 40, &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 0,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: native::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type: native::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            DomainSnapshot {
                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 0,
                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &vitals,
            },
        ])? {
            send(&mut stream, &frame).await?;
        }
        for (command_id, sequence, status, disposition) in [
            (7, 41, CommandStatus::Rejected, casts::SpellCastDisposition::Rejected),
            (8, 42, CommandStatus::Accepted, casts::SpellCastDisposition::CoolingDown),
            (9, 43, CommandStatus::Accepted, casts::SpellCastDisposition::Cast),
        ] {
            let bytes = read(&mut stream).await?;
            let command = native::decode_wire_envelope(&bytes)?.client_command(1)?;
            assert_eq!(command.command_id, command_id);
            assert_eq!(command.command_type, actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT);
            let intent = actor_spell::decode_spell_cast_intent(command.payload)
                .map_err(|error| format!("CAST request: {error:?}"))?;
            assert_eq!(intent.spell.get(), 1);
            assert_eq!(intent.target, actor_spell::SpellTarget::None);
            assert!(!intent.aim_at_target);
            send(&mut stream, &native::encode_command_result(
                1, sequence, command_id, status,
                &actor_spell::encode_spell_cast_result(disposition),
            )?).await?;
        }
        let vitals = actor_spell::encode_actor_vitals(&after)
            .map_err(|error| format!("CAST vitals delta: {error:?}"))?;
        send(&mut stream, &native::encode_state_delta(
            1, 44, actor_spell::STATE_DOMAIN_ACTOR_VITALS, 0, 1,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1, &vitals,
        )?).await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    let mut client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&v7(4, 4))?,
        admission_material: b"cast-status-fixture",
        client_build_id: "cast-status-tls-test",
        deadline: Duration::from_secs(5),
    }).await?;
    assert_eq!(client.actor_vitals(), Some(&before));
    let spell = std::num::NonZeroU32::new(1).ok_or("fixture spell index")?;
    let rejected = client.cast_spell(spell, actor_spell::SpellTarget::None, false).await?;
    assert_eq!(rejected.status, CommandStatus::Rejected);
    assert_eq!(rejected.disposition, casts::SpellCastDisposition::Rejected);
    assert!(rejected.actor_vitals_delta.is_none());
    assert_eq!(client.actor_vitals(), Some(&before));
    let cooldown = client.cast_spell(spell, actor_spell::SpellTarget::None, false).await?;
    assert_eq!(cooldown.status, CommandStatus::Accepted);
    assert_eq!(cooldown.disposition, casts::SpellCastDisposition::CoolingDown);
    assert!(cooldown.actor_vitals_delta.is_none());
    assert_eq!(client.actor_vitals(), Some(&before));
    let paid = client.cast_spell(spell, actor_spell::SpellTarget::None, false).await?;
    assert_eq!(paid.status, CommandStatus::Accepted);
    assert_eq!(paid.disposition, casts::SpellCastDisposition::Cast);
    assert_eq!(paid.actor_vitals_delta.as_ref().map(|delta| delta.value), Some(after));
    assert_eq!(client.actor_vitals(), Some(&after));
    server.await??;

    // The transport has delivered the exact native outcomes; now use the live journey predicates.
    let mut wrong_status = rejected.clone();
    wrong_status.status = CommandStatus::Accepted;
    let mut unexpected_delta = rejected.clone();
    unexpected_delta.actor_vitals_delta = paid.actor_vitals_delta.clone();
    match check {
        Check::C04 => {
            assert!(full_row_c04::pz_rejection_matches(&rejected, before.mana, before.mana));
            assert!(!full_row_c04::pz_rejection_matches(&wrong_status, before.mana, before.mana));
            assert!(!full_row_c04::pz_rejection_matches(&unexpected_delta, before.mana, before.mana));
            assert!(!full_row_c04::pz_rejection_matches(&rejected, before.mana, after.mana));
            assert!(!full_row_c04::pz_rejection_matches(&cooldown, before.mana, before.mana));
            assert!(!full_row_c04::pz_rejection_matches(&paid, before.mana, after.mana));
        }
        Check::ProtectionZone => {
            use client_spell_journey::protection_zone_outcome_matches as matches;
            assert!(matches(&rejected, before, Some(before), true));
            assert!(!matches(&wrong_status, before, Some(before), true));
            assert!(!matches(&unexpected_delta, before, Some(before), true));
            assert!(!matches(&rejected, before, Some(after), true));
            assert!(!matches(&rejected, before, None, true));
            assert!(!matches(&cooldown, before, Some(before), true));
            assert!(!matches(&rejected, before, Some(before), false));
            assert!(matches(&paid, before, Some(after), false));
            assert!(!matches(&paid, before, Some(before), false));
            let mut missing_delta = paid.clone();
            missing_delta.actor_vitals_delta = None;
            assert!(!matches(&missing_delta, before, Some(after), false));
            let mut refused_paid = paid;
            refused_paid.status = CommandStatus::Rejected;
            assert!(!matches(&refused_paid, before, Some(after), false));
        }
        Check::Party => {
            verify_party_refusal(&rejected, before, before, casts::SpellCastDisposition::Rejected)?;
            assert!(verify_party_refusal(&wrong_status, before, before, casts::SpellCastDisposition::Rejected).is_err());
            assert!(verify_party_refusal(&unexpected_delta, before, before, casts::SpellCastDisposition::Rejected).is_err());
            assert!(verify_party_refusal(&rejected, before, after, casts::SpellCastDisposition::Rejected).is_err());
            verify_party_refusal(&cooldown, before, before, casts::SpellCastDisposition::CoolingDown)?;
            let mut refused_cooldown = cooldown;
            refused_cooldown.status = CommandStatus::Rejected;
            assert!(verify_party_refusal(&refused_cooldown, before, before, casts::SpellCastDisposition::CoolingDown).is_err());
            verify_party_payment(&paid, before, after, 10)?;
        }
    }
    Ok(())
}
