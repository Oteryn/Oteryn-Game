//! S02 replay: the server's party shield/skull icon replies (invited peer, joined member, leader)
//! applied by the production client reducer `apply`, requested through the production `command`.
//!
//! Under `E2E_ALL_CAPTURE_DIR`, write raw query replies as `s02-invited.bin` (inviter during
//! invitation), `s02-member.bin` (inviter after join), and `s02-leader.bin` (peer after join).
//! In `s02-expected.txt`, keep the role keys `peer_identity`, `peer_generation`,
//! `leader_identity`, `leader_generation`, and `{invited,member,leader}_{shield,skull}`
//! as `key=value` lines. Each icon gets one tab-separated row after the blank line.
//! `view<TAB>identity<TAB>generation<TAB>shield<TAB>skull<TAB>summary`.
//! Use view names `invited`, `member`, and `leader`, 32 hex digits for identity, and decimal
//! numbers for generation, shield, and skull. Within each view, preserve reply order.
//! Summary is the full English panel line for that icon, including spaces and punctuation,
//! e.g. `Entity [11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11] / 3 | Shield: Member | Skull: None`.
//! Use LF in the capture text. The panel joins its summary lines with CRLF.
//! Without the environment variable, replay a two-icon encoded fixture for each view.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use std::{collections::BTreeMap, fmt::Write as _, fs, path::Path};

type Error = Box<dyn std::error::Error>;

struct Replay {
    invited: Vec<u8>,
    member: Vec<u8>,
    leader: Vec<u8>,
    expected: String,
}

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn identity(text: &str) -> Result<[u8; 16], Error> {
    let mut out = [0u8; 16];
    if text.len() != 32 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("identity must have 32 hex digits".into());
    }
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(out)
}

fn icon(entity: ([u8; 16], u64), shield: u8, skull: u8) -> wire::PartyIcon {
    wire::PartyIcon {
        entity: oteryn_client_runtime::party::EntityRef {
            identity: entity.0,
            generation: entity.1,
        },
        shield,
        skull,
    }
}

fn load_capture(root: &Path) -> Result<Replay, Error> {
    Ok(Replay {
        invited: fs::read(root.join("s02-invited.bin"))?,
        member: fs::read(root.join("s02-member.bin"))?,
        leader: fs::read(root.join("s02-leader.bin"))?,
        expected: fs::read_to_string(root.join("s02-expected.txt"))?,
    })
}

fn fixture() -> Result<Replay, Error> {
    let peer = ([0x11; 16], 3);
    let leader = ([0x22; 16], 5);
    let encode =
        |icons: &[wire::PartyIcon]| wire::encode_party_icons(icons).map_err(|e| format!("{e:?}"));
    let mut expected = format!(
        "peer_identity={}\npeer_generation=3\nleader_identity={}\nleader_generation=5\ninvited_shield=2\ninvited_skull=0\nmember_shield=3\nmember_skull=0\nleader_shield=4\nleader_skull=0\n\n",
        "11".repeat(16),
        "22".repeat(16)
    );
    for (view, icons) in [
        ("invited", [(peer, 2, 0), (leader, 0, 3)]),
        ("member", [(peer, 3, 0), (leader, 4, 3)]),
        ("leader", [(peer, 3, 0), (leader, 4, 0)]),
    ] {
        for (who, shield, skull) in icons {
            let hex: String = who.0.iter().map(|byte| format!("{byte:02x}")).collect();
            writeln!(
                expected,
                "{view}\t{hex}\t{}\t{shield}\t{skull}\t{}",
                who.1,
                line(who, shield, skull)
            )?;
        }
    }
    Ok(Replay {
        invited: encode(&[icon(peer, 2, 0), icon(leader, 0, 3)])?,
        member: encode(&[icon(peer, 3, 0), icon(leader, 4, 3)])?,
        leader: encode(&[icon(peer, 3, 0), icon(leader, 4, 0)])?,
        expected,
    })
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending() -> Panel {
    Panel {
        selected: true,
        pending: true,
        pending_kind: wire::COMMAND_TYPE_PARTY_SKULL_QUERY,
        ..Panel::default()
    }
}

const SHIELDS: [&str; 12] = [
    "None",
    "Invitation blue",
    "Invitation yellow",
    "Member",
    "Leader",
    "Shared member",
    "Shared leader",
    "Inactive blinking member",
    "Inactive blinking leader",
    "Inactive member",
    "Inactive leader",
    "Outsider gray",
];

fn line(who: ([u8; 16], u64), shield: u8, skull: u8) -> String {
    let skulls = ["None", "Yellow", "Green", "White", "Red", "Black", "Orange"];
    format!(
        "Entity {:02x?} / {} | Shield: {} | Skull: {}",
        who.0,
        who.1,
        SHIELDS[usize::from(shield)],
        skulls[usize::from(skull)]
    )
}

fn number(expected: &str, key: &str) -> Result<u8, Error> {
    Ok(value(expected, key)?.parse()?)
}

#[derive(Default)]
struct ExpectedView<'a> {
    icons: Vec<wire::PartyIcon>,
    summaries: Vec<&'a str>,
}

fn expected_views(rows: &str) -> Result<BTreeMap<&str, ExpectedView<'_>>, Error> {
    let mut views = BTreeMap::<_, ExpectedView<'_>>::new();
    for (index, row) in rows.lines().enumerate() {
        let fields: Vec<_> = row.split('\t').collect();
        let [view, id, generation, shield, skull, summary] = fields.as_slice() else {
            return Err(format!("invalid S02 icon row {}: {row}", index + 1).into());
        };
        if !matches!(*view, "invited" | "member" | "leader") {
            return Err(format!("unknown S02 view {view}").into());
        }
        let expected = views.entry(*view).or_default();
        let who = (identity(id)?, generation.parse()?);
        let shield: u8 = shield.parse()?;
        let skull: u8 = skull.parse()?;
        if usize::from(shield) >= SHIELDS.len() || skull > 6 {
            return Err(format!("invalid S02 icon codes in {view}: {shield}/{skull}").into());
        }
        assert_eq!(*summary, line(who, shield, skull), "{view} capture summary");
        expected.icons.push(icon(who, shield, skull));
        expected.summaries.push(summary);
    }
    Ok(views)
}

#[test]
fn e2e_s02_replays_party_shields_and_skulls() -> Result<(), Error> {
    let replay = match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        Some(root) => load_capture(Path::new(&root))?,
        None => fixture()?,
    };
    replay_party_shields_and_skulls(&replay)
}

#[test]
fn e2e_s02_replays_two_icons_from_capture_files() -> Result<(), Error> {
    let capture = tempfile::tempdir()?;
    let fixture = fixture()?;
    let root = capture.path();
    fs::write(root.join("s02-invited.bin"), fixture.invited)?;
    fs::write(root.join("s02-member.bin"), fixture.member)?;
    fs::write(root.join("s02-leader.bin"), fixture.leader)?;
    fs::write(root.join("s02-expected.txt"), fixture.expected)?;
    replay_party_shields_and_skulls(&load_capture(root)?)
}

#[test]
fn e2e_s02_rejects_invalid_role_actors() -> Result<(), Error> {
    let peer = format!("peer_identity={}", "11".repeat(16));
    let leader = format!("leader_identity={}", "22".repeat(16));
    for (case, from, to) in [
        ("nil peer", peer.clone(), format!("peer_identity={}", "00".repeat(16))),
        ("one actor in both roles", leader, format!("leader_identity={}", "11".repeat(16))),
        (
            "icon generation disagrees with the role",
            "leader_generation=5".to_owned(),
            "leader_generation=6".to_owned(),
        ),
    ] {
        let mut replay = fixture()?;
        assert_eq!(replay.expected.matches(&from).count(), 1, "{case}");
        replay.expected = replay.expected.replacen(&from, &to, 1);
        assert!(
            replay_party_shields_and_skulls(&replay).is_err(),
            "{case} was accepted"
        );
    }
    Ok(())
}

fn replay_party_shields_and_skulls(replay: &Replay) -> Result<(), Error> {
    let panel = pending();
    let query = command(&panel, Intent::Query)?;
    assert_eq!(
        (query.capability, query.kind, query.payload.as_slice()),
        (134, 155, [].as_slice())
    );
    let (roles, rows) = replay
        .expected
        .split_once("\n\n")
        .ok_or("S02 icon rows absent")?;
    let views = expected_views(rows)?;
    let peer = (
        identity(value(roles, "peer_identity")?)?,
        value(roles, "peer_generation")?.parse::<u64>()?,
    );
    let leader = (
        identity(value(roles, "leader_identity")?)?,
        value(roles, "leader_generation")?.parse::<u64>()?,
    );
    // The two roles are distinct, real actors, and an icon of either role carries that role's
    // generation.
    if peer.0 == [0; 16] || leader.0 == [0; 16] {
        return Err("S02 role identity is nil".into());
    }
    if peer.0 == leader.0 {
        return Err("S02 peer and leader are one actor".into());
    }
    for (view, expected) in &views {
        for shown in &expected.icons {
            for (role, (identity, generation)) in [("peer", peer), ("leader", leader)] {
                if shown.entity.identity == identity && shown.entity.generation != generation {
                    return Err(format!(
                        "S02 {view} icon of the {role} has generation {} not {generation}",
                        shown.entity.generation
                    )
                    .into());
                }
            }
        }
    }

    for (view, bytes, who, shields) in [
        ("invited", &replay.invited, peer, [1u8, 2]),
        ("member", &replay.member, peer, [3, 5]),
        ("leader", &replay.leader, leader, [4, 6]),
    ] {
        let expected = views
            .get(view)
            .ok_or_else(|| format!("S02 {view} icon rows absent"))?;
        let shield = number(roles, &format!("{view}_shield"))?;
        let skull = number(roles, &format!("{view}_skull"))?;
        assert!(shields.contains(&shield), "{view} role shield");
        assert!(
            expected.icons.contains(&icon(who, shield, skull)),
            "{view} role icon"
        );
        let mut applied = panel.clone();
        apply(
            &mut applied,
            &query,
            &result(CommandStatus::Accepted, bytes),
        );
        assert!(!applied.pending);
        assert_eq!(applied.feedback, "Party icons updated");
        assert_eq!(applied.icons, expected.icons, "{view} icons");
        assert_eq!(
            applied.summary,
            expected.summaries.join("\r\n"),
            "{view} summary"
        );
    }

    // A refused reply and an undecodable reply leave the shown icons untouched.
    let mut shown = panel.clone();
    apply(
        &mut shown,
        &query,
        &result(CommandStatus::Accepted, &replay.member),
    );
    let before = (shown.icons.clone(), shown.summary.clone());
    shown.pending = true;
    shown.pending_kind = wire::COMMAND_TYPE_PARTY_SKULL_QUERY;
    apply(
        &mut shown,
        &query,
        &result(CommandStatus::Rejected, &replay.invited),
    );
    assert_eq!(shown.feedback, "The server refused the request");
    assert_eq!((shown.icons.clone(), shown.summary.clone()), before);
    shown.pending = true;
    apply(
        &mut shown,
        &query,
        &result(
            CommandStatus::Accepted,
            &replay.invited[..replay.invited.len() - 1],
        ),
    );
    assert_eq!(shown.feedback, "Invalid party icons reply");
    assert_eq!((shown.icons, shown.summary), before);
    Ok(())
}
