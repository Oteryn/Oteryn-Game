fn test_varint(mut value: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            return out;
        }
    }
}

fn test_envelope(message_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut envelope = vec![0x08, message_type, 0x22];
    envelope.extend(test_varint(payload.len()));
    envelope.extend_from_slice(payload);
    envelope
}

fn test_state_revision(domain_id: usize) -> Vec<u8> {
    let mut nested = vec![0x08];
    nested.extend(test_varint(domain_id));
    nested.extend([0x10, 0x01]);
    nested
}

fn push_test_varint_field(payload: &mut Vec<u8>, field: usize, value: usize) {
    payload.extend(test_varint(field << 3));
    payload.extend(test_varint(value));
}

fn push_test_bytes_field(payload: &mut Vec<u8>, field: usize, value: &[u8]) {
    payload.extend(test_varint((field << 3) | 2));
    payload.extend(test_varint(value.len()));
    payload.extend_from_slice(value);
}

fn test_uuid_v7(marker: u8) -> [u8; 16] {
    let mut id = [0u8; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = marker;
    id
}

fn test_client_bootstrap_payload(
    protocol_major: usize,
    transport_profile: usize,
    schema_revision: usize,
    character_id: &[u8],
    supported_capabilities: &[usize],
) -> Vec<u8> {
    test_client_bootstrap_payload_with_material_and_build(
        protocol_major,
        transport_profile,
        schema_revision,
        character_id,
        supported_capabilities,
        &[0xaa],
        b"test-client",
    )
}

fn test_client_bootstrap_payload_with_material_and_build(
    protocol_major: usize,
    transport_profile: usize,
    schema_revision: usize,
    character_id: &[u8],
    supported_capabilities: &[usize],
    admission_material: &[u8],
    client_build_id: &[u8],
) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_varint_field(&mut payload, 1, protocol_major);
    push_test_varint_field(&mut payload, 2, transport_profile);
    push_test_varint_field(&mut payload, 3, schema_revision);
    for capability in supported_capabilities {
        push_test_varint_field(&mut payload, 4, *capability);
    }
    push_test_bytes_field(&mut payload, 5, admission_material);
    push_test_bytes_field(&mut payload, 6, character_id);
    push_test_bytes_field(&mut payload, 7, client_build_id);
    payload
}

fn test_client_resume_payload(
    protocol_major: usize,
    transport_profile: usize,
    schema_revision: usize,
    game_session_id: &[u8],
    supported_capabilities: &[usize],
) -> Vec<u8> {
    test_client_resume_payload_with_material(
        protocol_major,
        transport_profile,
        schema_revision,
        game_session_id,
        supported_capabilities,
        &[0xbb],
    )
}

fn test_client_resume_payload_with_material(
    protocol_major: usize,
    transport_profile: usize,
    schema_revision: usize,
    game_session_id: &[u8],
    supported_capabilities: &[usize],
    reconnect_material: &[u8],
) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_bytes_field(&mut payload, 1, game_session_id);
    push_test_bytes_field(&mut payload, 2, reconnect_material);
    push_test_varint_field(&mut payload, 4, protocol_major);
    push_test_varint_field(&mut payload, 5, transport_profile);
    push_test_varint_field(&mut payload, 6, schema_revision);
    for capability in supported_capabilities {
        push_test_varint_field(&mut payload, 7, *capability);
    }
    push_test_bytes_field(&mut payload, 8, b"test-client");
    payload
}

fn test_server_accepted_payload(
    connection_generation: usize,
    next_command_id: usize,
    schema_revision: usize,
    ids: [&[u8]; 3],
    selected_capabilities: &[usize],
) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_bytes_field(&mut payload, 1, ids[0]);
    push_test_bytes_field(&mut payload, 2, ids[1]);
    push_test_bytes_field(&mut payload, 3, ids[2]);
    push_test_varint_field(&mut payload, 4, connection_generation);
    push_test_varint_field(&mut payload, 6, next_command_id);
    push_test_varint_field(&mut payload, 7, PROTOCOL_MAJOR_V1 as usize);
    push_test_varint_field(&mut payload, 8, TRANSPORT_PROFILE_TCP_TLS13_V1 as usize);
    push_test_varint_field(&mut payload, 9, schema_revision);
    for capability in selected_capabilities {
        push_test_varint_field(&mut payload, 10, *capability);
    }
    payload
}

fn test_server_resume_accepted_payload(
    connection_generation: usize,
    next_command_id: usize,
    schema_revision: usize,
    game_session_id: &[u8],
    selected_capabilities: &[usize],
) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_bytes_field(&mut payload, 1, game_session_id);
    push_test_varint_field(&mut payload, 2, connection_generation);
    push_test_varint_field(&mut payload, 4, next_command_id);
    push_test_varint_field(&mut payload, 5, schema_revision);
    for capability in selected_capabilities {
        push_test_varint_field(&mut payload, 6, *capability);
    }
    payload
}
