use super::feature_local_world_view::live_snapshot;
use super::*;
use crate::play::feature_0014_container_tree::input::ground_handle;

#[test]
fn ground_container_projection_uses_only_the_selected_live_handle()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::world_map::MapOrigin;
    let mut map = live_snapshot(101);
    let ground = |item| GroundItem::Map {
        header: map.header,
        position: map.tiles[0].position,
        item,
    };
    let mut item = map.tiles[0].items[0];
    item.origin = MapOrigin::Handle(42.try_into()?);
    assert_eq!(ground_handle(ground(item)), Some(42.try_into()?));
    item.origin = MapOrigin::BaseOrdinal {
        ordinal: 1,
        object_revision: 9,
    };
    assert_eq!(ground_handle(ground(item)), None);
    map.tiles[0].items[0].origin = MapOrigin::Handle(42.try_into()?);
    let mut play = view()?;
    play.set_live_map(&map)?;
    play.click(TileCoord::new(101, 200))?;
    let (link, mut commands, outbox) = play_channel();
    outbox.container_initialize(Some(Default::default()));
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(18)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    assert!(play.container_input(&link, std::slice::from_ref(&key)));
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(
            oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: 42.try_into()?,
                replace_view: None
            }
        )
    );
    // Pending input is bounded to one request, and a removed selection sends nothing.
    assert!(play.container_input(&link, std::slice::from_ref(&key)));
    assert!(commands.try_recv().is_err());
    map.tiles[0].items.clear();
    play.set_live_map(&map)?;
    play.click(TileCoord::new(101, 200))?;
    assert!(play.container_input(&link, std::slice::from_ref(&key)));
    assert!(commands.try_recv().is_err());
    Ok(())
}
