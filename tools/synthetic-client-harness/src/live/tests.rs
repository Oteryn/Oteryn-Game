//! Unit tests for the pure mapping, and one headless controller test against a scripted fake TLS
//! server (same pattern as `tools/dev-client`'s tests). No GPU, no window.

use super::cli::{
    GrantSource, LineCommand, events_for, grant_material, parse_args, parse_character_id,
    parse_line, parse_script,
};
use super::controller::LiveController;
use super::input::LiveInput;
use super::model::{
    DOOR_PLACEMENT, DOOR_STATE_CLOSED, DOOR_STATE_OPEN, DOOR_TILE, DoorState, DoorView,
    LiveCommand, Notice, RenderModel, Tile, Viewport, command_for_click, render_text,
    step_direction_for_action, tile_at_pixel, tile_centre_pixel,
};
use oteryn_dev_client::{
    AppliedDelta, CastOutcome, CommandOutcome, EntityDetail, EntityKind, EntityRef, JoinRequest,
    JoinSnapshot, SessionEvent, StepOutcome, UseOutcome, WorldSpatialEntitiesDelta,
    WorldSpatialEntity, connect_session,
};
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
use oteryn_protocol_oteryn::actor_spell::{
    self, ActorVitals, SpellCastDisposition, SpellCastIntent, SpellTarget, SpellTargetPosition,
};
use oteryn_protocol_oteryn::world_object::{
    self, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition,
    WorldObjectOverlayEntry, WorldObjectTarget, encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    self, ActorPosition, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
    StepDirection, StepDisposition, WorldSpatialObservation, encode_world_spatial,
};
use oteryn_protocol_oteryn::{
    ALPN_OTERYN_GAME_V1, ChannelId, CharacterId, CommandStatus, DomainSnapshot, FrameLength,
    GameSessionId, LivenessAckView, ServerAcceptedValue, WorldId, decode_wire_envelope,
    encode_command_result, encode_liveness_probe, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type BoxError = Box<dyn Error + Send + Sync>;
type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

const CONTENT_GENERATION: [u8; 32] = [0x11; 32];
const GENERATION: u64 = 1;
const FIRST_COMMAND_ID: u64 = 7;
const JOIN_SEQUENCE: u64 = 40;
const JOIN_SPATIAL_REVISION: u64 = 5;
const JOIN_DOOR_REVISION: u64 = 2;

fn view() -> Viewport {
    Viewport::for_size(640, 384)
}

fn model(actor: (i32, i32), door: DoorState, revision: u64) -> RenderModel {
    RenderModel {
        actor: Tile {
            x: actor.0,
            y: actor.1,
            floor: 0,
        },
        door: Some(DoorView {
            tile: DOOR_TILE,
            state: door,
        }),
        overlay_revision: revision,
        vitals: None,
        entities: BTreeMap::new(),
        own_identity: None,
        selected: None,
        notice: Notice::Joined,
    }
}

fn door_entry(state: &[u8], revision: u64) -> WorldObjectOverlayEntry {
    WorldObjectOverlayEntry {
        content_generation: CONTENT_GENERATION,
        placement: DOOR_PLACEMENT.to_vec(),
        state: state.to_vec(),
        revision,
    }
}

fn observation(x: i32, y: i32) -> WorldSpatialObservation {
    WorldSpatialObservation {
        content_generation: CONTENT_GENERATION,
        actor_position: ActorPosition { x, y, floor: 0 },
    }
}

fn uuid_v7(marker: u8) -> [u8; 16] {
    [
        0x01, 0x93, 0x4f, 0x10, 0x7c, 0x00, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

fn snapshot() -> Result<JoinSnapshot, BoxError> {
    Ok(JoinSnapshot {
        game_session_id: GameSessionId::decode(&uuid_v7(1))?,
        world_spatial: observation(0, 0),
        world_spatial_revision: JOIN_SPATIAL_REVISION,
        world_object_overlay: vec![door_entry(DOOR_STATE_CLOSED, JOIN_DOOR_REVISION)],
        world_object_overlay_revision: JOIN_DOOR_REVISION,
    })
}

fn step_outcome(disposition: StepDisposition, moved_to: Option<(i32, i32)>) -> StepOutcome {
    CommandOutcome {
        command_id: 7,
        status: CommandStatus::Accepted,
        disposition,
        result_server_sequence: 41,
        world_spatial_delta: moved_to.map(|(x, y)| AppliedDelta {
            server_sequence: 42,
            base_revision: 5,
            new_revision: 6,
            value: observation(x, y),
        }),
        world_object_overlay_delta: None,
    }
}

fn use_outcome(disposition: UseDisposition) -> UseOutcome {
    CommandOutcome {
        command_id: 8,
        status: CommandStatus::Accepted,
        disposition,
        result_server_sequence: 43,
        world_spatial_delta: None,
        world_object_overlay_delta: None,
    }
}

fn door_event(state: &[u8]) -> SessionEvent {
    SessionEvent::WorldObjectOverlay(AppliedDelta {
        server_sequence: 44,
        base_revision: 2,
        new_revision: 3,
        value: door_entry(state, 3),
    })
}

fn vitals(health: u32) -> ActorVitals {
    ActorVitals {
        health,
        max_health: 150,
        mana: 30,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    }
}

#[test]
fn snapshot_maps_to_actor_and_closed_door() -> Result<(), BoxError> {
    let mapped = RenderModel::from_snapshot(&snapshot()?);
    assert_eq!(mapped, model((0, 0), DoorState::Closed, JOIN_DOOR_REVISION));
    Ok(())
}

#[test]
fn snapshot_without_a_door_maps_to_no_door() -> Result<(), BoxError> {
    let mut joined = snapshot()?;
    joined.world_object_overlay.clear();
    assert_eq!(RenderModel::from_snapshot(&joined).door, None);
    Ok(())
}

#[test]
fn step_delta_moves_the_actor_and_a_blocked_step_does_not() {
    let start = model((0, 0), DoorState::Closed, 2);
    let moved = start.apply_step(&step_outcome(StepDisposition::Moved, Some((1, 0))));
    assert_eq!(
        moved.actor,
        Tile {
            x: 1,
            y: 0,
            floor: 0
        }
    );
    assert_eq!(moved.notice, Notice::Moved);
    let blocked = moved.apply_step(&step_outcome(StepDisposition::Blocked, None));
    assert_eq!(blocked.actor, moved.actor);
    assert_eq!(blocked.notice, Notice::Blocked);
}

#[test]
fn a_committed_use_only_sets_the_notice_and_the_door_event_opens_the_door() {
    let start = model((1, 0), DoorState::Closed, 2);
    let used = start.apply_use(&use_outcome(UseDisposition::Committed));
    assert_eq!(used.door, start.door);
    assert_eq!(used.overlay_revision, 2);
    assert_eq!(used.notice, Notice::DoorCommitted);
    let opened = used.apply_events(&[door_event(DOOR_STATE_OPEN)]);
    assert_eq!(opened.door.map(|door| door.state), Some(DoorState::Open));
    assert_eq!(opened.overlay_revision, 3);
    assert_eq!(opened.notice, Notice::DoorCommitted);
    let closed = opened.apply_events(&[door_event(DOOR_STATE_CLOSED)]);
    assert_eq!(closed.door.map(|door| door.state), Some(DoorState::Closed));
}

#[test]
fn refused_use_leaves_the_door_and_revision_alone() {
    let start = model((1, 0), DoorState::Closed, 2);
    for (disposition, notice) in [
        (UseDisposition::StaleState, Notice::DoorStale),
        (UseDisposition::Occupied, Notice::DoorOccupied),
        (UseDisposition::TooFar, Notice::DoorTooFar),
        (UseDisposition::NothingToUse, Notice::DoorNothingToUse),
        (UseDisposition::Rejected, Notice::DoorRejected),
    ] {
        let after = start.apply_use(&use_outcome(disposition));
        assert_eq!(after.door, start.door);
        assert_eq!(after.overlay_revision, 2);
        assert_eq!(after.notice, notice);
    }
}

#[test]
fn an_unknown_state_key_is_flagged_not_guessed() {
    let start = model((1, 0), DoorState::Closed, 2);
    let after = start.apply_events(&[door_event(b"other")]);
    assert_eq!(
        after.door.map(|door| door.state),
        Some(DoorState::Unrecognised)
    );
}

#[test]
fn pushed_events_apply_in_order_and_the_vitals_line_shows() {
    let start = model((0, 0), DoorState::Closed, 2);
    let spatial = SessionEvent::WorldSpatial(AppliedDelta {
        server_sequence: 42,
        base_revision: 5,
        new_revision: 6,
        value: observation(2, 0),
    });
    let vitals_event = |health, sequence, base| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: sequence,
            base_revision: base,
            new_revision: base + 1,
            value: vitals(health),
        })
    };
    let after = start.apply_events(&[spatial, vitals_event(120, 43, 0), vitals_event(90, 44, 1)]);
    assert_eq!(
        after.actor,
        Tile {
            x: 2,
            y: 0,
            floor: 0
        }
    );
    assert_eq!(after.vitals, Some(vitals(90)));
    assert_eq!(after.notice, Notice::Joined);
    let text = render_text(view(), &after);
    assert!(text.contains("hp 90/150 mp 30/55"), "{text}");
    assert!(!render_text(view(), &start).contains("hp"));
}

#[test]
fn pixels_and_tiles_round_trip_around_the_actor() {
    let actor = Tile {
        x: 1,
        y: 0,
        floor: 0,
    };
    let (px, py) = tile_centre_pixel(view(), actor, DOOR_TILE).unwrap_or((-1, -1));
    assert_eq!(tile_at_pixel(view(), actor, px, py), Some(DOOR_TILE));
    // the actor sits at the grid centre
    let (ax, ay) = tile_centre_pixel(view(), actor, actor).unwrap_or((-1, -1));
    assert_eq!((ax / 32, ay / 32), (10, 6));
    assert_eq!(tile_at_pixel(view(), actor, -1, 5), None);
    assert_eq!(tile_at_pixel(view(), actor, 640, 5), None);
    let other_floor = Tile {
        floor: 1,
        ..DOOR_TILE
    };
    assert_eq!(tile_centre_pixel(view(), actor, other_floor), None);
}

#[test]
fn click_on_the_door_tile_is_use_under_the_mirror_revision() {
    let current = model((1, 0), DoorState::Closed, 9);
    let (px, py) = tile_centre_pixel(view(), current.actor, DOOR_TILE).unwrap_or((-1, -1));
    assert_eq!(
        command_for_click(view(), &current, px, py),
        Some(LiveCommand::UseDoor {
            expected_revision: 9
        })
    );
    // any pixel inside the tile counts, a neighbouring tile does not
    assert_eq!(
        command_for_click(view(), &current, px - 15, py + 15),
        Some(LiveCommand::UseDoor {
            expected_revision: 9
        })
    );
    // a neighbouring tile selects (its top entity, or nothing); off the grid does nothing
    assert_eq!(
        command_for_click(view(), &current, px + 32, py),
        Some(LiveCommand::Select(Tile {
            x: DOOR_TILE.x + 1,
            y: DOOR_TILE.y,
            floor: 0
        }))
    );
    assert_eq!(command_for_click(view(), &current, -1, py), None);
    let no_door = RenderModel {
        door: None,
        ..current
    };
    assert_eq!(
        command_for_click(view(), &no_door, px, py),
        Some(LiveCommand::Select(DOOR_TILE))
    );
}

const OWN_IDENTITY: [u8; 16] = [0xa0; 16];

fn position(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: 0 }
}

fn actor_entity(kind: EntityKind, marker: u8, at: ActorPosition) -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind,
        entity: EntityRef {
            identity: if kind == EntityKind::Player && marker == 0 {
                OWN_IDENTITY
            } else {
                [marker; 16]
            },
            generation: 1,
        },
        position: at,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}

fn object_entity(kind: EntityKind, marker: u8, at: ActorPosition) -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind,
        entity: EntityRef {
            identity: [marker; 16],
            generation: 0,
        },
        position: at,
        detail: EntityDetail::Object {
            item_definition_ref: 9,
            quantity: 1,
            item_handle: None,
        },
    }
}

fn entities_event(
    revision: u64,
    actor: ActorPosition,
    enter: Vec<WorldSpatialEntity>,
    update: Vec<WorldSpatialEntity>,
    leave: Vec<EntityRef>,
) -> SessionEvent {
    SessionEvent::WorldSpatialEntities(AppliedDelta {
        server_sequence: 50 + revision,
        base_revision: revision - 1,
        new_revision: revision,
        value: WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: actor,
            enter,
            update,
            leave,
        },
    })
}

fn entity_model() -> RenderModel {
    let mut current = model((0, 0), DoorState::Closed, 2);
    current.own_identity = Some(OWN_IDENTITY);
    let own = actor_entity(EntityKind::Player, 0, position(0, 0));
    current.entities.insert(own.entity, own);
    current
}

#[test]
fn an_entity_appears_moves_and_disappears_through_pushed_deltas() {
    let walker = actor_entity(EntityKind::Creature, 1, position(2, 0));
    let appeared = entity_model().apply_events(&[entities_event(
        6,
        position(0, 0),
        vec![walker],
        vec![],
        vec![],
    )]);
    assert!(
        render_text(view(), &appeared)
            .lines()
            .any(|line| line.contains("@.C"))
    );
    let moved = actor_entity(EntityKind::Creature, 1, position(2, 1));
    let after_move = appeared.apply_events(&[entities_event(
        7,
        position(0, 0),
        vec![],
        vec![moved],
        vec![],
    )]);
    assert!(
        !render_text(view(), &after_move)
            .lines()
            .any(|line| line.contains("@.C"))
    );
    assert_eq!(
        after_move.top_entity_at(Tile {
            x: 2,
            y: 1,
            floor: 0
        }),
        Some(&moved)
    );
    let gone = after_move.apply_events(&[entities_event(
        8,
        position(0, 0),
        vec![],
        vec![],
        vec![moved.entity],
    )]);
    assert!(!render_text(view(), &gone).contains('C'));
    assert_eq!(gone.entities.len(), 1);
}

#[test]
fn each_kind_has_its_own_glyph_and_the_own_actor_stays_an_at_sign() {
    let drawn = entity_model().apply_events(&[entities_event(
        6,
        position(0, 0),
        vec![
            actor_entity(EntityKind::Player, 1, position(1, 0)),
            actor_entity(EntityKind::Creature, 2, position(2, 0)),
            object_entity(EntityKind::Corpse, 3, position(3, 0)),
            object_entity(EntityKind::GroundItem, 4, position(4, 0)),
        ],
        vec![],
        vec![],
    )]);
    let text = render_text(view(), &drawn);
    assert!(text.lines().any(|line| line.contains("@PCxi")), "{text}");
    assert_eq!(text.matches('@').count(), 1);
}

#[test]
fn a_click_selects_the_top_entity_creature_then_player_then_corpse_then_item() {
    let at = position(1, 0);
    let tile = Tile {
        x: 1,
        y: 0,
        floor: 0,
    };
    let stacked = vec![
        object_entity(EntityKind::GroundItem, 4, at),
        object_entity(EntityKind::Corpse, 3, at),
        actor_entity(EntityKind::Player, 1, at),
        actor_entity(EntityKind::Creature, 2, at),
    ];
    let mut current = entity_model().apply_events(&[entities_event(
        6,
        position(0, 0),
        stacked.clone(),
        vec![],
        vec![],
    )]);
    for expected in [
        EntityKind::Creature,
        EntityKind::Player,
        EntityKind::Corpse,
        EntityKind::GroundItem,
    ] {
        let top = current.top_entity_at(tile).copied();
        assert_eq!(top.map(|entity| entity.kind), Some(expected));
        let selected = current.select_at(tile);
        assert_eq!(selected.selected, top.map(|entity| entity.entity));
        assert!(render_text(view(), &selected).contains("| selected "));
        // remove the top one and the next kind is on top
        let leave = top.map(|entity| entity.entity).into_iter().collect();
        current = selected.apply_events(&[entities_event(
            6 + 1 + u64::from(expected as u8),
            position(0, 0),
            vec![],
            vec![],
            leave,
        )]);
        assert_eq!(
            current.selected, None,
            "a selected entity that leaves is cleared"
        );
    }
    assert_eq!(current.top_entity_at(tile), None);
    assert_eq!(current.select_at(tile).selected, None);
}

#[test]
fn the_own_actor_is_not_selectable_and_a_move_recentres_the_view() {
    let current = entity_model();
    assert_eq!(
        current.top_entity_at(Tile {
            x: 0,
            y: 0,
            floor: 0
        }),
        None
    );
    let own = actor_entity(EntityKind::Player, 0, position(1, 0));
    let moved =
        current.apply_events(&[entities_event(6, position(1, 0), vec![], vec![own], vec![])]);
    assert_eq!(
        moved.actor,
        Tile {
            x: 1,
            y: 0,
            floor: 0
        }
    );
    assert!(!render_text(view(), &moved).contains('P'));
}

#[test]
fn movement_actions_map_to_wire_directions() {
    assert_eq!(
        step_direction_for_action("move.north"),
        Some(StepDirection::North)
    );
    assert_eq!(
        step_direction_for_action("move.west"),
        Some(StepDirection::West)
    );
    assert_eq!(step_direction_for_action("interact"), None);
}

#[test]
fn render_text_draws_actor_and_door_state() {
    let closed = render_text(view(), &model((1, 0), DoorState::Closed, 2));
    let open = render_text(view(), &model((1, 0), DoorState::Open, 3));
    // actor at column 10 / row 6, the door one row above it
    let rows = |text: &str| text.lines().map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(rows(&closed)[6].chars().nth(10), Some('@'));
    assert_eq!(rows(&closed)[5].chars().nth(10), Some('+'));
    assert_eq!(rows(&open)[5].chars().nth(10), Some('/'));
    assert!(open.contains("actor (1, 0, 0)"));
}

#[test]
fn keys_route_to_steps_and_clicks_need_a_pointer_position() -> Result<(), BoxError> {
    let mut input = LiveInput::new()?;
    let current = model((1, 0), DoorState::Closed, 2);
    let key = |code, state| NormalizedInputEvent::Key {
        code,
        state,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    let press = |code| key(code, ButtonState::Pressed);
    assert_eq!(
        input.map_event(&press(KeyCode::ARROW_UP), view(), &current),
        Some(LiveCommand::Step(StepDirection::North))
    );
    assert_eq!(
        input.map_event(
            &key(KeyCode::ARROW_UP, ButtonState::Released),
            view(),
            &current
        ),
        None
    );
    assert_eq!(
        input.map_event(&press(KeyCode::KEY_D), view(), &current),
        Some(LiveCommand::Step(StepDirection::East))
    );
    assert_eq!(
        input.map_event(&press(KeyCode::ESCAPE), view(), &current),
        None
    );

    // a click before any pointer move maps to nothing
    let mut fresh = LiveInput::new()?;
    let events = events_for(LineCommand::UseDoor, view(), &current)?;
    assert_eq!(
        fresh.map_event(&events[1], view(), &current),
        None,
        "press without a known pointer"
    );
    // move, press: the door click; release: nothing
    let mut commands = events
        .iter()
        .filter_map(|event| input.map_event(event, view(), &current));
    assert_eq!(
        commands.next(),
        Some(LiveCommand::UseDoor {
            expected_revision: 2
        })
    );
    assert_eq!(commands.next(), None);
    Ok(())
}

#[test]
fn line_commands_parse_strictly() {
    assert_eq!(parse_line("up"), Some(LineCommand::Key(KeyCode::ARROW_UP)));
    assert_eq!(
        parse_line(" d \n"),
        Some(LineCommand::Key(KeyCode::ARROW_RIGHT))
    );
    assert_eq!(parse_line("use"), Some(LineCommand::UseDoor));
    assert_eq!(parse_line("click 10 20"), Some(LineCommand::Click(10, 20)));
    assert_eq!(parse_line("quit"), Some(LineCommand::Quit));
    assert_eq!(parse_line("click 10"), None);
    assert_eq!(parse_line("up now"), None);
    assert_eq!(parse_line(""), None);
}

#[test]
fn cast_commands_parse_only_supported_targets_and_wire_bounds() -> Result<(), BoxError> {
    let spell = NonZeroU32::new(7).ok_or("nonzero")?;
    for (line, target, aim_at_target) in [
        ("cast 7 self", SpellTarget::None, false),
        ("cast 7 none", SpellTarget::None, false),
        ("cast 7 attack aim", SpellTarget::AttackTarget, true),
        (
            "cast 7 position -2147483648 2147483647 -32768 aim",
            SpellTarget::Position(SpellTargetPosition {
                x: i32::MIN,
                y: i32::MAX,
                floor: i16::MIN,
            }),
            true,
        ),
    ] {
        assert_eq!(
            parse_line(line),
            Some(LineCommand::Cast {
                spell,
                target,
                aim_at_target
            }),
            "{line}"
        );
    }
    for line in [
        "cast",
        "cast 0 self",
        "cast 4294967296 self",
        "cast -1 self",
        "cast 7",
        "cast 7 unknown",
        "cast 7 character 01934f10-7c00-7004-805b-3b1122334404",
        "cast 7 creature 1",
        "cast 7 position 1 2",
        "cast 7 position 1 2 32768",
        "cast 7 position 2147483648 2 0",
        "cast 7 position 1 -2147483649 0",
        "cast 7 self extra",
        "cast 7 attack aim extra",
        "cast 7 position 1 2 0 aim extra",
    ] {
        assert_eq!(parse_line(line), None, "{line}");
    }
    Ok(())
}

#[test]
fn scenario_scripts_validate_entire_input_and_bound_waits() -> Result<(), BoxError> {
    let script =
        "# casts with liveness-preserving wait\n\ncast 7 self\nwait 250\ncast 7 attack aim\nquit\n";
    let parsed = parse_script(script)?;
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[1], LineCommand::Wait(Duration::from_millis(250)));
    assert_eq!(
        parse_line("wait 30000"),
        Some(LineCommand::Wait(Duration::from_secs(30)))
    );
    for invalid in ["wait -1", "wait 30001", "wait 1 extra", "cast 0 self"] {
        assert!(parse_script(invalid).is_err(), "{invalid}");
    }
    assert!(parse_script("expect Success").is_err());
    assert!(parse_script("expect Cast extra").is_err());
    assert!(parse_script("quit\ncast 7 self").is_err());
    assert!(parse_script(&"wait 30000\n".repeat(11)).is_err());
    assert!(parse_script(&"up\n".repeat(4097)).is_err());
    assert!(parse_script(&format!("#{}", "x".repeat(1024 * 1024))).is_err());
    Ok(())
}

#[test]
fn cast_notice_preserves_the_actual_disposition_without_inventing_world_changes() {
    let start = model((1, 0), DoorState::Closed, 2);
    for disposition in [
        SpellCastDisposition::Cast,
        SpellCastDisposition::CoolingDown,
        SpellCastDisposition::LevelTooLow,
        SpellCastDisposition::MagicLevelTooLow,
        SpellCastDisposition::NotEnoughMana,
        SpellCastDisposition::NotEnoughSoul,
        SpellCastDisposition::NotAvailable,
        SpellCastDisposition::TargetRequired,
        SpellCastDisposition::TargetIllegal,
        SpellCastDisposition::Rejected,
    ] {
        assert_eq!(
            parse_line(&format!("expect {disposition:?}")),
            Some(LineCommand::Expect(disposition))
        );
        let after = start.apply_cast(&CastOutcome {
            command_id: 7,
            status: if disposition == SpellCastDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            disposition,
            result_server_sequence: 41,
            actor_vitals_delta: None,
        });
        assert_eq!(after.notice, Notice::SpellCast(disposition));
        assert_eq!(after.actor, start.actor);
        assert_eq!(after.door, start.door);
        assert_eq!(after.overlay_revision, start.overlay_revision);
    }
}

#[test]
fn arguments_come_from_flags_then_env_and_the_grant_is_never_a_flag() {
    let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let env = |name: &str| match name {
        "OTERYN_LIVE_ADDR" => Some("127.0.0.1:7000".to_owned()),
        "OTERYN_LIVE_CA" => Some("ca.pem".to_owned()),
        "OTERYN_LIVE_CHARACTER_ID" => Some("id".to_owned()),
        "OTERYN_LIVE_GRANT" => Some("grant-text".to_owned()),
        _ => None,
    };
    let parsed = parse_args(&args(&["--live"]), &env);
    assert_eq!(
        parsed
            .as_ref()
            .map(|value| (value.address.port(), value.server_name.as_str())),
        Ok((7000, "localhost"))
    );
    assert_eq!(
        parsed.map(|value| value.grant),
        Ok(GrantSource::Inline("grant-text".to_owned()))
    );

    let flagged = parse_args(
        &args(&[
            "--live",
            "--addr",
            "127.0.0.1:1",
            "--grant-file",
            "g.txt",
            "--surface",
            "320x192",
        ]),
        &env,
    );
    assert_eq!(
        flagged.map(|value| (value.address.port(), value.grant, value.surface)),
        Ok((1, GrantSource::File("g.txt".into()), (320, 192)))
    );

    assert!(parse_args(&args(&["--live", "--grant", "secret"]), &env).is_err());
    assert!(parse_args(&args(&["--live", "--addr"]), &env).is_err());
    assert!(parse_args(&args(&["--live"]), &|_| None).is_err());
}

#[test]
fn character_ids_parse_as_uuid_v7_only() {
    assert!(parse_character_id("01934f10-7c00-7004-805b-3b1122334404").is_ok());
    assert!(parse_character_id("01934f107c0070048 05b3b1122334404").is_err());
    assert!(parse_character_id("not-a-uuid").is_err());
    // a v4 UUID is refused by the protocol type
    assert!(parse_character_id("01934f10-7c00-4004-805b-3b1122334404").is_err());
}

// --- headless controller test against a scripted fake TLS server ---

async fn write_frame(stream: &mut ServerStream, body: &[u8]) -> Result<(), BoxError> {
    stream
        .write_all(&FrameLength::new(u32::try_from(body.len())?)?.to_prefix())
        .await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_frame(stream: &mut ServerStream) -> Result<Vec<u8>, BoxError> {
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).await?;
    let mut body = vec![0_u8; FrameLength::from_prefix(&prefix)?.get() as usize];
    stream.read_exact(&mut body).await?;
    Ok(body)
}

async fn send(stream: &mut ServerStream, frames: &[Vec<u8>]) -> Result<(), BoxError> {
    for frame in frames {
        write_frame(stream, frame).await?;
    }
    Ok(())
}

async fn read_command(stream: &mut ServerStream) -> Result<(u64, u32, Vec<u8>), BoxError> {
    let body = read_frame(stream).await?;
    let command = decode_wire_envelope(&body)?.client_command(GENERATION)?;
    Ok((
        command.command_id,
        command.command_type,
        command.payload.to_vec(),
    ))
}

async fn tls_listener() -> Result<
    (
        CertificateDer<'static>,
        tokio_rustls::TlsAcceptor,
        TcpListener,
    ),
    BoxError,
> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    Ok((
        certificate,
        tokio_rustls::TlsAcceptor::from(Arc::new(config)),
        TcpListener::bind("127.0.0.1:0").await?,
    ))
}

fn test_vitals(mana: u32) -> ActorVitals {
    ActorVitals {
        health: 150,
        max_health: 150,
        mana,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    }
}

/// Admits one client, sends the join snapshot (actor at (0,0), door closed at overlay revision
/// 2), then scripts: step East -> Moved to (1,0); a liveness probe; USE door at revision 2 ->
/// Committed, door open at revision 3.
async fn scripted_server(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
) -> Result<(), BoxError> {
    let (tcp, _) = listener.accept().await?;
    let mut stream = acceptor.accept(tcp).await?;
    decode_wire_envelope(&read_frame(&mut stream).await?)?.client_bootstrap()?;
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: GENERATION,
            current_server_sequence: JOIN_SEQUENCE,
            next_command_id: FIRST_COMMAND_ID,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let overlay =
        encode_world_object_overlay_snapshot(&[door_entry(DOOR_STATE_CLOSED, JOIN_DOOR_REVISION)])
            .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let spatial = encode_world_spatial(&observation(0, 0));
    let initial_vitals =
        actor_spell::encode_actor_vitals(&test_vitals(55)).map_err(|e| format!("vitals: {e:?}"))?;
    send(
        &mut stream,
        &encode_single_chunk_snapshot(
            GENERATION,
            1,
            JOIN_SEQUENCE,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: JOIN_SPATIAL_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: JOIN_DOOR_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
                DomainSnapshot {
                    domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    revision: 3,
                    snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    payload: &initial_vitals,
                },
            ],
        )?,
    )
    .await?;

    // step East -> Moved to (1, 0)
    let (id, command_type, payload) = read_command(&mut stream).await?;
    assert_eq!(
        (id, command_type),
        (
            FIRST_COMMAND_ID,
            world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
        )
    );
    assert_eq!(
        world_spatial::decode_step_intent(&payload),
        Ok(StepDirection::East)
    );
    send(
        &mut stream,
        &[
            encode_command_result(
                GENERATION,
                41,
                id,
                CommandStatus::Accepted,
                &world_spatial::encode_step_result(StepDisposition::Moved),
            )?,
            encode_state_delta(
                GENERATION,
                42,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                JOIN_SPATIAL_REVISION,
                JOIN_SPATIAL_REVISION + 1,
                world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
                &encode_world_spatial(&observation(1, 0)),
            )?,
        ],
    )
    .await?;

    // idle: a probe is acked with the last applied sequence
    send(&mut stream, &[encode_liveness_probe(GENERATION, 3)?]).await?;
    let ack = decode_wire_envelope(&read_frame(&mut stream).await?)?.liveness_ack(GENERATION)?;
    assert_eq!(
        ack,
        LivenessAckView {
            probe_id: 3,
            last_applied_server_sequence: Some(42),
        }
    );

    // USE the door under the mirror's revision -> Committed, open at revision 3
    let (id, command_type, payload) = read_command(&mut stream).await?;
    assert_eq!(
        (id, command_type),
        (FIRST_COMMAND_ID + 1, world_object::COMMAND_TYPE_USE_INTENT)
    );
    assert_eq!(
        world_object::decode_use_intent(&payload),
        Ok(WorldObjectTarget {
            placement: DOOR_PLACEMENT.to_vec(),
            expected_revision: JOIN_DOOR_REVISION,
        })
    );
    send(
        &mut stream,
        &[
            encode_command_result(
                GENERATION,
                43,
                id,
                CommandStatus::Accepted,
                &world_object::encode_use_result(UseDisposition::Committed),
            )?,
            encode_state_delta(
                GENERATION,
                44,
                STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                JOIN_DOOR_REVISION,
                JOIN_DOOR_REVISION + 1,
                world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                &world_object::encode_world_object_overlay_delta(&door_entry(
                    DOOR_STATE_OPEN,
                    JOIN_DOOR_REVISION + 1,
                ))
                .map_err(|error| format!("overlay delta: {error:?}"))?,
            )?,
        ],
    )
    .await?;

    // pushed with no command: a vitals change (the Serene cadence) after the door delta
    send(
        &mut stream,
        &[encode_state_delta(
            GENERATION,
            45,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &actor_spell::encode_actor_vitals(&vitals(120))
                .map_err(|error| format!("vitals: {error:?}"))?,
        )?],
    )
    .await?;

    // Exercise the actual actor_spell wire codecs over TLS. This peer scripts outcomes;
    // it does not qualify real server spell execution or damage calculation.
    for (offset, disposition, status, sequence) in [
        (2, SpellCastDisposition::Cast, CommandStatus::Accepted, 46),
        (
            3,
            SpellCastDisposition::NotEnoughMana,
            CommandStatus::Accepted,
            48,
        ),
        (
            4,
            SpellCastDisposition::Rejected,
            CommandStatus::Rejected,
            49,
        ),
    ] {
        let (id, command_type, payload) = read_command(&mut stream).await?;
        assert_eq!(
            (id, command_type),
            (
                FIRST_COMMAND_ID + offset,
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT
            )
        );
        assert_eq!(
            actor_spell::decode_spell_cast_intent(&payload),
            Ok(SpellCastIntent {
                spell: NonZeroU32::new(7).ok_or("nonzero")?,
                target: SpellTarget::AttackTarget,
                aim_at_target: true,
            })
        );
        write_frame(
            &mut stream,
            &encode_command_result(
                GENERATION,
                sequence,
                id,
                status,
                &actor_spell::encode_spell_cast_result(disposition),
            )?,
        )
        .await?;
        if disposition == SpellCastDisposition::Cast {
            write_frame(
                &mut stream,
                &encode_state_delta(
                    GENERATION,
                    47,
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    4,
                    5,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    &actor_spell::encode_actor_vitals(&test_vitals(30))
                        .map_err(|e| format!("vitals: {e:?}"))?,
                )?,
            )
            .await?;
        }
    }

    // hold the connection until the client closes it
    let mut buffer = [0_u8; 1];
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await;
    Ok(())
}

/// join -> arrow-key step -> idle liveness -> click the door tile (USE open): the render model
/// ends with the actor east of the origin, the door open and the overlay revision advanced.
#[test]
fn live_controller_tls_steps_uses_and_records_successful_and_denied_casts() -> Result<(), BoxError>
{
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (certificate, acceptor, listener) = tls_listener().await?;
            let address = listener.local_addr()?;
            let server = tokio::spawn(scripted_server(listener, acceptor));
            let session = connect_session(JoinRequest {
                address,
                server_name: "localhost",
                root_certificate: &certificate,
                schema_revision: 1,
                character_id: CharacterId::decode(&uuid_v7(4))?,
                admission_material: b"fixture-grant",
                client_build_id: "oteryn-synthetic-client-harness-test",
                deadline: Duration::from_secs(5),
            })
            .await?;
            let mut controller = LiveController::new(session, view(), LiveInput::new()?);

            // join snapshot: actor at the origin, door closed, the joined vitals
            assert_eq!(
                controller.model(),
                &RenderModel {
                    vitals: Some(test_vitals(55)),
                    ..model((0, 0), DoorState::Closed, JOIN_DOOR_REVISION)
                }
            );

            // arrow-right -> step East
            let press = NormalizedInputEvent::Key {
                code: KeyCode::ARROW_RIGHT,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
                repeat: false,
            };
            assert!(controller.handle_event(&press).await?);
            assert_eq!(
                controller.model().actor,
                Tile {
                    x: 1,
                    y: 0,
                    floor: 0
                }
            );

            // idle: the probe is answered
            controller.idle(Duration::from_millis(300)).await?;

            // click the door tile (as drawn around the moved actor) -> USE, door opens
            let mut redraw = false;
            for event in events_for(LineCommand::UseDoor, controller.view(), controller.model())? {
                redraw |= controller.handle_event(&event).await?;
            }
            assert!(redraw);
            // The use returned at its result; the door delta and the pushed vitals arrive idle.
            assert_eq!(
                controller.model().door.map(|door| door.state),
                Some(DoorState::Closed)
            );
            assert!(controller.idle(Duration::from_millis(300)).await?);
            let after = controller.model();
            assert_eq!(after.vitals, Some(vitals(120)));
            assert_eq!(after.door.map(|door| door.state), Some(DoorState::Open));
            assert_eq!(after.overlay_revision, JOIN_DOOR_REVISION + 1);
            assert_eq!(after.notice, Notice::DoorCommitted);
            let frame = render_text(controller.view(), after);
            assert_eq!(
                frame.lines().nth(5).and_then(|row| row.chars().nth(10)),
                Some('/')
            );

            assert_eq!(controller.actor_vitals(), Some(&vitals(120)));
            for (disposition, status, sequence) in [
                (SpellCastDisposition::Cast, CommandStatus::Accepted, 46),
                (
                    SpellCastDisposition::NotEnoughMana,
                    CommandStatus::Accepted,
                    48,
                ),
                (SpellCastDisposition::Rejected, CommandStatus::Rejected, 49),
            ] {
                controller
                    .dispatch(LiveCommand::Cast {
                        spell: NonZeroU32::new(7).ok_or("nonzero")?,
                        target: SpellTarget::AttackTarget,
                        aim_at_target: true,
                    })
                    .await?;
                let cast = controller.last_cast().ok_or("cast outcome")?;
                assert_eq!(
                    (cast.disposition, cast.status, cast.result_server_sequence),
                    (disposition, status, sequence)
                );
                assert_eq!(
                    cast.actor_vitals_delta.is_some(),
                    disposition == SpellCastDisposition::Cast
                );
                assert_eq!(controller.actor_vitals(), Some(&test_vitals(30)));
                assert_eq!(controller.model().notice, Notice::SpellCast(disposition));
                assert_eq!(
                    controller.model().actor,
                    Tile {
                        x: 1,
                        y: 0,
                        floor: 0
                    }
                );
                assert!(
                    controller
                        .status_text()
                        .contains(&format!("{disposition:?}"))
                );
            }
            drop(controller);
            server.await??;
            Ok::<(), BoxError>(())
        })
}

#[test]
fn grant_material_strips_trailing_line_endings_only() {
    assert_eq!(grant_material(b"a.b.c\n".to_vec()), b"a.b.c".to_vec());
    assert_eq!(grant_material(b"a.b.c\r\n".to_vec()), b"a.b.c".to_vec());
    assert_eq!(grant_material(b"a.b.c".to_vec()), b"a.b.c".to_vec());
    assert_eq!(grant_material(b" a.b.c ".to_vec()), b" a.b.c ".to_vec());
}
