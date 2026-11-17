use super::*;
use oteryn_client_ui::personal_map_marks::{PersonalMapMark, PersonalMapMarks};
#[test]
fn personal_map_marks_native_query_publishes_only_confirmed_sets()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    outbox.personal_map_marks_initialize();
    assert!(
        matches!(link.poll(),Some(PlayEvent::Pushed(p)) if p.personal_map_marks==Some(PersonalMapMarks::default()))
    );
    let expected = PersonalMapMarks(vec![PersonalMapMark {
        key: "sterling".into(),
        x: 31948,
        y: 31924,
        floor: -7,
        text: "Treasure".into(),
    }]);
    let mut result = oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: expected.encode().ok_or("encode")?,
    };
    outbox.personal_map_marks_result(&result)?;
    result.status = CommandStatus::DuplicateReplay;
    outbox.personal_map_marks_result(&result)?;
    assert!(
        matches!(link.poll(),Some(PlayEvent::Pushed(p)) if p.personal_map_marks==Some(expected))
    );
    result.status = CommandStatus::Rejected;
    result.payload.clear();
    outbox.personal_map_marks_result(&result)?;
    assert!(link.poll().is_none());
    result.status = CommandStatus::Accepted;
    result.payload = vec![255];
    assert!(outbox.personal_map_marks_result(&result).is_err());
    assert!(link.poll().is_none());
    Ok(())
}
