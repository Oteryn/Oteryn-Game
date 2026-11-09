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
        code: oteryn_input_actions::KeyCode::new(14)?,
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
    outbox.container_result(Some(
        oteryn_session::container_tree::ContainerViewOutcome::Opened,
    ));
    assert!(!link.container_panel().pending);
    map.tiles[0].items.clear();
    play.set_live_map(&map)?;
    play.click(TileCoord::new(101, 200))?;
    assert!(play.container_input(&link, std::slice::from_ref(&key)));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn inventory_browser_opens_only_when_an_intent_was_sent() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::feature_0014_container_tree::input::inventory_native_open;
    let (link, mut commands, outbox) = play_channel();
    let handle = 42.try_into()?;
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        assert!(!inventory_native_open(&mut mailbox, &link.commands, handle));
    }
    assert!(commands.try_recv().is_err());
    outbox.container_initialize(Some(Default::default()));
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        assert!(inventory_native_open(&mut mailbox, &link.commands, handle));
        assert!(!inventory_native_open(&mut mailbox, &link.commands, handle));
    }
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(_)
    ));
    assert!(commands.try_recv().is_err());
    outbox.container_result(Some(
        oteryn_session::container_tree::ContainerViewOutcome::Opened,
    ));
    drop(commands);
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    assert!(!inventory_native_open(&mut mailbox, &link.commands, handle));
    Ok(())
}

#[test]
fn newhaven_starter_wire_data_reaches_native_equipment_backpack_and_quiver_panels()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::container_tree::{
        ContainerView, ContainerViews, decode_container_views, encode_container_views,
    };
    use oteryn_session::item_movement::{EquipmentSlot, EquippedItem};
    use oteryn_session::{CharacterInventory, ItemEntry};
    let keys: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/content-schema/native-gameplay/item-keys.json"
    )))?;
    let records = keys["records"].as_array().ok_or("native Item index")?;
    let entry = |handle: u64,
                 definition: u32,
                 count: u32|
     -> Result<ItemEntry, Box<dyn std::error::Error>> {
        let key = format!("oteryn:item.tibia.i{definition}");
        let ordinal = records
            .iter()
            .position(|row| row[0].as_str() == Some(key.as_str()))
            .ok_or("native starter Item")?;
        let reference = u32::try_from(ordinal + 1)?;
        Ok(ItemEntry {
            handle: handle.try_into()?,
            item_definition_ref: reference.try_into()?,
            count: count.try_into()?,
            sub_type: 0,
        })
    };
    // Resolve the same compact Item indices that production native admission publishes.
    let backpack = entry(10, 2854, 1)?;
    let quiver = entry(12, 35562, 1)?;
    let supplies = vec![
        entry(13, 268, 7)?,
        entry(14, 266, 5)?,
        entry(15, 3031, 50)?,
        entry(16, 3578, 1)?,
    ];
    let bow = entry(11, 3350, 1)?;
    let inventory = CharacterInventory {
        main_backpack: Some(backpack),
        entries: supplies.clone(),
        equipment: vec![
            EquippedItem {
                slot: EquipmentSlot::RightHand,
                item: bow,
            },
            EquippedItem {
                slot: EquipmentSlot::LeftHand,
                item: quiver,
            },
        ],
    };
    let payload = oteryn_session::encode_character_inventory_with_equip_drop(&inventory, true)
        .map_err(|error| format!("{error:?}"))?;
    let inventory = oteryn_session::decode_character_inventory_with_equip_drop(&payload, true)
        .map_err(|error| format!("{error:?}"))?;
    let (link, mut commands, outbox) = play_channel();
    outbox.inventory_initialize(Some(inventory), true);
    outbox.equipment_initialize(true);
    let slots = link.equipment_slots();
    assert!(slots.iter().any(|slot| {
        slot.label == "Right"
            && slot
                .text()
                .contains(&format!("{} ×1", bow.item_definition_ref))
    }));
    assert!(slots.iter().any(|slot| {
        slot.label == "Left"
            && slot
                .text()
                .contains(&format!("{} ×1", quiver.item_definition_ref))
    }));
    assert!(link.inventory_select(supplies[2].handle));
    assert!(
        link.inventory_panel()
            .text()
            .contains(&format!("{} ×50", supplies[2].item_definition_ref))
    );
    outbox.container_initialize(Some(Default::default()));
    assert!(link.inventory_select(backpack.handle));
    assert!(link.inventory_open_container());
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(_)
    ));
    outbox.container_result(Some(
        oteryn_session::container_tree::ContainerViewOutcome::Opened,
    ));
    let views = ContainerViews {
        views: vec![
            ContainerView {
                view_id: 0,
                container_handle: backpack.handle,
                parent_handle: None,
                capacity: 20,
                entries: supplies,
                browse: None,
            },
            ContainerView {
                view_id: 1,
                container_handle: quiver.handle,
                parent_handle: None,
                capacity: 8,
                entries: vec![entry(17, 3447, 100)?, entry(18, 3447, 100)?],
                browse: None,
            },
        ],
    };
    let wire = encode_container_views(&views).map_err(|e| format!("{e:?}"))?;
    let decoded = decode_container_views(&wire).map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![oteryn_session::SessionEvent::ContainerViews(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: decoded,
        },
    )]);
    let mut panel = link.container_panel();
    assert!(panel.open);
    assert_eq!(panel.rows().len(), 4);
    assert!(panel.activate(1));
    assert_eq!(
        panel
            .rows()
            .iter()
            .map(|item| item.count.get())
            .sum::<u32>(),
        200
    );
    assert!(oteryn_client_ui::container_tree::view::panel_text(&panel).contains("2 / 8 items"));
    Ok(())
}

mod bulk {
    include!("bulk.rs");
}


#[path = "bulk_session.rs"]
mod bulk_session;
