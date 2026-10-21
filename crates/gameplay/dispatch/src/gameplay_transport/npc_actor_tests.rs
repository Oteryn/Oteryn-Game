#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "native NPC walking fixtures"
)]

use super::*;
use crate::content::{
    CanonicalProjectDocuments, DefinitionIdentityDocument, ProjectDraft, ProjectReferenceRecord,
    ProjectV2AuthoringProfile, ProjectV2DefinitionRef, ProjectV2Draft, ProjectV2Family,
    ProjectV2Identity, ProjectV2Movement, ProjectV2State, ProjectV2Targeting, ProjectV2Wander,
    ProjectionDocument, npc_catalogue_preproduction_limits, qualify_native_entry_room,
};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use oteryn_simulation_determinism::SemanticTimeMicros;
use {
    crate::foundation::{
        ChannelId, GameSessionId, NodeId, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
    },
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
};

const NPC: &str = "oteryn:npc.alice";
const BEHAVIOR: &str = "oteryn:behavior.npc.alice";

fn source_fields(speed: i64) -> Vec<ProjectV2CandidateField> {
    vec![
        ProjectV2CandidateField {
            field_path: BASE_SPEED.into(),
            value: ProjectV2CandidateValue::Integer(speed),
        },
        ProjectV2CandidateField {
            field_path: FLOOR_CHANGE.into(),
            value: ProjectV2CandidateValue::Boolean(false),
        },
    ]
}

fn behavior(interval_ms: u64, radius_tiles: u16) -> ProjectV2BehaviorAuthoring {
    ProjectV2BehaviorAuthoring {
        movement: ProjectV2Movement {
            can_walk: true,
            pass_through: false,
            pushable: false,
            push_items: false,
            push_creatures: false,
            walks_on_energy: false,
            walks_on_fire: false,
            walks_on_poison: false,
            wander: Some(ProjectV2Wander {
                interval_ms,
                radius_tiles,
            }),
        },
        targeting: ProjectV2Targeting {
            hostile: false,
            can_target: false,
            sense_invisible: false,
            target_distance_tiles: 1,
            static_attack_chance_ppm: 0,
            flee_health: 0,
            change_target: None,
            strategy_weights: None,
        },
        attacks: vec![],
        defenses: vec![],
        voices: None,
        summons: None,
        periodic_audio: None,
        faction: None,
        event_bindings: vec![],
    }
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: "definition-r1".into(),
    }
}

fn catalogue(revision: &str, speed: i64, interval_ms: u64, radius: u16) -> NpcDataCatalogue {
    let generic = |key: &str| ProjectReferenceRecord::Generic {
        identity: DefinitionIdentityDocument {
            family: "Behavior".into(),
            key: key.into(),
            revision: "definition-r1".into(),
        },
        client_projection: ProjectionDocument::ServerOnly,
    };
    let npc =
        |key: &str, fields: Vec<ProjectV2CandidateField>, linked: bool| ProjectV2Declaration::Npc {
            identity: ProjectV2Identity {
                key: key.into(),
                revision: "definition-r1".into(),
            },
            presentation: None,
            behavior: linked.then(|| reference(ProjectV2Family::Behavior, BEHAVIOR)),
            dialogue: None,
            services: vec![],
            fields,
        };
    let draft = ProjectV2Draft {
        core: ProjectDraft {
            project_revision: revision.into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "license:project-owned-v1".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: vec![
                generic(BEHAVIOR),
                generic("oteryn:behavior.npc.unreferenced"),
            ],
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            declarations: vec![
                npc(NPC, source_fields(speed), true),
                npc("oteryn:npc.no-source", vec![], true),
                npc("oteryn:npc.no-behavior", source_fields(speed), false),
            ],
            authoring_profiles: vec![
                ProjectV2AuthoringProfile {
                    target: reference(ProjectV2Family::Behavior, BEHAVIOR),
                    data: ProjectV2AuthoringProfileData::Behavior(behavior(interval_ms, radius)),
                },
                ProjectV2AuthoringProfile {
                    target: reference(
                        ProjectV2Family::Behavior,
                        "oteryn:behavior.npc.unreferenced",
                    ),
                    data: ProjectV2AuthoringProfileData::Behavior(behavior(1_000, 200)),
                },
            ],
            ..ProjectV2State::default()
        },
    };
    let limits = npc_catalogue_preproduction_limits().project;
    let project = CanonicalProjectDocuments::from_v2_draft(draft, limits)
        .expect("native canonical documents")
        .into_snapshot(limits)
        .expect("snapshot")
        .parse(limits)
        .expect("native project");
    NpcDataCatalogue::from_project(project, limits).expect("native NPC catalogue")
}

fn book(interval_ms: u64, radius: u16) -> NpcWalkBook {
    let catalogue = catalogue("walking-r1", 55, interval_ms, radius);
    let model = catalogue.service_model_for_project().expect("native model");
    NpcWalkBook::from_catalogue(&catalogue, &model).expect("same exact source")
}

struct Fixture {
    runtime: ChannelRuntimeV1,
    player: ExactActorRef,
    session: GameSessionId,
    npc: ExactActorRef,
    book: NpcWalkBook,
    chat: ChatRuntime,
}

impl Fixture {
    fn state(&mut self) -> NpcWalkState {
        NpcWalkState::new(&mut self.runtime, self.npc, &self.book)
            .expect("current native placement")
            .expect("source walking profile")
    }

    fn prepare(&mut self, state: &mut NpcWalkState, sequence: u64) -> Result<WalkThink, WalkError> {
        state.prepare(
            &mut self.runtime,
            &self.chat,
            occurrence(self.npc, sequence),
            &root(),
        )
    }
}

fn place(runtime: &mut ChannelRuntimeV1, x: u16, y: u16, floor: i8) -> ExactActorRef {
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: NPC.into(),
                    placements: vec![NpcPlacement {
                        floor,
                        x,
                        y,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .expect("real native placement")
        .actors[0]
}

fn fixture(interval_ms: u64, radius: u16) -> Fixture {
    let (mut runtime, player, session) = runtime_with_capacity(0x76, 3);
    runtime
        .initialize_pinned_test_position(player, at(10, 10))
        .expect("positioned native player");
    let npc = place(&mut runtime, 11, 10, -7);
    let chat = ChatRuntime::default();
    chat.admit(session, "Customer", SemanticTimeMicros::from_micros(0));
    Fixture {
        runtime,
        player,
        session,
        npc,
        book: book(interval_ms, radius),
        chat,
    }
}

fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

fn occurrence(actor: ExactActorRef, sequence: u64) -> ThinkOccurrence {
    ThinkOccurrence { actor, sequence }
}

fn root() -> GameplayDecisionRoot {
    GameplayDecisionRoot::from_bytes([9; 32])
}

fn proposal(result: Result<WalkThink, WalkError>) -> NpcWalkProposal {
    let WalkThink::Propose(proposal) = result.expect("prepared") else {
        panic!("interval is due");
    };
    *proposal
}

#[test]
fn a_retry_cannot_advance_the_interval_or_prepare_a_second_step() {
    let mut f = fixture(2_000, 2);
    let mut state = f.state();
    assert!(matches!(f.prepare(&mut state, 0), Ok(WalkThink::Waiting)));
    assert_eq!(state.ticks_ms, 1_000);
    assert!(matches!(f.prepare(&mut state, 0), Ok(WalkThink::Replay)));
    let before = f
        .runtime
        .borrow_movement_position()
        .read(f.npc)
        .expect("position");
    let proposal = proposal(f.prepare(&mut state, 1));
    assert_eq!(proposal.expected_position(), before);
    assert_eq!(proposal.occurrence(), occurrence(f.npc, 1));
    assert_eq!(
        f.runtime
            .borrow_movement_position()
            .read(f.npc)
            .expect("position"),
        before
    );
    assert!(matches!(f.prepare(&mut state, 1), Ok(WalkThink::Replay)));
    assert!(matches!(
        f.prepare(&mut state, 0),
        Err(WalkError::OccurrenceSuperseded)
    ));
    assert_eq!(state.ticks_ms, 0);
    assert!(matches!(f.prepare(&mut state, 2), Ok(WalkThink::Waiting)));
}

#[test]
fn unperceived_thinks_pause_existing_ticks_without_a_catch_up_burst() {
    let mut f = fixture(2_000, 2);
    let mut state = f.state();
    assert!(matches!(f.prepare(&mut state, 0), Ok(WalkThink::Waiting)));
    f.runtime
        .remove_test_actor(f.player)
        .expect("real player departure");
    for sequence in 1..20 {
        assert!(matches!(
            f.prepare(&mut state, sequence),
            Ok(WalkThink::Idle)
        ));
        assert_eq!(state.ticks_ms, 1_000);
    }
    let session = GameSessionId::decode(&uuid(0x77)).expect("returning session");
    let reservation = f
        .runtime
        .reserve_fresh_session(session)
        .expect("reserve returning player");
    let actor = f
        .runtime
        .commit_fresh_session(reservation)
        .expect("commit returning player");
    f.runtime
        .initialize_pinned_test_position(actor, at(10, 10))
        .expect("position");
    let _ = proposal(f.prepare(&mut state, 20));
    assert_eq!(state.ticks_ms, 0);
    assert!(matches!(f.prepare(&mut state, 21), Ok(WalkThink::Waiting)));
}

#[test]
fn actual_conversation_resets_the_interval_and_leaving_restarts_it() {
    let mut f = fixture(2_000, 2);
    let mut state = f.state();
    assert!(matches!(f.prepare(&mut state, 0), Ok(WalkThink::Waiting)));
    assert_eq!(
        f.chat.converse(f.session, |engaged| *engaged = Some(f.npc)),
        Some(())
    );
    assert!(matches!(f.prepare(&mut state, 1), Ok(WalkThink::Talking)));
    assert_eq!(state.ticks_ms, 0);
    assert!(matches!(f.prepare(&mut state, 2), Ok(WalkThink::Talking)));
    assert!(f.chat.end_engagement(f.session, f.npc));
    assert!(matches!(f.prepare(&mut state, 3), Ok(WalkThink::Waiting)));
    let _ = proposal(f.prepare(&mut state, 4));
}

#[test]
fn unavailable_conversation_owner_is_refused_without_consuming_the_occurrence() {
    let mut f = fixture(1_000, 2);
    let mut state = f.state();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        f.chat
            .converse(f.session, |_| panic!("poison the actual queue owner"));
    }));
    assert!(poisoned.is_err());
    assert!(matches!(
        f.prepare(&mut state, 0),
        Err(WalkError::ConversationUnavailable)
    ));
    assert_eq!(state.last_sequence, None);
    assert_eq!(state.ticks_ms, 0);
    assert!(matches!(
        f.prepare(&mut state, 0),
        Err(WalkError::ConversationUnavailable)
    ));
    assert_eq!(
        f.runtime
            .borrow_movement_position()
            .read(f.npc)
            .expect("unchanged real position")
            .revision(),
        1
    );
}

#[test]
fn wandering_uses_native_occupancy_and_keeps_its_original_radius_after_a_real_step() {
    let mut f = fixture(1_000, 1);
    let mut state = f.state();
    let prepared = proposal(f.prepare(&mut state, 0));
    let destinations = prepared
        .ordered_destinations(&mut f.runtime)
        .expect("current position");
    assert_eq!(destinations.len(), 3);
    assert!(
        !destinations.contains(&at(10, 10)),
        "real player blocks west"
    );
    assert!(destinations.contains(&at(12, 10)));
    let moved = f
        .runtime
        .borrow_movement_position()
        .commit_cardinal(prepared.expected_position(), at(12, 10))
        .expect("native Movement step");
    assert_eq!(moved.position(), at(12, 10));
    assert!(matches!(
        prepared.ordered_destinations(&mut f.runtime),
        Err(WalkError::PositionChanged)
    ));
    let next = proposal(f.prepare(&mut state, 1));
    let destinations = next
        .ordered_destinations(&mut f.runtime)
        .expect("new real position");
    assert_eq!(state.anchor, at(11, 10));
    assert!(
        !destinations.contains(&at(13, 10)),
        "radius never recentres after a move"
    );
    assert!(
        destinations.contains(&at(11, 10)),
        "the vacated real slot is now free"
    );
    assert!(matches!(
        NpcWalkState::new(&mut f.runtime, f.npc, &f.book),
        Err(WalkError::InitialPlacementUnavailable)
    ));
}

#[test]
fn zero_radius_and_native_coordinate_edges_never_propose_a_relocation() {
    let mut f = fixture(1_000, 0);
    let mut state = f.state();
    let prepared = proposal(f.prepare(&mut state, 0));
    assert!(
        prepared
            .ordered_destinations(&mut f.runtime)
            .expect("current")
            .is_empty()
    );
    for (from, direction) in [
        (at(0, 0), MovementFacing::North),
        (at(0, 0), MovementFacing::West),
        (at(65_535, 65_535), MovementFacing::East),
        (at(65_535, 65_535), MovementFacing::South),
        (at(i32::MAX, 0), MovementFacing::East),
        (at(0, i32::MIN), MovementFacing::North),
    ] {
        assert_eq!(cardinal_destination(from, direction), None);
    }
}

#[test]
fn player_identity_retired_owner_and_exhausted_sequence_do_not_create_new_work() {
    let mut f = fixture(1_000, 2);
    assert!(matches!(
        NpcWalkState::new(&mut f.runtime, f.player, &f.book),
        Err(WalkError::NotCurrentNpc)
    ));
    let mut state = f.state();
    assert!(matches!(
        state.prepare(&mut f.runtime, &f.chat, occurrence(f.player, 0), &root()),
        Err(WalkError::NotCurrentNpc)
    ));
    assert_eq!(state.last_sequence, None);
    let prepared = proposal(f.prepare(&mut state, u64::MAX));
    assert!(matches!(
        f.prepare(&mut state, u64::MAX),
        Ok(WalkThink::Replay)
    ));
    assert!(matches!(
        f.prepare(&mut state, 0),
        Err(WalkError::OccurrenceSuperseded)
    ));
    f.runtime.retire_owner_cycle();
    assert!(matches!(
        prepared.ordered_destinations(&mut f.runtime),
        Err(WalkError::NotCurrentNpc)
    ));
    assert!(matches!(
        f.prepare(&mut state, u64::MAX),
        Err(WalkError::NotCurrentNpc)
    ));
}

#[test]
fn every_deterministic_wander_draw_is_a_permutation_in_the_same_native_occurrence() {
    let f = fixture(1_000, 2);
    let first = shuffled_directions(&root(), occurrence(f.npc, 0)).expect("draw");
    let mut different = false;
    for sequence in 0..32 {
        let id = occurrence(f.npc, sequence);
        let order = shuffled_directions(&root(), id).expect("draw");
        assert_eq!(order, shuffled_directions(&root(), id).expect("same draw"));
        for direction in [
            MovementFacing::North,
            MovementFacing::West,
            MovementFacing::East,
            MovementFacing::South,
        ] {
            assert_eq!(order.iter().filter(|found| **found == direction).count(), 1);
        }
        different |= order != first;
    }
    assert!(
        different,
        "different native occurrences must not reuse one fixed direction order"
    );
}

fn uuid(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
    ]
}

#[test]
fn a_qualified_collision_cell_without_terrain_metadata_is_never_a_step_approval() {
    let world = WorldId::decode(&uuid(0x79)).expect("world");
    let room = qualify_native_entry_room(world).expect("real entry qualification");
    let start = room.entry_start();
    assert_eq!((start.x, start.y, start.floor), (0, 0, 0));
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(0x7a)).expect("channel"),
        NodeId::decode(&uuid(0x7b)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        3,
        pin,
    )
    .expect("native owner");
    let session = GameSessionId::decode(&uuid(0x7c)).expect("session");
    let reserved = runtime.reserve_fresh_session(session).expect("reserve");
    let player = runtime.commit_fresh_session(reserved).expect("commit");
    runtime
        .initialize_first_entry_position(player)
        .expect("actual entry");
    let npc = place(&mut runtime, 1, 0, 0);
    let mut state = NpcWalkState::new(&mut runtime, npc, &book(1_000, 2))
        .expect("owner")
        .expect("profile");
    let prepared = proposal(state.prepare(
        &mut runtime,
        &ChatRuntime::default(),
        occurrence(npc, 0),
        &root(),
    ));
    let east = MovementLocalPosition {
        x: 2,
        y: 0,
        floor: 0,
    };
    assert!(
        prepared
            .ordered_destinations(&mut runtime)
            .expect("current")
            .contains(&east)
    );
    let cells = room.movement_cells();
    assert_eq!(
        cells.index().lookup(cells.scope(), cell(east)),
        Ok(CollisionClass::Walkable)
    );
    assert!(
        cells
            .spell_tiles()
            .lookup(cells.scope(), cell(east))
            .expect("entry source explicitly contains the tile")
            .source_step()
            .is_none()
    );
    assert!(
        prepared
            .terrain_candidates(&mut runtime, &room)
            .expect("the anchor exists; candidates without step facts are omitted")
            .is_empty()
    );
    assert_eq!(
        runtime
            .borrow_movement_position()
            .read(npc)
            .expect("position"),
        prepared.expected_position()
    );
}

#[test]
fn terrain_from_another_full_runtime_pin_is_rejected_before_any_candidate_is_returned() {
    let mut f = fixture(1_000, 2);
    let mut state = f.state();
    let prepared = proposal(f.prepare(&mut state, 0));
    let room = qualify_native_entry_room(f.runtime.binding().world_id()).expect("real content");
    assert!(matches!(
        prepared.terrain_candidates(&mut f.runtime, &room),
        Err(WalkError::ContentSourceMismatch)
    ));
}
