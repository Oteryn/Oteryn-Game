use crate::play::play_channel;
use oteryn_client_runtime::task_board::task_board::full::*;
fn reply() -> Reply {
    Reply {
        refusal: 0,
        period: 42,
        points: 0,
        bounty_points: 0,
        soulseals: 0,
        weekly: Weekly {
            started: true,
            difficulty: 0,
            expanded: false,
            multiplier: 1,
            kill_experience: 0,
            delivery_experience: 0,
            pending_points: 0,
            pending_soulseals: 0,
            kills: vec![KillTask {
                race: "oteryn:creature.tracker_fox".into(),
                required: 10,
                current: 3,
            }],
            deliveries: vec![DeliveryTask {
                item: 9,
                required: 4,
                current: 4,
                delivered: false,
            }],
        },
        bounty: Bounty {
            state: 2,
            difficulty: 0,
            selected: 1,
            current: 2,
            rerolls: 0,
            daily_reroll_at: 0,
            offers: vec![],
            active: Some(BountyOffer {
                race: "oteryn:creature.tracker_wolf".into(),
                required: 9,
                experience: 0,
                points: 0,
                grade: 0,
            }),
            lists: vec![],
        },
        talisman: [TalismanPath::default(); 4],
        shop: vec![],
    }
}
#[test]
fn analyser_hunting_task_tracker_reads_real_weekly_query_and_retains_refused_progress()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.task_board_initialize(true);
    assert!(link.analyser_select("kills"));
    outbox.push(vec![oteryn_session::SessionEvent::AnalyserFacts(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: vec![oteryn_session::analyser_transport::AnalyserFact::Kill {
                race: std::num::NonZeroU32::new(7),
            }],
        },
    )]);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let tools = oteryn_client_ui::analyser::Tool::ALL.len();
        let submit = crate::play::feature_0010_analyser::input::submit;
        submit(
            &mut mailbox,
            &link.commands,
            tools + 10,
            vec![String::new(), "7".into(), "2".into()],
        );
        assert_eq!(mailbox.analyser.kill_goals.get(&7), Some(&2));
        submit(&mut mailbox, &link.commands, tools + 12, vec![]);
    }
    assert!(!link.task_board_panel().open);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("task query".into());
    };
    assert_eq!((command.capability, command.kind), (72, 89));
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|_| "request")?
            .action,
        4
    );
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_reply(&reply()).map_err(|_| "reply")?,
    });
    let text = oteryn_client_ui::analyser::panel_text(&link.analyser_panel());
    assert!(text.contains("Week 42 · kill 1 · tracker fox: 3/10 · active"));
    assert!(text.contains("delivery item #9 (available for delivery): 4/4 · active"));
    assert!(text.contains("Bounty · tracker wolf: 2/9 · active"));
    assert!(text.contains("tracker fox: 3/10 · active · 7 remaining"));
    assert!(text.contains("tracker wolf: 2/9 · active · 7 remaining"));
    assert!(text.contains("Race #7: 1 / 2 · 1 remaining"));
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        crate::play::feature_0010_analyser::input::submit(
            &mut mailbox,
            &link.commands,
            oteryn_client_ui::analyser::Tool::ALL.len() + 11,
            vec![String::new(), "7".into()],
        );
        assert!(mailbox.analyser.kill_goals.is_empty());
    }
    let confirmed = link.analyser_panel().task_progress;
    link.analyser_refresh_tasks();
    commands.try_recv()?;
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![255],
    });
    assert_eq!(link.analyser_panel().task_progress, confirmed);
    assert!(link.analyser_panel().task_feedback.is_some());
    link.analyser_refresh_tasks();
    commands.try_recv()?;
    let mut refused = reply();
    refused.refusal = 13;
    refused.weekly.kills[0].current = 9;
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 3,
        server_sequence: 3,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_reply(&refused).map_err(|_| "refusal")?,
    });
    assert_eq!(link.analyser_panel().task_progress, confirmed);
    link.analyser_refresh_tasks();
    commands.try_recv()?;
    let mut complete = reply();
    complete.weekly.kills[0].current = 10;
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 4,
        server_sequence: 4,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_reply(&complete).map_err(|_| "complete")?,
    });
    assert!(link.analyser_text().contains("tracker fox: 10/10 · complete · 0 remaining"));
    outbox.logged_out();
    assert!(link.analyser_panel().task_progress.is_none());
    Ok(())
}
