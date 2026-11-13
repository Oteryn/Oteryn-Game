use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use oteryn_client_runtime::{market::market_details as wire, raw_feature::FeatureCommand};
use oteryn_client_ui::market::details::Filter;
use oteryn_renderer::TileCoord;
use oteryn_session::{CommandStatus, GatedCommandResult};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type CommandsReceiver = tokio::sync::mpsc::UnboundedReceiver<PlayCommand>;
fn result(reply: &wire::Reply) -> Result<GatedCommandResult, Box<dyn std::error::Error>> {
    Ok(GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::encode_reply(reply).map_err(|e| format!("{e:?}"))?,
    })
}
fn feature(commands: &mut CommandsReceiver) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
    match commands.try_recv()? {
        PlayCommand::Feature(command) => Ok(command),
        _ => Err("unexpected transaction command".into()),
    }
}
fn catalogue() -> wire::Reply {
    wire::Reply::Catalogue {
        items: vec![Ware {
            definition: 42,
            name: "Test blade".into(),
            categories: vec!["Swords".into()],
            vocations: None,
            level: Some(30),
            forge_class: None,
            depot_units: 4,
        }],
        categories: vec!["Swords".into()],
        page: 0,
        has_next: true,
    }
}
type Fixture = (App, CommandsReceiver, Outbox, Entity);
fn app() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_market_initialize(true);
    link.market_set_open(true);
    assert!(link.market_query(MarketQuery::Catalogue {
        filter: Filter::default(),
        page: 0
    }));
    let command = feature(&mut commands)?;
    outbox.bevy_market_reply(&command, &result(&catalogue())?);
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let key = Key {
        generation: host.generation(),
        revision: 1,
    };
    let seen = host.play.as_ref().ok_or("play")?.link.market_panel();
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_resource(State {
            key: Some(key),
            seen: Some(seen),
            ..default()
        })
        .add_systems(PostUpdate, actions::finish);
    let root = app
        .world_mut()
        .spawn((Overlay, GameModal, Node::default()))
        .id();
    Ok((app, commands, outbox, root))
}
fn button(app: &mut App, root: Entity, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    let key = app.world().resource::<State>().key.ok_or("key")?;
    let entity = app
        .world_mut()
        .spawn((Action { key, kind }, Node::default()))
        .observe(activate)
        .id();
    app.world_mut().entity_mut(root).add_child(entity);
    Ok(entity)
}
fn sync(app: &mut App) -> TestResult {
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .market_panel();
    app.world_mut().resource_mut::<State>().seen = Some(panel);
    Ok(())
}
#[test]
fn native_catalogue_selection_queries_book_then_statistics_through_retained_owner() -> TestResult {
    let (mut app, mut commands, outbox, root) = app()?;
    let select = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: select });
    let book = feature(&mut commands)?;
    assert_eq!(
        wire::decode_query(&book.payload).map_err(|e| format!("{e:?}"))?,
        MarketQuery::Book {
            definition: 42,
            page: 0
        }
    );
    let stats = wire::Statistics {
        definition: 42,
        as_of_us: 2592000000000,
        ..default()
    };
    outbox.bevy_market_reply(
        &book,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![],
            statistics: stats.clone(),
            has_next: false,
        })?,
    );
    sync(&mut app)?;
    assert_eq!(
        app.world()
            .resource::<State>()
            .selected
            .as_ref()
            .ok_or("selected named ware")?
            .name,
        "Test blade"
    );
    let statistics = button(&mut app, root, Kind::Statistics)?;
    app.world_mut().trigger(Activate { entity: statistics });
    let command = feature(&mut commands)?;
    assert_eq!(
        wire::decode_query(&command.payload).map_err(|e| format!("{e:?}"))?,
        MarketQuery::Statistics { definition: 42 }
    );
    outbox.bevy_market_reply(&command, &result(&wire::Reply::Statistics(stats))?);
    assert!(
        commands.try_recv().is_err(),
        "read-only UI must not create trade commands"
    );
    Ok(())
}
#[test]
fn native_market_rejects_stale_catalogue_generation_and_background_modal_events() -> TestResult {
    let (mut app, mut commands, outbox, root) = app()?;
    let select = button(&mut app, root, Kind::Item(0))?;
    let key = app.world().get::<Action>(select).ok_or("action")?.key;
    app.world_mut()
        .get_mut::<Action>(select)
        .ok_or("action")?
        .key
        .generation = key.generation + 1;
    app.world_mut().trigger(Activate { entity: select });
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .get_mut::<Action>(select)
        .ok_or("action")?
        .key = key;
    let top = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: select });
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(top);
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.market_query(MarketQuery::Catalogue {
        filter: Filter {
            search: "other".into(),
            ..default()
        },
        page: 1
    }));
    let catalogue_query = feature(&mut commands)?;
    app.world_mut().trigger(Activate { entity: select });
    assert!(
        commands.try_recv().is_err(),
        "changed pending catalogue must reject cached row"
    );
    outbox.bevy_market_reply(
        &catalogue_query,
        &result(&wire::Reply::Catalogue {
            items: vec![],
            categories: vec!["Swords".into()],
            page: 1,
            has_next: false,
        })?,
    );
    sync(&mut app)?;
    app.world_mut().trigger(Activate { entity: select });
    assert!(
        commands.try_recv().is_err(),
        "a removed current ware must never be selected"
    );
    Ok(())
}
#[test]
fn closing_market_preserves_pending_result_without_reopening() -> TestResult {
    let (mut app, mut commands, outbox, root) = app()?;
    let select = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: select });
    let book = feature(&mut commands)?;
    sync(&mut app)?;
    let close = button(&mut app, root, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(!link.market_panel().open);
    assert!(link.market_panel().details.pending.is_some());
    outbox.bevy_market_reply(
        &book,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![],
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
            has_next: false,
        })?,
    );
    assert!(!link.market_panel().open);
    assert!(link.market_panel().details.pending.is_none());
    assert_eq!(link.market_panel().definition, 42);
    Ok(())
}
#[test]
fn native_search_defers_pending_paste_and_rechecks_field_and_owner() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, _, root) = app()?;
    let mut text = EditableText::new("old");
    text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::new(
        Mutex::new(None),
    )));
    let field = app.world_mut().spawn((Search, text, Node::default())).id();
    app.world_mut().entity_mut(root).add_child(field);
    let search = button(&mut app, root, Kind::Search)?;
    app.world_mut().trigger(Activate { entity: search });
    app.update();
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("blade"));
    app.update();
    let command = feature(&mut commands)?;
    assert_eq!(
        wire::decode_query(&command.payload).map_err(|e| format!("{e:?}"))?,
        MarketQuery::Catalogue {
            filter: Filter {
                search: "blade".into(),
                ..default()
            },
            page: 0
        }
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn production_projection_builds_named_source_dialog_and_activates_current_ware() -> TestResult {
    let (mut app, mut commands, _, _) = app()?;
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
    let captions: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(captions.iter().any(|text| text == "Test blade"));
    assert!(captions.iter().any(|text| text == "Swords"));
    assert!(!captions.iter().any(|text| text.contains("Item 42")));
    let select = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.kind, Kind::Item(0)).then_some(entity))
        .ok_or("projected ware button")?;
    app.world_mut().trigger(Activate { entity: select });
    let command = feature(&mut commands)?;
    assert_eq!(
        wire::decode_query(&command.payload).map_err(|e| format!("{e:?}"))?,
        MarketQuery::Book {
            definition: 42,
            page: 0
        }
    );
    Ok(())
}

#[test]
fn pending_search_is_cancelled_when_its_original_editor_is_removed() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, _, root) = app()?;
    let mut text = EditableText::new("old");
    text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::new(
        Mutex::new(None),
    )));
    let field = app.world_mut().spawn((Search, text, Node::default())).id();
    app.world_mut().entity_mut(root).add_child(field);
    let search = button(&mut app, root, Kind::Search)?;
    app.world_mut().trigger(Activate { entity: search });
    assert!(app.world().resource::<State>().pending.is_some());
    app.world_mut().despawn(field);
    app.update();
    assert!(app.world().resource::<State>().pending.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}

mod lifecycle;

mod purchase;
