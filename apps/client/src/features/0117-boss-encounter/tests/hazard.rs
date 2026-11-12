use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::raw_feature::GatedCommandResult;
use oteryn_client_ui::boss_encounter::{Intent, panel_text};
use oteryn_session::CommandStatus;

#[test]
fn hazard_confirmation_updates_entry_guide_and_next_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.boss_encounter_initialize(true);
    link.boss_encounter_edit(0, "boss:test".into());
    link.boss_encounter_edit(2, "12".into());
    link.boss_encounter_request(Intent::HazardQuery);
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 169 && c.payload.is_empty())
    );
    outbox.boss_encounter_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![5, 2],
    });
    assert_eq!(link.boss_encounter_panel().fields[2], "2");
    let text = panel_text(&link.boss_encounter_panel());
    assert!(text.contains("Enter boss:test at hazard 2"));
    assert!(text.contains("■■□□□×××××××"));
    link.boss_encounter_request(Intent::HazardRaise);
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 169 && c.payload == vec![3])
    );
    outbox.boss_encounter_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: vec![5, 3],
    });
    assert_eq!(link.boss_encounter_panel().fields[2], "3");
    link.boss_encounter_request(Intent::Enter);
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 178 && c.payload == b"\x0a\x09boss:test")
    );
    Ok(())
}
