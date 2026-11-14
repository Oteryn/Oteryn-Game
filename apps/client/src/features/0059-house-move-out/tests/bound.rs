use super::*;

#[test]
fn house_move_out_owner_dispatch_is_atomic() {
    let (link, mut commands, outbox) = crate::play::play_channel();
    let panel = Panel {
        selected: true,
        open: true,
        fields: ["oteryn:content.house.test".into(), "2000000000000".into()],
        ..Default::default()
    };
    outbox.bevy_house_move_out_panel(panel.clone());
    let snapshot = link.house_move_out_snapshot();
    let action = |confirmed| Action::Request {
        intent: Intent::Notice,
        fields: panel.fields.clone(),
        confirmed,
    };
    assert!(!link.house_move_out_dispatch_bound(&snapshot, action(false)));
    assert_eq!(link.house_move_out_panel(), panel);
    assert!(commands.try_recv().is_err());
    let mut invalid = panel.fields.clone();
    invalid[1] = "invalid".into();
    assert!(!link.house_move_out_dispatch_bound(
        &snapshot,
        Action::Request {
            intent: Intent::Notice,
            fields: invalid,
            confirmed: true,
        }
    ));
    assert_eq!(link.house_move_out_panel(), panel);
    assert!(commands.try_recv().is_err());
    assert!(link.house_move_out_dispatch_bound(&snapshot, action(true)));
    let pending = link.house_move_out_panel();
    assert!(pending.pending);
    assert_eq!(pending.summary, panel.summary);
    assert!(!link.house_move_out_dispatch_bound(&snapshot, action(true)));
    let pending_snapshot = link.house_move_out_snapshot();
    assert!(!link.house_move_out_dispatch_bound(&pending_snapshot, action(true)));
    assert_eq!(link.house_move_out_panel(), pending);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::Feature(_))));
    assert!(commands.try_recv().is_err());
    outbox.bevy_house_move_out_panel(panel.clone());
    let snapshot = link.house_move_out_snapshot();
    drop(commands);
    assert!(!link.house_move_out_dispatch_bound(&snapshot, action(true)));
    assert_eq!(link.house_move_out_panel(), panel);
}
