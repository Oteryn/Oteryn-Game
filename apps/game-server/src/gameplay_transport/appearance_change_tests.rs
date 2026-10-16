#![allow(clippy::expect_used)]
//! Pure publication/settlement mechanism cases. Native APP-1 PostgreSQL cases qualify the
//! writer/current reader; these do not replace admission, content or runtime integration tests.
use super::*;
use oteryn_protocol_oteryn::world_spatial::{ActorPosition, StepDirection};
use oteryn_protocol_oteryn::world_spatial_entities::{
    WorldSpatialEntity, decode_world_spatial_entities_delta,
};
use oteryn_protocol_oteryn::{decode_framed_envelope, decode_state_delta};

fn context() -> wire::Context {
    wire::Context {
        content: [1; 32],
        cosmetics: [2; 32],
    }
}
fn catalogue() -> wire::Catalogue {
    wire::Catalogue {
        context: context(),
        character_revision: 1,
        selected: None,
        palette: vec![[1, 2, 3]; 133],
        outfits: Vec::new(),
        mounts: Vec::new(),
    }
}
fn observer_session() -> GameSessionId {
    let mut bytes = [8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    GameSessionId::decode(&bytes).expect("valid fixture or native operation")
}
fn session(generation: u64, caps: &[u32]) -> SessionAppearance {
    SessionAppearance::new(
        ConnectionGeneration::new(generation).expect("valid fixture or native operation"),
        observer_session(),
        caps,
        catalogue(),
    )
    .expect("valid fixture or native operation")
}
fn binding() -> wire::Binding {
    wire::Binding {
        reference: 0,
        entity: EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        context: context(),
        character_revision: 1,
        selection_revision: None,
        condition_revision: 1,
        owner_time_us: 100,
        look: wire::Look::Outfit {
            look_type: 128,
            colours: [0, 1, 2, 132],
            addons: 0,
        },
    }
}
fn visible() -> WorldSpatialEntitiesSnapshot {
    let at = ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    WorldSpatialEntitiesSnapshot {
        content_generation: context().content,
        actor_position: at,
        own_identity: binding().entity.identity,
        entities: vec![WorldSpatialEntity {
            kind: EntityKind::Player,
            entity: binding().entity,
            position: at,
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 0,
                health_percent: 100,
            },
        }],
    }
}

#[test]
fn stationary_update_is_preceded_by_its_mapping_in_native_envelope_order() {
    let mut publisher = session(7, &[6, 25]);
    let visible = visible();
    let update = publisher
        .stage_visible(binding(), &visible, &EntityLights::new(), 40, 6)
        .expect("valid fixture or native operation")
        .expect("valid fixture or native operation");
    assert_eq!(update.last_sequence, 42);
    assert_eq!(update.visibility_revision, 7);
    let frames = update.into_frames();
    let first = decode_framed_envelope(&frames[0]).expect("valid fixture or native operation");
    let second = decode_framed_envelope(&frames[1]).expect("valid fixture or native operation");
    let map = decode_state_delta(first.payload()).expect("valid fixture or native operation");
    let spatial = decode_state_delta(second.payload()).expect("valid fixture or native operation");
    assert_eq!(first.connection_generation(), 7);
    assert_eq!(second.connection_generation(), 7);
    assert_eq!(map.domain_id, 21);
    assert_eq!(spatial.domain_id, 1);
    let mappings = wire::decode_mappings(map.payload).expect("valid fixture or native operation");
    let delta = decode_world_spatial_entities_delta(spatial.payload)
        .expect("valid fixture or native operation");
    assert!(delta.enter.is_empty());
    assert!(delta.leave.is_empty());
    assert_eq!(delta.update.len(), 1);
    assert_eq!(delta.update[0].position, visible.entities[0].position);
    let appearance_ref = match delta.update[0].detail {
        EntityDetail::Actor { appearance_ref, .. } => Some(appearance_ref),
        _ => None,
    }
    .expect("actor detail");
    assert_eq!(appearance_ref, mappings.bindings[0].reference);
    assert_ne!(appearance_ref, 0);
}

#[test]
fn hidden_actor_context_mismatch_and_counter_failure_publish_nothing() {
    for case in ["hidden", "content", "generation", "sequence", "visibility"] {
        let mut publisher = session(1, &[6, 25]);
        let mut view = visible();
        let mut value = binding();
        let mut sequence = 0;
        let mut revision = 0;
        match case {
            "hidden" => view.entities.clear(),
            "content" => value.context.content = [9; 32],
            "generation" => value.entity.generation = 2,
            "sequence" => sequence = u64::MAX - 1,
            "visibility" => revision = u64::MAX,
            _ => return,
        }
        assert!(
            publisher
                .stage_visible(value, &view, &EntityLights::new(), sequence, revision)
                .is_err(),
            "{case}"
        );
        assert_eq!(publisher.next, 1);
        assert_eq!(publisher.revision, 0);
        assert!(publisher.bindings.is_empty());
    }
}

#[test]
fn capabilities_are_required_and_item_dictionary_is_independently_gated() {
    for caps in [&[][..], &[6][..], &[25][..]] {
        assert!(matches!(
            SessionAppearance::new(
                ConnectionGeneration::new(1).expect("valid fixture or native operation"),
                observer_session(),
                caps,
                catalogue()
            ),
            Err(Error::Unsupported)
        ));
    }
    let mut publisher = session(1, &[6, 25]);
    let mut value = binding();
    value.look = wire::Look::Item { definition_ref: 42 };
    assert!(matches!(
        publisher.stage_visible(value.clone(), &visible(), &EntityLights::new(), 0, 0),
        Err(Error::Unsupported)
    ));
    assert!(publisher.bindings.is_empty());
    assert_eq!(publisher.next, 1);
    let mut capable = session(1, &[4, 6, 25]);
    assert!(
        capable
            .stage_visible(value, &visible(), &EntityLights::new(), 0, 0)
            .expect("valid fixture or native operation")
            .is_some()
    );
}

#[test]
fn unchanged_look_is_quiet_but_explicit_temporary_clear_restores_latest_base() {
    let mut publisher = session(1, &[4, 6, 25]);
    let mut value = binding();
    value.look = wire::Look::Item { definition_ref: 42 };
    publisher
        .stage_visible(value.clone(), &visible(), &EntityLights::new(), 0, 0)
        .expect("valid fixture or native operation");
    value.owner_time_us += 1;
    assert!(
        publisher
            .stage_visible(value.clone(), &visible(), &EntityLights::new(), 2, 1)
            .expect("valid fixture or native operation")
            .is_none()
    );
    // The durable base advances while the temporary owner still displays the item.
    value.character_revision = 2;
    value.selection_revision = Some(2);
    publisher
        .stage_visible(value.clone(), &visible(), &EntityLights::new(), 2, 1)
        .expect("valid fixture or native operation");
    value.owner_time_us += 1;
    value.condition_revision += 1;
    value.look = wire::Look::Outfit {
        look_type: 136,
        colours: [1; 4],
        addons: 1,
    };
    assert!(
        publisher
            .stage_visible(value.clone(), &visible(), &EntityLights::new(), 4, 2)
            .expect("valid fixture or native operation")
            .is_some()
    );
    let stored = &publisher.bindings[&value.entity];
    assert_eq!(stored.character_revision, 2);
    assert_eq!(stored.look, value.look);
    let old_next = publisher.next;
    value.owner_time_us = 99;
    assert!(matches!(
        publisher.stage_visible(value, &visible(), &EntityLights::new(), 6, 3),
        Err(Error::Stale)
    ));
    assert_eq!(publisher.next, old_next);
}

#[test]
fn leave_reentry_generation_reset_and_reconnect_do_not_rebind_old_refs() {
    let mut publisher = session(1, &[6, 25]);
    publisher
        .stage_visible(binding(), &visible(), &EntityLights::new(), 0, 0)
        .expect("valid fixture or native operation");
    publisher.retire(binding().entity);
    publisher
        .stage_visible(binding(), &visible(), &EntityLights::new(), 2, 1)
        .expect("valid fixture or native operation");
    assert_eq!(publisher.bindings[&binding().entity].reference, 2);
    let mut value = binding();
    value.entity.generation = 2;
    let mut view = visible();
    view.entities[0].entity = value.entity;
    publisher
        .stage_visible(value.clone(), &view, &EntityLights::new(), 4, 2)
        .expect("valid fixture or native operation");
    publisher.retire(binding().entity);
    assert_eq!(publisher.bindings.len(), 1);
    assert_eq!(publisher.bindings[&value.entity].reference, 3);
    let mut resumed = session(2, &[6, 25]);
    assert!(
        wire::decode_snapshot(
            &resumed
                .snapshot()
                .expect("valid fixture or native operation")
                .1
        )
        .expect("valid fixture or native operation")
        .bindings
        .is_empty()
    );
    resumed
        .stage_visible(value.clone(), &view, &EntityLights::new(), 0, 0)
        .expect("valid fixture or native operation");
    assert_eq!(resumed.bindings[&value.entity].reference, 1);
}

#[test]
fn reference_exhaustion_never_wraps_or_publishes_partial_state() {
    let mut publisher = session(1, &[6, 25]);
    publisher.next = u64::from(u32::MAX);
    publisher
        .stage_visible(binding(), &visible(), &EntityLights::new(), 0, 0)
        .expect("valid fixture or native operation");
    let mut value = binding();
    value.character_revision = 2;
    value.owner_time_us += 1;
    assert!(matches!(
        publisher.stage_visible(value, &visible(), &EntityLights::new(), 2, 1),
        Err(Error::Exhausted)
    ));
    assert_eq!(publisher.bindings[&binding().entity].reference, u32::MAX);
    assert_eq!(publisher.revision, 1);
}

#[test]
fn known_precommit_rejection_and_unknown_storage_outcome_have_different_results() {
    use crate::durability::DurabilityError;
    let known = settlement_error(AppearanceWriteError::StateMismatch);
    assert_eq!(
        wire::decode_result(
            &known
                .result_payload()
                .expect("valid fixture or native operation")
                .expect("valid fixture or native operation")
        )
        .expect("valid fixture or native operation"),
        wire::ChangeResult::Rejected
    );
    for unknown in [
        AppearanceWriteError::Unavailable(DurabilityError::Unavailable),
        AppearanceWriteError::Fence(CharacterProgressionError::Unavailable(
            DurabilityError::RootUnavailable,
        )),
    ] {
        assert!(
            settlement_error(unknown)
                .result_payload()
                .expect("valid fixture or native operation")
                .is_none()
        );
    }
}
