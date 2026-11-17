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
            invitations: std::fs::read(root.join("s03-invitations.bin"))?,
            expected: std::fs::read_to_string(root.join("s03-expected.txt"))?,
        });
    }
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
                title: "Founder".into(),
                message: "Welcome to the guild".into(),
                ranks: vec!["Member".into(), "Leader".into(), "Vice-Leader".into()],
            },
        )))
        .map_err(|e| format!("own {e:?}"))?,
        roster: guild::encode_guild_query_result(&guild::GuildQueryResult::Roster {
            guild: id,
            revision: 1,
            members: vec![guild::GuildMember {
                character: uuid7(8),
                name: "Aurelia".into(),
                level: 1,
                title: "Leader".into(),
            }],
            next: None,
        })
        .map_err(|e| format!("roster {e:?}"))?,
        invitations: guild::encode_guild_query_result(&guild::GuildQueryResult::Invitations(
            vec![guild::GuildInvitation {
                guild: id,
                name: "Qualification Guard".into(),
                expires_at_unix_ms: 1_900_000_000_000,
            }],
        ))
        .map_err(|e| format!("invitations {e:?}"))?,
        expected: "guild_name=Qualification Guard\nfounder=Aurelia\nrank_count=3\nown_rank=1\nroster_level=1\ninvitee=Berenike\n".into(),
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
    let replay = load()?;
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
    apply_result(
        &mut panel,
        &GuildReply::Intent {
            status: CommandStatus::Accepted,
            result: Some(result),
        },
    );
    assert!(!panel.pending);
    assert_eq!(panel.feedback.as_deref(), Some("Guild action accepted"));

    assert!(request(&mut panel, Intent::RefreshOwn).is_some());
    apply_result(&mut panel, &query(&replay.own)?);
    let own = panel.own.as_ref().ok_or("own guild absent")?;
    assert_eq!(own.name, name);
    assert_eq!(own.ranks.len(), rank_count);
    assert_eq!(own.own_rank, own_rank);
    let rank = own.ranks[usize::from(own_rank)].clone();
    assert_eq!(
        panel_text(&panel).lines().next(),
        Some(format!("{name} — {}", own.title).as_str())
    );
    assert!(
        panel_text(&panel)
            .lines()
            .any(|l| l == format!("Rank: {rank}"))
    );
    assert_eq!(panel.feedback, None);

    assert!(request(&mut panel, Intent::RefreshRoster).is_some());
    apply_result(&mut panel, &query(&replay.roster)?);
    let [member] = panel.roster.as_slice() else {
        return Err("expected one roster member".into());
    };
    assert_eq!(
        (member.name.as_str(), member.level),
        (founder, roster_level)
    );
    assert!(
        panel_text(&panel)
            .lines()
            .any(|l| l.starts_with(&format!("{founder} — "))
                && l.ends_with(&format!("(rank {roster_level})")))
    );
    assert!(panel_text(&panel).lines().any(|l| l == "Roster (1):"));

    // The invitee sees the pending invitation and no guild.
    let mut invitee = GuildPanel {
        selected: true,
        ..GuildPanel::default()
    };
    assert!(request(&mut invitee, Intent::RefreshInvitations).is_some());
    apply_result(&mut invitee, &query(&replay.invitations)?);
    let [invitation] = invitee.invitations.as_slice() else {
        return Err("expected one invitation".into());
    };
    assert_eq!(invitation.name, name);
    let text = panel_text(&invitee);
    assert!(text.lines().next() == Some("No guild"));
    assert!(text.lines().any(|l| l == "Invitations (1):"));
    assert!(text.lines().any(|l| l == name));

    // A refused query leaves the shown guild, roster and invitations untouched.
    let before = panel.clone();
    assert!(request(&mut panel, Intent::RefreshOwn).is_some());
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
    // A truncated reply cannot be decoded and never reaches the panel.
    assert!(guild::decode_guild_query_result(&replay.own[..replay.own.len() - 2]).is_err());
    Ok(())
}
