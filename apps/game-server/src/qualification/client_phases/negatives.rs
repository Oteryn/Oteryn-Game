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
    for (label, profile, raw) in [
        (
            "tls12_exact_alpn",
            ClientProfile {
                tls12: true,
                alpn: Some(b"oteryn-game/1"),
            },
            valid_frame.clone(),
        ),
        (
            "wrong_alpn",
            ClientProfile {
                tls12: false,
                alpn: Some(b"wrong"),
            },
            valid_frame.clone(),
        ),
        (
            "missing_alpn",
            ClientProfile {
                tls12: false,
                alpn: None,
            },
            valid_frame.clone(),
        ),
    ] {
        let reply = exchange(address, &connector(certificate, &profile)?, &raw).await?;
        if !matches!(reply, Reply::TlsRefused | Reply::Closed) {
            return Err(format!("{label} reached the frame layer: {reply:?}").into());
        }
    }
    let mut plaintext = TcpStream::connect(address).await?;
    plaintext.write_all(&valid_frame).await?;
    let mut output = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(20), plaintext.read_to_end(&mut output)).await;
    // The TLS layer may answer with an alert record (content type 0x15);
    // nothing else, and never a Foundation frame, may come back.
    if output
        .first()
        .is_some_and(|content_type| *content_type != 0x15)
    {
        return Err(format!("plaintext received a non-alert reply: {output:02x?}").into());
    }
    if committed_admissions(url).await? != 0 {
        return Err("a transport negative committed an admission".into());
    }
    evidence(
        "transport tls12_exact_alpn=refused wrong_alpn=refused missing_alpn=refused plaintext=refused admissions=0",
    );

    Ok((grant, plaintext))
}

pub(super) async fn admission_negatives(
    clients: &SeamClients<'_>,
    exact: &TlsConnector,
    grant: &OwnedGrant<'_>,
) -> TestResult<()> {
    let &SeamClients {
        address,
        characters,
        url,
        ..
    } = clients;
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
            None,
        ),
        ("oversized_frame", 1_048_577u32.to_be_bytes().to_vec(), None),
        ("truncated_frame", vec![0, 0, 0, 9, 1, 2], None),
    ] {
        let reply = exchange(address, exact, &raw).await?;
        if let Some(error) = error {
            if reply != Reply::Frames(vec![encode_protocol_error(error, 0)?]) {
                return Err(format!("{label}: {reply:?}").into());
            }
        } else if accepted_session(&reply).is_some() {
            return Err(format!("{label} was admitted").into());
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
