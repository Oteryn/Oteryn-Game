//! S02 replay: the server's party shield/skull icon replies (invited peer, joined member, leader)
//! applied by the production client reducer `apply`, requested through the production `command`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

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
    if text.len() != 32 {
        return Err("identity hex has the wrong length".into());
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

fn load() -> Result<Replay, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Replay {
            invited: std::fs::read(root.join("s02-invited.bin"))?,
            member: std::fs::read(root.join("s02-member.bin"))?,
            leader: std::fs::read(root.join("s02-leader.bin"))?,
            expected: std::fs::read_to_string(root.join("s02-expected.txt"))?,
        });
    }
    let peer = ([0x11; 16], 3u64);
    let leader = ([0x22; 16], 5u64);
    let encode = |icons: &[wire::PartyIcon]| wire::encode_party_icons(icons).map_err(|e| format!("{e:?}"));
    Ok(Replay {
        invited: encode(&[icon(peer, 2, 0)])?,
        member: encode(&[icon(peer, 3, 0)])?,
        leader: encode(&[icon(leader, 4, 0)])?,
        expected: format!(
            "peer_identity={}\npeer_generation=3\nleader_identity={}\nleader_generation=5\ninvited_shield=2\ninvited_skull=0\nmember_shield=3\nmember_skull=0\nleader_shield=4\nleader_skull=0\n",
            "11".repeat(16),
            "22".repeat(16)
        ),
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

#[test]
fn e2e_s02_replays_party_shields_and_skulls() -> Result<(), Error> {
    let replay = load()?;
    let panel = pending();
    let query = command(&panel, Intent::Query)?;
    assert_eq!(
        (query.capability, query.kind, query.payload.as_slice()),
        (134, 155, [].as_slice())
    );
    let peer = (
        identity(value(&replay.expected, "peer_identity")?)?,
        value(&replay.expected, "peer_generation")?.parse::<u64>()?,
    );
    let leader = (
        identity(value(&replay.expected, "leader_identity")?)?,
        value(&replay.expected, "leader_generation")?.parse::<u64>()?,
    );

    for (bytes, who, shield_key, skull_key, shields) in [
        (&replay.invited, peer, "invited_shield", "invited_skull", [1u8, 2]),
        (&replay.member, peer, "member_shield", "member_skull", [3, 5]),
        (&replay.leader, leader, "leader_shield", "leader_skull", [4, 6]),
    ] {
        let shield = number(&replay.expected, shield_key)?;
        let skull = number(&replay.expected, skull_key)?;
        assert!(shields.contains(&shield));
        let mut applied = panel.clone();
        apply(&mut applied, &query, &result(CommandStatus::Accepted, bytes));
        assert!(!applied.pending);
        assert_eq!(applied.feedback, "Party icons updated");
        assert_eq!(applied.icons.len(), 1);
        assert_eq!(
            (applied.icons[0].entity.identity, applied.icons[0].entity.generation),
            who
        );
        assert_eq!((applied.icons[0].shield, applied.icons[0].skull), (shield, skull));
        assert_eq!(applied.summary, line(who, shield, skull));
    }

    // A refused reply and an undecodable reply leave the shown icons untouched.
    let mut shown = panel.clone();
    apply(&mut shown, &query, &result(CommandStatus::Accepted, &replay.member));
    let before = (shown.icons.clone(), shown.summary.clone());
    shown.pending = true;
    shown.pending_kind = wire::COMMAND_TYPE_PARTY_SKULL_QUERY;
    apply(&mut shown, &query, &result(CommandStatus::Rejected, &replay.invited));
    assert_eq!(shown.feedback, "The server refused the request");
    assert_eq!((shown.icons.clone(), shown.summary.clone()), before);
    shown.pending = true;
    apply(&mut shown, &query, &result(CommandStatus::Accepted, &replay.invited[..replay.invited.len() - 1]));
    assert_eq!(shown.feedback, "Invalid party icons reply");
    assert_eq!((shown.icons, shown.summary), before);
    Ok(())
}
