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
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(2),
    ));
    app.update();
    assert!(
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .any(|(text, parent)| text.0 == "0m 59s"
                && app
                    .world()
                    .get::<Slot>(parent.parent())
                    .and_then(|slot| slot.item)
                    .is_some_and(|item| item.handle == inspect.target))
    );
    Ok(())
}

#[test]
fn equipped_item_expiry_reads_charges_without_inspecting_bag_contents() -> Result {
    equipped_expiry(true)
}

#[test]
fn legacy_equipped_item_suppresses_repeat_inspection() -> Result {
    equipped_expiry(false)
}

fn equipped_expiry(selected: bool) -> Result {
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
    if selected {
        outbox.bevy_property_items_selected();
    }
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .expiry_inventory = true;
    app.update();
    if !selected {
        assert!(commands.try_recv().is_err());
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(61),
        ));
        app.update();
        assert!(
            commands.try_recv().is_err(),
            "unchanged item was inspected again"
        );
        return Ok(());
    }
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
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(61),
    ));
    app.update();
    let refresh = property_command(&mut commands)?;
    assert_eq!((refresh.capability, refresh.kind), (39, 38));
    let passive = wire::decode_timed_item_intent(&refresh.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(passive.action, wire::TimedAction::Inspect);
    assert_eq!(passive.target, inspect.target);
    assert_eq!(passive.expected_revision, 0);
    app.update();
    assert!(commands.try_recv().is_err(), "duplicate in-flight Inspect");
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

#[test]
fn passive_inspect_refreshes_visible_item_on_the_wire() -> Result {
    use oteryn_client_runtime::timed_item as wire;
    let (mut app, mut commands, outbox) = app()?;
    let mut containers = views()?;
    containers.views[0].entries.truncate(1);
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(Default::default()),
        Some(containers),
        true,
        true,
    );
    outbox.bevy_property_items_selected();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .expiry_containers = true;
    app.update();
    let command = property_command(&mut commands)?;
    let reply = wire::encode_timed_item_result(wire::TimedItemResult {
        disposition: wire::TimedDisposition::Ok,
        revision: 9,
        charges: None,
        remaining_ms: Some(60_000),
        retired: false,
        fee_gold: 0,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.bevy_properties_result(&command, &property_reply(reply));
    app.update();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(61),
    ));
    app.update();
    let refresh = property_command(&mut commands)?;
    assert_eq!((refresh.capability, refresh.kind), (39, 38));
    let intent = wire::decode_timed_item_intent(&refresh.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(intent.target.get(), 11);
    assert_eq!(intent.action, wire::TimedAction::Inspect);
    assert_eq!(intent.expected_revision, 0);
    Ok(())
}
