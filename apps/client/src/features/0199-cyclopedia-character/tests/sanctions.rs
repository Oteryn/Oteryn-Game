use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    raw_feature::{CommandStatus, GatedCommandResult},
    sanction_record::{SanctionRecord, encode_reply},
};

#[test]
fn sanction_window_requests_only_when_selected_and_shows_the_reply()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    link.sanctions_request();
    assert!(incoming.try_recv().is_err());
    outbox.sanction_initialize(true);
    link.sanctions_request();
    let command = incoming.try_recv()?;
    assert!(
        matches!(command, PlayCommand::Feature(c) if c.capability == 497 && c.kind == 6240 && c.payload.is_empty())
    );
    let payload = encode_reply(&SanctionRecord {
        more: false,
        entries: vec![],
    })
    .ok_or("encode reply")?;
    outbox.sanction_record_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
    let panel = link.sanctions_panel();
    assert!(!panel.pending);
    assert!(panel.record.is_some());
    // A new session starts without the record and without the selection.
    outbox.sanction_initialize(false);
    assert!(link.sanctions_panel().record.is_none());
    link.sanctions_request();
    assert!(incoming.try_recv().is_err());
    Ok(())
}

#[test]
fn activity_handoff_releases_cancelled_sanction_request() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut incoming, outbox) = play_channel();
    outbox.sanction_initialize(true);
    link.sanctions_request();
    assert!(link.sanctions_panel().pending);
    use oteryn_session::activity_scope::{
        ActivityScope, ActivityScopeSnapshot, STATE_DOMAIN_ACTIVITY_SCOPE, encode_activity_scope,
    };
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut successor = id;
    successor[15] = 2;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session: successor,
        scope: ActivityScope::Instance {
            world: id,
            instance: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![7, 80, 117, 493, 494, 495, 496, 497],
        requires_ack: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![oteryn_session::SessionEvent::GatedSnapshotReplaced(
        oteryn_session::AppliedDelta {
            server_sequence: 3,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: STATE_DOMAIN_ACTIVITY_SCOPE,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        },
    )]);
    assert_eq!(
        outbox.death_command(&mut incoming),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    );
    assert!(!link.sanctions_panel().pending);
    link.sanctions_request();
    assert!(
        matches!(outbox.death_command(&mut incoming)?, PlayCommand::Feature(c) if c.capability == 497 && c.kind == 6240)
    );
    Ok(())
}
