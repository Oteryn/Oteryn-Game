use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_runtime::task_board::task_board::soulpit as wire;
#[test]
fn t114_fight_dialog_uses_server_catalogue_costs_authorization_and_confirmed_spend()
-> Result<(), Error> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.task_board_initialize(true);
    link.soulseal_catalogue_request(0);
    let PlayCommand::Feature(query) = incoming.try_recv()? else {
        return Err("catalogue request".into());
    };
    assert_eq!((query.capability, query.kind), (72, wire::COMMAND_TYPE));
    assert_eq!(
        wire::Request::decode(&query.payload).map_err(|_| "request")?,
        wire::Request::Catalogue { after: 0 }
    );
    link.soulseal_catalogue_request(0);
    assert!(incoming.try_recv().is_err());
    assert!(!link.soulseal_select(5));
    let answer = |command: &oteryn_client_runtime::raw_feature::FeatureCommand,
                  reply: &wire::Reply|
     -> Result<(), Error> {
        outbox.soulseal_result(
            command,
            &oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: reply.encode().map_err(|_| "reply")?,
            },
        );
        Ok(())
    };
    let catalogue = wire::Reply {
        status: wire::Status::Catalogue,
        balance: Some(37),
        authorization: Some([7; 16]),
        next: None,
        total: 2,
        rules: wire::EntryRules {
            required_level: 8,
            time_limit_seconds: 600,
            authorization_ms: 10000,
        },
        entries: vec![
            wire::Fight {
                race: 5,
                name: "Grove watcher".into(),
                stars: 2,
                cost: 30,
            },
            wire::Fight {
                race: 6,
                name: "Moss guardian".into(),
                stars: 5,
                cost: 60,
            },
        ],
        reason: String::new(),
    };
    answer(&query, &catalogue)?;
    let text = oteryn_client_ui::task_board::soulseals_text(&link.task_board_panel());
    assert!(text.contains("Grove watcher"));
    assert!(text.contains("30 Soulseals"));
    assert!(text.contains("Level 8"));
    assert!(
        incoming.try_recv().is_err(),
        "loading balance and offers must never start a fight"
    );
    assert!(link.soulseal_select(6));
    link.soulseal_fight_selected();
    assert!(incoming.try_recv().is_err(), "unaffordable choice");
    assert!(link.soulseal_select(5));
    link.soulseal_fight_selected();
    let PlayCommand::Feature(entry) = incoming.try_recv()? else {
        return Err("entry".into());
    };
    assert_eq!(
        wire::Request::decode(&entry.payload).map_err(|_| "entry")?,
        wire::Request::Enter {
            authorization: [7; 16],
            race: 5
        }
    );
    assert!(
        oteryn_client_ui::task_board::soulseals_text(&link.task_board_panel())
            .contains("Balance: 37"),
        "no optimistic debit"
    );
    link.soulseal_fight_selected();
    assert!(incoming.try_recv().is_err());
    answer(&query, &catalogue)?;
    assert!(
        link.task_board_panel().fights.pending.is_some(),
        "stale catalogue callback cannot complete entry"
    );
    let started = wire::Reply {
        status: wire::Status::Started,
        balance: Some(7),
        authorization: None,
        next: None,
        total: 0,
        rules: catalogue.rules.clone(),
        entries: vec![],
        reason: String::new(),
    };
    answer(&entry, &started)?;
    assert!(
        oteryn_client_ui::task_board::soulseals_text(&link.task_board_panel())
            .contains("Balance: 7")
    );
    link.soulseal_fight_selected();
    assert!(incoming.try_recv().is_err(), "one-use authorization");
    link.soulseal_catalogue_request(0);
    let PlayCommand::Feature(query) = incoming.try_recv()? else {
        return Err("refresh".into());
    };
    answer(&query, &catalogue)?;
    assert!(link.soulseal_select(5));
    crate::play::mailbox::lock(&link.mailbox)
        .task_board
        .fights
        .received_at = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(11));
    link.soulseal_fight_selected();
    assert!(incoming.try_recv().is_err(), "expired authorization");
    outbox.logged_out();
    link.soulseal_catalogue_request(0);
    assert!(incoming.try_recv().is_err());
    assert!(link.task_board_panel().fights.reply.is_none());
    Ok(())
}
