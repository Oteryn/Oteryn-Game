//! Kernel qualification only. Synthetic scoped grants do not admit activity
//! sessions, compose an InstanceRuntime or authorize Channel death/custody.
use super::*;

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

fn grant(scope: RuntimeScopeRefV1, generation: u64) -> PreProductionContinuityGrant {
    PreProductionContinuityGrant {
        runtime_scope: scope,
        scope_generation: ScopeOwnershipGeneration::new(generation).expect("generation"),
    }
}

fn context(owner: &NamespaceContinuityGuard) -> PreProductionPositionContext {
    PreProductionPositionContext {
        scope: positions::PositionScope::new(owner.runtime_scope).expect("valid scoped context"),
        scope_generation: owner.current_generation,
        coordinate_frame_marker: 1,
        map_revision_marker: 2,
        content_generation_marker: 3,
    }
}

fn scopes() -> [RuntimeScopeRefV1; 4] {
    let world = WorldId::decode(&id(1)).expect("world");
    [
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(2)).expect("channel")),
        RuntimeScopeRefV1::instance(world, id(2)).expect("instance with same UUID"),
        RuntimeScopeRefV1::instance(world, id(3)).expect("other instance"),
        RuntimeScopeRefV1::instance(WorldId::decode(&id(4)).expect("other world"), id(2))
            .expect("same instance UUID in other world"),
    ]
}

#[test]
fn boss_activity_carrier_preserves_channel_position_receipt_binding_text() {
    let scope = scopes()[0];
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = scope
    else {
        unreachable!("Channel fixture");
    };
    let owner = NamespaceContinuityGuard::from_pre_production_grant(grant(scope, 1));
    let expected = format!(
        "PreProductionPositionContext {{ world_id: {world_id:?}, channel_id: {channel_id:?}, scope_generation: {:?}, coordinate_frame_marker: 1, map_revision_marker: 2, content_generation_marker: 3 }}",
        owner.current_generation
    );
    assert_eq!(format!("{:?}", context(&owner)), expected);
    let instance = NamespaceContinuityGuard::from_pre_production_grant(grant(scopes()[1], 1));
    assert_ne!(format!("{:?}", context(&instance)), expected);
}

#[test]
fn boss_activity_carrier_compact_position_scope_is_lossless_and_refuses_invalid_instances() {
    assert_eq!(std::mem::size_of::<positions::PositionScope>(), 32);
    let world = scopes()[0].world_id();
    for seed in 0..=u8::MAX {
        for scope in [
            RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(seed)).expect("channel")),
            RuntimeScopeRefV1::instance(world, id(seed)).expect("instance"),
        ] {
            assert_eq!(
                positions::PositionScope::new(scope)
                    .expect("compact")
                    .runtime_scope(),
                Ok(scope)
            );
        }
    }
    let mut wrong_version = id(2);
    wrong_version[6] = 0x40;
    let mut wrong_variant = id(2);
    wrong_variant[8] = 0;
    for instance_id in [[0; 16], wrong_version, wrong_variant] {
        let scope = RuntimeScopeRefV1::Instance {
            world_id: world,
            instance_id,
        };
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant(scope, 1));
        assert!(matches!(
            positions::PositionScope::new(scope),
            Err(CarrierError::WrongScope)
        ));
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1),
            Err(CarrierError::WrongScope)
        );
        assert!(!owner.current_generation_claimed);
    }
}

#[test]
fn boss_activity_carrier_isolates_slots_positions_and_damage_by_typed_scope() {
    let mut owners = scopes().map(|scope| {
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant(scope, 1));
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1)
            .expect("one kernel namespace");
        let actor = carrier
            .admit_creature(&owner, ActorState(7), "boss:kernel", 10)
            .expect("kernel creature");
        assert_eq!(actor.runtime_scope, scope);
        let position = carrier
            .initialize_position(
                &owner,
                actor,
                context(&owner),
                LocalPosition {
                    x: 10,
                    y: 20,
                    floor: 7,
                },
            )
            .expect("own position");
        (owner, carrier, actor, position)
    });
    // Identical local slots, generations, coordinates and content markers. Only
    // World/scope kind/identity distinguish these independently live kernels.
    for i in 0..owners.len() {
        for j in 0..owners.len() {
            if i == j {
                continue;
            }
            let foreign_actor = owners[j].2;
            let foreign_context = context(&owners[j].0);
            let (owner, carrier, actor, position) = &mut owners[i];
            let before = carrier.clone();
            assert_eq!(
                carrier.lookup(owner, foreign_actor),
                Err(CarrierError::WrongScope)
            );
            assert_eq!(
                carrier.compare_commit_position(
                    owner,
                    PositionSnapshot {
                        actor_ref: foreign_actor,
                        ..*position
                    },
                    context(owner),
                    LocalPosition {
                        x: 11,
                        y: 20,
                        floor: 7
                    },
                ),
                Err(CarrierError::WrongScope)
            );
            assert_eq!(
                carrier.compare_commit_position(
                    owner,
                    *position,
                    foreign_context,
                    LocalPosition {
                        x: 11,
                        y: 20,
                        floor: 7
                    },
                ),
                Err(CarrierError::InvalidPreProductionPositionContext)
            );
            assert_eq!(
                carrier.commit_creature_damage_inner(
                    owner,
                    foreign_actor,
                    OwnerDamageCommand {
                        target: b"boss:kernel",
                        occurrence: b"hit",
                        binding: b"hit\0kernel",
                        damage: 3
                    },
                    None,
                    false,
                ),
                Err(CarrierError::WrongScope)
            );
            assert_eq!(
                *carrier, before,
                "foreign reads/writes leave every ledger unchanged"
            );
            assert_eq!(carrier.lookup(owner, *actor), Ok(&ActorState(7)));
        }
    }
    for (owner, carrier, actor, position) in &mut owners {
        let next = carrier
            .compare_commit_position(
                owner,
                *position,
                context(owner),
                LocalPosition {
                    x: 11,
                    y: 20,
                    floor: 7,
                },
            )
            .expect("own movement");
        assert_eq!(next.version.revision, position.version.revision + 1);
        let hit = || OwnerDamageCommand {
            target: b"boss:kernel",
            occurrence: b"hit",
            binding: b"hit\0kernel",
            damage: 10,
        };
        let result = carrier
            .commit_creature_damage_inner(owner, *actor, hit(), None, false)
            .expect("own lethal damage");
        assert_eq!(result.health_before, 10);
        assert_eq!(result.health_after, 0);
        assert_eq!(
            carrier.commit_creature_damage_inner(owner, *actor, hit(), None, false),
            Ok(OwnerDamageResult {
                applied: false,
                ..result
            })
        );
        match actor.runtime_scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                let receipt = carrier
                    .committed_lethal_receipt_inner(owner, *actor)
                    .expect("Channel death");
                assert_eq!(receipt.projection.occurrence.channel_id, channel_id);
            }
            RuntimeScopeRefV1::Instance { .. } => {
                assert!(matches!(
                    carrier.committed_lethal_receipt_inner(owner, *actor),
                    Err(CarrierError::WrongScope)
                ));
                assert!(carrier.corpse_projections.is_empty());
                assert!(carrier.death_reward_occurrences.is_empty());
            }
        }
    }
}

#[test]
fn boss_activity_carrier_continuity_claim_replacement_and_retirement_are_scope_exact() {
    for scope in scopes() {
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant(scope, 1));
        let mut old =
            ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1).expect("first claim");
        let actor = old.admit(&owner, ActorState(1)).expect("old actor");
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
        for other in scopes().into_iter().filter(|other| *other != scope) {
            assert_eq!(
                owner.advance(grant(other, 2)),
                Err(CarrierError::WrongScope)
            );
            let foreign = NamespaceContinuityGuard::from_pre_production_grant(grant(other, 1));
            assert_eq!(old.lookup(&foreign, actor), Err(CarrierError::WrongScope));
            assert_eq!(old.remove(&foreign, actor), Err(CarrierError::WrongScope));
            assert_eq!(old.lookup(&owner, actor), Ok(&ActorState(1)));
        }
        owner
            .advance(grant(scope, 2))
            .expect("same scope successor");
        assert_eq!(old.lookup(&owner, actor), Err(CarrierError::WrongScope));
        assert_eq!(
            old.admit(&owner, ActorState(2)),
            Err(CarrierError::WrongScope)
        );
        let mut current =
            ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1).expect("successor claim");
        let successor = current.admit(&owner, ActorState(3)).expect("new actor");
        assert_eq!(successor.runtime_scope, scope);
        assert_eq!(current.lookup(&owner, actor), Err(CarrierError::WrongScope));
        assert_eq!(
            owner.advance(grant(scope, 2)),
            Err(CarrierError::ContinuityGenerationNotNewer)
        );
        owner.active = false;
        assert_eq!(
            current.lookup(&owner, successor),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            current.remove(&owner, successor),
            Err(CarrierError::WrongScope)
        );
    }
}
