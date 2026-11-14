use super::*;

#[test]
fn battle_config_actions_are_bound_and_confirmed() {
    let (link, mut commands, _) = crate::play::play_channel();
    link.use_shared_frontend();
    let binding = link.battle_config_binding();
    assert!(link.battle_config_apply(&binding, BattleConfigAction::Open));
    let binding = link.battle_config_binding();
    assert!(link.battle_config_apply(&binding, BattleConfigAction::Add));
    assert!(!link.battle_config_apply(&binding, BattleConfigAction::Add));
    let binding = link.battle_config_binding();
    assert!(!link.battle_config_apply(&binding, BattleConfigAction::Remove(false)));
    assert_eq!(link.battle_config_binding().lists.preferences.len(), 2);
    assert!(link.battle_config_apply(&binding, BattleConfigAction::Remove(true)));
    let binding = link.battle_config_binding();
    let mut draft = binding.lists.preferences[0].clone();
    draft.name = "Players".into();
    draft.creatures = false;
    draft.search = "knight".into();
    draft.sort = BattleSort::HealthAscending;
    assert!(link.battle_config_apply(&binding, BattleConfigAction::Save(draft.clone())));
    assert_eq!(link.battle_config_binding().lists.preferences[0], draft);
    assert!(commands.try_recv().is_err());
    let mut saved = default_lists();
    let mut active = 0;
    assert!(
        link.sync_battle_list_state(&mut saved, &mut active, 0)
            .is_some()
    );
    assert_eq!(saved, vec![draft]);
}

#[test]
fn battle_config_rejected_when_session_replaced_or_target_changed() {
    let (link, _commands, _) = crate::play::play_channel();
    link.use_shared_frontend();
    assert!(link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Open));
    let stale = link.battle_config_binding();
    let (replacement, _other_commands, _) = crate::play::play_channel();
    replacement.use_shared_frontend();
    let before = replacement.battle_config_binding().lists;
    assert!(!replacement.battle_config_apply(&stale, BattleConfigAction::Add));
    assert_eq!(replacement.battle_config_binding().lists, before);
    assert!(link.battle_config_apply(&stale, BattleConfigAction::Add));
    let before = link.battle_config_binding().lists;
    assert!(!link.battle_config_apply(
        &stale,
        BattleConfigAction::Save(BattleListPreferences::default())
    ));
    assert_eq!(link.battle_config_binding().lists, before);
    let current = link.battle_config_binding();
    assert!(link.battle_config_apply(&current, BattleConfigAction::Close));
    assert!(!link.battle_config_apply(&current, BattleConfigAction::Add));
    assert!(link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Open));
    assert!(!link.battle_config_apply(&current, BattleConfigAction::Add));
    assert!(link.lens_toggle());
    assert!(!link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Add));
    assert!(!link.lens_toggle());
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Add));
    assert_eq!(
        link.battle_config_binding().lists.preferences,
        before.preferences
    );
    drop(modal);
    lock(&link.mailbox).battle.shutdown = true;
    assert!(!link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Add));
    assert_eq!(
        link.battle_config_binding().lists.preferences,
        before.preferences
    );
}

#[test]
fn battle_config_close_while_pending_and_blocked_preserves_owner() {
    let (link, _commands, _) = crate::play::play_channel();
    link.use_shared_frontend();
    assert!(link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Open));
    let binding = link.battle_config_binding();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.battle.pending = true;
        mailbox.battle.pending_command = Some(crate::combat_input::CombatCommand::Stop);
        mailbox.death.dead = true;
        mailbox.battle.combat = None;
    }
    let modal = link.bevy_server_modal_fixture();
    let before = link.battle_config_binding().lists;
    let pending = link.battle_panel();
    assert!(link.battle_config_apply(&binding, BattleConfigAction::Close));
    let mut closed = pending.clone();
    closed.open = false;
    assert_eq!(link.battle_panel(), closed);
    assert!(link.battle_panel().pending);
    assert_eq!(
        link.battle_config_binding().lists.preferences,
        before.preferences
    );
    drop(modal);
    lock(&link.mailbox).death.dead = false;
    assert!(link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Open));
    assert_eq!(link.battle_panel(), pending);
}

#[test]
fn battle_config_invalid_and_capped_edits_leave_owner_unchanged() {
    let (link, _commands, _) = crate::play::play_channel();
    link.use_shared_frontend();
    assert!(link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Open));
    for _ in 0..3 {
        assert!(link.battle_config_apply(&link.battle_config_binding(), BattleConfigAction::Add));
    }
    let binding = link.battle_config_binding();
    assert!(!link.battle_config_apply(&binding, BattleConfigAction::Add));
    assert!(!link.battle_config_apply(&binding, BattleConfigAction::Select(4)));
    let mut invalid = binding.lists.preferences[0].clone();
    invalid.max_health = 101;
    assert!(!link.battle_config_apply(&binding, BattleConfigAction::Save(invalid)));
    assert_eq!(link.battle_config_binding().lists, binding.lists);
    assert!(link.battle_config_apply(&binding, BattleConfigAction::Select(0)));
    let binding = link.battle_config_binding();
    assert!(!link.battle_config_apply(
        &binding,
        BattleConfigAction::Save(binding.lists.preferences[1].clone())
    ));
    assert_eq!(link.battle_config_binding().lists, binding.lists);
}
