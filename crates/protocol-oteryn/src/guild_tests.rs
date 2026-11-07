use super::*;
fn id(n: u8) -> [u8; 16] {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
#[test]
fn guild_codec_intents_queries_results_roundtrip() -> Result<(), GuildWireError> {
    let guild = id(1);
    let character = id(2);
    for intent in [
        GuildIntent::Found {
            name: "Guild Name".into(),
        },
        GuildIntent::Invite {
            guild,
            name: "Guest One".into(),
        },
        GuildIntent::Join { guild },
        GuildIntent::Leave { guild },
        GuildIntent::Kick { guild, character },
        GuildIntent::SetRank {
            guild,
            character,
            level: 2,
            expected_revision: u64::MAX,
        },
        GuildIntent::RevokeInvite { guild, character },
    ] {
        let b = encode_guild_intent(&intent)?;
        assert_eq!(decode_guild_intent(&b)?, intent);
    }
    for q in [
        GuildQuery::Own,
        GuildQuery::Invitations,
        GuildQuery::Roster {
            guild,
            after: Some(GuildRosterCursor {
                guild,
                character,
                level: 20,
            }),
        },
    ] {
        assert_eq!(decode_guild_query(&encode_guild_query(&q)?)?, q);
    }
    for code in 1..=14 {
        let r = GuildIntentResult {
            code,
            guild,
            revision: Some(u64::MAX),
        };
        assert_eq!(
            decode_guild_intent_result(&encode_guild_intent_result(&r)?)?,
            r
        );
    }
    for result in [
        GuildQueryResult::Own(None),
        GuildQueryResult::Own(Some(GuildSummary {
            guild,
            name: "Guild Name".into(),
            state: 1,
            revision: 1,
            own_rank: 1,
            title: "".into(),
            message: "".into(),
            ranks: vec!["Leader".into(), "Vice Leader".into(), "Member".into()],
        })),
        GuildQueryResult::Roster {
            guild,
            revision: 1,
            members: vec![GuildMember {
                character,
                name: "Hero".into(),
                level: 3,
                title: "".into(),
            }],
            next: None,
        },
        GuildQueryResult::Invitations(vec![GuildInvitation {
            guild,
            name: "Guild Name".into(),
            expires_at_unix_ms: 1,
        }]),
    ] {
        assert_eq!(
            decode_guild_query_result(&encode_guild_query_result(&result)?)?,
            result
        );
    }
    Ok(())
}
#[test]
fn guild_codec_bounds_and_malformed_fields() -> Result<(), GuildWireError> {
    let guild = id(1);
    let character = id(2);
    let good = encode_guild_intent(&GuildIntent::SetRank {
        guild,
        character,
        level: 2,
        expected_revision: 1,
    })?;
    for end in 0..good.len() {
        assert!(decode_guild_intent(&good[..end]).is_err());
    }
    let mut duplicate = good.clone();
    duplicate.extend_from_slice(&good);
    assert!(decode_guild_intent(&duplicate).is_err());
    for bytes in [
        &[0x0a, 2, 0x08, 1][..],
        &[0x0a, 3, 0x0a, 1, 0xff],
        &[0x0a, 4, 0x0a, 0, 0x0a, 0],
        &[0x42, 0],
        &[0x0a, 0],
    ] {
        assert!(decode_guild_intent(bytes).is_err());
    }
    assert!(
        encode_guild_intent(&GuildIntent::Found {
            name: "a".repeat(30)
        })
        .is_err()
    );
    assert!(decode_guild_intent(&[0; 129]).is_err());
    assert!(decode_guild_query_result(&vec![0; 65537]).is_err());
    assert!(
        encode_guild_intent(&GuildIntent::SetRank {
            guild,
            character,
            level: 0,
            expected_revision: 1
        })
        .is_err()
    );
    assert!(
        encode_guild_intent(&GuildIntent::SetRank {
            guild,
            character,
            level: 21,
            expected_revision: 1
        })
        .is_err()
    );
    assert!(
        encode_guild_intent(&GuildIntent::SetRank {
            guild,
            character,
            level: 2,
            expected_revision: 0
        })
        .is_err()
    );
    assert!(
        encode_guild_query(&GuildQuery::Roster {
            guild,
            after: Some(GuildRosterCursor {
                guild: id(3),
                character,
                level: 2
            })
        })
        .is_err()
    );
    let members = (1..=100)
        .map(|n| GuildMember {
            character: id(n),
            name: "a".repeat(29),
            level: 3,
            title: "x".repeat(116),
        })
        .collect::<Vec<_>>();
    // 29 Unicode scalars, 116 bytes is permitted; 116 ASCII scalars is not.
    let mut page = GuildQueryResult::Roster {
        guild,
        revision: 1,
        members,
        next: None,
    };
    assert!(encode_guild_query_result(&page).is_err());
    if let GuildQueryResult::Roster { members, next, .. } = &mut page {
        for m in members.iter_mut() {
            m.title = "🦀".repeat(29);
        }
        *next = Some(GuildRosterCursor {
            guild,
            character: id(100),
            level: 3,
        });
    }
    assert_eq!(
        decode_guild_query_result(&encode_guild_query_result(&page)?)?,
        page
    );
    if let GuildQueryResult::Roster { members, .. } = &mut page {
        members.push(GuildMember {
            character: id(101),
            name: "Hero".into(),
            level: 3,
            title: "".into(),
        });
    }
    assert!(encode_guild_query_result(&page).is_err());
    Ok(())
}
