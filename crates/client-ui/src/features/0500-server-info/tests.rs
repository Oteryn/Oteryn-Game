use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn server_info_query_reply_is_bound_to_the_requested_owner() -> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let command = request(&mut panel, Query::Quests).ok_or("request")?;
    assert_eq!(Query::decode(&command.payload), Some(Query::Quests));
    let reply = Reply::Quests(vec![(73, "Rat Investigation".into())]);
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: reply.encode().ok_or("encode")?,
    };
    assert_eq!(
        apply_result(&mut panel, &Query::Quests, &result),
        Some(reply)
    );
    assert!(apply_result(&mut panel, &Query::Quests, &result).is_none());
    assert!(request(&mut Panel::default(), Query::Quests).is_none());
    Ok(())
}
