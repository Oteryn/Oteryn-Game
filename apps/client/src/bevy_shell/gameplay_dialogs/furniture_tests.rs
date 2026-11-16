//! Rotate and Wrap/Unwrap on a clicked item reach the house furniture lists (445, 207).
use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{FeatureForm, PlayView, play_channel},
};
use oteryn_client_runtime::raw_feature::{
    CommandStatus, FeatureCommand, GatedCommandResult, house_rotation, house_wrap,
};
use oteryn_session::{ActorPosition, EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn furniture() -> Result<WorldSpatialEntity> {
    Ok(WorldSpatialEntity {
        motion: None,
        display_name: None,
        entity: EntityRef {
            identity: [5; 16],
            generation: 3,
        },
        kind: EntityKind::GroundItem,
        position: ActorPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Object {
            item_definition_ref: 42,
            quantity: 1,
            item_handle: Some(7.try_into()?),
        },
    })
}
fn app() -> Result<Fixture> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, events) = play_channel();
    link.use_shared_frontend();
    host.play = Some(super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?.with_entities(vec![furniture()?])?,
        link,
    });
    host.phase = Phase::Playing;
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_resource(Skin {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            frame: Handle::default(),
            font: Handle::default(),
        })
        .init_resource::<InputFocus>()
        .add_systems(Update, project);
    app.world_mut().spawn((
        Overlay,
        super::super::game_ui::GameModal,
        GlobalZIndex(160),
        Node {
            display: Display::None,
            ..default()
        },
    ));
    Ok((app, commands, events))
}
fn choose(app: &mut App, caption: &str) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Reply, &Children)>()
        .iter(app.world())
        .find(|(_, _, children)| {
            children.iter().any(|child| {
                app.world()
                    .get::<Text>(child)
                    .is_some_and(|text| text.0 == caption)
            })
        })
        .map(|(entity, _, _)| entity)
        .ok_or_else(|| format!("missing choice: {caption}"))?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn open_context(app: &mut App) {
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_world_context(TileCoord::new(101, 100));
    app.update();
}
fn form(link: &crate::play::PlayLink, title: &str) -> Result<FeatureForm> {
    FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(link) == title)
        .ok_or_else(|| format!("missing form: {title}").into())
}
fn feature(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<FeatureCommand> {
    match commands.try_recv()? {
        PlayCommand::Feature(command) => Ok(command),
        _ => Err("feature command".into()),
    }
}
fn accepted(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    }
}

#[test]
fn rotate_on_clicked_item_lists_house_furniture_and_rotates_the_chosen_one() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    open_context(&mut app);
    assert!(
        choose(&mut app, "Rotate").is_err(),
        "no Rotate before the session admits capability 445"
    );
    app.world_mut().non_send_mut::<HostOwner>().dialogs.clear();
    app.update();
    outbox.bevy_property_items_selected();
    open_context(&mut app);
    choose(&mut app, "Rotate")?;
    let query = feature(&mut commands)?;
    assert_eq!((query.capability, query.kind), (445, 730));
    assert_eq!(
        house_rotation::Request::decode(&query.payload),
        Some(house_rotation::Request::Query(0))
    );
    let catalogue = house_rotation::Catalogue {
        page: 0,
        more: false,
        entries: vec![house_rotation::Entry {
            label: "Wooden cupboard".into(),
            target: house_rotation::Target::Item {
                item: [1; 16],
                expected_revision: 8,
            },
        }],
    };
    let reply = house_rotation::Reply::Catalogue(catalogue.clone())
        .encode()
        .ok_or("catalogue")?;
    outbox.bevy_properties_result(&query, &accepted(reply));
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let window = form(link, "Rotate house furniture")?;
    let view = link.feature_form_view(window);
    assert!(view.open);
    assert!(view.text.contains("1. Wooden cupboard — Rotate"));
    link.feature_form_submit(window, 3, vec!["1".into()]);
    let rotate = feature(&mut commands)?;
    assert_eq!(
        house_rotation::Request::decode(&rotate.payload),
        catalogue.rotate(0)
    );
    let reply = house_rotation::Reply::Applied(house_rotation::UseDisposition::Committed)
        .encode()
        .ok_or("applied")?;
    outbox.bevy_properties_result(&rotate, &accepted(reply));
    assert!(
        link.feature_form_view(window)
            .text
            .contains("Furniture rotated. Refresh the list")
    );
    Ok(())
}

#[test]
fn wrap_on_clicked_item_lists_house_furniture_and_wraps_the_chosen_one() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    open_context(&mut app);
    choose(&mut app, "Wrap / Unwrap")?;
    let query = feature(&mut commands)?;
    assert_eq!((query.capability, query.kind), (207, 402));
    assert_eq!(house_wrap::decode_catalogue_query(&query.payload), Some(0));
    let catalogue = house_wrap::Catalogue {
        page: 0,
        more: false,
        acl_revision: 4,
        position: [101, 100, 7],
        entries: vec![house_wrap::CatalogueEntry {
            item: [7; 16],
            revision: 1,
            action: house_wrap::Action::Wrap,
            name: "oven".into(),
        }],
    };
    outbox.bevy_properties_result(&query, &accepted(catalogue.encode().ok_or("catalogue")?));
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let window = form(link, "House furniture")?;
    let view = link.feature_form_view(window);
    assert!(view.open);
    assert!(view.text.contains("1. oven — Wrap"));
    link.feature_form_submit(window, 3, vec!["1".into()]);
    let wrap = feature(&mut commands)?;
    assert_eq!(
        wrap.payload,
        catalogue.request(0).ok_or("request")?.encode()
    );
    outbox.bevy_properties_result(&wrap, &accepted(house_wrap::Reply::Committed(2).encode()));
    let text = link.feature_form_view(window).text;
    assert!(text.contains("Furniture updated (revision 2)"), "{text}");
    Ok(())
}
