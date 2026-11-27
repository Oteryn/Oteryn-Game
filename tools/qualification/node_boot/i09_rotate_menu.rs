// Included by the Bevy furniture test module. The live row supplies its
// received map window and forwards only the command emitted by the menu.
#[test]
#[ignore = "requires a live I09 map window"]
fn qualification_i09_rotate_menu() -> Result {
    let args = std::env::var("OTERYN_I09_CLICK")?
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let [actor_x, actor_y, floor, click_x, click_y] = args.as_slice() else {
        return Err("I09 menu requires actor x/y/floor and click x/y".into());
    };
    let snapshot = std::fs::read(std::env::var_os("OTERYN_I09_MENU_MAP").ok_or("no live map")?)?;
    let map = oteryn_session::world_map::decode_world_map_snapshot(&snapshot)
        .map_err(|error| format!("I09 menu map: {error:?}"))?;
    let mut view = PlayView::new(
        TileCoord::new(*actor_x, *actor_y),
        (*floor).try_into()?,
        &[],
    )?;
    view.set_live_map(&map)?;
    let (mut app, mut commands, _) = app_with_view(view)?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(*click_x, *click_y));
    app.update();
    choose(&mut app, "Rotate")?;
    let PlayCommand::WorldContext(Command {
        target: Target::Object(object),
        action: Action::Use,
        selection,
    }) = commands.try_recv()?
    else {
        return Err("I09 Rotate did not emit a world-object USE".into());
    };
    assert!(
        commands.try_recv().is_err(),
        "Rotate emitted extra commands"
    );
    let selection = selection.map(|value| {
        let pos = value.position;
        ((pos.x, pos.y, pos.floor), value.binding)
    });
    let command = (object.placement, object.expected_revision, selection);
    println!("I09_MENU_USE={}", serde_json::to_string(&command)?);
    Ok(())
}
