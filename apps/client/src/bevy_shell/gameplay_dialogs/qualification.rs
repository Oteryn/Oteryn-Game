//! Drives the production native Rotate interaction against the already admitted TLS session.
use super::*;
use crate::play::{GroundItem, PlayView};
use oteryn_client_runtime::world_interaction::Command;
use oteryn_session::{
    ActorPosition, AppliedDelta, GatedSnapshot, Session, SessionEvent, SessionStream,
    world_map::{MapOrigin, WorldMapViewDelta},
};
use std::{cell::RefCell, time::Duration};
use tokio::time::Instant;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct Trace {
    commands: Vec<Exchange>,
    map_events: Vec<(Instant, SessionEvent)>,
}

struct Exchange {
    command: Command,
    command_id: u64,
    next_id: u64,
    before_sequence: u64,
    reply_sequence: u64,
    reply_at: Instant,
    predecessor: Option<GatedSnapshot>,
}

tokio::task_local! {
    static TRACE: RefCell<Trace>;
}

pub(crate) fn record_world_context(
    command: &Command,
    (command_id, before_sequence, predecessor): (u64, u64, Option<GatedSnapshot>),
    (next_id, reply_sequence): (u64, u64),
) {
    let _ = TRACE.try_with(|trace| {
        trace.borrow_mut().commands.push(Exchange {
            command: command.clone(),
            command_id,
            next_id,
            before_sequence,
            reply_sequence,
            reply_at: Instant::now(),
            predecessor,
        });
    });
}

pub(crate) fn record_map_events(events: &[SessionEvent]) {
    let received_at = Instant::now();
    let _ = TRACE.try_with(|trace| {
        trace.borrow_mut().map_events.extend(
            events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        SessionEvent::WorldMap(_) | SessionEvent::WorldMapSnapshot(_)
                    )
                })
                .cloned()
                .map(|event| (received_at, event)),
        );
    });
}

pub struct RotateProof {
    pub command_id: u64,
    pub before_sequence: u64,
    pub reply_sequence: u64,
    pub predecessor: GatedSnapshot,
    pub delta: AppliedDelta<WorldMapViewDelta>,
}

/// Opens the native menu by pointer input and checks feedback and scene output on the same session.
pub async fn rotate_live_chair<S: SessionStream + Send>(
    session: Session<S>,
    actor: ActorPosition,
    target: ActorPosition,
    ordinal: u8,
    expected: &Command,
) -> Result<(Session<S>, RotateProof)> {
    let view = PlayView::new(TileCoord::new(actor.x, actor.y), actor.floor, &[])?
        .with_join_scene(&session)?;
    let session_id = *session.join_snapshot().game_session_id.as_bytes();
    let textures = ChairTextures::load(target)?;
    let (mut app, commands, outbox) = crate::bevy_shell::input::hits::qualification_input(view)?;
    install_test_dialogs(&mut app);
    let (session, observed, trace) = TRACE
        .scope(RefCell::new(Trace::default()), async move {
            let (session, observed) = tokio::join!(
                crate::play::qualification_run_session(session, commands, outbox),
                observe_rotation(app, session_id, target, ordinal, textures),
            );
            (session, observed, TRACE.with(|trace| trace.take()))
        })
        .await;
    observed?;
    let [exchange]: [Exchange; 1] = trace
        .commands
        .try_into()
        .map_err(|_| "I09 native loop did not dispatch exactly one world-context command")?;
    if &exchange.command != expected || exchange.command_id.checked_add(1) != Some(exchange.next_id)
    {
        return Err(
            "I09 native loop did not send the exact clicked placement and selection".into(),
        );
    }
    let mut deltas = trace
        .map_events
        .into_iter()
        .filter_map(|(received_at, event)| match event {
            SessionEvent::WorldMapSnapshot(snapshot) => Some(Err(format!(
                "I09 native map replaced at sequence {}",
                snapshot.server_sequence
            ))),
            SessionEvent::WorldMap(delta)
                if delta.server_sequence > exchange.before_sequence
                    && (delta.value.cleared.contains(&target)
                        || delta.value.tiles.iter().any(|tile| tile.position == target)) =>
            {
                Some(Ok((received_at, delta)))
            }
            _ => None,
        });
    let (received_at, delta) = deltas
        .next()
        .ok_or("I09 native session saw no chair update")??;
    require_map_window(exchange.reply_at, received_at)?;
    if deltas.next().is_some() {
        return Err("I09 native session saw a second chair update or replacement".into());
    }
    Ok((
        session,
        RotateProof {
            command_id: exchange.command_id,
            before_sequence: exchange.before_sequence,
            reply_sequence: exchange.reply_sequence,
            predecessor: exchange
                .predecessor
                .ok_or("I09 native USE predecessor absent")?,
            delta,
        },
    ))
}

async fn observe_rotation(
    mut app: App,
    session_id: [u8; 16],
    target: ActorPosition,
    ordinal: u8,
    textures: ChairTextures,
) -> Result {
    let tile = TileCoord::new(target.x, target.y);
    // The row deadline also bounds the native renderer. The map deadline is
    // checked separately against the exchange and its received map event.
    let mut feedback_seen = false;
    let mut clicked = false;
    loop {
        app.update();
        let ready_to_click = {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            let play = owner.play.as_mut().ok_or("I09 native play missing")?;
            let feedback = play.link.world_context_feedback();
            if clicked {
                observe_rotate_feedback(&mut feedback_seen, feedback.as_deref())?;
            }
            play.view
                .tick(&play.link)
                .map_err(|error| format!("I09 native play ended: {error:?}"))?;
            let orientation = displayed_chair(&play.view, tile, ordinal);
            if clicked {
                if rotate_presented(feedback_seen, orientation) {
                    return textures.check(&play.view, target, true);
                }
                false
            } else if play.link.qualification_scope_ready(session_id) {
                if orientation != Some((2358, 0)) || feedback.is_some() {
                    return Err(format!(
                        "I09 native Rotate pre-state differs: orientation={orientation:?} feedback={feedback:?}"
                    ).into());
                }
                textures.check(&play.view, target, false)?;
                true
            } else {
                false
            }
        };
        if ready_to_click {
            crate::bevy_shell::input::hits::qualification_right_click(&mut app, tile)?;
            click_rotate(&mut app)?;
            clicked = true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn observe_rotate_feedback(seen: &mut bool, feedback: Option<&str>) -> Result {
    match feedback {
        Some("Use: completed") => *seen = true,
        None | Some("Waiting for server") => (),
        Some(feedback) => return Err(format!("I09 native Rotate feedback: {feedback}").into()),
    }
    Ok(())
}

fn rotate_presented(feedback_seen: bool, orientation: Option<(u16, u64)>) -> bool {
    feedback_seen && orientation == Some((2359, 1))
}

fn require_map_window(reply: Instant, map_event: Instant) -> Result {
    let elapsed = map_event.saturating_duration_since(reply);
    if elapsed >= Duration::from_secs(5) {
        return Err(format!(
            "I09 native map event arrived {} ms after the USE reply",
            elapsed.as_millis()
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn i09_native_requires_both_feedback_and_projected_orientation() -> Result {
    let mut feedback_seen = false;
    observe_rotate_feedback(&mut feedback_seen, None)?;
    assert!(!rotate_presented(feedback_seen, Some((2359, 1))));
    observe_rotate_feedback(&mut feedback_seen, Some("Waiting for server"))?;
    assert!(!rotate_presented(feedback_seen, Some((2359, 1))));
    assert!(observe_rotate_feedback(&mut feedback_seen, Some("Target changed")).is_err());
    observe_rotate_feedback(&mut feedback_seen, Some("Use: completed"))?;
    assert!(!rotate_presented(feedback_seen, Some((2358, 0))));
    assert!(!rotate_presented(feedback_seen, Some((2359, 0))));
    assert!(rotate_presented(feedback_seen, Some((2359, 1))));
    Ok(())
}

#[cfg(test)]
#[test]
fn i09_map_deadline_is_based_on_received_event() -> Result {
    let reply = Instant::now();
    require_map_window(reply, reply + Duration::from_millis(4999))?;
    assert!(require_map_window(reply, reply + Duration::from_secs(5)).is_err());
    Ok(())
}

fn displayed_chair(view: &PlayView, tile: TileCoord, expected_ordinal: u8) -> Option<(u16, u64)> {
    let GroundItem::Map { item, .. } = view.scene().item_target_at(tile)? else {
        return None;
    };
    let MapOrigin::BaseOrdinal {
        ordinal,
        object_revision,
    } = item.origin
    else {
        return None;
    };
    (ordinal == expected_ordinal).then_some((item.appearance_id, object_revision))
}

struct ChairTextures {
    original: Vec<u8>,
    rotated: Vec<u8>,
}

impl ChairTextures {
    fn load(target: ActorPosition) -> Result<Self> {
        use oteryn_client_assets::{
            AppearanceIndex, AssetStore, Catalog, Placement, SpriteSheets,
            world::{MANIFEST_PATH, MANIFEST_SHA256},
        };
        let root = std::path::PathBuf::from(
            std::env::var_os(crate::map::ASSET_DIR_ENV).ok_or("I09 native assets missing")?,
        );
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )?;
        let catalog = Catalog::load(&store)?;
        let index = AppearanceIndex::load(&store, &catalog)?;
        let mut sheets = SpriteSheets::new(store, catalog);
        let mut texture = |id| -> Result<Vec<u8>> {
            let placement = Placement::at(target.x, target.y, i32::from(target.floor));
            let resolved = index.resolve(&sheets, id, placement)?;
            let [cell] = resolved.cells.as_slice() else {
                return Err("I09 expected a single-cell chair appearance".into());
            };
            Ok(sheets.cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)?)
        };
        let original = texture(2358)?;
        let rotated = texture(2359)?;
        if original == rotated {
            return Err("I09 chair orientation assets are identical".into());
        }
        Ok(Self { original, rotated })
    }

    fn check(&self, view: &PlayView, target: ActorPosition, rotated: bool) -> Result {
        let scene = view.scene();
        let tile = scene
            .view()
            .tile_to_screen(TileCoord::new(target.x, target.y));
        let (expected, forbidden) = if rotated {
            (&self.rotated, &self.original)
        } else {
            (&self.original, &self.rotated)
        };
        let mut expected_count = 0;
        let mut forbidden_count = 0;
        for (range, floor) in scene.floor_quad_ranges() {
            if *floor != target.floor {
                continue;
            }
            for quad in &scene.sprites().instances()[range.clone()] {
                if (quad.position[0] - tile[0]).abs() >= quad.size[0]
                    || (quad.position[1] - tile[1]).abs() >= quad.size[1]
                {
                    continue;
                }
                let atlas = scene.unlit_atlas();
                let width = atlas.width() as usize;
                let height = atlas.height() as usize;
                let x = (quad.uv[0] * width as f32).round() as usize;
                let y = (quad.uv[1] * height as f32).round() as usize;
                let cell = atlas.cell_px() as usize;
                if x + cell > width || y + cell > height || expected.len() != cell * cell * 4 {
                    continue;
                }
                let matches = |pixels: &[u8]| {
                    (0..cell).all(|row| {
                        let start = ((y + row) * width + x) * 4;
                        atlas.rgba()[start..start + cell * 4]
                            == pixels[row * cell * 4..(row + 1) * cell * 4]
                    })
                };
                expected_count += usize::from(matches(expected));
                forbidden_count += usize::from(matches(forbidden));
            }
        }
        if (expected_count, forbidden_count) != (1, 0) {
            return Err(format!(
                "I09 native chair texture mismatch rotated={rotated} expected={expected_count} forbidden={forbidden_count}"
            ).into());
        }
        Ok(())
    }
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
fn click_rotate(app: &mut App) -> Result {
    use bevy::{
        camera::RenderTarget,
        picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        ui::UiGlobalTransform,
        window::PrimaryWindow,
    };
    // Run layout and visibility before locating the actual button geometry.
    app.update();
    for mut visibility in app
        .world_mut()
        .query::<&mut InheritedVisibility>()
        .iter_mut(app.world_mut())
    {
        *visibility = InheritedVisibility::VISIBLE;
    }
    let button = choice_entity(app, "Rotate").ok_or("missing choice: Rotate")?;
    let position = app
        .world()
        .get::<UiGlobalTransform>(button)
        .ok_or("Rotate geometry")?
        .translation;
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    let location = Location {
        target: RenderTarget::default()
            .normalize(Some(window))
            .ok_or("I09 window target")?,
        position,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
    Ok(())
}
