use super::*;

#[cfg(all(test, feature = "bevy-shell"))]
mod action_bar_editor_tests {
    use super::*;
    #[test]
    fn action_bar_editor_actions_are_bound_and_confirmed() {
        let (link, mut commands, _) = crate::play::play_channel();
        link.use_shared_frontend();
        assert!(link.action_bar_editor_open());
        let expected = link.action_bar_editor_snapshot();
        let mut layout = expected.layout.clone();
        layout.rows[1].enabled = true;
        assert!(link.action_bar_editor_layout_bound(&expected, layout.clone()));
        assert_eq!(link.bar_layout(), layout);
        assert!(!link.action_bar_editor_layout_bound(&expected, expected.layout.clone()));
        assert!(commands.try_recv().is_err());
    }
    #[test]
    fn action_bar_editor_rejected_when_session_replaced_or_target_changed() {
        let (link, _commands, _) = crate::play::play_channel();
        link.use_shared_frontend();
        let (other, _other_commands, _) = crate::play::play_channel();
        other.use_shared_frontend();
        assert!(link.action_bar_editor_open());
        let expected = link.action_bar_editor_snapshot();
        let action = BarAction::Multi {
            actions: vec![BarAction::Use {
                definition: NonZeroU32::MIN,
                mode: crate::play::ItemUseMode::SelfActor,
            }],
        };
        assert!(!other.action_bar_editor_multi_bound(&expected, 0, action.clone()));
        assert!(link.assign_bar_action(
            0,
            BarAction::Text {
                text: "changed".into(),
                send_automatically: true
            }
        ));
        assert!(!link.action_bar_editor_multi_bound(&expected, 0, action));
        assert_eq!(link.action_bar_editor_snapshot().actions.0.len(), 1);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod action_bar_editor_close_tests {
    use super::*;
    #[test]
    fn action_bar_editor_close_preserves_pending_while_blocked_or_unavailable() {
        let (link, commands, _) = crate::play::play_channel();
        link.use_shared_frontend();
        assert!(link.action_bar_editor_open());
        let expected = link.action_bar_editor_snapshot();
        let modal = link.bevy_server_modal_fixture();
        let mut modal_layout = expected.layout.clone();
        modal_layout.rows[1].enabled = true;
        assert!(!link.action_bar_editor_layout_bound(&expected, modal_layout));
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.hotkey_equip.pending = true;
            mailbox.apply_death_vitals(oteryn_session::ActorVitals {
                health: 0,
                max_health: 100,
                ..Default::default()
            });
            mailbox.spell_catalog = Default::default();
        }
        let mut layout = expected.layout.clone();
        layout.rows[1].enabled = true;
        assert!(!link.action_bar_editor_layout_bound(&expected, layout));
        assert!(link.action_bar_editor_close_bound(&expected));
        assert!(!link.action_bar_editor_snapshot().open);
        assert!(lock(&link.mailbox).hotkey_equip.pending);
        drop(modal);
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.apply_death_vitals(oteryn_session::ActorVitals {
                health: 100,
                max_health: 100,
                ..Default::default()
            });
            mailbox.death_reset_commands = false;
        }
        assert!(link.action_bar_editor_open());
        assert!(lock(&link.mailbox).hotkey_equip.pending);
        assert!(!link.action_bar_editor_layout_bound(&expected, expected.layout.clone()));
        let current = link.action_bar_editor_snapshot();
        drop(commands);
        assert!(!link.action_bar_editor_layout_bound(&current, current.layout.clone()));
        let closed = link.action_bar_editor_snapshot();
        assert!(!closed.open);
        assert_eq!(closed.layout, current.layout);
        assert_eq!(closed.actions, current.actions);
    }
}
