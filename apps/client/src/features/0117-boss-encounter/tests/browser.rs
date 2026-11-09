use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::raw_feature::GatedCommandResult;
use oteryn_client_ui::boss_encounter::{BossCategory, Intent, boss_records, panel_text};
use oteryn_session::CommandStatus;

#[test]
fn bosstiary_browser_actions_select_details_and_enter_confirmed_record()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.boss_encounter_initialize(true);
    link.boss_encounter_request(Intent::Refresh);
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
    // Two progress records, one cooldown, all from the server's page.
    outbox.boss_encounter_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![
            8, 100, 18, 7, 10, 3, b'a', b':', b'b', 16, 120, 34, 7, 10, 3, b'a', b':', b'b', 16, 7,
            34, 7, 10, 3, b'a', b':', b'c', 16, 2,
        ],
    });
    link.boss_encounter_browse(" A:C ", BossCategory::Progress, false);
    let p = link.boss_encounter_panel();
    assert_eq!(boss_records(&p), ["a:c"]);
    assert!(panel_text(&p).contains("Boss details: a:c\nProgress: 2 kills"));
    link.boss_encounter_browse("", BossCategory::Cooldown, false);
    let p = link.boss_encounter_panel();
    assert_eq!(boss_records(&p), ["a:b"]);
    assert!(panel_text(&p).contains("Cooldown: 20 ms"));
    link.boss_encounter_request(Intent::Enter);
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 178 && c.payload == b"\x0a\x03a:b")
    );
    link.boss_encounter_browse("a:c", BossCategory::Progress, false);
    assert_eq!(
        link.boss_encounter_panel().fields[0],
        "a:b",
        "pending entry freezes the displayed selection"
    );
    Ok(())
}
