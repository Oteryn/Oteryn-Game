use super::*;

#[test]
fn visible_container_expiry_uses_the_bound_inspection_reply() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .expiry_containers = true;
    app.update();
    let command = property_command(&mut commands)?;
    let inspect = wire::decode_timed_item_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(inspect.action, wire::TimedAction::Inspect);
    let reply = wire::encode_timed_item_result(wire::TimedItemResult {
        disposition: wire::TimedDisposition::Ok,
        revision: 0,
        charges: None,
        remaining_ms: Some(61000),
        retired: false,
        fee_gold: 0,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.bevy_properties_result(&command, &property_reply(reply));
    app.update();
    let label = |app: &mut App| {
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .any(|(label, parent)| {
                label.0 == "1m 1s"
                    && app
                        .world()
                        .get::<Slot>(parent.parent())
                        .and_then(|slot| slot.item)
                        .is_some_and(|item| item.handle == inspect.target)
            })
    };
    assert!(!label(&mut app));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .expiry_unused = true;
    app.update();
    assert!(label(&mut app));
    Ok(())
}

#[test]
fn equipped_item_expiry_reads_charges_without_inspecting_bag_contents() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            equipment: vec![oteryn_session::item_movement::EquippedItem {
                slot: EquipmentSlot::Head,
                item: tests::entry(91, 1)?,
            }],
            ..default()
        }),
        Some(views()?),
        true,
        true,
    );
    outbox.bevy_property_items_selected();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .expiry_inventory = true;
    app.update();
    let command = property_command(&mut commands)?;
    let inspect = wire::decode_timed_item_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(inspect.target.get(), 91);
    let reply = wire::encode_timed_item_result(wire::TimedItemResult {
        disposition: wire::TimedDisposition::Ok,
        revision: 1,
        charges: Some(5),
        remaining_ms: None,
        retired: false,
        fee_gold: 0,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.bevy_properties_result(&command, &property_reply(reply));
    app.update();
    assert!(
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .any(|(label, parent)| label.0 == "5×"
                && app
                    .world()
                    .get::<Slot>(parent.parent())
                    .and_then(|slot| slot.item)
                    .is_some_and(|item| item.handle == inspect.target))
    );
    Ok(())
}

#[test]
fn inventory_expiry_includes_the_worn_backpack() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(tests::entry(91, 1)?),
            entries: vec![tests::entry(93, 1)?],
            ..default()
        }),
        Some(views()?),
        true,
        true,
    );
    outbox.bevy_property_items_selected();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .expiry_inventory = true;
    app.update();
    let command = property_command(&mut commands)?;
    assert_eq!(
        wire::decode_timed_item_intent(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .target
            .get(),
        91
    );
    Ok(())
}
