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
