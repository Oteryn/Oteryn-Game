use crate::play::play_channel;
use oteryn_session::{AppliedDelta, SessionEvent, analyser_transport::AnalyserFact};
#[path = "bosses.rs"]
mod boss_tracker_tests;
#[path = "outgoing.rs"]
mod outgoing_view_tests;
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
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("oteryn-hunt.txt");
    assert!(link.analyser_save_session(&path).is_err());
    assert!(!path.exists());
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::Kill { race: None },
            AnalyserFact::Experience {
                raw: 29,
                gained: 37,
            },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 12,
            },
        ],
    })]);
    std::fs::write(&path, "old report")?;
    {
        crate::play::feature_0010_analyser::input::submit_shared(
            &link.mailbox,
            &link.commands,
            oteryn_client_ui::analyser::Tool::ALL.len() + 4,
            vec![
                String::new(),
                String::new(),
                String::new(),
                path.to_string_lossy().into_owned(),
            ],
        );
        assert_eq!(
            crate::play::mailbox::lock(&link.mailbox).analyser.feedback,
            Some("Session report exported")
        );
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
        "Session gained XP per minute",
        "Session raw XP per minute",
        "peak 37",
        "peak 29",
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
    assert!(text.contains("balance"));
    assert!(text.contains("Scout | 0 | 0 | 0 | 0/100 | 0/0 | 100 | 0/0"));
    assert!(text.contains("Healer | 0 | 0 | 0 | 0/0 | 0/20 | -20 | 0/0"));
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
    let filtered = oteryn_client_ui::analyser::panel_text(&mailbox.analyser);
    assert!(!filtered.contains("#1"));
    assert!(filtered.contains("No recent drops match the tracked item selection"));
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

#[path = "activity.rs"]
mod activity;

#[cfg(bevy_shell)]
impl crate::play::Outbox {
    pub(crate) fn bevy_analyser_catalogue_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        self.analyser_catalogue_result(result);
    }
}

#[test]
fn analyser_reset_isolates_every_report_through_playlink_and_fact_feed()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::analyser::AnalyserImpact;
    use oteryn_client_ui::analyser::Tool;
    use std::num::{NonZeroU16, NonZeroU32};
    let mut facts = vec![
        AnalyserFact::Kill {
            race: Some(NonZeroU32::MIN),
        },
        AnalyserFact::Experience { raw: 7, gained: 9 },
        AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![oteryn_session::analyser_transport::AnalyserItemCount {
                item_type: NonZeroU32::MIN,
                count: NonZeroU16::MIN,
            }],
            gold: 11,
        },
        AnalyserFact::SupplyUsed(oteryn_session::analyser_transport::AnalyserItemCount {
            item_type: NonZeroU32::MIN,
            count: NonZeroU16::MIN,
        }),
        AnalyserFact::Impact(AnalyserImpact::Healing(NonZeroU32::MIN)),
    ];
    facts.extend(
        oteryn_client_runtime::analyser::decode_analyser_facts(&[
            10, 8, 34, 6, 8, 1, 16, 120, 24, 2, 10, 6, 34, 4, 8, 2, 16, 60, 10, 10, 42, 8, 8, 60,
            16, 2, 24, 1, 32, 7,
        ])
        .map_err(|_| "impact wire")?,
    );
    for reset in Tool::ALL
        .into_iter()
        .filter(|t| !matches!(t, Tool::Party | Tool::Bosses))
    {
        let (link, _, outbox) = play_channel();
        outbox.analyser_initialize(true);
        crate::play::mailbox::lock(&link.mailbox).analyser.open = true;
        let push = |seq| {
            outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
                server_sequence: seq,
                base_revision: seq,
                new_revision: seq + 1,
                value: facts.clone(),
            })])
        };
        push(1);
        let mut before = Vec::new();
        for tool in Tool::ALL {
            assert!(link.analyser_select(tool.key()));
            before.push(link.analyser_text());
        }
        assert!(link.analyser_select(reset.key()));
        link.analyser_reset();
        for (tool, expected) in Tool::ALL.into_iter().zip(&before) {
            assert!(link.analyser_select(tool.key()));
            let session = |t| matches!(t, Tool::Overview | Tool::Hunting | Tool::Session);
            if tool != reset && !(session(reset) && session(tool)) {
                assert_eq!(
                    &link.analyser_text(),
                    expected,
                    "reset {reset:?} changed {tool:?}"
                );
            }
        }
        if matches!(reset, Tool::Overview | Tool::Hunting | Tool::Session) {
            assert!(link.analyser_select("loot-tracker"));
            link.analyser_track_item("1", true);
            assert!(link.analyser_panel().loot_tracked.contains(&1));
            crate::play::feature_0010_analyser::input::submit(
                &mut crate::play::mailbox::lock(&link.mailbox),
                &link.commands,
                Tool::ALL.len() + 10,
                vec![String::new(), "1".into(), "10".into()],
            );
            assert_eq!(link.analyser_panel().kill_goals.get(&1), Some(&10));
        }
        push(2);
        assert!(link.analyser_select(reset.key()));
        // A reset starts a new interval which keeps consuming the real feed.
        assert_eq!(
            link.analyser_text(),
            before[Tool::ALL.iter().position(|t| *t == reset).ok_or("tool")?]
        );
    }
    Ok(())
}

#[test]
fn reset_minute_charts_hunting() {
    check_reset_chart(oteryn_client_ui::analyser::Tool::Hunting);
}

#[test]
fn reset_minute_charts_loot_tracker() {
    check_reset_chart(oteryn_client_ui::analyser::Tool::LootTracker);
}

#[test]
fn reset_minute_charts_xp() {
    check_reset_chart(oteryn_client_ui::analyser::Tool::Xp);
}

fn check_reset_chart(tool: oteryn_client_ui::analyser::Tool) {
    use oteryn_client_ui::analyser::Tool;
    use oteryn_session::analyser_transport::AnalyserItemCount;
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    let clock = |second| {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.analyser.clock =
            (std::time::Instant::now() - std::time::Duration::from_secs(second)).into();
        mailbox.analyser.open = true;
    };
    let start = if tool == Tool::Xp { 58 } else { 59 * 60 };
    clock(start - 1);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::Experience {
            raw: 90,
            gained: 100,
        }],
    })]);
    clock(start);
    assert!(link.analyser_select(tool.key()));
    link.analyser_reset();
    clock(start + 1);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 2,
        base_revision: 2,
        new_revision: 3,
        value: vec![
            AnalyserFact::Kill { race: None },
            AnalyserFact::Experience {
                raw: 11,
                gained: 15,
            },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![AnalyserItemCount {
                    item_type: std::num::NonZeroU32::MIN,
                    count: std::num::NonZeroU16::MIN,
                }],
                gold: 0,
            },
        ],
    })]);
    clock(start + 2);
    let mut text = link.analyser_text();
    if tool == Tool::Hunting {
        text.push_str(&oteryn_client_ui::analyser::session_text(
            &link.analyser_panel(),
        ));
    }
    if tool == Tool::Xp {
        assert!(
            text.lines()
                .any(|line| line == "Experience: 11 raw / 15 gained"),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line == "Elapsed: 2 seconds"),
            "{text}"
        );
    }
    let expected = if tool == Tool::Xp { "█▁" } else { "█" };
    let bars: Vec<_> = text
        .lines()
        .filter(|line| line.contains('█') && line.chars().all(|c| ('▁'..='█').contains(&c)))
        .collect();
    assert!(
        bars.len()
            >= match tool {
                Tool::Hunting => 3,
                Tool::Xp => 2,
                _ => 1,
            },
        "{text}"
    );
    assert!(bars.iter().all(|line| *line == expected), "{text}");
}
