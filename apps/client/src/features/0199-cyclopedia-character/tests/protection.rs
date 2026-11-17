use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    cyclopedia_character::protection::{Consumption, Forecast, Scenario},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_character::{Intent, protection_cards};
#[test]
fn o015_forecast_caller_renders_server_xp_pvp_item_risk_and_consumption() -> Result<(), &'static str>
{
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::Protection);
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("command");
    };
    assert_eq!((command.capability, command.kind), (199, 388));
    assert_eq!(command.payload, [131]);
    let scenario = Scenario {
        experience_lost: 1000,
        equipment_per_mille: 25,
        container_per_mille: 250,
        consumption: Consumption::Regular,
        amulet_consumed: false,
    };
    let forecast = Forecast {
        observed_at_unix_ms: 12345,
        level: 100,
        experience: 1_000_000,
        regular_blessings: 3,
        twist: true,
        promotion_benefit: true,
        red_or_black: false,
        adventurer: false,
        pve: scenario,
        pvp: Some(Scenario {
            consumption: Consumption::Twist,
            ..scenario
        }),
    };
    let reply = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: forecast.encode().ok_or("encode")?,
    };
    outbox.cyclopedia_character_result(&reply);
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.protection.forecast, Some(forecast));
    let cards = protection_cards(panel.protection.forecast.as_ref().ok_or("forecast")?);
    assert!(cards[0].1.starts_with("1000 XP"));
    assert!(cards[2].1.contains("2.5%"));
    assert!(cards[2].1.contains("25.0%"));
    assert!(cards[5].1.contains("PvP safeguard consumed"));
    link.cyclopedia_character_request(Intent::Protection);
    assert!(commands.try_recv().is_ok());
    outbox.cyclopedia_character_result(&GatedCommandResult {
        payload: vec![131],
        ..reply
    });
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.protection.forecast, Some(forecast));
    assert!(panel.feedback.is_some());
    let unavailable = Forecast {
        pvp: None,
        ..forecast
    };
    assert!(
        protection_cards(&unavailable)[1]
            .1
            .contains("Not available")
    );
    Ok(())
}
