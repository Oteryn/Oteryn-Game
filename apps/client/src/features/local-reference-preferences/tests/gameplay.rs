use super::*;

#[test]
fn gameplay_options_cancel_chase_before_manual_step() -> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::{CombatCommand, PlayCommand};
    use crate::play::play_channel;
    use oteryn_session::{ActorCombatState, ChaseMode, FightModes};
    let (link, mut commands, outbox) = play_channel();
    outbox.battle_publish(
        Some(ActorCombatState {
            target: None,
            modes: FightModes {
                chase: ChaseMode::Chase,
                ..FightModes::DEFAULT
            },
            in_fight: false,
        }),
        Some(0),
        &[],
    );
    let mut play = view()?;
    let mut preferences = play.preferences().clone();
    preferences.gameplay.cancel_chase_on_walk = true;
    play.set_preferences(preferences)?;
    play.arrow(crate::input::StepDir::North);
    play.tick(&link).map_err(|_| "tick failed")?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Combat(CombatCommand::Modes(modes)) if modes.chase == ChaseMode::Stand)
    );
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Step(crate::input::StepDir::North)
    );
    Ok(())
}

#[test]
fn gameplay_mouse_loot_uses_negotiated_caller_and_saved_choices()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, SettingsState, mailbox::lock, play_channel};
    let (link, mut commands, _outbox) = play_channel();
    let mut play = view()?;
    assert!(!play.gameplay_secondary_click(&link));
    let fixture = crate::play::screenshot::TestDir::new()?;
    let path = fixture.join("t069-settings.json");
    let mut state = SettingsState::new(Some(path.clone()));
    let mut preferences = state.preferences.clone();
    preferences.gameplay.secondary_click_loots_nearby = true;
    preferences.gameplay.inspect_context_menu = false;
    state.apply(preferences.clone());
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &preferences
    );
    play.set_preferences(preferences)?;
    assert!(play.gameplay_secondary_click(&link));
    assert!(commands.try_recv().is_err());
    lock(&link.mailbox).discovery.loot.selected = true;
    assert!(play.gameplay_secondary_click(&link));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ServerMechanic(command) if command.capability == 98)
    );
    assert!(commands.try_recv().is_err());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn shifted_loot_click_targets_the_last_arriving_corpse() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ActorPosition, EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
    let (link, mut commands, outbox) = play_channel();
    let mut play = view()?;
    let mut preferences = play.preferences().clone();
    preferences.gameplay.loot_click = 2;
    play.set_preferences(preferences)?;
    crate::play::mailbox::lock(&link.mailbox)
        .discovery
        .loot
        .selected = true;
    let id = [1, 144, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1];
    let tile = play.scene().view().origin();
    let top = WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: EntityKind::Corpse,
        entity: EntityRef {
            identity: id,
            generation: 0,
        },
        position: ActorPosition {
            x: tile.x,
            y: tile.y,
            floor: play.satellite_player().2,
        },
        detail: EntityDetail::Object {
            item_definition_ref: 42,
            quantity: 1,
            item_handle: None,
        },
    };
    // Identity order is deliberately the reverse of the tile stack.
    let mut bottom = top;
    bottom.entity.identity[0] = 2;
    outbox.entities(vec![bottom, top]);
    play.tick(&link).map_err(|_| "tick")?;
    assert!(!play.loot_click(&link, tile, true, None));
    crate::play::mailbox::lock(&link.mailbox).hireling_picking = true;
    assert!(!play.loot_click(
        &link,
        tile,
        true,
        Some(oteryn_input_actions::Modifier::Shift)
    ));
    crate::play::mailbox::lock(&link.mailbox).hireling_picking = false;
    assert!(play.loot_click(
        &link,
        tile,
        true,
        Some(oteryn_input_actions::Modifier::Shift)
    ));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("loot command".into());
    };
    let request = oteryn_client_ui::quick_loot::decode_quick_loot_request(&command.payload)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        request.action,
        oteryn_client_ui::quick_loot::QuickLootAction::LootAll
    );
    assert_eq!(request.corpse, Some(id));
    // A second click while the server handles the first still belongs to looting.
    assert!(play.loot_click(
        &link,
        tile,
        true,
        Some(oteryn_input_actions::Modifier::Shift)
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}
