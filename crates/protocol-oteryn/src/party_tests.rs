use super::*;
#[test]
fn party_wire_roundtrips() -> Result<()> {
    for intent in [
        PartyIntent::Invite(EntityRef {
            identity: [7; 16],
            generation: u64::MAX,
        }),
        PartyIntent::Accept(u64::MAX),
        PartyIntent::Leave,
        PartyIntent::Kick(3),
        PartyIntent::PassLeadership(4),
    ] {
        let bytes = encode_party_intent(intent)?;
        assert!(bytes.len() <= MAX_PARTY_INTENT_BYTES);
        assert_eq!(decode_party_intent(&bytes)?, intent);
    }
    let result = PartyResult {
        disposition: PartyDisposition::Ok,
        shared_xp_enabled: true,
        members: vec![PartyEntry {
            list: Vec::new(),
            handle: 1,
            name: "Leader".into(),
            leader: true,
            expires_at: 0,
            party_spell_remaining_ms: None,
        }],
        invitations: vec![PartyEntry {
            list: Vec::new(),
            handle: 2,
            name: "Inviter".into(),
            leader: false,
            expires_at: 100,
            party_spell_remaining_ms: None,
        }],
    };
    assert_eq!(decode_party_result(&encode_party_result(&result)?)?, result);
    Ok(())
}
#[test]
fn party_wire_bounds_and_malformed_inputs() -> Result<()> {
    for input in [
        vec![],
        vec![8, 0],
        vec![8, 6],
        vec![8, 3, 8, 3],
        vec![8, 3, 32, 1],
        vec![8, 2, 32, 0],
        vec![8, 1, 18, 1, 2, 24, 1],
        vec![8, 3, 40, 0],
        vec![8; 49],
    ] {
        assert!(decode_party_intent(&input).is_err(), "{input:?}");
    }
    for disposition in [
        PartyDisposition::Ok,
        PartyDisposition::NotLeader,
        PartyDisposition::AlreadyInParty,
        PartyDisposition::NotInvited,
        PartyDisposition::NotVisible,
        PartyDisposition::LogoutBlocked,
        PartyDisposition::Unavailable,
        PartyDisposition::Rejected,
        PartyDisposition::NotMember,
    ] {
        let result = PartyResult::refused(disposition);
        assert_eq!(decode_party_result(&encode_party_result(&result)?)?, result);
    }
    let mut maximum = PartyResult::refused(PartyDisposition::Ok);
    for i in 1..=50 {
        maximum.members.push(PartyEntry {
            list: Vec::new(),
            handle: i,
            name: "a".repeat(64),
            leader: i == 1,
            expires_at: 0,
            party_spell_remaining_ms: None,
        });
    }
    for i in 51..=70 {
        maximum.invitations.push(PartyEntry {
            list: Vec::new(),
            handle: i,
            name: "a".repeat(64),
            leader: false,
            expires_at: u64::MAX,
            party_spell_remaining_ms: None,
        });
    }
    let encoded = encode_party_result(&maximum)?;
    assert_eq!(decode_party_result(&encoded)?, maximum);
    maximum.invitations.push(maximum.invitations[0].clone());
    assert!(encode_party_result(&maximum).is_err());
    assert!(decode_party_result(&vec![0; 8193]).is_err());
    assert!(decode_party_result(&[8, 1, 16, 2]).is_err());
    assert!(decode_party_result(&[8, 10, 16, 0]).is_err());
    assert!(decode_party_result(&[8, 1, 16, 0, 16, 0]).is_err());
    Ok(())
}

#[test]
fn party_member_effect_durations_decode_and_replace_on_refresh() -> Result<()> {
    // Member field 5 carries all five remaining durations in fixed spell order.
    let mut member = Vec::new();
    push_varint_field(&mut member, 1, 1);
    push_message_field(&mut member, 2, b"Leader");
    push_varint_field(&mut member, 3, 1);
    push_varint_field(&mut member, 4, 0);
    let mut durations = [0_u8; 20];
    durations[4..8].copy_from_slice(&120_000_u32.to_be_bytes());
    push_message_field(&mut member, 5, &durations);
    let mut bytes = vec![8, 1, 16, 0];
    push_message_field(&mut bytes, 3, &member);
    let result = decode_party_result(&bytes)?;
    assert_eq!(
        result.members[0].party_spell_remaining_ms,
        Some([0, 120_000, 0, 0, 0])
    );
    assert_eq!(encode_party_result(&result)?, bytes);
    let mut duplicate = member.clone();
    push_message_field(&mut duplicate, 5, &durations);
    let mut invalid = vec![8, 1, 16, 0];
    push_message_field(&mut invalid, 3, &duplicate);
    assert!(decode_party_result(&invalid).is_err());
    let mut short = member;
    short.pop();
    let mut invalid = vec![8, 1, 16, 0];
    push_message_field(&mut invalid, 3, &short);
    assert!(decode_party_result(&invalid).is_err());
    let mut invitation = result.members[0].clone();
    invitation.leader = false;
    invitation.expires_at = 1;
    let mut invited = PartyResult::refused(PartyDisposition::Ok);
    invited.invitations.push(invitation);
    assert!(encode_party_result(&invited).is_err());
    Ok(())
}

#[test]
fn party_list_wire_bounds_range_and_portrait_identity() -> Result<()> {
    let entity = EntityRef {
        identity: [1; 16],
        generation: 2,
    };
    let row = PartyListEntity {
        entity,
        name: "Knight".into(),
        vocation: 1,
        summon: false,
        distance: 30,
        health_percent: Some(0),
        mana_percent: Some(100),
        skull: Some(6),
        portrait: Some(crate::appearance::Binding {
            context: crate::appearance::Context {
                content_generation: [1; 32],
                cosmetics_digest: [2; 32],
            },
            appearance_ref: 1,
            entity,
            root_revision: 1,
            condition_revision: 0,
            owner_time_us: 1,
            look: crate::appearance::Look::Outfit {
                look_type: 128,
                colours: [0; 4],
                addons: 3,
            },
            selection_revision: None,
            forge_stacks: None,
        }),
        portrait_colours: [[255, 0, 0]; 4],
    };
    let mut result = PartyResult::refused(PartyDisposition::Ok);
    result.members.push(PartyEntry {
        list: vec![row],
        handle: 1,
        name: "Knight".into(),
        leader: true,
        expires_at: 0,
        party_spell_remaining_ms: None,
    });
    let encoded = encode_party_result(&result)?;
    assert_eq!(decode_party_result(&encoded)?, result);
    let row_bytes = result.members[0].list[0].encode()?;
    for end in 0..row_bytes.len() {
        assert!(PartyListEntity::decode(&row_bytes[..end]).is_err());
    }
    result.members[0].list[0].distance = 31;
    assert!(encode_party_result(&result).is_err());
    result.members[0].list[0].health_percent = None;
    result.members[0].list[0].mana_percent = None;
    result.members[0].list[0].skull = None;
    assert!(encode_party_result(&result).is_ok());
    result.members[0].list[0]
        .portrait
        .as_mut()
        .ok_or(PartyWireError::Malformed)?
        .entity
        .generation = 3;
    assert!(encode_party_result(&result).is_err());
    Ok(())
}

fn large_party_list_result() -> PartyResult {
    let mut result = PartyResult::refused(PartyDisposition::Ok);
    for handle in 1..=50 {
        let mut list = Vec::new();
        for index in 0..3 {
            list.push(PartyListEntity {
                entity: EntityRef {
                    identity: [handle as u8; 16],
                    generation: index + 1,
                },
                name: "n".repeat(64),
                vocation: 1,
                summon: index != 0,
                distance: 1,
                health_percent: Some(100),
                mana_percent: None,
                skull: None,
                portrait: None,
                portrait_colours: [[0; 3]; 4],
            });
        }
        result.members.push(PartyEntry {
            handle,
            name: "n".repeat(64),
            leader: handle == 1,
            expires_at: 0,
            party_spell_remaining_ms: None,
            list,
        });
    }
    result
}

#[test]
fn party_result_legacy_peer_enforces_8192_bytes() -> Result<()> {
    let mut result = large_party_list_result();
    for member in &mut result.members {
        member.list.truncate(1);
    }
    for member in &mut result.members[..19] {
        member.name.truncate(1);
    }
    result.members[19].name.truncate(18);
    assert_eq!(encode_party_result(&result)?.len(), 8192);
    // The byte budget accepts 8192, then the capability guard refuses list fields.
    assert_eq!(
        encode_party_result_for_peer(&result, false),
        Err(PartyWireError::Malformed)
    );
    result.members[19].name.push('n');
    assert_eq!(encode_party_result(&result)?.len(), 8193);
    assert_eq!(
        encode_party_result_for_peer(&result, false),
        Err(PartyWireError::LimitExceeded)
    );
    for member in &mut result.members {
        member.list.clear();
    }
    let bytes = encode_party_result_for_peer(&result, false)?;
    assert!(bytes.len() <= 8192);
    assert_eq!(decode_party_result(&bytes)?, result);
    Ok(())
}

#[test]
fn party_result_party_list_peer_supports_65536_bytes() -> Result<()> {
    let result = large_party_list_result();
    let bytes = encode_party_result_for_peer(&result, true)?;
    assert!(bytes.len() > 8192 && bytes.len() <= 65536);
    assert_eq!(decode_party_result(&bytes)?, result);
    assert_eq!(
        decode_party_result(&vec![0; 65536]),
        Err(PartyWireError::Malformed)
    );
    assert_eq!(
        decode_party_result(&vec![0; 65537]),
        Err(PartyWireError::LimitExceeded)
    );
    Ok(())
}
