use super::*;
use bevy::{
    camera::NormalizedRenderTarget,
    picking::{
        backend::HitData,
        events::*,
        pointer::{Location, PointerButton, PointerId},
    },
    window::{PrimaryWindow, WindowRef},
};
fn pointer(window: Entity, position: Vec2) -> Result<Pointer, Box<dyn std::error::Error>> {
    Ok(Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(None).ok_or("window")?,
            ),
            position,
        },
    ))
}
fn current_action(
    app: &mut App,
    kind: fn(Kind) -> bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| kind(action.kind).then_some(entity))
        .ok_or_else(|| "market action".into())
}
fn frame(app: &App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world()
        .resource::<State>()
        .root
        .ok_or_else(|| "market frame".into())
}
#[test]
fn dragged_market_frame_and_caption_survive_book_statistics_and_replies() -> TestResult {
    let (mut app, mut commands, outbox, _) = app()?;
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
    crate::bevy_shell::window_drag::install(&mut app);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1600, 1000).into(),
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.world_mut().resource_mut::<State>().seen = None;
    app.update();
    let original = frame(&app)?;
    let caption = app
        .world_mut()
        .query_filtered::<Entity, With<source::DialogCaption>>()
        .single(app.world())?;
    app.world_mut().entity_mut(original).insert((
        ComputedNode {
            size: Vec2::new(1000., 650.),
            inverse_scale_factor: 1.,
            ..default()
        },
        UiGlobalTransform::from_translation(Vec2::new(800., 500.)),
    ));
    let start = Vec2::new(320., 180.);
    let distance = Vec2::new(45., 25.);
    app.world_mut().trigger(PointerPress {
        entity: original,
        pointer: pointer(window, start)?,
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    app.world_mut().trigger(PointerDragStart {
        entity: original,
        pointer: pointer(window, start + distance)?,
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    app.world_mut().trigger(PointerDrag {
        entity: original,
        pointer: pointer(window, start + distance)?,
        button: PointerButton::Primary,
        distance,
        delta: distance,
    });
    let offset = app
        .world()
        .get::<UiTransform>(original)
        .ok_or("drag transform")?
        .translation;
    assert_eq!(
        offset,
        Val2::px(45., 25.),
        "real pointer drag must move the source frame"
    );
    let select = current_action(&mut app, |kind| matches!(kind, Kind::Item(0)))?;
    app.world_mut().trigger(Activate { entity: select });
    let book = feature(&mut commands)?;
    app.update();
    assert_eq!(
        frame(&app)?,
        original,
        "pending Book must retain frame lifetime and placement"
    );
    assert_eq!(
        app.world()
            .get::<UiTransform>(original)
            .ok_or("retained drag")?
            .translation,
        offset
    );
    assert!(app.world().get::<source::DialogCaption>(caption).is_some());
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
    app.update();
    assert_eq!(frame(&app)?, original);
    let statistics = current_action(&mut app, |kind| matches!(kind, Kind::Statistics))?;
    app.world_mut().trigger(Activate { entity: statistics });
    let command = feature(&mut commands)?;
    app.update();
    assert_eq!(frame(&app)?, original);
    outbox.bevy_market_reply(&command, &result(&wire::Reply::Statistics(stats))?);
    app.update();
    assert_eq!(frame(&app)?, original);
    assert_eq!(
        app.world()
            .get::<UiTransform>(original)
            .ok_or("retained final drag")?
            .translation,
        offset
    );
    assert!(app.world().get::<source::DialogCaption>(caption).is_some());
    // Exercise the same generation mismatch branch used after owner replacement.
    app.world_mut()
        .resource_mut::<State>()
        .key
        .as_mut()
        .ok_or("snapshot")?
        .generation += 1;
    app.update();
    let replacement = frame(&app)?;
    assert_ne!(replacement, original);
    assert!(app.world().get_entity(original).is_err());
    assert!(app.world().get_entity(caption).is_err());
    assert_eq!(
        app.world()
            .get::<UiTransform>(replacement)
            .ok_or("new transform")?
            .translation,
        Val2::ZERO
    );
    let close = current_action(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(app.world().get_entity(replacement).is_err());
    Ok(())
}
