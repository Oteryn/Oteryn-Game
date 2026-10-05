//! Unit tests for the pure mapping, and one headless controller test against a scripted fake TLS
//! server (same pattern as `tools/dev-client`'s tests). No GPU, no window.

use super::cli::{
    CHAT_USAGE, GrantSource, ITEM_USAGE, LineCommand, USAGE, events_for, grant_material,
    parse_args, parse_character_id, parse_line, parse_script, usage_for,
};
use super::controller::LiveController;
use super::input::LiveInput;
use super::model::{
    ChatPane, DOOR_PLACEMENT, DOOR_STATE_CLOSED, DOOR_STATE_OPEN, DOOR_TILE, DoorState, DoorView,
    ItemPanes, LiveCommand, Notice, RenderModel, Tile, Viewport, command_for_click, render_text,
    step_direction_for_action, tile_at_pixel, tile_centre_pixel,
};
use oteryn_dev_client::{
    AppliedDelta, CastOutcome, CharacterInventory, ChatDisposition, ChatIntent, ChatLine, ChatLog,
    ChatOutcome, ChatRoom, ChatRoomSet, ChatSpeaker, ChatSpeechMode, CommandOutcome, EntityDetail,
    EntityKind, EntityRef, ItemEntry, ItemHandle, ItemMoveDestination, ItemMoveIntent,
    ItemMoveOutcome, ItemMoveOutcomeResult, JoinRequest, JoinSnapshot, MAX_CHAT_LOG_LINES,
    MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES, OpenContainer, SessionEvent, StepOutcome, UseOutcome,
    WorldSpatialEntitiesDelta, WorldSpatialEntity, connect_session,
};
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
use oteryn_protocol_oteryn::actor_spell::{
    self, ActorVitals, SpellCastDisposition, SpellCastIntent, SpellTarget, SpellTargetPosition,
};
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::world_object::{
    self, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition,
    WorldObjectOverlayEntry, WorldObjectTarget, encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    self, ActorPosition, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
    StepDirection, StepDisposition, WorldSpatialObservation, encode_world_spatial,
};
use oteryn_protocol_oteryn::world_spatial_entities::{
    self, WorldSpatialEntitiesSnapshot, encode_world_spatial_entities_snapshot_with_item_handles,
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
        chat: None,
        items: None,
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
    assert_eq!(parse_line("loot 2"), Some(LineCommand::Loot(2)));
    for rejected in ["loot", "loot 0", "loot -1", "loot x", "loot 1 2"] {
        assert_eq!(parse_line(rejected), None, "{rejected}");
    }
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

// --- CHAT-CLIENT-1: the chat pane, the chat input lines and the viewport edge.

/// A model whose session selected capability 7: an empty pane, not an absent one.
fn chat_model() -> RenderModel {
    RenderModel {
        chat: Some(ChatPane::default()),
        ..model((0, 0), DoorState::Closed, 2)
    }
}

fn chat_event(line: ChatLine, revision: u64) -> SessionEvent {
    SessionEvent::ChatLine(AppliedDelta {
        server_sequence: 50 + revision,
        base_revision: revision,
        new_revision: revision + 1,
        value: line,
    })
}

fn room_line(text: &str) -> ChatLine {
    ChatLine::Room {
        room: ChatRoom::Help,
        speaker_name: "Ada".to_owned(),
        text: text.to_owned(),
    }
}

fn chat_outcome(disposition: ChatDisposition, wait_seconds: u32) -> ChatOutcome {
    ChatOutcome {
        command_id: 9,
        status: CommandStatus::Accepted,
        disposition,
        wait_seconds,
        result_server_sequence: 60,
    }
}

#[test]
fn chat_input_lines_map_to_every_intent() {
    let say = |mode, text: &str| {
        Some(LineCommand::Chat(ChatIntent::Say {
            mode,
            text: text.to_owned(),
        }))
    };
    assert_eq!(
        parse_line("say hello  there \n"),
        say(ChatSpeechMode::Say, "hello  there")
    );
    assert_eq!(parse_line("yell HEY"), say(ChatSpeechMode::Yell, "HEY"));
    assert_eq!(
        parse_line("whisper psst"),
        say(ChatSpeechMode::Whisper, "psst")
    );
    assert_eq!(
        parse_line("pm Bob how are you"),
        Some(LineCommand::Chat(ChatIntent::Private {
            recipient_name: "Bob".to_owned(),
            text: "how are you".to_owned(),
        }))
    );
    assert_eq!(
        parse_line("room 3 anyone?"),
        Some(LineCommand::Chat(ChatIntent::Room {
            room: ChatRoom::Help,
            text: "anyone?".to_owned(),
        }))
    );
    assert_eq!(
        parse_line("open 1"),
        Some(LineCommand::Chat(ChatIntent::OpenRoom(ChatRoom::World)))
    );
    assert_eq!(
        parse_line("close 4"),
        Some(LineCommand::Chat(ChatIntent::CloseRoom(
            ChatRoom::Advertising
        )))
    );
    for rejected in [
        "say",
        "say   ",
        "pm Bob",
        "room 5 x",
        "room x y",
        "open",
        "open 9",
        "close 1 2",
    ] {
        assert_eq!(parse_line(rejected), None, "{rejected}");
    }
    let at_bound = format!("say {}", "x".repeat(MAX_CHAT_TEXT_BYTES));
    assert!(matches!(parse_line(&at_bound), Some(LineCommand::Chat(_))));
    assert_eq!(parse_line(&format!("{at_bound}x")), None);
    // Chat is not an input event.
    assert!(
        events_for(
            LineCommand::Chat(ChatIntent::OpenRoom(ChatRoom::World)),
            view(),
            &model((0, 0), DoorState::Closed, 2)
        )
        .is_ok_and(|events| events.is_empty())
    );
}

#[test]
fn every_line_kind_renders_in_the_pane_with_the_dropped_marker() {
    let speaker = ChatSpeaker {
        identity: [9; 16],
        generation: std::num::NonZeroU64::MIN,
    };
    let local = |mode| ChatLine::Local {
        speaker,
        speaker_name: "Bob".to_owned(),
        mode,
        text: "hi".to_owned(),
        position: ActorPosition {
            x: 1,
            y: 1,
            floor: 0,
        },
    };
    let events = [
        chat_event(local(ChatSpeechMode::Say), 0),
        chat_event(local(ChatSpeechMode::Whisper), 1),
        chat_event(local(ChatSpeechMode::Yell), 2),
        chat_event(
            ChatLine::Private {
                speaker_name: "Cy".to_owned(),
                text: "psst".to_owned(),
            },
            3,
        ),
        chat_event(room_line("anyone?"), 4),
        chat_event(ChatLine::Dropped, 5),
        SessionEvent::ChatRooms(AppliedDelta {
            server_sequence: 70,
            base_revision: 6,
            new_revision: 7,
            value: {
                let mut set = ChatRoomSet::default();
                set.insert(ChatRoom::World);
                set.insert(ChatRoom::Help);
                set
            },
        }),
    ];
    let start = chat_model();
    let after = start.apply_events(&events);
    // Applied while idle: the notice is kept, the pane changed.
    assert_eq!(after.notice, Notice::Joined);
    let text = render_text(view(), &after);
    let pane: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("chat ["))
        .collect();
    assert_eq!(
        pane,
        [
            "chat [World, Help]",
            "Bob says: hi",
            "Bob whispers: hi",
            "Bob yells: hi",
            "Cy (private): psst",
            "[Help] Ada: anyone?",
            "-- DROPPED: chat lines were lost --",
        ]
    );
    // Without capability 7 there is no pane at all, and chat deltas change nothing.
    let unselected = model((0, 0), DoorState::Closed, 2);
    assert!(!render_text(view(), &unselected).contains("chat"));
    assert_eq!(unselected.apply_events(&events), unselected);
}

#[test]
fn the_pane_keeps_the_last_64_lines() {
    let events: Vec<SessionEvent> = (0..=MAX_CHAT_LOG_LINES)
        .map(|index| chat_event(room_line(&format!("line {index}")), index as u64))
        .collect();
    let after = chat_model().apply_events(&events);
    let lines = after.chat.map(|chat| chat.lines).unwrap_or_default();
    assert_eq!(lines.len(), MAX_CHAT_LOG_LINES);
    assert_eq!(lines.first(), Some(&room_line("line 1")));
    assert_eq!(
        lines.last(),
        Some(&room_line(&format!("line {MAX_CHAT_LOG_LINES}")))
    );
}

#[test]
fn muted_and_exhausted_show_their_wait_and_change_nothing_else() {
    let start = chat_model().apply_events(&[chat_event(room_line("x"), 0)]);
    for (disposition, wait, text) in [
        (ChatDisposition::Muted, 30, "muted, wait 30 s"),
        (
            ChatDisposition::Exhausted,
            1_280,
            "chat exhausted, wait 1280 s",
        ),
    ] {
        let after = start.apply_chat(&chat_outcome(disposition, wait));
        assert_eq!(after.notice.text(), text);
        assert_eq!(
            RenderModel {
                notice: Notice::Joined,
                ..after
            },
            start
        );
    }
    assert_eq!(
        start
            .apply_chat(&chat_outcome(ChatDisposition::Ok, 0))
            .notice,
        Notice::ChatSent
    );
    assert_eq!(
        start
            .apply_chat(&chat_outcome(ChatDisposition::RoomNotOpen, 0))
            .notice,
        Notice::ChatRefused
    );
    assert_eq!(ChatPane::default().lines.len(), 0);
}

#[test]
fn cells_beyond_the_i32_world_alias_no_real_tile() {
    // The actor stands at the maximum x: the cells to its east are off the world, and the entity
    // at x = i32::MAX must be drawn once (under the actor's column), not repeated eastwards.
    let mut edge = model((i32::MAX, 0), DoorState::Closed, 2);
    let corpse = object_entity(
        EntityKind::Corpse,
        7,
        ActorPosition {
            x: i32::MAX,
            y: -1,
            floor: 0,
        },
    );
    edge.entities.insert(corpse.entity, corpse);
    let text = render_text(view(), &edge);
    assert_eq!(text.matches('x').count(), 1, "{text}");
}

// --- ITEM-CLIENT-1: the backpack and corpse panes, `loot N`, and the CHAT-CLIENT-1 follow-ups.

fn handle(value: u64) -> ItemHandle {
    ItemHandle::new(value).unwrap_or(ItemHandle::MIN)
}

fn entry(handle_value: u64, definition: u32, count: u32) -> ItemEntry {
    ItemEntry {
        handle: handle(handle_value),
        item_definition_ref: std::num::NonZeroU32::new(definition)
            .unwrap_or(std::num::NonZeroU32::MIN),
        count: std::num::NonZeroU32::new(count).unwrap_or(std::num::NonZeroU32::MIN),
        sub_type: 0,
    }
}

fn corpse_with_handle(marker: u8, handle_value: u64, at: ActorPosition) -> WorldSpatialEntity {
    let mut entity = object_entity(EntityKind::Corpse, marker, at);
    entity.detail = EntityDetail::Object {
        item_definition_ref: 9,
        quantity: 1,
        item_handle: Some(handle(handle_value)),
    };
    entity
}

fn backpack() -> CharacterInventory {
    CharacterInventory {
        main_backpack: Some(entry(1, 2854, 1)),
        entries: vec![entry(2, 3031, 5)],
        equipment: vec![],
    }
}

fn open_corpse() -> OpenContainer {
    OpenContainer {
        container_handle: Some(handle(40)),
        entries: vec![entry(41, 3031, 2), entry(42, 3035, 1)],
    }
}

/// A model whose session selected capabilities 4 and 6: empty panes, not absent ones.
fn item_model() -> RenderModel {
    RenderModel {
        items: Some(ItemPanes::default()),
        ..entity_model()
    }
}

fn move_outcome(outcome: ItemMoveOutcome) -> ItemMoveOutcomeResult {
    ItemMoveOutcomeResult {
        command_id: 9,
        status: CommandStatus::Accepted,
        outcome,
        result_server_sequence: 60,
    }
}

fn corpse_tile() -> Tile {
    Tile {
        x: 1,
        y: 0,
        floor: 0,
    }
}

#[test]
fn a_chat_input_without_capability_7_is_a_notice_and_the_help_hides_chat() {
    let plain = model((0, 0), DoorState::Closed, 2);
    assert_eq!(plain.chat, None);
    assert_eq!(usage_for(&plain), USAGE);
    assert!(!usage_for(&plain).contains("say TEXT"));
    assert!(!usage_for(&plain).contains("loot"));
    assert!(usage_for(&chat_model()).contains(CHAT_USAGE));
    assert!(!usage_for(&chat_model()).contains(ITEM_USAGE));
    assert!(usage_for(&item_model()).contains(ITEM_USAGE));
    assert!(!usage_for(&item_model()).contains(CHAT_USAGE));
    // The notice is its own state, with its own text.
    assert!(Notice::ChatUnavailable.text().contains("capability 7"));
    assert!(Notice::ItemsUnavailable.text().contains("capability 4"));
}

#[test]
fn a_pm_recipient_may_be_quoted_to_hold_spaces() {
    let private = |name: &str, text: &str| {
        Some(LineCommand::Chat(ChatIntent::Private {
            recipient_name: name.to_owned(),
            text: text.to_owned(),
        }))
    };
    assert_eq!(
        parse_line("pm \"Al Dric\" hello"),
        private("Al Dric", "hello")
    );
    assert_eq!(
        parse_line("pm  \"Al  Dric\"   hi  there "),
        private("Al  Dric", "hi  there")
    );
    // One unquoted word is still the recipient.
    assert_eq!(
        parse_line("pm Bob how are you"),
        private("Bob", "how are you")
    );
    let long = "n".repeat(MAX_CHAT_NAME_BYTES);
    assert_eq!(parse_line(&format!("pm \"{long}\" x")), private(&long, "x"));
    for rejected in [
        "pm \"\" hello",
        "pm \"Al Dric hello",
        "pm \"Al Dric\"",
        "pm \"Al Dric\"   ",
    ] {
        assert_eq!(parse_line(rejected), None, "{rejected}");
    }
    assert_eq!(parse_line(&format!("pm \"{long}n\" x")), None);
    assert_eq!(parse_line(&format!("pm {long}n x")), None);
}

#[test]
fn a_selected_but_empty_chat_pane_renders_and_an_unselected_one_does_not() {
    let selected = model((0, 0), DoorState::Closed, 2).with_chat(&ChatLog::default());
    assert_eq!(selected.chat, Some(ChatPane::default()));
    let text = render_text(view(), &selected);
    assert!(text.lines().any(|line| line == "chat []"), "{text}");
    let unselected = model((0, 0), DoorState::Closed, 2);
    assert!(!render_text(view(), &unselected).contains("chat"));
}

#[test]
fn clicking_a_corpse_with_a_handle_uses_it_only_with_capability_4() -> Result<(), BoxError> {
    let corpse = corpse_with_handle(3, 40, position(1, 0));
    let enter = |current: &RenderModel| {
        current.apply_events(&[entities_event(
            6,
            position(0, 0),
            vec![corpse],
            vec![],
            vec![],
        )])
    };
    let with_items = enter(&item_model());
    let (px, py) = tile_centre_pixel(view(), with_items.actor, corpse_tile())
        .ok_or("the corpse tile is on the grid")?;
    assert_eq!(
        command_for_click(view(), &with_items, px, py),
        Some(LiveCommand::UseItem {
            handle: handle(40),
            entity: corpse.entity,
        })
    );
    // Without capability 4 the same click only selects.
    let without = enter(&entity_model());
    assert_eq!(
        command_for_click(view(), &without, px, py),
        Some(LiveCommand::Select(corpse_tile()))
    );
    // A corpse with no handle (or a ground item) only selects too.
    let bare = object_entity(EntityKind::Corpse, 5, position(1, 0));
    let bare_model = item_model().apply_events(&[entities_event(
        6,
        position(0, 0),
        vec![bare],
        vec![],
        vec![],
    )]);
    assert_eq!(
        command_for_click(view(), &bare_model, px, py),
        Some(LiveCommand::Select(corpse_tile()))
    );
    Ok(())
}

#[test]
fn the_backpack_and_corpse_panes_render_and_follow_their_deltas() {
    let current = item_model();
    let text = render_text(view(), &current);
    assert!(text.contains("backpack [no backpack]\n"), "{text}");
    assert!(text.contains("corpse [closed]\n"), "{text}");
    let inventory = SessionEvent::Inventory(AppliedDelta {
        server_sequence: 51,
        base_revision: 3,
        new_revision: 4,
        value: backpack(),
    });
    let container = SessionEvent::OpenContainer(AppliedDelta {
        server_sequence: 52,
        base_revision: 4,
        new_revision: 5,
        value: open_corpse(),
    });
    let after = current.apply_events(&[inventory, container]);
    let text = render_text(view(), &after);
    assert!(
        text.contains("backpack [main item 2854 x1]\n  1: item 3031 x5\n"),
        "{text}"
    );
    assert!(
        text.contains("corpse [open]\n  1: item 3031 x2\n  2: item 3035 x1\n"),
        "{text}"
    );
    // Entries are 1-based, and loot names an entry of the open corpse.
    assert_eq!(
        after.loot_intent(2),
        Some(ItemMoveIntent {
            source: handle(42),
            destination: ItemMoveDestination::MainBackpack,
        })
    );
    assert_eq!(after.loot_intent(0), None);
    assert_eq!(after.loot_intent(3), None);
    // Without capability 4 the item events change nothing and there are no panes.
    let plain = entity_model();
    assert_eq!(plain.apply_events(&[container_event_for(&plain)]), plain);
    assert!(!render_text(view(), &plain).contains("backpack"));
    assert_eq!(plain.loot_intent(1), None);
}

fn container_event_for(_model: &RenderModel) -> SessionEvent {
    SessionEvent::OpenContainer(AppliedDelta {
        server_sequence: 52,
        base_revision: 4,
        new_revision: 5,
        value: open_corpse(),
    })
}

#[test]
fn a_stale_move_result_changes_only_the_notice() {
    let opened = item_model().with_items(&backpack(), &open_corpse());
    for (outcome, notice) in [
        (ItemMoveOutcome::Moved, Notice::ItemMoved),
        (ItemMoveOutcome::Stale, Notice::ItemStale),
        (ItemMoveOutcome::TooFar, Notice::ItemTooFar),
        (ItemMoveOutcome::NoRoom, Notice::ItemNoRoom),
        (ItemMoveOutcome::NoBackpack, Notice::ItemNoRoom),
    ] {
        let after = opened.apply_move(&move_outcome(outcome));
        assert_eq!(after.notice, notice);
        assert_eq!(after, opened.with_notice(notice));
        assert_eq!(after.items, opened.items);
    }
    let stale = opened.apply_use_item(&use_outcome(UseDisposition::StaleState));
    assert_eq!(stale, opened.with_notice(Notice::ItemStale));
    let committed = opened.apply_use_item(&use_outcome(UseDisposition::Committed));
    assert_eq!(committed, opened.with_notice(Notice::CorpseOpened));
}

/// Admits one client with capabilities 4 and 6 selected: the corpse at (1, 0) carries handle 40,
/// the backpack holds a main backpack and one entry, and no corpse is open. Then scripts: a USE
/// of the corpse -> Committed and a domain 11 delta (two entries); `loot 1` -> Moved with the
/// domain 11 and 9 deltas; `loot 1` again (on the same view) -> Stale with no delta.
async fn item_server(
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
            selected_capabilities: &[4, 6],
        })?,
    )
    .await?;
    let overlay =
        encode_world_object_overlay_snapshot(&[door_entry(DOOR_STATE_CLOSED, JOIN_DOOR_REVISION)])
            .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let entities =
        encode_world_spatial_entities_snapshot_with_item_handles(&WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: position(0, 0),
            own_identity: OWN_IDENTITY,
            entities: vec![
                actor_entity(EntityKind::Player, 0, position(0, 0)),
                corpse_with_handle(3, 40, position(1, 0)),
            ],
        })
        .map_err(|error| format!("entities snapshot: {error:?}"))?;
    let inventory = item_view::encode_character_inventory(&backpack_before())
        .map_err(|error| format!("inventory: {error:?}"))?;
    let container = item_view::encode_open_container(&OpenContainer::default())
        .map_err(|error| format!("container: {error:?}"))?;
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
                    snapshot_type: world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    payload: &entities,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: JOIN_DOOR_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
                DomainSnapshot {
                    domain_id: item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                    revision: 3,
                    snapshot_type: item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                    payload: &inventory,
                },
                DomainSnapshot {
                    domain_id: item_view::STATE_DOMAIN_OPEN_CONTAINER,
                    revision: 4,
                    snapshot_type: item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
                    payload: &container,
                },
            ],
        )?,
    )
    .await?;

    // USE the corpse (item target) -> Committed, then the container opens as a pushed delta.
    let (id, command_type, payload) = read_command(&mut stream).await?;
    assert_eq!(
        (id, command_type),
        (FIRST_COMMAND_ID, world_object::COMMAND_TYPE_USE_INTENT)
    );
    assert_eq!(payload, world_object::encode_use_item_intent(handle(40)));
    send(
        &mut stream,
        &[
            encode_command_result(
                GENERATION,
                41,
                id,
                CommandStatus::Accepted,
                &world_object::encode_use_result(UseDisposition::Committed),
            )?,
            encode_state_delta(
                GENERATION,
                42,
                item_view::STATE_DOMAIN_OPEN_CONTAINER,
                4,
                5,
                item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
                &item_view::encode_open_container(&open_corpse())
                    .map_err(|error| format!("container delta: {error:?}"))?,
            )?,
        ],
    )
    .await?;

    // loot 1 -> Moved, then the corpse loses the entry and the backpack gains it.
    let (id, command_type, payload) = read_command(&mut stream).await?;
    assert_eq!(
        (id, command_type),
        (
            FIRST_COMMAND_ID + 1,
            item_view::COMMAND_TYPE_ITEM_MOVE_INTENT
        )
    );
    assert_eq!(
        payload,
        item_view::encode_item_move_intent(&ItemMoveIntent {
            source: handle(41),
            destination: ItemMoveDestination::MainBackpack,
        })
        .map_err(|error| format!("{error:?}"))?
    );
    let mut after_container = open_corpse();
    after_container.entries.remove(0);
    let mut after_backpack = backpack_before();
    after_backpack.entries.push(entry(41, 3031, 2));
    send(
        &mut stream,
        &[
            encode_command_result(
                GENERATION,
                43,
                id,
                CommandStatus::Accepted,
                &item_view::encode_item_move_result(ItemMoveOutcome::Moved)
                    .map_err(|error| format!("{error:?}"))?,
            )?,
            encode_state_delta(
                GENERATION,
                44,
                item_view::STATE_DOMAIN_OPEN_CONTAINER,
                5,
                6,
                item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
                &item_view::encode_open_container(&after_container)
                    .map_err(|error| format!("{error:?}"))?,
            )?,
            encode_state_delta(
                GENERATION,
                45,
                item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                3,
                4,
                item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                &item_view::encode_character_inventory(&after_backpack)
                    .map_err(|error| format!("{error:?}"))?,
            )?,
        ],
    )
    .await?;

    // loot 1 again names the entry that is now first (handle 42) -> Stale, no delta follows.
    let (id, command_type, _) = read_command(&mut stream).await?;
    assert_eq!(
        (id, command_type),
        (
            FIRST_COMMAND_ID + 2,
            item_view::COMMAND_TYPE_ITEM_MOVE_INTENT
        )
    );
    send(
        &mut stream,
        &[encode_command_result(
            GENERATION,
            46,
            id,
            CommandStatus::Accepted,
            &item_view::encode_item_move_result(ItemMoveOutcome::Stale)
                .map_err(|error| format!("{error:?}"))?,
        )?],
    )
    .await?;

    let mut buffer = [0_u8; 1];
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await;
    Ok(())
}

fn backpack_before() -> CharacterInventory {
    CharacterInventory {
        main_backpack: Some(entry(1, 2854, 1)),
        entries: vec![],
        equipment: vec![],
    }
}

/// Click a corpse (USE) -> the container opens; `loot 1` -> Moved and both deltas follow; a
/// second `loot 1` -> Stale and the panes stay as they were. A chat input with capability 7 not
/// selected is a notice that sends nothing and leaves the session usable.
#[test]
fn live_controller_opens_a_corpse_and_loots_one_entry_through_both_deltas() -> Result<(), BoxError>
{
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (certificate, acceptor, listener) = tls_listener().await?;
            let address = listener.local_addr()?;
            let server = tokio::spawn(item_server(listener, acceptor));
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

            // join: the panes exist (capability 4), the chat pane does not (capability 7)
            let joined = controller.model().clone();
            assert_eq!(joined.chat, None);
            assert_eq!(
                joined.items,
                Some(ItemPanes {
                    backpack: backpack_before(),
                    corpse: OpenContainer::default(),
                })
            );

            // chat without capability 7: a notice, nothing sent (the server reads no command)
            controller
                .dispatch(LiveCommand::Chat(ChatIntent::OpenRoom(ChatRoom::World)))
                .await?;
            assert_eq!(
                controller.model(),
                &joined.with_notice(Notice::ChatUnavailable)
            );

            // click the corpse tile -> USE of its item handle
            let (px, py) = tile_centre_pixel(view(), joined.actor, corpse_tile())
                .ok_or("the corpse tile is on the grid")?;
            let command = command_for_click(view(), controller.model(), px, py)
                .ok_or("the click maps to a command")?;
            assert!(matches!(command, LiveCommand::UseItem { .. }));
            controller.dispatch(command).await?;
            assert_eq!(controller.model().notice, Notice::CorpseOpened);
            assert!(controller.idle(Duration::from_millis(300)).await?);
            let opened = controller.model().clone();
            assert_eq!(
                opened.items,
                Some(ItemPanes {
                    backpack: backpack_before(),
                    corpse: open_corpse(),
                })
            );
            assert!(render_text(view(), &opened).contains("corpse [open]\n  1: item 3031 x2"));

            // loot 1 -> Moved; the domain 11 and domain 9 deltas follow the result
            controller.dispatch(LiveCommand::Loot { entry: 1 }).await?;
            assert_eq!(controller.model().notice, Notice::ItemMoved);
            assert_eq!(controller.model().items, opened.items);
            assert!(controller.idle(Duration::from_millis(300)).await?);
            let looted = controller.model().clone();
            let mut corpse = open_corpse();
            corpse.entries.remove(0);
            let mut pack = backpack_before();
            pack.entries.push(entry(41, 3031, 2));
            assert_eq!(
                looted.items,
                Some(ItemPanes {
                    backpack: pack,
                    corpse,
                })
            );

            // a Stale result refreshes nothing locally
            controller.dispatch(LiveCommand::Loot { entry: 1 }).await?;
            assert_eq!(controller.model(), &looted.with_notice(Notice::ItemStale));
            assert!(!controller.idle(Duration::from_millis(200)).await?);

            // an entry the corpse does not hold sends nothing
            controller.dispatch(LiveCommand::Loot { entry: 9 }).await?;
            assert_eq!(controller.model().notice, Notice::NoSuchEntry);

            drop(controller);
            server.await??;
            Ok::<(), BoxError>(())
        })
}
