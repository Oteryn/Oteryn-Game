use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{
        CharacterTab, TabView,
        combat::{CombatStat as S, CombatStats},
    },
    raw_feature::GatedCommandResult,
};
use oteryn_client_ui::cyclopedia_character::{Intent, combat_detail, combat_rows};
#[test]
fn offence_sources_reach_the_native_table_through_the_real_adapter() -> Result<(), &'static str> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::SelectTab(CharacterTab::Offence));
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("Offence request");
    };
    assert_eq!(command.payload, vec![8, 0, 0, 20]);
    let mut stats = CombatStats::new(CharacterTab::Offence, 123, 4, 5).map_err(|_| "stats")?;
    for (key, value) in [
        (S::AttackTotal, 35),
        (S::AttackEquipment, 7),
        (S::AttackSkill, 6),
        (S::AttackFightMode, 2),
        (S::FlatDamageHealing, 20),
        (S::CriticalChanceEquipment, 15000),
        (S::CriticalChanceImbuement, -5000),
    ] {
        stats.set(key, value).map_err(|_| "stat")?;
    }
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: TabView::Combat(stats.clone())
            .encode()
            .map_err(|_| "payload")?,
    });
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.combat, Some(stats));
    let rows = combat_rows(&panel);
    assert_eq!(rows.len(), S::for_tab(CharacterTab::Offence).len());
    assert_eq!(
        rows.iter()
            .find(|r| r.key == S::AttackFightMode)
            .ok_or("stance")?
            .source,
        "Combat stance"
    );
    assert_eq!(
        rows.iter()
            .find(|r| r.key == S::CriticalChanceWheel)
            .ok_or("wheel")?
            .value,
        "Unavailable"
    );
    assert_eq!(
        rows.iter()
            .find(|r| r.key == S::CriticalChanceImbuement)
            .ok_or("imbuement")?
            .value,
        "-0.5000%"
    );
    let detail = combat_detail(&panel, S::AttackTotal);
    for expected in [
        "Equipment revision: 5",
        "Attack Equipment: 7",
        "Attack Skill: 6",
        "Attack Fight Mode: 2",
        "Flat Damage Healing: 20",
    ] {
        assert!(detail.contains(expected), "{detail}");
    }
    Ok(())
}
