use super::*;

#[test]
fn container_first_order_and_bulk_exclusion_use_item_classification() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    let mut views = views()?;
    views.views[0].entries[0].appearance_id = 3031;
    views.views[0].entries[1].appearance_id = 2854;
    let loose = views.views[0].entries[0].handle;
    let bag = views.views[0].entries[1].handle;
    replace_views(&outbox, views);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .prime_icon_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    app.update();
    click(&mut app, "inventory.container.0.containers-first")?;
    app.update();
    let order: Vec<_> = app
        .world_mut()
        .query::<(&Slot, &ChildOf)>()
        .iter(app.world())
        .find_map(|(s, p)| {
            matches!(s.location, Location::Container { view: 0, .. }).then_some(p.parent())
        })
        .and_then(|grid| app.world().get::<Children>(grid))
        .ok_or("container grid")?
        .iter()
        .filter_map(|e| app.world().get::<Slot>(e)?.item.map(|item| item.handle))
        .collect();
    assert_eq!(order, [bag, loose]);
    click(&mut app, "inventory.container.0.move-nested")?;
    app.update();
    click(&mut app, "inventory.bulk.open")?;
    app.update();
    click(&mut app, "inventory.bulk.destination")?;
    let PlayCommand::MoveContainerContents(requests) = commands.try_recv()? else {
        return Err("bulk command".into());
    };
    assert_eq!(
        requests.iter().map(|r| r.source).collect::<Vec<_>>(),
        [loose]
    );
    Ok(())
}
