use super::*;
fn find(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, value)| (value.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing {name}").into())
}
fn offer() -> oteryn_client_runtime::market::market::MarketRow {
    let mut id = [7; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    oteryn_client_runtime::market::market::MarketRow {
        offer: id,
        definition: 42,
        side: 2,
        amount: 5,
        price: 100,
        state: 1,
        anonymous: true,
    }
}
fn book(app: &mut App, commands: &mut CommandsReceiver, outbox: &Outbox) -> TestResult {
    app.world_mut().non_send_mut::<HostOwner>().phase = crate::bevy_shell::owner::Phase::Playing;
    app.init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        })
        .add_systems(Update, project);
    app.world_mut().resource_mut::<State>().seen = None;
    app.update();
    let select = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.kind, Kind::Item(0)).then_some(entity))
        .ok_or("named catalogue row")?;
    app.world_mut().trigger(Activate { entity: select });
    let command = feature(commands)?;
    outbox.bevy_market_reply(
        &command,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![offer()],
            has_next: false,
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
        })?,
    );
    super::super::purchase::install(app);
    app.update();
    Ok(())
}
#[test]
fn native_named_sell_row_quantity_confirmation_submits_original_offer_once() -> TestResult {
    let (mut app, mut commands, outbox, _) = app()?;
    book(&mut app, &mut commands, &outbox)?;
    let row = find(&mut app, "market.purchase.row")?;
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let amount = find(&mut app, "market.purchase.quantity")?;
    app.world_mut()
        .entity_mut(amount)
        .insert(EditableText::new("2"));
    let review = find(&mut app, "market.purchase.review")?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    let confirmation = find(&mut app, "market.purchase.confirmation")?;
    assert_eq!(
        app.world()
            .get::<Text>(confirmation)
            .ok_or("confirmation text")?
            .0,
        "Buy 2 × Test blade at 100 gold each (total 200 gold)?"
    );
    assert!(
        commands.try_recv().is_err(),
        "reviewing must not settle a purchase"
    );
    let confirm = find(&mut app, "market.purchase.confirm")?;
    app.world_mut().trigger(Activate { entity: confirm });
    let PlayCommand::Market(command) = commands.try_recv()? else {
        return Err("purchase command missing".into());
    };
    assert_eq!(command.0.action, 8);
    assert_eq!(command.0.offer, offer().offer);
    assert_eq!(command.0.amount, 2);
    assert_eq!(command.0.price, 0);
    assert_eq!(command.0.definition, 0);
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(
        commands.try_recv().is_err(),
        "confirmation must enqueue at most once"
    );
    Ok(())
}

fn ready(amount: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (mut app, mut commands, outbox, _) = app()?;
    book(&mut app, &mut commands, &outbox)?;
    let row = find(&mut app, "market.purchase.row")?;
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let field = find(&mut app, "market.purchase.quantity")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new(amount));
    let review = find(&mut app, "market.purchase.review")?;
    app.world_mut().trigger(Activate { entity: review });
    app.update();
    let confirm = find(&mut app, "market.purchase.confirm")?;
    Ok((app, commands, outbox, confirm))
}
fn panel(app: &App) -> Result<MarketPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .market_panel())
}
fn replace_book(
    app: &App,
    commands: &mut CommandsReceiver,
    outbox: &Outbox,
    rows: Vec<oteryn_client_runtime::market::market::MarketRow>,
) -> TestResult {
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.market_query(MarketQuery::Book {
        definition: 42,
        page: 0
    }));
    let command = feature(commands)?;
    outbox.bevy_market_reply(
        &command,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows,
            has_next: false,
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
        })?,
    );
    Ok(())
}
#[test]
fn consent_rejects_removed_replaced_repriced_reduced_or_changed_side_books() -> TestResult {
    for change in [
        "removed",
        "identity",
        "price",
        "remaining",
        "side",
        "selected item",
    ] {
        let (mut app, mut commands, outbox, confirm) = ready("2")?;
        if change == "selected item" {
            assert!(
                app.world()
                    .non_send::<HostOwner>()
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .market_details("43", "1", "100", true, false)
            );
        } else {
            let mut row = offer();
            match change {
                "identity" => row.offer[0] = 9,
                "price" => row.price = 101,
                "remaining" => row.amount = 4,
                "side" => row.side = 1,
                _ => {}
            }
            replace_book(
                &app,
                &mut commands,
                &outbox,
                if change == "removed" {
                    vec![]
                } else {
                    vec![row]
                },
            )?;
        }
        let before = panel(&app)?;
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(
            commands.try_recv().is_err(),
            "{change}: old consent must not settle"
        );
        assert_eq!(
            panel(&app)?,
            before,
            "{change}: refusal must not mutate retained data"
        );
    }
    Ok(())
}
#[test]
fn consent_rejects_pending_queries_stale_generation_disconnect_and_modal_replacement() -> TestResult
{
    for change in [
        "details pending",
        "market pending",
        "generation",
        "closed channel",
        "disconnected",
        "modal",
    ] {
        let (mut app, mut commands, _, confirm) = ready("2")?;
        match change {
            "details pending" => {
                assert!(
                    app.world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .ok_or("play")?
                        .link
                        .market_query(MarketQuery::Statistics { definition: 42 })
                );
                let _ = feature(&mut commands)?;
            }
            "market pending" => {
                app.world()
                    .non_send::<HostOwner>()
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .market_request(oteryn_client_ui::market::Intent::OwnOffers);
                assert!(
                    matches!(commands.try_recv()?,PlayCommand::Market(command) if command.0.action==3)
                );
            }
            "generation" => {
                app.world_mut()
                    .resource_mut::<State>()
                    .key
                    .as_mut()
                    .ok_or("market key")?
                    .generation += 1
            }
            "closed channel" => {
                commands.close();
            }
            "disconnected" => app.world_mut().non_send_mut::<HostOwner>().close(),
            "modal" => {
                app.world_mut()
                    .spawn((GameModal, GlobalZIndex(999), Node::default()));
            }
            _ => {}
        }
        let before = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .map(|play| play.link.market_panel());
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(
            commands.try_recv().is_err(),
            "{change}: stale confirmation must not send"
        );
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .map(|play| play.link.market_panel()),
            before,
            "{change}: no optimistic state"
        );
    }
    Ok(())
}
#[test]
fn quantity_rejects_zero_above_remaining_negative_and_overflow_without_settlement() -> TestResult {
    for amount in [
        "0",
        "6",
        "64001",
        "-1",
        "4294967296",
        "18446744073709551616",
        "not a number",
    ] {
        let (mut app, mut commands, outbox, _) = app()?;
        book(&mut app, &mut commands, &outbox)?;
        let row = find(&mut app, "market.purchase.row")?;
        app.world_mut().trigger(Activate { entity: row });
        app.update();
        let field = find(&mut app, "market.purchase.quantity")?;
        app.world_mut()
            .entity_mut(field)
            .insert(EditableText::new(amount));
        let review = find(&mut app, "market.purchase.review")?;
        app.world_mut().trigger(Activate { entity: review });
        app.update();
        assert!(
            find(&mut app, "market.purchase.confirm").is_err(),
            "{amount}: invalid amount must not reach consent"
        );
        assert!(find(&mut app, "market.purchase.feedback").is_ok());
        assert!(commands.try_recv().is_err());
        assert!(panel(&app)?.pending.is_none());
    }
    Ok(())
}
#[test]
fn quantity_waits_for_pending_paste_and_destructive_edit_and_uses_the_resolved_value() -> TestResult
{
    use std::sync::{Arc, Mutex};
    for mode in ["paste", "edit", "detached"] {
        let (mut app, mut commands, outbox, _) = app()?;
        book(&mut app, &mut commands, &outbox)?;
        let row = find(&mut app, "market.purchase.row")?;
        app.world_mut().trigger(Activate { entity: row });
        app.update();
        let field = find(&mut app, "market.purchase.quantity")?;
        {
            let mut text = app
                .world_mut()
                .get_mut::<EditableText>(field)
                .ok_or("quantity")?;
            if mode == "edit" {
                text.queue_edit(bevy::text::TextEdit::Insert("3".into()));
            } else {
                text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::new(
                    Mutex::new(None),
                )));
            }
        }
        let review = find(&mut app, "market.purchase.review")?;
        app.world_mut().trigger(Activate { entity: review });
        app.update();
        assert!(find(&mut app, "market.purchase.confirm").is_err());
        assert!(commands.try_recv().is_err());
        if mode == "detached" {
            app.world_mut().despawn(field);
            app.update();
            assert!(find(&mut app, "market.purchase.confirm").is_err());
            assert!(commands.try_recv().is_err());
        } else {
            app.world_mut()
                .entity_mut(field)
                .insert(EditableText::new("3"));
            app.update();
            app.update();
            let confirm = find(&mut app, "market.purchase.confirm")?;
            app.world_mut().trigger(Activate { entity: confirm });
            assert!(
                matches!(commands.try_recv()?,PlayCommand::Market(command) if command.0.action==8 && command.0.amount==3)
            );
            assert!(commands.try_recv().is_err());
        }
    }
    Ok(())
}
#[test]
fn accepted_and_refused_settlement_replies_release_pending_and_display_actual_feedback()
-> TestResult {
    use oteryn_client_runtime::market::{MarketResponse, market::MarketReply};
    for (code, feedback) in [
        (1, "The server confirmed the market request"),
        (2, "Market refused: insufficient funds"),
    ] {
        let (mut app, mut commands, outbox, confirm) = ready("2")?;
        let before = panel(&app)?;
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(matches!(commands.try_recv()?,PlayCommand::Market(command) if command.0.action==8));
        assert_eq!(
            panel(&app)?.rows,
            before.rows,
            "submission must not predict an offer fill"
        );
        assert!(panel(&app)?.pending.is_some());
        outbox.bevy_market_response(&MarketResponse {
            status: CommandStatus::Accepted,
            reply: Some(MarketReply {
                action: 8,
                code,
                offer: offer().offer,
                rows: vec![],
            }),
        });
        assert!(panel(&app)?.pending.is_none());
        assert_eq!(
            panel(&app)?.rows,
            before.rows,
            "settlement receipt is not a book delta"
        );
        assert_eq!(panel(&app)?.feedback, Some(feedback));
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == feedback)
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn atomic_market_purchase_rejects_server_modal_published_after_the_ui_snapshot() -> TestResult {
    let (app, mut commands, _, _) = ready("2")?;
    let expected = panel(&app)?;
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(!link.server_modal_active());
    // The real driver publishes after the UI read, before the helper's owner lock.
    let modal = link.bevy_server_modal_fixture();
    assert!(link.server_modal_active());
    assert!(
        !link.market_buy_bound(&expected, &offer(), 2),
        "the atomic owner must recheck the current server modal"
    );
    assert!(commands.try_recv().is_err());
    assert_eq!(
        link.market_panel(),
        expected,
        "blocked consent must have no retained side effects"
    );
    drop(modal);
    Ok(())
}
