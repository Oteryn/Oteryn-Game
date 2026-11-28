//! S03 replay: the guild Found reply, own-guild summary, roster and the invitation list applied
//! through the production `request`, `apply_result` and `panel_text`. The reply bytes are decoded
//! by the guild codec exactly as the runtime `send` does before it hands the reply to the panel.
use super::*;
use oteryn_client_runtime::guild::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Replay {
    found: Vec<u8>,
    own: Vec<u8>,
    roster: Vec<u8>,
    invitations: Vec<u8>,
    invite: Vec<u8>,
    invitee_own: Vec<u8>,
    persisted_roster: Vec<u8>,
    own_after_invite: Vec<u8>,
    expected: String,
}

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn load() -> Result<Replay, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Replay {
            found: std::fs::read(root.join("s03-found.bin"))?,
            own: std::fs::read(root.join("s03-own.bin"))?,
            roster: std::fs::read(root.join("s03-roster.bin"))?,
            own_after_invite: std::fs::read(root.join("s03-own-after-invite.bin"))?,
            invite: std::fs::read(root.join("s03-invite.bin"))?,
            invitee_own: std::fs::read(root.join("s03-invitee-own.bin"))?,
            persisted_roster: std::fs::read(root.join("s03-persisted-roster.bin"))?,
            invitations: std::fs::read(root.join("s03-invitations.bin"))?,
            expected: std::fs::read_to_string(root.join("s03-expected.txt"))?,
        });
    }
    fixture()
}

fn fixture() -> Result<Replay, Error> {
    let uuid7 = |fill: u8| {
        let mut id = [fill; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    };
    let id = uuid7(7);
    Ok(Replay {
        found: guild::encode_guild_intent_result(&guild::GuildIntentResult {
            code: 1,
            guild: id,
            revision: Some(1),
        })
        .map_err(|e| format!("found {e:?}"))?,
        own: guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(
            guild::GuildSummary {
                guild: id,
                name: "Qualification Guard".into(),
                state: 1,
                revision: 1,
                own_rank: 1,
                title: String::new(),
                message: String::new(),
                ranks: vec!["Leader".into(), "Vice Leader".into(), "Member".into()],
            },
        )))
        .map_err(|e| format!("own {e:?}"))?,
        own_after_invite: guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(
            guild::GuildSummary {
                guild: id,
                name: "Qualification Guard".into(),
                state: 1,
                revision: 2,
                own_rank: 1,
                title: String::new(),
                message: String::new(),
                ranks: vec!["Leader".into(), "Vice Leader".into(), "Member".into()],
            },
        )))
        .map_err(|e| format!("own {e:?}"))?,
        roster: guild::encode_guild_query_result(&guild::GuildQueryResult::Roster {
            guild: id,
            revision: 1,
            members: vec![guild::GuildMember { channel: None,
                character: uuid7(8),
                name: "Aurelia".into(),
                level: 1,
                title: String::new(),
            }],
            next: None,
        })
        .map_err(|e| format!("roster {e:?}"))?,
        persisted_roster: guild::encode_guild_query_result(&guild::GuildQueryResult::Roster {
            guild: id,
            revision: 2,
            members: vec![guild::GuildMember { channel: None,
                character: uuid7(8),
                name: "Aurelia".into(),
                level: 1,
                title: String::new(),
            }],
            next: None,
        })
        .map_err(|e| format!("roster {e:?}"))?,
        invite: guild::encode_guild_intent_result(&guild::GuildIntentResult {
            code: 1, guild: id, revision: Some(2),
        }).map_err(|e| format!("invite {e:?}"))?,
        invitee_own: guild::encode_guild_query_result(&guild::GuildQueryResult::Own(None))
            .map_err(|e| format!("invitee own {e:?}"))?,
        invitations: guild::encode_guild_query_result(&guild::GuildQueryResult::Invitations(
            vec![guild::GuildInvitation {
                guild: id,
                name: "Qualification Guard".into(),
                expires_at_unix_ms: 1_900_000_000_000,
            }],
        ))
        .map_err(|e| format!("invitations {e:?}"))?,
        expected: "guild_name=Qualification Guard\nfounder=Aurelia\nrank_count=3\nown_rank=1\nroster_level=1\ninvitee=Berenike\nroster_channel=None\n".into(),
    })
}

fn query(bytes: &[u8]) -> Result<GuildReply, Error> {
    Ok(GuildReply::Query {
        status: CommandStatus::Accepted,
        result: Some(guild::decode_guild_query_result(bytes).map_err(|e| format!("{e:?}"))?),
    })
}

#[test]
fn guild_replay_found_summary_roster_and_invitation() -> Result<(), Error> {
    verify(&load()?, request, apply_result)
}

fn verify(
    replay: &Replay,
    request: impl Fn(&mut GuildPanel, Intent) -> Option<GuildCommand>,
    apply_result: impl Fn(&mut GuildPanel, &GuildReply),
) -> Result<(), Error> {
    let name = value(&replay.expected, "guild_name")?;
    let founder = value(&replay.expected, "founder")?;
    let rank_count: usize = value(&replay.expected, "rank_count")?.parse()?;
    let own_rank: u8 = value(&replay.expected, "own_rank")?.parse()?;
    let roster_level: u8 = value(&replay.expected, "roster_level")?.parse()?;
    let mut panel = GuildPanel {
        selected: true,
        ..GuildPanel::default()
    };

    let found = request(&mut panel, Intent::Found(name.into())).ok_or("Found refused")?;
    assert!(panel.pending);
    assert_eq!(
        found,
        GuildCommand::Intent(guild::GuildIntent::Found { name: name.into() })
    );
    let result = guild::decode_guild_intent_result(&replay.found).map_err(|e| format!("{e:?}"))?;
    assert_eq!(result.code, 1);
    assert_eq!(result.revision, Some(1));
    let guild_id = result.guild;
    apply_result(
        &mut panel,
        &GuildReply::Intent {
            status: CommandStatus::Accepted,
            result: Some(result),
        },
    );
    assert!(!panel.pending);
    assert_eq!(panel.feedback.as_deref(), Some("Guild action accepted"));

    assert_eq!(
        request(&mut panel, Intent::RefreshOwn),
        Some(GuildCommand::Query(guild::GuildQuery::Own))
    );
    apply_result(&mut panel, &query(&replay.own)?);
    let own = panel.own.as_ref().ok_or("own guild absent")?;
    assert_eq!(own.name, name);
    assert!(own.title.is_empty());
    assert!(own.message.is_empty());
    assert_eq!(own.guild, guild_id);
    assert_eq!(own.state, 1);
    assert_eq!(own.revision, 1);
    assert_eq!(own.ranks, ["Leader", "Vice Leader", "Member"]);
    assert_eq!(own.ranks.len(), rank_count);
    assert_eq!(own.own_rank, own_rank);
    assert_eq!(
        panel_text(&panel).lines().next(),
        Some(format!("{name} — ").as_str())
    );
    assert!(
        panel_text(&panel)
            .lines()
            .any(|line| line == "Rank: Leader")
    );
    assert_eq!(panel.feedback, None);

    let mut expected_invited_summary = own.clone();
    expected_invited_summary.revision = 2;
    let roster_query = GuildCommand::Query(guild::GuildQuery::Roster {
        guild: guild_id,
        after: None,
    });
    assert_eq!(
        request(&mut panel, Intent::RefreshRoster),
        Some(roster_query.clone())
    );
    apply_result(&mut panel, &query(&replay.roster)?);
    let [member] = panel.roster.as_slice() else {
        return Err("expected one roster member".into());
    };
    assert_eq!(
        (member.name.as_str(), member.level),
        (founder, roster_level)
    );
    assert!(member.title.is_empty());
    let expected_channel = value(&replay.expected, "roster_channel")?;
    assert_eq!(format!("{:?}", member.channel), expected_channel);
    let mut roster_text = format!("{founder} —  (rank {roster_level})");
    if expected_channel != "None" {
        roster_text.push_str(" — —");
    }
    assert!(panel_text(&panel).lines().any(|line| line == roster_text));
    assert!(panel_text(&panel).lines().any(|l| l == "Roster (1):"));

    let invitee_name = value(&replay.expected, "invitee")?;
    assert_eq!(
        request(&mut panel, Intent::Invite(invitee_name.into())),
        Some(GuildCommand::Intent(guild::GuildIntent::Invite {
            guild: guild_id,
            name: invitee_name.into()
        }))
    );
    let invite_result =
        guild::decode_guild_intent_result(&replay.invite).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        invite_result,
        guild::GuildIntentResult {
            code: 1,
            guild: guild_id,
            revision: Some(2)
        }
    );
    apply_result(
        &mut panel,
        &GuildReply::Intent {
            status: CommandStatus::Accepted,
            result: Some(invite_result),
        },
    );
    assert_eq!(panel.feedback.as_deref(), Some("Guild action accepted"));
    assert_eq!(
        request(&mut panel, Intent::RefreshOwn),
        Some(GuildCommand::Query(guild::GuildQuery::Own))
    );
    let after_invite = query(&replay.own_after_invite)?;
    assert!(
        matches!(&after_invite, GuildReply::Query { result: Some(guild::GuildQueryResult::Own(Some(summary))), .. } if summary == &expected_invited_summary)
    );
    apply_result(&mut panel, &after_invite);
    assert_eq!(panel.own.as_ref(), Some(&expected_invited_summary));

    assert_eq!(
        request(&mut panel, Intent::RefreshRoster),
        Some(roster_query)
    );
    let persisted = query(&replay.persisted_roster)?;
    let GuildReply::Query {
        result:
            Some(guild::GuildQueryResult::Roster {
                guild,
                revision,
                members,
                next,
            }),
        ..
    } = &persisted
    else {
        return Err("relogged roster has the wrong shape".into());
    };
    assert_eq!((*guild, *revision, *next), (guild_id, 2, None));
    assert_eq!(members, &panel.roster);
    // Clear the previous snapshot so a dropped relog reply cannot satisfy the check.
    panel.roster.clear();
    apply_result(&mut panel, &persisted);
    assert!(!panel.pending);
    assert_eq!(&panel.roster, members);
    assert_eq!(panel.next, *next);

    // Exercise clearing a stale projection using the live founder summary as prior panel state.
    let stale_membership = panel.own.clone();
    assert!(stale_membership.is_some());
    let mut invitee = GuildPanel {
        selected: true,
        own: stale_membership,
        ..GuildPanel::default()
    };
    assert_eq!(
        request(&mut invitee, Intent::RefreshOwn),
        Some(GuildCommand::Query(guild::GuildQuery::Own))
    );
    let own_reply = query(&replay.invitee_own)?;
    assert!(matches!(
        own_reply,
        GuildReply::Query {
            result: Some(guild::GuildQueryResult::Own(None)),
            ..
        }
    ));
    apply_result(&mut invitee, &own_reply);
    assert!(!invitee.pending);
    assert!(invitee.own.is_none());
    assert_eq!(
        request(&mut invitee, Intent::RefreshInvitations),
        Some(GuildCommand::Query(guild::GuildQuery::Invitations))
    );
    apply_result(&mut invitee, &query(&replay.invitations)?);
    let [invitation] = invitee.invitations.as_slice() else {
        return Err("expected one invitation".into());
    };
    assert_eq!(invitation.name, name);
    assert_eq!(invitation.guild, guild_id);
    let text = panel_text(&invitee);
    assert!(text.lines().next() == Some("No guild"));
    assert!(text.lines().any(|l| l == "Invitations (1):"));
    assert!(text.lines().any(|l| l == name));

    // A refused query leaves the shown guild, roster and invitations untouched.
    panel.invitations.clone_from(&invitee.invitations);
    let before = panel.clone();
    assert_eq!(
        request(&mut panel, Intent::RefreshOwn),
        Some(GuildCommand::Query(guild::GuildQuery::Own))
    );
    apply_result(
        &mut panel,
        &GuildReply::Query {
            status: CommandStatus::Rejected,
            result: None,
        },
    );
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Guild request refused by the server")
    );
    assert_eq!((&panel.own, &panel.roster), (&before.own, &before.roster));
    assert_eq!(panel.invitations, before.invitations);
    // A truncated reply cannot be decoded and never reaches the panel.
    assert!(guild::decode_guild_query_result(&replay.own[..replay.own.len() - 2]).is_err());
    Ok(())
}

#[test]
fn rejects_audited_false_positives() -> Result<(), Error> {
    let rejected = |case, replay: &Replay| {
        assert!(
            std::panic::catch_unwind(|| verify(replay, request, apply_result))
                .map_or(true, |result| result.is_err()),
            "accepted {case}"
        );
    };
    let mut replay = fixture()?;
    let guild::GuildQueryResult::Own(Some(mut summary)) =
        guild::decode_guild_query_result(&replay.own).map_err(|e| format!("{e:?}"))?
    else {
        return Err("fixture lacks founder summary".into());
    };
    summary.state = 2;
    replay.own =
        guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(summary.clone())))
            .map_err(|e| format!("{e:?}"))?;
    rejected("active founding state", &replay);
    summary.state = 1;
    summary.ranks[1] = "Wrong Rank".into();
    replay.own =
        guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(summary.clone())))
            .map_err(|e| format!("{e:?}"))?;
    rejected("wrong rank names", &replay);
    replay = fixture()?;
    replay.invitee_own =
        guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(summary)))
            .map_err(|e| format!("{e:?}"))?;
    rejected("invitee membership", &replay);

    for (case, wrong_guild) in [
        ("wrong Invite revision", false),
        ("wrong Invite guild", true),
    ] {
        let mut replay = fixture()?;
        let mut receipt =
            guild::decode_guild_intent_result(&replay.invite).map_err(|e| format!("{e:?}"))?;
        if wrong_guild {
            receipt.guild[0] ^= 1
        } else {
            receipt.revision = Some(1)
        }
        replay.invite =
            guild::encode_guild_intent_result(&receipt).map_err(|e| format!("{e:?}"))?;
        rejected(case, &replay);
    }
    replay = fixture()?;
    replay.persisted_roster = replay.roster.clone();
    rejected("stale relog roster revision", &replay);

    let replay = fixture()?;
    for wrong in [
        Intent::RefreshOwn,
        Intent::RefreshRoster,
        Intent::RefreshInvitations,
        Intent::Invite("Berenike".into()),
    ] {
        let checked = std::panic::catch_unwind(|| {
            verify(
                &replay,
                |panel, intent| {
                    if intent == wrong {
                        let _ = request(panel, intent);
                        let query = if wrong == Intent::RefreshOwn {
                            guild::GuildQuery::Invitations
                        } else {
                            guild::GuildQuery::Own
                        };
                        Some(GuildCommand::Query(query))
                    } else {
                        request(panel, intent)
                    }
                },
                apply_result,
            )
        });
        assert!(
            checked.map_or(true, |result| result.is_err()),
            "accepted wrong command for {wrong:?}"
        );
    }
    Ok(())
}

#[test]
fn rejects_dropped_or_corrupted_application() -> Result<(), Error> {
    let replay = fixture()?;
    for case in 0..4 {
        let result = std::panic::catch_unwind(|| {
            verify(&replay, request, |panel, reply| match (case, reply) {
                (
                    0,
                    GuildReply::Query {
                        result: Some(guild::GuildQueryResult::Roster { revision: 2, .. }),
                        ..
                    },
                )
                | (
                    2,
                    GuildReply::Query {
                        result: Some(guild::GuildQueryResult::Own(None)),
                        ..
                    },
                ) => {
                    panel.pending = false;
                }
                (
                    1,
                    GuildReply::Query {
                        result: Some(guild::GuildQueryResult::Roster { revision: 2, .. }),
                        ..
                    },
                ) => {
                    apply_result(panel, reply);
                    panel.roster[0].title = "Corrupted".into();
                }
                (
                    3,
                    GuildReply::Query {
                        status: CommandStatus::Rejected,
                        ..
                    },
                ) => {
                    apply_result(panel, reply);
                    panel.invitations.clear();
                }
                _ => apply_result(panel, reply),
            })
        });
        assert!(
            result.map_or(true, |result| result.is_err()),
            "accepted application mutation {case}"
        );
    }
    Ok(())
}

#[test]
fn rejects_wrong_titles_channel_and_invitation_side_effects() -> Result<(), Error> {
    let rejected = |replay: &Replay| {
        assert!(
            std::panic::catch_unwind(|| verify(replay, request, apply_result))
                .map_or(true, |result| result.is_err())
        );
    };
    let mut replay = fixture()?;
    let guild::GuildQueryResult::Own(Some(mut summary)) =
        guild::decode_guild_query_result(&replay.own).map_err(|e| format!("{e:?}"))?
    else {
        return Err("fixture lacks founder summary".into());
    };
    summary.title = "Wrong".into();
    replay.own =
        guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(summary.clone())))
            .map_err(|e| format!("{e:?}"))?;
    rejected(&replay);
    summary.revision = 2;
    for title in [true, false] {
        let mut replay = fixture()?;
        summary.title = if title { "Wrong".into() } else { String::new() };
        summary.message = if title { String::new() } else { "Wrong".into() };
        replay.own_after_invite =
            guild::encode_guild_query_result(&guild::GuildQueryResult::Own(Some(summary.clone())))
                .map_err(|e| format!("{e:?}"))?;
        rejected(&replay);
    }
    for title in [true, false] {
        let mut replay = fixture()?;
        let mut reply =
            guild::decode_guild_query_result(&replay.roster).map_err(|e| format!("{e:?}"))?;
        let guild::GuildQueryResult::Roster { guild, members, .. } = &mut reply else {
            return Err("fixture lacks roster".into());
        };
        if title {
            members[0].title = "Wrong".into();
        } else {
            members[0].channel = Some(*guild);
        }
        replay.roster = guild::encode_guild_query_result(&reply).map_err(|e| format!("{e:?}"))?;
        rejected(&replay);
    }
    Ok(())
}
