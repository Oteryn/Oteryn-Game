use super::*;
use crate::play::{feature_0010_analyser::input::submit, mailbox::lock};
use oteryn_client_runtime::{
    analyser::party::{self as wire, Member},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[test]
fn party_summary_file_io_is_bounded_advisory_and_keeps_live_members_untouched()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.analyser_initialize(true);
    let rows = vec![
        Member {
            handle: 9,
            name: "Alice".into(),
            available: true,
            loot_value: 1000,
            supply_cost: 100,
            ..Default::default()
        },
        Member {
            handle: 7,
            name: "Bob".into(),
            available: true,
            supply_cost: 300,
            ..Default::default()
        },
    ];
    lock(&link.mailbox).analyser.party_pending = true;
    outbox.analyser_result(
        &FeatureCommand {
            capability: 10,
            kind: wire::COMMAND_TYPE_ANALYSER_PARTY_QUERY,
            payload: wire::encode_request(&[])?,
        },
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: wire::encode_reply(&rows)?,
        },
    );
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("oteryn-party-{}-{suffix}.tsv", std::process::id()));
    link.analyser_export_party(&path)?;
    let original = std::fs::read_to_string(&path)?;
    let tools = oteryn_client_ui::analyser::Tool::ALL.len();
    {
        let mut mailbox = lock(&link.mailbox);
        submit(
            &mut mailbox,
            &link.commands,
            tools + 18,
            vec![
                String::new(),
                String::new(),
                String::new(),
                path.to_string_lossy().into_owned(),
            ],
        );
    }
    assert!(link.analyser_text().contains("Alice pays Bob: 600 gold"));
    assert_eq!(link.analyser_panel().party, rows);
    assert!(
        incoming.try_recv().is_err(),
        "import never sends a gameplay command"
    );
    let prior = link.analyser_panel().party_import.ok_or("import")?;
    std::fs::write(
        &path,
        vec![b'x'; oteryn_client_ui::analyser::party_import::MAX_BYTES + 2],
    )?;
    assert!(link.analyser_import_party(&path).is_err());
    assert_eq!(link.analyser_panel().party_import.as_ref(), Some(&prior));
    std::fs::write(&path, &original)?;
    {
        let mut mailbox = lock(&link.mailbox);
        submit(
            &mut mailbox,
            &link.commands,
            tools + 19,
            vec![
                String::new(),
                String::new(),
                String::new(),
                path.to_string_lossy().into_owned(),
            ],
        );
    }
    assert_eq!(std::fs::read_to_string(&path)?, original);
    link.analyser_party_refresh();
    assert!(link.analyser_panel().party_import.is_none());
    assert!(incoming.try_recv().is_err());
    outbox.logged_out();
    assert!(link.analyser_panel().party_import.is_none());
    assert!(link.analyser_import_party(&path).is_err());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn party_split_edits_recalculate_and_roundtrip_through_native_actions()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.analyser_initialize(true);
    let path = std::env::temp_dir().join(format!("oteryn-split-{}.tsv", std::process::id()));
    std::fs::write(
        &path,
        "OTERYN-PARTY-SUMMARY\t1\nName\tLoot\tSupplies\nAlice\t1000\t100\nBob\t0\t300\nCara\t0\t0\n",
    )?;
    link.analyser_import_party(&path)?;
    let tools = oteryn_client_ui::analyser::Tool::ALL.len();
    let edit = |action, name: &str, cost: &str| {
        submit(
            &mut lock(&link.mailbox),
            &link.commands,
            tools + action,
            vec![
                String::new(),
                name.into(),
                cost.into(),
                path.to_string_lossy().into_owned(),
            ],
        );
    };
    edit(20, "Bob", "200");
    edit(21, "Cara", "");
    assert!(link.analyser_text().contains("Alice pays Bob: 700 gold"));
    assert!(
        link.analyser_text()
            .contains("Cara | extra cost 0 | excluded")
    );
    let prior = link.analyser_panel().party_import.ok_or("import")?;
    for (name, cost) in [
        ("Bob", "-1"),
        ("Bob", "1.5"),
        ("Bob", "18446744073709551616"),
        ("Nobody", "100"),
    ] {
        edit(20, name, cost);
        assert_eq!(link.analyser_panel().party_import.as_ref(), Some(&prior));
    }
    edit(19, "", "");
    link.analyser_import_party(&path)?;
    assert!(link.analyser_text().contains("Alice pays Bob: 700 gold"));
    assert!(
        link.analyser_text()
            .contains("Cara | extra cost 0 | excluded")
    );
    edit(22, "Cara", "");
    assert!(
        !link
            .analyser_text()
            .contains("Cara | extra cost 0 | excluded")
    );
    edit(20, "Bob", "0");
    assert!(link.analyser_text().contains("Alice pays Bob: 500 gold"));
    assert!(
        incoming.try_recv().is_err(),
        "advisory edits never move currency"
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn live_party_split_edits_survive_refresh_reset_and_export_without_changing_facts()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.analyser_initialize(true);
    lock(&link.mailbox).party.selected = true;
    let mut rows = vec![
        Member {
            handle: 9,
            name: "Alice".into(),
            available: true,
            loot_value: 1000,
            supply_cost: 100,
            ..Default::default()
        },
        Member {
            handle: 7,
            name: "Bob".into(),
            available: true,
            supply_cost: 300,
            ..Default::default()
        },
        Member {
            handle: 5,
            name: "Cara".into(),
            available: true,
            loot_value: 600,
            ..Default::default()
        },
    ];
    let tools = oteryn_client_ui::analyser::Tool::ALL.len();
    let edit = |action, name: &str, cost: &str| {
        submit(
            &mut lock(&link.mailbox),
            &link.commands,
            tools + action,
            vec![String::new(), name.into(), cost.into()],
        )
    };
    for round in 0..2 {
        link.analyser_party_refresh();
        let crate::combat_input::PlayCommand::Feature(command) = incoming.try_recv()? else {
            return Err("party query".into());
        };
        outbox.analyser_result(
            &command,
            &GatedCommandResult {
                command_id: round + 1,
                server_sequence: round + 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: wire::encode_reply(&rows)?,
            },
        );
        if round == 0 {
            edit(20, "bob", "200");
            edit(21, "Cara", "");
        }
        assert!(link.analyser_text().contains("Alice pays Bob: 700 gold"));
        assert_eq!(link.analyser_panel().party, rows);
        assert!(incoming.try_recv().is_err());
        // Local analyser reset keeps independent server party counters and their settings.
        submit(&mut lock(&link.mailbox), &link.commands, tools, vec![]);
        assert!(link.analyser_text().contains("Alice pays Bob: 700 gold"));
        rows[0].handle = 19;
    }
    let path = std::env::temp_dir().join(format!("oteryn-live-split-{}.tsv", std::process::id()));
    link.analyser_export_party(&path)?;
    link.analyser_import_party(&path)?;
    assert!(link.analyser_text().contains("Alice pays Bob: 700 gold"));
    edit(21, "Alice", "");
    edit(21, "Bob", "");
    assert!(link.analyser_text().contains("Split unavailable"));
    edit(22, "Bob", "");
    assert!(link.analyser_text().contains("Already balanced"));
    edit(20, "Bob", "18446744073709551615");
    assert!(
        link.analyser_text()
            .contains("extra cost 18446744073709551615")
    );
    std::fs::remove_file(path)?;
    Ok(())
}
