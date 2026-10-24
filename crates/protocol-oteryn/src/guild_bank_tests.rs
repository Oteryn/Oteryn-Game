#![allow(clippy::unwrap_used)]
use super::*;
fn id() -> [u8; 16] {
    [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16]
}
#[test]
fn guild_bank_codec_roundtrip_and_fail_closed() {
    for action in [
        GuildBankAction::Deposit(1),
        GuildBankAction::Withdraw(BALANCE_MAX),
        GuildBankAction::Claim,
        GuildBankAction::Disband("Guild Name".into()),
    ] {
        let value = GuildBankIntent {
            guild: id(),
            action,
        };
        let bytes = encode_guild_bank_intent(&value).unwrap();
        assert_eq!(decode_guild_bank_intent(&bytes), Ok(value));
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&bytes);
        assert!(decode_guild_bank_intent(&duplicate).is_err());
        for i in 0..bytes.len() {
            assert!(decode_guild_bank_intent(&bytes[..i]).is_err());
        }
    }
    for amount in [0, BALANCE_MAX + 1, u64::MAX] {
        assert!(
            encode_guild_bank_intent(&GuildBankIntent {
                guild: id(),
                action: GuildBankAction::Deposit(amount)
            })
            .is_err()
        );
    }
    assert!(decode_guild_bank_intent(&[0; MAX_REQUEST_BYTES + 1]).is_err());
    assert!(
        decode_guild_bank_intent(&[
            10, 16, 0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0, 16, 3, 24, 1
        ])
        .is_err()
    );
    for before in [None, Some(1), Some(i64::MAX as u64)] {
        let q = GuildBankQuery {
            guild: Some(id()),
            before,
            after_claim: None,
        };
        assert_eq!(
            decode_guild_bank_query(&encode_guild_bank_query(&q).unwrap()),
            Ok(q)
        );
    }
    assert!(decode_guild_bank_query(&[]).is_ok());
    assert!(decode_guild_bank_query(&[0; MAX_REQUEST_BYTES + 1]).is_err());
    let mut r = GuildBankReply {
        code: 1,
        bank_balance: HARD_MAX,
        guild_balance: 1,
        claim: BALANCE_MAX,
        entries: vec![GuildBankEntry {
            ordinal: 1,
            kind: 1,
            amount: 1,
            before: 0,
            after: 1,
            actor: id(),
        }],
        next: None,
        claims: Vec::new(),
        next_claim: None,
    };
    assert_eq!(
        decode_guild_bank_reply(&encode_guild_bank_reply(&r).unwrap()),
        Ok(r.clone())
    );
    r.code = 2;
    assert!(encode_guild_bank_reply(&r).is_err());
    r.code = 1;
    r.entries[0].after = 2;
    assert!(encode_guild_bank_reply(&r).is_err());
    assert!(decode_guild_bank_reply(&[0; MAX_REPLY_BYTES + 1]).is_err());
}

#[test]
fn guild_bank_claim_query_and_reply_bounds() {
    let query = GuildBankQuery {
        guild: None,
        before: None,
        after_claim: Some(id()),
    };
    assert_eq!(
        decode_guild_bank_query(&encode_guild_bank_query(&query).unwrap()),
        Ok(query)
    );
    assert!(
        encode_guild_bank_query(&GuildBankQuery {
            guild: Some(id()),
            before: None,
            after_claim: Some(id())
        })
        .is_err()
    );
    assert!(
        encode_guild_bank_query(&GuildBankQuery {
            guild: None,
            before: Some(1),
            after_claim: None
        })
        .is_err()
    );
    let mut reply = GuildBankReply {
        code: 1,
        bank_balance: 0,
        guild_balance: 0,
        claim: 0,
        entries: Vec::new(),
        next: None,
        claims: (0..100)
            .map(|n| {
                let mut guild = id();
                guild[15] = n;
                GuildBankClaim {
                    guild,
                    amount: BALANCE_MAX,
                }
            })
            .collect(),
        next_claim: None,
    };
    reply.next_claim = reply.claims.last().map(|c| c.guild);
    assert_eq!(
        decode_guild_bank_reply(&encode_guild_bank_reply(&reply).unwrap()),
        Ok(reply.clone())
    );
    reply.claims.push(GuildBankClaim {
        guild: id(),
        amount: 1,
    });
    assert!(encode_guild_bank_reply(&reply).is_err());
    reply.claims.pop();
    reply.claims[0].amount = 0;
    assert!(encode_guild_bank_reply(&reply).is_err());
}
