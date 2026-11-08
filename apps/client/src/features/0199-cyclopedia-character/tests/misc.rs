use super::*;
use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::cyclopedia_character::{
    CharacterTab, GeneralStats, TabView, combat::CombatStats,
};
#[test]
fn misc_resource_read_uses_the_real_command_adapter_without_navigating_away()
-> Result<(), &'static str> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::SelectTab(CharacterTab::Misc));
    assert!(
        matches!(commands.try_recv(), Ok(PlayCommand::Feature(c)) if c.kind == 388 && c.payload[0] == 10)
    );
    let stats = CombatStats::new(CharacterTab::Misc, 1, 1, 1).map_err(|_| "stats")?;
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: TabView::Combat(stats.clone())
            .encode()
            .map_err(|_| "misc")?,
    });
    link.cyclopedia_character_request(Intent::MiscResources);
    assert!(
        matches!(commands.try_recv(), Ok(PlayCommand::Feature(c)) if c.kind == 388 && c.payload[0] == 1)
    );
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 2,
        status: CommandStatus::Accepted,
        server_sequence: 2,
        payload: TabView::GeneralStats(GeneralStats {
            stamina_minutes: 2520,
            skills: [20; 7],
            ..Default::default()
        })
        .encode()
        .map_err(|_| "general")?,
    });
    let p = link.cyclopedia_character_panel();
    assert_eq!(p.tab, CharacterTab::Misc);
    assert_eq!(p.combat, Some(stats));
    assert_eq!(
        oteryn_client_ui::cyclopedia_character::misc_cards(&p).len(),
        10
    );
    assert!(!p.pending);
    let mut p = p;
    let command = crate::play::feature_0199_cyclopedia_character::view::native::submit(
        &mut p,
        110,
        ["0", "20", "02020202020202020202020202020202"],
    )
    .ok_or("new profile")?;
    assert_eq!(&command.payload[4..], &[2; 16]);
    assert!(p.combat.is_none());
    assert!(p.misc_general.is_none());
    Ok(())
}
