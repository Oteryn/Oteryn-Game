use super::*;

#[test]
fn ground_click_does_not_generate_client_side_steps() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(view.next_step(), None);
    assert_eq!(view.own(), TileCoord::new(100, 200));
    Ok(())
}

#[test]
fn ground_click_uses_session_floor_and_waits_for_the_previous_receipt() -> Result<(), BatchError> {
    let mut view = PlayView::new(TileCoord::new(-10, 200), -3, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    view.click(TileCoord::new(-8, 201))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::TargetWalk(
            ActorPosition {
                x: -8,
                y: 201,
                floor: -3
            }
        ))
    );
    view.click(TileCoord::new(-9, 202))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert_eq!(view.own(), TileCoord::new(-10, 200));
    oteryn_client_ui::target_move::apply(
        &mut super::super::mailbox::lock(&outbox.0).target_walk,
        oteryn_client_runtime::target_move::WalkResponse {
            status: CommandStatus::Rejected,
            disposition: Some(oteryn_client_runtime::target_move::WalkDisposition::Rejected),
        },
    );
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::TargetWalk(
            ActorPosition {
                x: -9,
                y: 202,
                floor: -3
            }
        ))
    );
    assert_eq!(view.own(), TileCoord::new(-10, 200));
    Ok(())
}

#[test]
fn an_unselected_walk_sends_nothing_and_leaves_arrows_usable() -> Result<(), BatchError> {
    let mut view = view()?;
    let (link, mut commands, _outbox) = play_channel();
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.hud_status().contains("Server walking unavailable"));
    view.arrow(StepDir::North);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::Step(StepDir::North))
    );
    Ok(())
}
