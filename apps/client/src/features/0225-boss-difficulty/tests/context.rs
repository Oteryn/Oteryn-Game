use crate::combat_input::PlayCommand;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_runtime::{
    boss_difficulty::{Refusal, Reply, Request, decode_request, encode_reply},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::boss_difficulty::Intent;

#[test]
fn difficulty_real_caller_uses_server_context_and_invalidates_refused_or_reopened_windows()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.boss_difficulty_initialize(true);
    lock(&link.mailbox).boss_difficulty.boss_key = "oteryn:dragon".into();
    link.boss_difficulty_request(Intent::Open);
    let Ok(PlayCommand::Feature(command)) = incoming.try_recv() else {
        return Err("open command".into());
    };
    assert_eq!((command.capability, command.kind), (225, 431));
    assert_eq!(
        decode_request(&command.payload),
        Ok(Request::Open {
            boss_key: "oteryn:dragon".into()
        })
    );
    let window = Reply::Window {
        context: [7; 16],
        boss_key: "oteryn:dragon".into(),
        content_revision: "r1".into(),
        selected_difficulty: 7,
    };
    let reply = |reply: &Reply| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_reply(reply).unwrap_or_default(),
    };
    outbox.boss_difficulty_result(&reply(&window));
    link.boss_difficulty_request(Intent::Preview);
    let Ok(PlayCommand::Feature(command)) = incoming.try_recv() else {
        return Err("preview command".into());
    };
    assert_eq!(
        decode_request(&command.payload),
        Ok(Request::Preview {
            context: [7; 16],
            difficulty: 7
        })
    );
    outbox.boss_difficulty_result(&reply(&Reply::Refused {
        context: Some([7; 16]),
        reason: Refusal::ContextClosed,
    }));
    link.boss_difficulty_request(Intent::Start);
    assert!(
        incoming.try_recv().is_err(),
        "closed context must not start"
    );
    outbox.boss_difficulty_result(&reply(&window));
    lock(&link.mailbox).boss_difficulty.boss_key = "oteryn:other".into();
    link.boss_difficulty_request(Intent::Open);
    assert!(matches!(incoming.try_recv(), Ok(PlayCommand::Feature(_))));
    assert!(
        lock(&link.mailbox).boss_difficulty.context.is_none(),
        "new boss must discard old context immediately"
    );
    outbox.boss_difficulty_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Rejected,
        payload: vec![],
    });
    link.boss_difficulty_request(Intent::Start);
    assert!(
        incoming.try_recv().is_err(),
        "failed open must not start previous boss"
    );
    Ok(())
}
