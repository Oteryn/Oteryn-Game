//! Clicked furniture actions and the independent house rotation catalogue.
use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{FeatureForm, PlayView, play_channel},
};
use oteryn_client_runtime::raw_feature::{
    CommandStatus, FeatureCommand, GatedCommandResult, house_rotation, house_wrap,
};
use oteryn_client_runtime::world_interaction::{Action, Command, Selection, Target, world_object};
use oteryn_session::world_map::{
    MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewSnapshot,
};
use oteryn_session::{ActorPosition, EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
use std::num::NonZeroU32;
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
fn app_with_view(view: PlayView) -> Result<Fixture> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, events) = play_channel();
    link.use_shared_frontend();
    host.play = Some(super::super::owner::ActivePlay { view, link });
    host.phase = Phase::Playing;
    let mut app = App::new();
    app.insert_non_send(host);
    install_test_dialogs(&mut app);
    Ok((app, commands, events))
}
fn install_test_dialogs(app: &mut App) {
    app.insert_resource(Skin {
        buttons: source::ButtonImages {
            up: Handle::default(),
            down: Handle::default(),
        },
        frame: Handle::default(),
        font: Handle::default(),
        checked: Handle::default(),
        unchecked: Handle::default(),
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
}
fn app() -> Result<Fixture> {
    app_with_view(
        PlayView::new(TileCoord::new(100, 100), 7, &[])?.with_entities(vec![furniture()?])?,
    )
}
fn choice_entity(app: &mut App, caption: &str) -> Option<Entity> {
    app.world_mut()
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
}
fn choose(app: &mut App, caption: &str) -> Result {
    let entity = choice_entity(app, caption).ok_or_else(|| format!("missing choice: {caption}"))?;
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

const CHAIR: u16 = 2358;
const STONE_TILE: u16 = 406;
const CLICKED: ActorPosition = ActorPosition {
    x: 101,
    y: 100,
    floor: 7,
};
const HEADER: MapViewHeader = MapViewHeader {
    content_generation: [2; 32],
    bundle_digest: [3; 32],
    reset_epoch: 4,
    origin: CLICKED,
    first_visible_floor: 7,
};

fn map_app(appearance_id: u16) -> Result<Fixture> {
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    let item = MapItem {
        definition: MapDefinition::Item(
            NonZeroU32::new(u32::from(appearance_id)).ok_or("appearance id")?,
        ),
        count: 1,
        sub_type: 0,
        origin: MapOrigin::BaseOrdinal {
            ordinal: 2,
            object_revision: 8,
        },
        appearance_id,
    };
    view.set_live_map(&WorldMapViewSnapshot {
        header: HEADER,
        tiles: vec![MapTile {
            position: CLICKED,
            items: vec![item],
            more: false,
            ground_speed: 0,
        }],
    })?;
    app_with_view(view)
}

// Use the pinned appearance assets without changing the environment of parallel tests.
fn with_pinned_assets(test: impl FnOnce() -> Result) -> Result {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if std::env::var_os(crate::map::ASSET_DIR_ENV).as_deref() == Some(root.as_os_str()) {
        return test();
    }
    let thread = std::thread::current();
    let output = std::process::Command::new(std::env::current_exe()?)
        .env(crate::map::ASSET_DIR_ENV, root)
        .arg(thread.name().ok_or("test name")?)
        .args(["--exact", "--nocapture"])
        .output()?;
    crate::play::screenshot::assert_one_test_passed(&output);
    Ok(())
}

#[test]
fn rotate_choice_sends_rotate_for_clicked_placement() -> Result {
    with_pinned_assets(|| {
        let (mut app, mut commands, outbox) = map_app(CHAIR)?;
        outbox.bevy_property_items_selected();
        open_context(&mut app);
        choose(&mut app, "Rotate")?;
        let mut placement = HEADER.bundle_digest.to_vec();
        // The base placement key packs x, y, floor and ordinal into the low six bytes.
        placement.extend([0, 0, 0, 101, 0, 100, 7, 2]);
        assert_eq!(
            commands.try_recv()?,
            PlayCommand::WorldContext(Command {
                target: Target::Object(world_object::WorldObjectTarget {
                    placement,
                    expected_revision: 8,
                }),
                action: Action::Use,
                selection: Some(Selection {
                    position: CLICKED,
                    binding: Some((
                        HEADER.content_generation,
                        HEADER.bundle_digest,
                        HEADER.reset_epoch,
                    )),
                }),
            })
        );
        assert!(commands.try_recv().is_err());

        let (mut public_app, _, _) = map_app(CHAIR)?;
        open_context(&mut public_app);
        assert!(choice_entity(&mut public_app, "Rotate").is_some());
        Ok(())
    })
}

#[test]
fn rotate_choice_absent_for_non_rotatable_item() -> Result {
    with_pinned_assets(|| {
        let (mut app, mut commands, outbox) = map_app(STONE_TILE)?;
        open_context(&mut app);
        assert!(choose(&mut app, "Rotate").is_err());
        app.world_mut().non_send_mut::<HostOwner>().dialogs.clear();
        app.update();
        outbox.bevy_property_items_selected();
        open_context(&mut app);
        assert!(choice_entity(&mut app, "Use").is_some());
        assert!(choose(&mut app, "Rotate").is_err());
        assert!(commands.try_recv().is_err());
        Ok(())
    })
}

#[test]
fn house_rotation_catalogue_opens_and_rotates_the_chosen_furniture() -> Result {
    let (app, mut commands, outbox) = app()?;
    outbox.bevy_property_items_selected();
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    assert!(
        link.house_rotation_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(9)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            repeat: false,
        }])
    );
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
    choose(&mut app, "Wrap/Unwrap")?;
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
    // The first press only reviews the exact wrap; the second press confirms and sends it.
    link.feature_form_submit(window, 3, vec!["1".into()]);
    assert!(commands.try_recv().is_err(), "the review sends nothing");
    assert!(
        link.house_furniture_panel()
            .feedback
            .contains("Press Wrap / place again to confirm")
    );
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

// Exercise the production menu against the live qualification row's map.
