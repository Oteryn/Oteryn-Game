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
            handle: 1,
            name: "Leader".into(),
            leader: true,
            expires_at: 0,
        }],
        invitations: vec![PartyEntry {
            handle: 2,
            name: "Inviter".into(),
            leader: false,
            expires_at: 100,
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
            handle: i,
            name: "a".repeat(64),
            leader: i == 1,
            expires_at: 0,
        });
    }
    for i in 51..=70 {
        maximum.invitations.push(PartyEntry {
            handle: i,
            name: "a".repeat(64),
            leader: false,
            expires_at: u64::MAX,
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
