use super::*;

#[test]
fn nested_item_menu_can_reuse_or_retain_the_source_window() -> Result {
    for (label, replace_view) in [("Open", Some(0)), ("Open in New Window", None)] {
        let (mut app, mut commands, _) = app()?;
        let slot = app
            .world_mut()
            .query::<&Slot>()
            .iter(app.world())
            .find(|slot| slot.item.is_some_and(|item| item.handle.get() == 11))
            .copied()
            .ok_or("nested item slot")?;
        app.world_mut().resource_mut::<InventoryUi>().menu = Some((slot, Vec2::ZERO));
        app.update();
        let button = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find(|(text, parent)| {
                text.0 == label && app.world().get::<Control>(parent.parent()).is_some()
            })
            .map(|(_, parent)| parent.parent())
            .ok_or("open menu choice")?;
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            commands.try_recv()?,
            PlayCommand::ContainerView(oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: 11.try_into()?,
                replace_view,
            })
        );
    }
    Ok(())
}

#[test]
fn move_up_menu_moves_the_clicked_stack_into_its_parent() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    let mut nested = views()?;
    nested.views[0].parent_handle = Some(20.try_into()?);
    replace_views(&outbox, nested);
    app.update();
    let slot = app
        .world_mut()
        .query::<&Slot>()
        .iter(app.world())
        .find(|slot| slot.item.is_some_and(|item| item.handle.get() == 11))
        .copied()
        .ok_or("nested item slot")?;
    app.world_mut().resource_mut::<InventoryUi>().menu = Some((slot, Vec2::ZERO));
    app.update();
    let button = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find(|(text, _)| text.0 == "Move up")
        .map(|(_, parent)| parent.parent())
        .ok_or("Move up")?;
    app.world_mut().trigger(Activate { entity: button });
    let PlayCommand::MoveItem(request) = commands.try_recv()? else {
        return Err("move command".into());
    };
    assert_eq!(request.source, 11.try_into()?);
    assert_eq!(
        request.destination,
        ItemMoveDestination::Container(20.try_into()?, None)
    );
    assert_eq!(request.count, None);
    Ok(())
}
