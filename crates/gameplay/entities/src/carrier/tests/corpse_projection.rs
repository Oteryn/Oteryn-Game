#[test]
fn corpse_decay_forgets_only_retired_bindings_under_the_current_owner() {
    let mut runtime = runtime(4);
    for local in 1..=2 {
        let projection = corpse_projection_at(&runtime.continuity, local);
        let actor = projection.occurrence.actor;
        runtime.carrier.retain_corpse_projection(projection);
        runtime
            .bind_corpse_item(
                actor,
                uuid_v7(u64::from(local)),
                "Item",
                "fixture:corpse",
                "r1",
            )
            .expect("a retained death binds its committed corpse");
    }
    runtime
        .retain_live_corpse_items(&[uuid_v7(2)])
        .expect("the current owner reconciles a complete live read");
    let visible = runtime.carrier.bound_corpses(None);
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].item.item_instance_id, uuid_v7(2));
    assert_eq!(
        runtime.carrier.corpse_projections.len(),
        2,
        "bounded death evidence survives"
    );

    runtime.retire_owner_cycle();
    assert!(runtime.retain_live_corpse_items(&[]).is_err());
    assert_eq!(runtime.carrier.bound_corpses(None), visible);
}

#[test]
fn evicting_a_corpse_projection_evicts_its_reward_occurrence() {
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    let actor = |local: u32| {
        ActorRef::from_owner_parts(
            continuity.world_id,
            continuity.channel_id,
            continuity.current_generation,
            ActorLocalId(local),
            ActorLocalGeneration(1),
        )
    };
    let projection = |local: u32| RuntimeCorpseProjection {
        death_at_ms: None,
        occurrence: CreatureDeathOccurrenceRef {
            actor: ExactActorRef(actor(local)),
            commit_binding: Box::new([]),
            damage: 1,
            health_before: 1,
        },
        position: VersionedPosition {
            actor_local_id: ActorLocalId(local),
            actor_local_generation: ActorLocalGeneration(1),
            context,
            position: LocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            revision: 1,
            facing: None,
        },
    };
    for local in 0..2 {
        carrier.retain_corpse_projection(projection(local));
        carrier
            .death_reward_occurrences
            .push(DeathRewardOccurrence {
                actor: actor(local),
                character: [1; 16],
                occurrence: [2; 16],
            });
    }
    let max = u32::try_from(MAX_RETAINED_CORPSE_PROJECTIONS).expect("bounded");
    for local in 2..=max {
        carrier.retain_corpse_projection(projection(local));
    }
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_RETAINED_CORPSE_PROJECTIONS
    );
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .all(|reward| reward.actor != actor(0))
    );
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .any(|reward| reward.actor == actor(1))
    );
}

/// Codex 4248192466 (LOOT-LIVE-1): a projection pinned by an accepted kill settlement
/// survives more than 64 newer projections, such as those of kills a full queue refuses,
/// while the unpinned ones keep their FIFO bound. Once the settle is final and the pin is
/// released, the next projection evicts it as the oldest unpinned one.
#[test]
fn a_pinned_corpse_projection_survives_newer_projections_until_unpinned() {
    let (continuity, mut carrier) = carrier(4);
    let pinned = corpse_actor_at(&continuity, 0);
    carrier.retain_corpse_projection(corpse_projection_at(&continuity, 0));
    carrier
        .death_reward_occurrences
        .push(DeathRewardOccurrence {
            actor: pinned.0,
            character: [1; 16],
            occurrence: [2; 16],
        });
    assert_eq!(carrier.pin_corpse_projection_inner(pinned), Ok(()));
    // Pinning again is a no-op.
    assert_eq!(carrier.pin_corpse_projection_inner(pinned), Ok(()));
    assert_eq!(carrier.corpse_projection_pins, vec![pinned]);
    let max = u32::try_from(MAX_RETAINED_CORPSE_PROJECTIONS).expect("bounded");
    for local in 1..=2 * max {
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
    }
    assert!(carrier.retains_corpse_projection(pinned));
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .any(|reward| reward.actor == pinned.0)
    );
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_RETAINED_CORPSE_PROJECTIONS + 1
    );
    assert!(!carrier.retains_corpse_projection(corpse_actor_at(&continuity, max)));
    assert!(carrier.retains_corpse_projection(corpse_actor_at(&continuity, max + 1)));

    carrier.unpin_corpse_projection_inner(pinned);
    assert!(carrier.corpse_projection_pins.is_empty());
    carrier.retain_corpse_projection(corpse_projection_at(&continuity, 2 * max + 1));
    assert!(!carrier.retains_corpse_projection(pinned));
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .all(|reward| reward.actor != pinned.0)
    );
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_RETAINED_CORPSE_PROJECTIONS
    );
}

/// Codex 4248192466: at most `MAX_PINNED_CORPSE_PROJECTIONS` pins, each on a retained
/// projection of this generation, so `corpse_projections` never holds more than 128.
#[test]
fn corpse_projection_pins_are_bounded_and_name_retained_projections() {
    let (continuity, mut carrier) = carrier(4);
    let max = u32::try_from(MAX_PINNED_CORPSE_PROJECTIONS).expect("bounded");
    assert_eq!(
        carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, 0)),
        Err(CarrierError::CommittedLethalUnavailable),
        "no projection is retained yet"
    );
    for local in 0..max {
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
        assert_eq!(
            carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, local)),
            Ok(())
        );
    }
    carrier.retain_corpse_projection(corpse_projection_at(&continuity, max));
    assert_eq!(
        carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, max)),
        Err(CarrierError::CapacityExceeded)
    );
    for local in max + 1..=3 * max {
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
    }
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_PINNED_CORPSE_PROJECTIONS + MAX_RETAINED_CORPSE_PROJECTIONS
    );
    assert!(
        (0..max)
            .all(|local| carrier.retains_corpse_projection(corpse_actor_at(&continuity, local)))
    );
}
