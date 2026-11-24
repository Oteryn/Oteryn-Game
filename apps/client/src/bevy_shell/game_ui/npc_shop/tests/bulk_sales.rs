#[test]
fn sell_all_visits_owned_sell_quotes_without_row_selection() -> Result {
    bulk_sale(false)
}

#[test]
fn sell_all_skips_an_excluded_first_offer_and_finishes_the_queue() -> Result {
    bulk_sale(true)
}

fn bulk_sale(exclude_first: bool) -> Result {
    let (host, mut commands, outbox) = fixture()?;
    let link = &host.play.as_ref().ok_or("play")?.link;
    if exclude_first {
        use oteryn_client_ui::inspection as wire;
        link.bevy_item_details_selected(true);
        link.item_catalogue_open();
        let PlayCommand::ServerMechanic(query) = commands.try_recv()? else {
            return Err("catalogue query".into());
        };
        assert!(matches!(
            wire::decode_request(&query.payload),
            Ok(wire::Request::CatalogueQuery(_))
        ));
        link.bevy_item_details_response(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: wire::encode_reply(&wire::Reply {
                    status: 1,
                    requests: vec![],
                    catalogue: None,
                    items: vec![wire::InspectedItem {
                        slot: 0,
                        definition_ref: 42,
                        count: 1,
                        tier: 0,
                        name: "Sword".into(),
                        attributes: vec![("Category".into(), "Weapons".into())],
                    }],
                })
                .map_err(|e| format!("{e:?}"))?,
            },
        );
        link.item_catalogue_select(42);
        link.catalogue_loot_preference(42, false, true);
        assert!(link.npc_shop_panel().sale_exclusions.contains(&42));
        link.item_catalogue_set_open(false);
    }
    let mut window = link.npc_shop_panel().live.window;
    window.offers = [42, 43, 44]
        .map(|item| npc_service::NpcOffer {
            item_definition_ref: item,
            count: 1,
            side: NpcTradeSide::Sell,
            unit_price: 7,
        })
        .into();
    let item = |id: u32| -> std::result::Result<_, Box<dyn std::error::Error>> {
        Ok(oteryn_session::ItemEntry {
            appearance_id: 0,
            handle: u64::from(id).try_into()?,
            item_definition_ref: id.try_into()?,
            count: 3.try_into()?,
            sub_type: 0,
        })
    };
    outbox.push(vec![
        SessionEvent::NpcTradeWindow(AppliedDelta {
            server_sequence: 8,
            base_revision: 7,
            new_revision: 8,
            value: window,
        }),
        SessionEvent::Inventory(AppliedDelta {
            server_sequence: 9,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::CharacterInventory {
                main_backpack: Some(item(1)?),
                entries: vec![item(42)?, item(43)?, item(44)?],
                equipment: vec![],
            },
        }),
    ]);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    let button = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| matches!(a.kind, Kind::SellAll).then_some(e))
        .ok_or("sell all")?;
    app.world_mut().trigger(Activate { entity: button });
    let first = u32::from(exclude_first);
    for expected in [first, 2] {
        let PlayCommand::NpcShop(request) = commands.try_recv()? else {
            return Err("sell all".into());
        };
        assert_eq!(request.0.offer_index, expected);
        assert!(commands.try_recv().is_err());
        outbox.bevy_npc_sale_accepted();
        app.update();
        if expected == first {
            assert!(
                commands.try_recv().is_err(),
                "inventory still belongs to the previous sale"
            );
            outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
                server_sequence: 10,
                base_revision: 1,
                new_revision: 2,
                value: oteryn_session::CharacterInventory {
                    main_backpack: Some(item(1)?),
                    entries: vec![item(44)?],
                    equipment: vec![],
                },
            })]);
        }
        app.update();
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}
