//! S01 replay: the party invitation view, the accepted join and the leadership pass applied
//! through the production `request`, `apply_result` and `panel_text`.
use super::*;
use oteryn_client_runtime::party::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Replay {
    invitations: Vec<u8>,
    invitation_view: Vec<u8>,
    joined_op: Vec<u8>,
    joined_view: Vec<u8>,
    passed_op: Vec<u8>,
    passed_view: Vec<u8>,
    expected: String,
    statuses: [CommandStatus; 3],
    live: bool,
}

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn entry(handle: u64, name: &str, leader: bool, expires_at: u64) -> party::PartyEntry {
    party::PartyEntry {
        channel: None,
        list: Vec::new(),
        handle,
        name: name.into(),
        leader,
        expires_at,
        party_spell_remaining_ms: None,
    }
}

fn encode(
    members: Vec<party::PartyEntry>,
    invitations: Vec<party::PartyEntry>,
) -> Result<Vec<u8>, Error> {
    party::encode_party_result(&party::PartyResult {
        disposition: party::PartyDisposition::Ok,
        shared_xp_enabled: false,
        members,
        invitations,
    })
    .map_err(|e| format!("{e:?}").into())
}

fn load() -> Result<Replay, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let mut previous = None;
        let mut statuses = [CommandStatus::Rejected; 3];
        for (index, label) in ["invitation", "join", "leadership"].iter().enumerate() {
            let text = std::fs::read_to_string(root.join(format!("s01-{label}-receipt.txt")))?;
            let mut fields = text.splitn(4, ' ');
            statuses[index] = match fields.next() {
                Some("Accepted") => CommandStatus::Accepted,
                _ => return Err("live party reply was not accepted".into()),
            };
            let command: u64 = fields.next().ok_or("command absent")?.parse()?;
            let sequence: u64 = fields.next().ok_or("sequence absent")?.parse()?;
            let session = fields
                .next()
                .filter(|value| !value.is_empty())
                .ok_or("session absent")?;
            if let Some((last_command, last_sequence, last_session)) = &previous {
                assert!(command > *last_command && sequence > *last_sequence);
                assert_eq!(session, last_session);
            }
            previous = Some((command, sequence, session.to_owned()));
        }
        return Ok(Replay {
            statuses,
            live: true,
            invitations: std::fs::read(root.join("s01-invitations.bin"))?,
            invitation_view: std::fs::read(root.join("s01-invitations-view.bin"))?,
            joined_op: std::fs::read(root.join("s01-joined-op.bin"))?,
            joined_view: std::fs::read(root.join("s01-joined-view.bin"))?,
            passed_op: std::fs::read(root.join("s01-passed-op.bin"))?,
            passed_view: std::fs::read(root.join("s01-passed-view.bin"))?,
            expected: std::fs::read_to_string(root.join("s01-expected.txt"))?,
        });
    }
    let joined = encode(
        vec![entry(1, "Aurelia", true, 0), entry(2, "Berenike", false, 0)],
        vec![],
    )?;
    let passed = encode(
        vec![entry(1, "Aurelia", false, 0), entry(2, "Berenike", true, 0)],
        vec![],
    )?;
    let invitations = encode(vec![], vec![entry(9, "Aurelia", false, 1_900_000_000)])?;
    Ok(Replay {
        statuses: [CommandStatus::Accepted; 3],
        live: false,
        invitation_view: invitations.clone(),
        invitations,
        joined_op: joined.clone(),
        joined_view: joined,
        passed_op: passed.clone(),
        passed_view: passed,
        expected:
            "inviter=Aurelia\njoiner=Berenike\nmembers=2\nshared_xp=false\nnew_leader=Berenike\n"
                .into(),
    })
}

fn response(status: CommandStatus, operation: &[u8], view: Option<&[u8]>) -> PartyResponse {
    PartyResponse {
        status,
        operation: operation.to_vec(),
        view: view.map(<[u8]>::to_vec),
    }
}

fn has_line(text: &str, prefix: &str) -> bool {
    text.lines().any(|line| line.starts_with(prefix))
}

#[test]
fn party_replay_invite_join_and_leadership_pass() -> Result<(), Error> {
    let replay = load()?;
    let inviter = value(&replay.expected, "inviter")?;
    let joiner = value(&replay.expected, "joiner")?;
    let shared = value(&replay.expected, "shared_xp")?;
    let mut panel = PartyPanel {
        selected: true,
        chat_selected: true,
        own_character_name: joiner.into(),
        ..PartyPanel::default()
    };

    assert!(request(&mut panel, Intent::Refresh).is_some());
    assert!(apply_result(
        &mut panel,
        &response(
            replay.statuses[0],
            &replay.invitations,
            Some(&replay.invitation_view)
        )
    ));
    let state = panel.state.as_ref().ok_or("state absent")?;
    assert!(state.members.is_empty());
    let [invitation] = state.invitations.as_slice() else {
        return Err("expected one invitation".into());
    };
    assert_eq!(invitation.name, inviter);
    let handle = invitation.handle;
    assert_eq!(
        panel.feedback,
        Some("The server confirmed the party request")
    );
    let text = panel_text(&panel);
    assert!(text.lines().any(|line| line == "Members: none"));
    assert!(
        text.lines()
            .any(|line| line == format!("Invitation: {inviter}"))
    );

    panel.selected_invitation = Some(handle);
    assert!(request(&mut panel, Intent::Join).is_some());
    assert!(apply_result(
        &mut panel,
        &response(
            replay.statuses[1],
            &replay.joined_op,
            Some(&replay.joined_view)
        )
    ));
    let state = panel.state.as_ref().ok_or("state absent")?;
    assert_eq!(
        state.members.len(),
        value(&replay.expected, "members")?.parse::<usize>()?
    );
    assert!(state.invitations.is_empty());
    assert!(state.members.iter().any(|m| m.leader && m.name == inviter));
    assert!(state.members.iter().any(|m| !m.leader && m.name == joiner));
    assert!(!panel.is_leader());
    let text = panel_text(&panel);
    assert!(text.lines().any(|line| line
        == if shared == "true" {
            "Shared experience: enabled"
        } else {
            "Shared experience: disabled"
        }));
    assert!(has_line(&text, &format!("Leader: {inviter}")));
    assert!(has_line(&text, &format!("Member: {joiner}")));
    assert!(!text.contains("Invitation:"));

    if !replay.live {
        let before = panel.state.clone();
        assert!(request(&mut panel, Intent::Refresh).is_some());
        assert!(!apply_result(
            &mut panel,
            &response(CommandStatus::Accepted, &replay.joined_op[..1], None)
        ));
        assert_eq!(panel.state, before);
        assert_eq!(
            panel.feedback,
            Some("The server returned an invalid party result")
        );
        assert!(request(&mut panel, Intent::Refresh).is_some());
        let refused = PartyResponse {
            status: CommandStatus::Rejected,
            operation: replay.joined_op.clone(),
            view: None,
        };
        assert!(!apply_result(&mut panel, &refused));
        assert_eq!(panel.state, before);
        assert_eq!(panel.feedback, Some("The server refused the party command"));
    }
    assert!(request(&mut panel, Intent::Refresh).is_some());
    assert!(apply_result(
        &mut panel,
        &response(
            replay.statuses[2],
            &replay.passed_op,
            Some(&replay.passed_view)
        )
    ));
    assert!(panel.is_leader());
    let new_leader = value(&replay.expected, "new_leader")?;
    assert_eq!(new_leader, joiner);
    let text = panel_text(&panel);
    assert!(has_line(&text, &format!("Leader: {joiner}")));
    assert!(has_line(&text, &format!("Member: {inviter}")));
    Ok(())
}
