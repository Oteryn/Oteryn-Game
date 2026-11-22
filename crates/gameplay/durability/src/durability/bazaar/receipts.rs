pub(super) async fn terminal(
    tx: &mut Tx<'_>,
    r: &Request,
    code: &str,
    data: Option<&RawValue>,
    now: i64,
    world: Option<&str>,
) -> Result<Decision, DurabilityError> {
    let receipt = serde_json::to_vec(&Receipt {
        contract_version: 2,
        operation: "PublishCharacterCommandReceiptV1",
        source_authority: &r.source,
        command: &r.command,
        operation_id: &r.operation,
        outcome: if code == OK { "COMMITTED" } else { "REJECTED" },
        result_code: code,
        result_data: data,
        recovery_generation: r.generation.to_string(),
        decided_at: now.to_string(),
    })
    .map_err(|_| DurabilityError::Unavailable)?;
    store(tx, r, code, receipt, now, world).await
}

pub(super) async fn store(
    tx: &mut Tx<'_>,
    r: &Request,
    code: &str,
    receipt: Vec<u8>,
    now: i64,
    world: Option<&str>,
) -> Result<Decision, DurabilityError> {
    if receipt.len() > 270336 || (code != OK && receipt.len() > 1024) {
        return Err(DurabilityError::Unavailable);
    }
    // Registered BazaarAuthorityDecisionV1: two exact SHA256 bytes and the
    // current nonrollback generation, not the potentially large receipt body.
    let mut payload = vec![0x0a, 32];
    payload.extend_from_slice(&Sha256::digest(&r.raw));
    payload.extend_from_slice(&[0x12, 32]);
    payload.extend_from_slice(&Sha256::digest(&receipt));
    payload.push(0x18);
    let mut generation = r.generation;
    while generation >= 128 {
        payload.push((generation as u8 & 127) | 128);
        generation >>= 7;
    }
    payload.push(generation as u8);
    sqlx::query("INSERT INTO game_bazaar_decisions(issuer,operation_id,command,intent,receipt,result_code,recovery_generation,decided_at,event_id,transaction_id,world_id,payload,server_build_id) VALUES($1,$2::uuid,$3,$4,$5,$6,$7::numeric,$8,game_character_uuid_v7(),COALESCE((SELECT transaction_id FROM game_bazaar_item_transitions WHERE issuer=$1 AND operation_id=$2::uuid),game_character_uuid_v7()),$9::uuid,$10,$11)")
        .bind(&r.issuer).bind(&r.operation).bind(&r.command).bind(&r.raw).bind(&receipt).bind(code).bind(r.generation.to_string()).bind(now).bind(world).bind(payload).bind(super::character_authority::SERVER_BUILD_ID).execute(&mut **tx).await?;
    Ok(Decision::Publish {
        receipt,
        result_code: code.to_owned(),
    })
}

/// Existing command consumer delegates transfers here without changing v1 bytes.
pub(super) async fn transfer_input(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    source: &str,
    operation: &str,
    input: Input<'_>,
) -> Decision {
    let (raw, technical) = match input {
        Input::Intent(raw) => (raw.to_vec(), None),
        Input::ReadConflict => (
            super::character_command::read_conflict_binding(TRANSFER_COMMAND),
            Some("CHAR_CMD_OPERATION_CONFLICT"),
        ),
        Input::Unsupported => (
            super::character_command::unsupported_binding(TRANSFER_COMMAND),
            Some("CHAR_CMD_UNSUPPORTED"),
        ),
        Input::Oversized => (
            super::character_command::oversized_binding(TRANSFER_COMMAND),
            Some("CHAR_CMD_INTENT_INVALID"),
        ),
    };
    root.decide_authority_command(
        authority,
        proof,
        source,
        TRANSFER_COMMAND,
        operation,
        &raw,
        CHARACTER_ISSUER,
        technical,
    )
    .await
}
