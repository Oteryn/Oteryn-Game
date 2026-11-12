use super::*;
pub fn request(panel: &mut BattlePanel, action: Action) -> Option<CombatCommand> {
    let state = panel.combat.filter(|_| !panel.shutdown && !panel.pending)?;
    let command = match action {
        Action::Attack(target) => panel
            .rows
            .iter()
            .any(|e| e.entity == target)
            .then_some(CombatCommand::Attack(target)),
        Action::Stop => Some(CombatCommand::Stop),
        Action::Fight(fight_mode) => Some(CombatCommand::Modes(FightModes {
            fight_mode,
            ..state.modes
        })),
        Action::Chase(chase) => Some(CombatCommand::Modes(FightModes {
            chase,
            ..state.modes
        })),
    };
    panel.pending = command.is_some();
    panel.pending_command = command;
    command
}

/// Alt+B toggles the battle panel; Ctrl+I remains available to inventory.
pub fn toggle_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 5 && *modifiers == Modifiers::one(Modifier::Alt)))
}
