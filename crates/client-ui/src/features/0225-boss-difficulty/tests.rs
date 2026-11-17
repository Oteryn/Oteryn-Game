use super::*;
use oteryn_client_runtime::{
    boss_difficulty::{Reply, Request, decode_request, encode_reply},
    raw_feature::{CommandStatus, GatedCommandResult},
};
#[test]
fn server_window_drives_difficulty_and_closed_context_cannot_start() -> Result<(), &'static str> {
    let mut p = BossDifficultyPanel {
        selected: true,
        ..Default::default()
    };
    let result = |reply| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_reply(&reply).unwrap_or_default(),
    };
    assert!(apply_result(
        &mut p,
        &result(Reply::Window {
            context: [1; 16],
            boss_key: "oteryn:dragon".into(),
            boss_creature_key: "oteryn:dragon".into(),
            content_revision: "r1".into(),
            selected_difficulty: 7,
            personal_highest: 0,
            unlocked_max: 1,
            maximum_difficulty: 25,
        })
    ));
    let preview = request(&mut p, Intent::Preview).ok_or("preview")?;
    assert_eq!(
        decode_request(&preview.payload),
        Ok(Request::Preview {
            context: [1; 16],
            difficulty: 7
        })
    );
    assert!(apply_result(
        &mut p,
        &result(Reply::Closed { context: [1; 16] })
    ));
    assert!(p.context.is_none());
    assert!(request(&mut p, Intent::Start).is_none());
    Ok(())
}
