use crate::play::play_channel;
use oteryn_session::{AppliedDelta, SessionEvent, analyser_transport::AnalyserFact};
#[path = "bosses.rs"]
mod boss_tracker_tests;
#[path = "party_import.rs"]
mod party_summary_tests;
#[test]
fn analyser_push_reaches_closed_panel_and_end_clears_totals() {
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::Kill { race: None }],
    })]);
    assert_eq!(link.analyser_panel().kills, 1);
    assert!(!link.analyser_panel().open);
    outbox.logged_out();
    assert_eq!(link.analyser_panel().kills, 0);
    assert!(!link.analyser_panel().selected);
}

#[test]
fn analyser_selector_projects_session_facts_without_resetting_other_tools() {
    use oteryn_client_ui::analyser::{Tool, panel_text};
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::Kill { race: None },
            AnalyserFact::Experience { raw: 7, gained: 9 },
        ],
    })]);
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, 2);
    assert_eq!(mailbox.analyser.tool, Tool::Experience);
    assert!(panel_text(&mailbox.analyser).contains("Experience: 7 raw / 9 gained"));
    assert!(!panel_text(&mailbox.analyser).contains("Kills:"));
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, 1);
    assert!(panel_text(&mailbox.analyser).contains("Kills: 1"));
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, 7);
    assert_eq!(mailbox.analyser.kills, 0);
    assert_eq!(mailbox.analyser.tool, Tool::Hunting);
}

#[test]
fn analyser_item_query_and_reply_use_the_production_command_mailbox()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{
        CommandStatus,
        analyser_catalogue::{ItemInfo, encode_reply},
        analyser_transport::AnalyserItemCount,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![AnalyserItemCount {
                item_type: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU16::MIN,
            }],
            gold: 1,
        }],
    })]);
    link.analyser_resolve_items();
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.kind), (10, 734));
    assert_eq!(
        oteryn_session::analyser_catalogue::decode_request(&command.payload)?,
        vec![1]
    );
    outbox.analyser_catalogue_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 9,
        payload: encode_reply(&[ItemInfo {
            item_type: 1,
            name: "Gold piece".into(),
            coin_worth: 1,
        }])?,
    });
    assert_eq!(
        link.analyser_panel()
            .item_info
            .get(&1)
            .map(|info| info.coin_worth),
        Some(1)
    );
    assert!(link.analyser_panel().catalogue_pending.is_empty());
    Ok(())
}

#[test]
fn merged_analyser_controls_keep_selectors_prices_resolve_and_reset_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::analyser::Tool;
    let (link, mut incoming, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::Kill { race: None },
            AnalyserFact::SupplyUsed(oteryn_session::analyser_transport::AnalyserItemCount {
                item_type: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU16::MIN,
            }),
        ],
    })]);
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    let controls = crate::play::feature_0010_analyser::input::submit;
    controls(&mut mailbox, &link.commands, 4, vec![]);
    assert_eq!(mailbox.analyser.tool, Tool::Supplies);
    controls(
        &mut mailbox,
        &link.commands,
        Tool::ALL.len() + 2,
        vec![String::new(), "1".into(), "7".into()],
    );
    assert_eq!(mailbox.analyser.supply_prices.get(&1), Some(&7));
    controls(&mut mailbox, &link.commands, Tool::ALL.len() + 3, vec![]);
    assert!(
        matches!(incoming.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if command.kind == 734)
    );
    controls(&mut mailbox, &link.commands, 7, vec![]);
    assert_eq!(mailbox.analyser.tool, Tool::Impact);
    assert_eq!(mailbox.analyser.kills, 1);
    controls(&mut mailbox, &link.commands, Tool::ALL.len(), vec![]);
    assert_eq!(mailbox.analyser.kills, 0);
    assert_eq!(mailbox.analyser.tool, Tool::Impact);
    Ok(())
}

#[test]
fn analyser_session_export_writes_confirmed_report_through_native_action()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    let path = std::env::temp_dir().join(format!(
        "oteryn-hunt-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    assert!(link.analyser_save_session(&path).is_err());
    assert!(!path.exists());
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::Kill { race: None },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 12,
            },
        ],
    })]);
    std::fs::write(&path, "old report")?;
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        crate::play::feature_0010_analyser::input::submit(
            &mut mailbox,
            &link.commands,
            oteryn_client_ui::analyser::Tool::ALL.len() + 4,
            vec![
                String::new(),
                String::new(),
                String::new(),
                path.to_string_lossy().into_owned(),
            ],
        );
        assert_eq!(mailbox.analyser.feedback, Some("Session report exported"));
    }
    let report = std::fs::read_to_string(&path)?;
    std::fs::remove_file(path)?;
    for expected in [
        "Hunting Session",
        "Kills: 1",
        "Priced balance: 12 gold",
        "Race unavailable: 1",
        "Loot Analyser",
        "Supply Analyser",
    ] {
        assert!(report.contains(expected), "missing {expected}: {report}");
    }
    assert!(!report.contains("old report"));
    Ok(())
}

#[test]
fn analyser_party_query_and_confirmed_table_use_production_mailbox()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::analyser::party::{Member, decode_request, encode_reply};
    let (link, mut commands, outbox) = play_channel();
    outbox.analyser_initialize(true);
    link.analyser_party_refresh();
    assert!(commands.try_recv().is_err());
    crate::play::mailbox::lock(&link.mailbox).party.selected = true;
    link.analyser_party_refresh();
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("party query".into());
    };
    assert_eq!((command.capability, command.kind), (10, 735));
    assert!(decode_request(&command.payload)?.is_empty());
    let result = oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_reply(&[
            Member {
                handle: 1,
                name: "Scout".into(),
                available: true,
                loot_value: 100,
                ..Default::default()
            },
            Member {
                handle: 2,
                name: "Healer".into(),
                available: true,
                supply_cost: 20,
                ..Default::default()
            },
        ])?,
    };
    outbox.analyser_result(&command, &result);
    assert!(link.analyser_select("party"));
    let text = oteryn_client_ui::analyser::panel_text(&link.analyser_panel());
    assert!(text.contains("Scout pays Healer: 60 gold"));
    assert!(!link.analyser_panel().party_pending);
    link.party_request(oteryn_client_ui::party::Intent::Refresh);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Party(_)
    ));
    let party_view = oteryn_session::party::encode_party_result(
        &oteryn_session::party::PartyResult::refused(oteryn_session::party::PartyDisposition::Ok),
    )
    .map_err(|_| "party view fixture")?;
    outbox.party_result(&oteryn_client_runtime::party::PartyResponse {
        status: oteryn_session::CommandStatus::Accepted,
        operation: party_view.clone(),
        view: Some(party_view),
    });
    assert!(link.analyser_panel().party.is_empty());
    assert_eq!(
        link.analyser_panel().party_feedback,
        Some("Party changed; refresh party accounting")
    );
    outbox.logged_out();
    assert!(link.analyser_panel().party.is_empty());
    Ok(())
}

#[test]
fn analyser_loot_tracking_controls_filter_real_confirmed_drops() {
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![oteryn_session::analyser_transport::AnalyserItemCount {
                item_type: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU16::MIN,
            }],
            gold: 0,
        }],
    })]);
    assert!(link.analyser_select("loot-tracker"));
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    let submit = crate::play::feature_0010_analyser::input::submit;
    let tools = oteryn_client_ui::analyser::Tool::ALL.len();
    submit(
        &mut mailbox,
        &link.commands,
        tools + 7,
        vec![String::new(), "1".into(), String::new()],
    );
    submit(&mut mailbox, &link.commands, tools + 9, vec![]);
    let text = oteryn_client_ui::analyser::panel_text(&mailbox.analyser);
    assert!(text.contains("#1 Name unresolved (tracked) · 1 confirmed items"));
    assert!(text.contains("#1 × 1"));
    submit(
        &mut mailbox,
        &link.commands,
        tools + 8,
        vec![String::new(), "1".into(), String::new()],
    );
    assert!(!oteryn_client_ui::analyser::panel_text(&mailbox.analyser).contains("#1"));
}

#[path = "tasks.rs"]
mod task_tracker;

#[test]
fn auxiliary_analyser_launch_preserves_session_owner_and_existing_measurements() {
    let (link, _incoming, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::Kill { race: None }],
    })]);
    let handles = std::sync::Arc::strong_count(&link.mailbox);
    assert!(crate::play::feature_0010_analyser::view::open_shared(
        &link.mailbox,
        &link.commands
    ));
    assert!(!crate::play::feature_0010_analyser::view::open_shared(
        &link.mailbox,
        &link.commands
    ));
    {
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        assert!(!mailbox.cyclopedia_character_shutdown);
    }
    assert_eq!(std::sync::Arc::strong_count(&link.mailbox), handles);
    assert_eq!(link.analyser_panel().kills, 1);
    assert!(link.analyser_panel().open && link.analyser_panel().selected);
    outbox.logged_out();
    assert!(!link.analyser_panel().open);
}
