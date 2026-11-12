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
    outbox.boss_difficulty_initialize(true, false);
    lock(&link.mailbox).boss_difficulty.boss_key = "oteryn:dragon".into();
    link.boss_difficulty_request(Intent::Open);
    let Ok(PlayCommand::Feature(command)) = incoming.try_recv() else {
        return Err("open command".into());
    };
    assert_eq!((command.capability, command.kind), (225, 431));
    assert_eq!(
        decode_request(&command.payload),
        Ok(Request::OpenWithProgress {
            boss_key: "oteryn:dragon".into()
        })
    );
    let window = Reply::Window {
        context: [7; 16],
        boss_key: "oteryn:dragon".into(),
        boss_creature_key: "oteryn:dragon".into(),
        content_revision: "r1".into(),
        selected_difficulty: 7,
        personal_highest: 0,
        unlocked_max: 1,
        maximum_difficulty: 25,
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

#[test]
fn difficulty_unlock_state_reaches_play_queue_and_archfoe_view()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.boss_difficulty_initialize(true, false);
    lock(&link.mailbox).boss_difficulty.boss_key = "oteryn:boss.mimar_haffar".into();
    link.boss_difficulty_request(Intent::Open);
    assert!(matches!(incoming.try_recv(), Ok(PlayCommand::Feature(_))));
    let payload = encode_reply(&Reply::Window {
        context: [7; 16],
        boss_key: "oteryn:boss.mimar_haffar".into(),
        boss_creature_key: "oteryn:creature.mimar_haffar".into(),
        content_revision: "r1".into(),
        selected_difficulty: 1,
        personal_highest: 3,
        unlocked_max: 4,
        maximum_difficulty: 25,
    })
    .map_err(|e| format!("{e:?}"))?;

    outbox.boss_difficulty_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
    lock(&link.mailbox).boss_encounter.fields[0] = "oteryn:creature.mimar_haffar".into();
    lock(&link.mailbox).boss_encounter.browser_category =
        oteryn_client_ui::boss_encounter::BossCategory::Archfoe;
    let mailbox = lock(&link.mailbox);
    assert!(
        oteryn_client_ui::boss_difficulty::panel_text(&mailbox.boss_difficulty)
            .contains("Personal highest: 3")
    );
    assert!(
        oteryn_client_ui::boss_encounter::browser_text(&mailbox.boss_encounter)
            .contains("Unlocked difficulty: 4")
    );
    drop(mailbox);
    lock(&link.mailbox).boss_encounter.fields[0] = "oteryn:creature.other".into();
    link.boss_encounter_difficulty_request(Intent::Start, 1);
    assert!(
        incoming.try_recv().is_err(),
        "a different boss cannot borrow the confirmed context"
    );
    lock(&link.mailbox).boss_encounter.fields[0] = "oteryn:creature.mimar_haffar".into();
    link.boss_encounter_difficulty_request(Intent::Start, 1);
    assert!(
        matches!(incoming.try_recv(), Ok(PlayCommand::Feature(c)) if decode_request(&c.payload) == Ok(Request::Start { context: [7;16], difficulty: 1 }))
    );
    Ok(())
}

#[test]
fn c0492_pity_window_through_the_boss_caller_reaches_the_window_and_clears_on_a_plain_window()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::boss_difficulty::{PityTracker, encode_reply_with_pity};
    let (link, _incoming, outbox) = play_channel();
    outbox.boss_encounter_initialize(true);
    outbox.boss_difficulty_initialize(true, true);
    let window = Reply::Window {
        context: [3; 16],
        boss_key: "oteryn:moonsilver".into(),
        boss_creature_key: "oteryn:moonsilver_creature".into(),
        content_revision: "r1".into(),
        selected_difficulty: 1,
        personal_highest: 0,
        unlocked_max: 1,
        maximum_difficulty: 25,
    };
    let result = |payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    };
    let tracker = PityTracker {
        kills_without_drop: 40,
        bonus_permille: 200,
        tracker_active: true,
    };
    outbox.boss_encounter_result(&result(encode_reply_with_pity(&window, Some(&tracker)).map_err(|_| "encode")?));
    {
        let mailbox = lock(&link.mailbox);
        assert_eq!(mailbox.boss_difficulty.pity, Some(tracker));
        assert_eq!(
            mailbox.boss_encounter.pity_windows.get("oteryn:moonsilver"),
            Some(&tracker)
        );
        assert_eq!(
            mailbox
                .boss_encounter
                .pity_windows
                .get("oteryn:moonsilver_creature"),
            Some(&tracker)
        );
        assert!(!mailbox.boss_encounter.pending);
    }
    outbox.boss_encounter_result(&result(encode_reply(&window).map_err(|_| "encode")?));
    let mailbox = lock(&link.mailbox);
    assert_eq!(mailbox.boss_difficulty.pity, None);
    assert!(mailbox.boss_encounter.pity_windows.is_empty());
    Ok(())
}
