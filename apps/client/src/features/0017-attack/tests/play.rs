use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::battle::Action;
use oteryn_session::{ActorCombatState, AttackIntentDisposition, FightModes};

#[test]
fn world_click_report_does_not_unlock_a_pending_panel_command() {
    let (link, _commands, outbox) = play_channel();
    let state = ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    };
    lock(&link.mailbox).battle.combat = Some(state);
    link.battle_request(Action::Stop);
    assert!(link.battle_panel().pending);
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        Some(state),
        None,
    );
    assert!(link.battle_panel().pending);
    // The panel's matching stop response releases the controls.
    outbox.battle_result(
        CombatReport::Stop(AttackIntentDisposition::Ok),
        Some(state),
        Some(CombatCommand::Stop),
    );
    assert!(!link.battle_panel().pending);
    assert_eq!(link.battle_panel().feedback, Some("Attack stopped"));
}

#[test]
fn feedback_matches_both_panel_origin_and_command_kind() {
    let (link, _commands, outbox) = play_channel();
    let command = CombatCommand::Attack(oteryn_session::EntityRef {
        identity: [9; 16],
        generation: 1,
    });
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.battle.pending = true;
        mailbox.battle.pending_command = Some(command);
    }
    // An earlier world click can answer with the same report kind as a panel attack.
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        None,
        None,
    );
    assert!(link.battle_panel().pending);
    // Even a panel-origin report must answer the pending command kind.
    outbox.battle_result(
        CombatReport::Modes(AttackIntentDisposition::Ok),
        None,
        Some(command),
    );
    assert!(link.battle_panel().pending);
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        None,
        Some(command),
    );
    assert!(!link.battle_panel().pending);
    assert_eq!(link.battle_panel().pending_command, None);
}

#[test]
fn failed_panel_send_releases_pending_controls() {
    let (link, commands, _outbox) = play_channel();
    drop(commands);
    lock(&link.mailbox).battle.combat = Some(ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    });
    link.battle_request(Action::Stop);
    assert!(!link.battle_panel().pending);
    assert_eq!(link.battle_panel().pending_command, None);
}
