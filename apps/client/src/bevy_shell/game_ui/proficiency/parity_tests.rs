use super::*;

#[test]
fn proficiency_search_edits_filter_rows_without_changing_identity() -> Result {
    let (mut app, _commands, _) = app()?;
    let mut fields = app.world_mut().query::<&mut bevy::text::EditableText>();
    fields
        .single_mut(app.world_mut())?
        .editor_mut()
        .set_text("AxE");
    app.update();
    let rows: Vec<_> = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .filter_map(|control| match control.kind {
            Kind::Weapon(row) => Some(row),
            _ => None,
        })
        .collect();
    assert_eq!(rows, [1]);
    Ok(())
}

#[test]
fn reset_committed_weapon_perks_requires_apply_and_clears_selected_level() -> Result {
    let (mut app, mut commands, _) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Weapon(0)))?;
    app.update();
    let label = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(entity, text)| (text.0 == "Reset committed perks").then_some(entity))
        .ok_or("missing reset committed perks")?;
    let entity = app.world().get::<ChildOf>(label).ok_or("button")?.parent();
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, |kind| matches!(kind, Kind::Apply))?;
    let sent = command(&mut commands)?;
    let selection = SelectPerkRequest::decode(&sent.payload).ok_or("perk request")?;
    assert_eq!(
        (selection.definition.get(), selection.level, selection.perk),
        (258, 0, None)
    );
    Ok(())
}

#[test]
fn equipped_weapon_hud_opens_live_track_and_flags_an_unspent_perk() -> Result {
    use oteryn_client_runtime::equipment::{
        CharacterInventory, EquipmentSlot, EquippedItem, ItemEntry,
    };
    let (mut app, _commands, outbox) = app()?;
    let bar = app.world_mut().spawn_empty().id();
    app.insert_resource(crate::bevy_shell::game_ui::StatusBar(bar));
    app.world_mut().resource_scope(|world, skin: Mut<Skins>| {
        crate::bevy_shell::ui::toolbar::grid::spawn(&mut world.commands(), &skin, bar);
    });
    app.world_mut().flush();
    let mut page = catalogue(200);
    page.entries[0].levels[1].selected = None;
    // Publish a fresh catalogue over the live owner.
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.weapon_proficiency_request(Intent::Catalogue);
    publish(&outbox, &page)?;
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            equipment: vec![EquippedItem {
                slot: EquipmentSlot::LeftHand,
                item: ItemEntry {
                    appearance_id: 0,
                    handle: 5.try_into()?,
                    item_definition_ref: 258.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                },
            }],
            ..Default::default()
        }),
        None,
        false,
        true,
    );
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(e, t)| (t.0 == "Sword · 200 / 400 XP (50%)").then_some(e))
        .ok_or("equipped progress")?;
    let mut badges = app
        .world_mut()
        .query_filtered::<&Node, With<crate::bevy_shell::ui::toolbar::grid::UnusedPerk>>();
    assert_eq!(badges.single(app.world())?.display, Display::Flex);
    assert!(
        app.world_mut()
            .query::<(&Name, &Node)>()
            .iter(app.world())
            .any(
                |(name, node)| name.as_str() == "Progress to next weapon level"
                    && node.width == percent(0.)
            )
    );
    let button = app
        .world()
        .get::<ChildOf>(entity)
        .ok_or("HUD track")?
        .parent();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .weapon_proficiency_request(Intent::Browse);
    app.world_mut().trigger(Activate { entity: button });
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .weapon_proficiency_panel();
    assert!(panel.open);
    assert_eq!(panel.weapon, 258);
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory::default()),
        None,
        false,
        true,
    );
    app.update();
    assert_eq!(badges.single(app.world())?.display, Display::Flex);
    Ok(())
}

#[test]
fn weapon_filters_use_inspected_requirements_and_keep_catalogue_identity() -> Result {
    use oteryn_client_ui::inspection::{InspectedItem, Reply, Request, encode_reply};
    let (mut app, _commands, outbox) = app()?;
    outbox.bevy_hud_skills_fixture()?;
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.bevy_item_details_selected(true);
    use oteryn_client_runtime::cyclopedia_character::{GeneralStats, TabExtension, TabView};
    outbox.bevy_cyclopedia_initialize(true, true, false);
    link.cyclopedia_character_request(oteryn_client_ui::cyclopedia_character::Intent::Tab);
    outbox.bevy_cyclopedia_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: TabView::GeneralStats(GeneralStats::default())
            .encode_extended(&TabExtension {
                vocation_id: Some(3),
                deaths: None,
            })
            .map_err(|e| format!("{e:?}"))?,
    });
    for (definition, kind, level) in [(258, "sword, two-handed", "25"), (259, "axe", "45")] {
        assert!(link.inspect(Request::Catalogue(definition)));
        link.bevy_item_details_response(&GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_reply(&Reply {
                status: 1,
                requests: vec![],
                catalogue: None,
                items: vec![InspectedItem {
                    slot: 0,
                    definition_ref: definition,
                    count: 1,
                    tier: 0,
                    name: kind.into(),
                    attributes: vec![
                        ("Weapon Type".into(), kind.into()),
                        ("Required Level".into(), level.into()),
                    ],
                }],
            })
            .map_err(|_| "encode")?,
        });
    }
    let press = |app: &mut App, caption: &str| -> Result {
        let label = app
            .world_mut()
            .query::<(Entity, &Text)>()
            .iter(app.world())
            .find_map(|(e, t)| (t.0 == caption).then_some(e))
            .ok_or_else(|| format!("missing {caption}"))?;
        let entity = app.world().get::<ChildOf>(label).ok_or("button")?.parent();
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    let rows = |app: &mut App| -> Vec<_> {
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .filter_map(|c| {
                if let Kind::Weapon(i) = c.kind {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    };
    press(&mut app, "Usable by my vocation")?;
    assert!(
        rows(&mut app).is_empty(),
        "unrestricted melee catalogue entries use the Knight class fallback"
    );
    press(&mut app, "All vocations")?;
    press(&mut app, "Usable at my level")?;
    assert_eq!(rows(&mut app), [0]);
    press(&mut app, "Weapons: All")?;
    assert_eq!(rows(&mut app), [0]);
    press(&mut app, "Hands: All")?;
    assert!(rows(&mut app).is_empty());
    press(&mut app, "Hands: One")?;
    assert_eq!(rows(&mut app), [0]);
    Ok(())
}
