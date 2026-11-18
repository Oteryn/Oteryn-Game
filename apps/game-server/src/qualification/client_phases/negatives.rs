use super::*;

pub(super) async fn transport<'a>(
    clients: &SeamClients<'a>,
    exact: &TlsConnector,
    next_grant: &mut impl FnMut(&str, [u8; 16], u64) -> OwnedGrant<'a>,
) -> TestResult<(OwnedGrant<'a>, TcpStream)> {
    let &SeamClients {
        address,
        certificate,
        accounts,
        characters,
        descriptor,
        url,
        ..
    } = clients;
    // Positive control first: the exact profile completes TLS, so every
    // refusal below is the node's policy and never a broken trust setup.
    let tcp = TcpStream::connect(address).await?;
    tokio::time::timeout(
        Duration::from_secs(30),
        exact.connect(ServerName::try_from("localhost")?, tcp),
    )
    .await
    .map_err(|_| "exact TLS handshake did not complete")?
    .map_err(|error| format!("exact TLS handshake refused: {error}"))?;
    evidence("transport exact_profile_handshake=completed");

    evidence("stage=transport_negatives");
    // Transport negatives: nothing reaches the frame layer.
    let generation = platform_generation(descriptor, &accounts[0]).await?;
    let grant = next_grant(&accounts[0], characters[0], generation);
    let valid_frame = framed(&bootstrap(
        1,
        1,
        &characters[0],
        &sign_grant(&grant.borrowed(), now_seconds()?),
    ));
    for (label, profile) in [
        (
            "tls12_exact_alpn",
            ClientProfile {
                tls12: true,
                alpn: Some(b"oteryn-game/1"),
            },
        ),
        (
            "wrong_alpn",
            ClientProfile {
                tls12: false,
                alpn: Some(b"wrong"),
            },
        ),
        (
            "missing_alpn",
            ClientProfile {
                tls12: false,
                alpn: None,
            },
        ),
    ] {
        let bad_profile = connector(certificate, &profile)?;
        tokio::time::timeout(Duration::from_secs(20), async {
            let tcp = TcpStream::connect(address).await?;
            match bad_profile
                .connect(ServerName::try_from("localhost")?, tcp)
                .await
            {
                Err(error) => {
                    let alert = error
                        .get_ref()
                        .and_then(|source| source.downcast_ref::<rustls::Error>());
                    if matches!(
                        alert,
                        Some(rustls::Error::AlertReceived(
                            rustls::AlertDescription::ProtocolVersion
                                | rustls::AlertDescription::NoApplicationProtocol
                        ))
                    ) {
                        Ok(())
                    } else {
                        Err(error.into())
                    }
                }
                Ok(mut stream) => {
                    // An absent ALPN offer can finish TLS. The node must close
                    // that profile before it waits for a Foundation frame.
                    let mut output = Vec::new();
                    if stream.get_ref().1.alpn_protocol().is_some()
                        || read_exchange_chunk(&mut stream, &mut output).await?
                    {
                        return Err(format!("{label} was not refused before framing").into());
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
            }
        })
        .await??;
    }
    let mut plaintext = TcpStream::connect(address).await?;
    tokio::time::timeout(Duration::from_secs(20), async {
        plaintext.write_all(&valid_frame).await?;
        plaintext.shutdown().await?;
        let mut output = Vec::new();
        while read_exchange_chunk(&mut plaintext, &mut output).await? {}
        if !output.is_empty() && !matches!(output.as_slice(), [0x15, 0x03, 0x01..=0x03, 0, 2, 2, _])
        {
            return Err(
                format!("plaintext received an invalid TLS rejection: {output:02x?}").into(),
            );
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;
    if committed_admissions(url).await? != 0 {
        return Err("a transport negative committed an admission".into());
    }
    evidence(
        "transport tls12_exact_alpn=refused wrong_alpn=refused missing_alpn=refused plaintext=refused admissions=0",
    );

    Ok((grant, plaintext))
}

pub(super) async fn admission_negatives(
    address: SocketAddr,
    characters: &[[u8; 16]],
    url: &str,
    exact: &TlsConnector,
    grant: &OwnedGrant<'_>,
) -> TestResult<()> {
    evidence("stage=foundation_negatives");
    // Foundation negatives through the real listener, before FND-04.
    let token = sign_grant(&grant.borrowed(), now_seconds()?);
    let malformed_session = v7(1, 0x09);
    let mut malformed_resume = resume_payload(&malformed_session, b"unissued-reconnect-proof");
    // All other fields are canonical; only this singular session identity repeats.
    bytes_field(&mut malformed_resume, 1, &malformed_session);
    let reply =
        exchange_must_close(address, exact, &framed(&envelope(3, 0, &malformed_resume))).await?;
    if reply
        != Reply::Frames(vec![encode_protocol_error(
            FoundationProtocolError::MalformedEnvelope,
            0,
        )?])
        || committed_admissions(url).await? != 0
    {
        return Err(format!("malformed ClientResume admitted or misreported: {reply:?}").into());
    }
    for (label, raw, error) in [
        (
            "wrong_protocol_major",
            framed(&bootstrap(2, 1, &characters[0], &token)),
            Some(FoundationProtocolError::ProtocolMajorMismatch),
        ),
        (
            "wrong_transport_profile",
            framed(&bootstrap(1, 2, &characters[0], &token)),
            Some(FoundationProtocolError::TransportProfileMismatch),
        ),
        (
            "client_command_before_admission",
            framed(&envelope(7, 0, &[])),
            Some(FoundationProtocolError::MalformedEnvelope),
        ),
        ("oversized_frame", 1_048_577u32.to_be_bytes().to_vec(), None),
        ("truncated_frame", vec![0, 0, 0, 9, 1, 2], None),
    ] {
        let (reply, _) =
            exchange_with_write_end(address, exact, &raw, label == "truncated_frame").await?;
        if let Some(error) = error {
            if reply != Reply::Frames(vec![encode_protocol_error(error, 0)?]) {
                return Err(format!("{label}: {reply:?}").into());
            }
        } else if reply != Reply::Closed {
            return Err(format!("{label} did not close without frames: {reply:?}").into());
        }
    }
    if committed_admissions(url).await? != 0 {
        return Err("a Foundation negative committed an admission".into());
    }
    evidence(
        "foundation wrong_protocol_major=rejected wrong_transport_profile=rejected malformed_resume=protocol_error phase_invalid=rejected oversized=closed truncated=closed admissions=0",
    );

    evidence("stage=fnd04_negatives");
    // FND-04 negatives: one invariant each, every other fact valid.
    // Flip a decoded signature byte: editing the last base64url character can leave a
    // non-canonical encoding, which FND-04A classifies as malformed, not unauthenticated.
    let signed = sign_grant(&grant.borrowed(), now_seconds()?);
    let (signing_input, signature) = signed.rsplit_once('.').ok_or("unsigned token")?;
    let mut signature = URL_SAFE_NO_PAD.decode(signature)?;
    signature[0] ^= 0x01;
    let tampered = format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signature));
    let expired = sign_grant(&grant.borrowed(), now_seconds()? - 60);
    // The grant names another character while the bootstrap names the admitted one, so the
    // evidence composed for the bootstrap character is current and the only failing fact is the
    // claimed character, which the durable verifier classifies as an account/character conflict.
    let mut other_grant = grant.borrowed();
    other_grant.character_id = characters[1];
    let other_character = framed(&bootstrap(
        1,
        1,
        &characters[0],
        &sign_grant(&other_grant, now_seconds()?),
    ));
    let mut foreign_key = grant.borrowed();
    let foreign = SigningKey::from_bytes(&[0x7e; 32]);
    foreign_key.signing = &foreign;
    // N8: each classified refusal is exactly one ProtocolError carrying its FND-04A row.
    for (label, raw, error) in [
        (
            "invalid_signature",
            framed(&bootstrap(1, 1, &characters[0], &tampered)),
            FoundationProtocolError::AdmissionGrantAuthenticationFailed,
        ),
        (
            "expired",
            framed(&bootstrap(1, 1, &characters[0], &expired)),
            FoundationProtocolError::AdmissionGrantExpired,
        ),
        (
            "wrong_character_binding",
            other_character,
            FoundationProtocolError::AdmissionAccountCharacterConflict,
        ),
        (
            "untrusted_signer",
            framed(&bootstrap(
                1,
                1,
                &characters[0],
                &sign_grant(&foreign_key, now_seconds()?),
            )),
            FoundationProtocolError::AdmissionGrantAuthenticationFailed,
        ),
    ] {
        let reply = exchange_must_close(address, exact, &raw).await?;
        if reply != Reply::Frames(vec![encode_protocol_error(error, 0)?]) {
            return Err(format!("{label}: {reply:?}").into());
        }
    }
    if committed_admissions(url).await? != 0 {
        return Err("an FND-04 negative committed an admission".into());
    }
    evidence(
        "fnd04 invalid_signature=refused expired=refused wrong_character_binding=refused untrusted_signer=refused admissions=0",
    );

    evidence("stage=capability_negatives");
    // ADMIT-CAP-1 (ADMIT0 §4): the Channel requires `SEAM_REQUIRED_CAPABILITIES`. A bootstrap
    // whose grant is valid and current but whose support omits them reaches the owner, which
    // refuses it after grant authentication with exactly one ADMISSION_CAPABILITY_REQUIRED
    // (1116) frame. Nothing commits and the grant's nonce stays unconsumed: the admission
    // below presents this same grant.
    let unsupported = framed(&bootstrap_supporting(
        1,
        1,
        &characters[0],
        &sign_grant(&grant.borrowed(), now_seconds()?),
        &[],
    ));
    let reply = exchange_must_close(address, exact, &unsupported).await?;
    if reply
        != Reply::Frames(vec![encode_protocol_error(
            FoundationProtocolError::AdmissionCapabilityRequired,
            0,
        )?])
    {
        return Err(format!("capability_required: {reply:?}").into());
    }
    if committed_admissions(url).await? != 0 {
        return Err("a capability refusal committed an admission".into());
    }
    evidence(
        "capability required=21 unsupported_bootstrap=refused_admission_capability_required admissions=0",
    );

    Ok(())
}

#[test]
fn qualification_foundation_negative_runs_on_node_boot_scenario_stack() -> TestResult {
    run_node_boot_seam(|runtime| {
        assert_eq!(
            std::thread::current().name(),
            Some("node-boot-seam-scenario")
        );
        runtime.block_on(Box::pin(async {
            let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
            let certificate = generated.cert.der().clone();
            let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
            let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![certificate.clone()], key.into())?;
            config.alpn_protocols = vec![b"oteryn-game/1".to_vec()];
            let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let exact = connector(&certificate, &EXACT)?;
            let character = v7(1, 0x01);
            let signing = SigningKey::from_bytes(&[0x42; 32]);
            let grant = Grant {
                signing: &signing,
                key_id: "stack-probe",
                account_id: "",
                character_id: character,
                world_id: v7(1, 0x02),
                channel_id: v7(1, 0x03),
                security_generation: 1,
                scope_generation: 1,
                nonce: [0x42; 32],
                attempt: v7(1, 0x04),
            }
            .with_account(&uuid_text(&v7(1, 0x05)));
            let characters = [character];
            let peer = async {
                let (tcp, _) = listener.accept().await?;
                let mut tls = acceptor.accept(tcp).await?;
                let length = tls.read_u32().await?;
                assert!((1..=4096).contains(&length));
                let mut frame = vec![0; length as usize];
                tls.read_exact(&mut frame).await?;
                // A clean close without a ProtocolError must fail the actual stage before any DB read.
                tls.shutdown().await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            let (result, peer_result) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(
                    admission_negatives(address, &characters, "", &exact, &grant),
                    peer
                )
            })
            .await?;
            peer_result?;
            assert_eq!(
                result
                    .err()
                    .ok_or("Foundation stage accepted the missing reply")?
                    .to_string(),
                "malformed ClientResume admitted or misreported: Closed"
            );
            Ok(())
        }))
    })
}
