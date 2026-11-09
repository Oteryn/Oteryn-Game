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
    let path = std::env::temp_dir().join(format!("t069-settings-{}.json", std::process::id()));
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
