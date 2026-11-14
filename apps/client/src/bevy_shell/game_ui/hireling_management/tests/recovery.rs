use super::*;
use oteryn_client_runtime::{cui3::store, server_mechanic::ServerMechanicCommand};
use oteryn_session::CommandStatus;
fn pending_purchase()
-> std::result::Result<(Fixture, ServerMechanicCommand), Box<dyn std::error::Error>> {
    let (mut app, mut commands, outbox) = fixture()?;
    open_loaded(&mut app, &mut commands, &outbox)?;
    click(&mut app, Kind::Prices)?;
    let _ = commands.try_recv()?;
    let page = store::StoreCataloguePage {
        catalog_revision: "r1".into(),
        total_coins: 100,
        transferable_coins: 100,
        next_page_cursor: String::new(),
        offers: vec![store::StoreOffer {
            offer_key: "store:hireling:25438".into(),
            title: "Rename".into(),
            price: 73,
            currency: "coins".into(),
        }],
    };
    outbox.bevy_hireling_management_store_reply(
        store::encode_catalogue_page(&page).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    let field = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| matches!(field, Field::Name).then_some(entity))
        .ok_or("name missing")?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("Bella"));
    click(&mut app, Kind::Configure(131))?;
    app.update();
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("purchase missing".into());
    };
    app.update();
    Ok(((app, commands, outbox), command))
}
fn retry(app: &mut App) -> Result {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "Retry configuration").then_some(parent.parent()))
        .ok_or("unresolved purchase has no retry control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn hireling_management_lost_refused_and_invalid_purchase_replies_retain_exact_retry() -> Result {
    for status in [
        None,
        Some(CommandStatus::Rejected),
        Some(CommandStatus::Accepted),
    ] {
        let ((mut app, mut commands, outbox), original) = pending_purchase()?;
        let receipt_id = store::decode_purchase_request(&original.payload)
            .map_err(|e| format!("{e:?}"))?
            .purchase_request_id;
        if let Some(status) = status {
            outbox.bevy_hireling_management_store_status(status, vec![]);
            app.update();
        }
        assert_eq!(panel(&app)?.entries, vec![entry()]);
        if status.is_none() {
            click(&mut app, Kind::Close)?;
            app.update();
            open(&mut app.world_mut().non_send_mut::<HostOwner>());
            app.update();
            assert!(
                commands.try_recv().is_err(),
                "reopening cannot replace the unresolved receipt"
            );
        }
        retry(&mut app)?;
        assert!(
            commands.try_recv().is_err(),
            "retry requires explicit confirmation"
        );
        app.update();
        click(&mut app, Kind::Confirm)?;
        click(&mut app, Kind::Confirm)?;
        let PlayCommand::ServerMechanic(replayed) = commands.try_recv()? else {
            return Err("retry missing".into());
        };
        assert_eq!(
            (replayed.capability, replayed.command_type, replayed.payload),
            (original.capability, original.command_type, original.payload)
        );
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?.entries, vec![entry()]);
        outbox.bevy_hireling_management_store_reply(
            store::encode_purchase_result(&store::StorePurchaseResult {
                code: store::StorePurchaseCode::Accepted,
                order_id: Some(receipt_id),
                operation_id: Some(receipt_id),
            })
            .map_err(|e| format!("{e:?}"))?,
        );
        app.update();
        let store = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .store_panel();
        assert!(!store.pending);
        assert!(store.confirmed_purchase.is_none());
        assert_eq!(
            store.purchase.as_ref().map(|result| result.code),
            Some(store::StorePurchaseCode::Accepted)
        );
        assert_eq!(
            panel(&app)?.entries,
            vec![entry()],
            "only a hireling state reply changes the hireling"
        );
    }
    Ok(())
}

#[test]
fn hireling_management_terminal_purchase_refusal_can_refresh_and_review_again() -> Result {
    let ((mut app, mut commands, outbox), original) = pending_purchase()?;
    outbox.bevy_hireling_management_store_reply(
        store::encode_purchase_result(&store::StorePurchaseResult {
            code: store::StorePurchaseCode::InsufficientBalance,
            order_id: None,
            operation_id: None,
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert_eq!(panel(&app)?.entries, vec![entry()]);
    assert!(
        retry(&mut app).is_err(),
        "a terminal result no longer needs receipt replay"
    );
    click(&mut app, Kind::Refresh)?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("refresh missing".into());
    };
    assert_eq!(
        wire::Request::decode(&command.payload),
        Some(wire::Request::List)
    );
    outbox.bevy_hireling_management_reply(wire::Reply::State(vec![entry()]));
    app.update();
    click(&mut app, Kind::Configure(131))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("new purchase missing".into());
    };
    let before = store::decode_purchase_request(&original.payload).map_err(|e| format!("{e:?}"))?;
    let after = store::decode_purchase_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_ne!(before.purchase_request_id, after.purchase_request_id);
    assert_eq!(
        (before.offer_key, before.price),
        (after.offer_key, after.price)
    );
    assert_eq!(panel(&app)?.entries, vec![entry()]);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn hireling_management_hides_retry_for_other_store_purchases() -> Result {
    use bevy::ecs::system::RunSystemOnce;
    let ((mut app, mut commands, _outbox), original) = pending_purchase()?;
    let mut quote =
        store::decode_purchase_request(&original.payload).map_err(|e| format!("{e:?}"))?;
    quote.offer_key = "store:outfit:1001".into();
    let (key, mut snapshot) = app
        .world()
        .resource::<State>()
        .snapshot
        .clone()
        .ok_or("snapshot")?;
    snapshot.store.confirmed_purchase =
        Some(store::encode_purchase_request(&quote).map_err(|e| format!("{e:?}"))?);
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, skins: Res<Skins>, mut state: ResMut<State>| {
                let mut current = state.widgets.take().ok_or("widgets")?;
                widgets::render(&mut commands, &skins, &mut current, key, &snapshot, &state);
                state.widgets = Some(current);
                Ok::<(), &str>(())
            },
        )
        .map_err(|e| format!("{e:?}"))??;
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Retry configuration")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
