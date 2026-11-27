use super::*;
use oteryn_protocol_oteryn::{
    attack::{CAPABILITY_ATTACK_V1, STATE_DOMAIN_ACTOR_COMBAT_STATE, decode_actor_combat_state},
    decode_state_delta,
    world_spatial_entities::decode_world_spatial_entities_delta,
};

const DEMON: WireEntityRef = WireEntityRef {
    identity: [0xd0; 16],
    generation: 1,
};

fn corridor() -> WorldSpatialObservation {
    WorldSpatialObservation {
        actor_position: ActorPosition {
            x: 33222,
            y: 31659,
            floor: 13,
        },
        ..observation()
    }
}

#[derive(Default)]
struct CorridorOwner {
    dead: Cell<bool>,
    busy: Cell<bool>,
    corpse_pending: Cell<bool>,
}

impl FreshAdmissionAuthority for CorridorOwner {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(corridor())
    }

    async fn observe_projected_entities(
        &self,
        actor: ExactActorRef,
        _: SelectedCapabilities,
    ) -> Result<ChannelEntities, AdmissionRefusal> {
        if self.busy.get() {
            return Err(AdmissionRefusal::Unavailable);
        }
        let position = corridor().actor_position;
        let mut entities = vec![ChannelEntity {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
            kind: VisibleKind::Player,
            position,
            revision: 1,
            health_percent: 100,
            direction: StepDirection::East,
            display_name: None,
            motion: None,
            source_icons: Default::default(),
        }];
        if !self.dead.get() || self.corpse_pending.get() {
            entities.push(ChannelEntity {
                identity: DEMON.identity,
                generation: DEMON.generation,
                kind: if self.dead.get() {
                    VisibleKind::PendingDeath
                } else {
                    VisibleKind::Creature
                },
                position: ActorPosition {
                    x: position.x + 1,
                    ..position
                },
                ..entities[0]
            });
        }
        Ok(ChannelEntities {
            content_generation: corridor().content_generation,
            observer: actor.placement_identity(),
            entities,
            lights: EntityLights::new(),
            observed_at_ms: 0,
        })
    }

    async fn observe_combat_state(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<ActorCombatState> {
        Some(ActorCombatState {
            target: (!self.dead.get()).then_some(DEMON),
            modes: FightModes::DEFAULT,
            in_fight: true,
        })
    }

    async fn tick_combat(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<ActorCombatState> {
        self.observe_combat_state(actor, session).await
    }
}

#[test]
fn p11_combat_tick_publishes_dead_demon_removal_before_target_clear() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let authority = CorridorOwner::default();
                    let (mut stream, mut client) = tokio::io::duplex(1 << 20);
                    let mut admitted = admitted(false)?;
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[6, CAPABILITY_ATTACK_V1],
                    )
                    .ok_or("visibility and combat capabilities")?;
                    let actor = admitted.runtime_actor.ok_or("actor")?;
                    let mut session = setup::prepare(
                        &mut stream,
                        &mut admitted,
                        &authority,
                        IDLE_LIVENESS,
                        actor,
                        corridor(),
                        integrity::Integrity::default(),
                    )
                    .await
                    .map_err(|_| "setup")?;
                    loop {
                        let snapshot = read_frame(&mut client).await?;
                        let message = decode_wire_envelope(&snapshot)?.message_type();
                        if message == MessageType::SnapshotCommit {
                            break;
                        }
                        assert!(matches!(
                            message,
                            MessageType::SnapshotBegin | MessageType::SnapshotChunk
                        ));
                    }
                    // Seed the object-capable view that was sent before the death.
                    let live = authority
                        .observe_projected_entities(actor, session.selected_capabilities)
                        .await
                        .map_err(|_| "live entities")?;
                    let mut view =
                        crate::gameplay_transport::world_spatial::SessionVisibility::with_objects(
                            true,
                        );
                    view.snapshot(&live, &mut |_| Ok(()))
                        .map_err(|err| format!("{err:?}"))?;
                    session.visibility = Some(view);
                    assert!(
                        session
                            .visibility
                            .as_ref()
                            .ok_or("visibility")?
                            .shows(DEMON)
                    );
                    let retained = session.admitted.continuity.combat;
                    assert_eq!(retained.sent.and_then(|sent| sent.target), Some(DEMON));
                    let sequence = session.sequence;

                    authority.dead.set(true);
                    authority.busy.set(true);
                    receive(&mut stream, &authority, &mut session, Next::Combat)
                        .await
                        .map_err(|_| "busy combat tick")?;
                    assert_eq!(
                        session.sequence, sequence,
                        "busy visibility must defer the clear"
                    );
                    assert_eq!(session.admitted.continuity.combat, retained);

                    authority.busy.set(false);
                    authority.corpse_pending.set(true);
                    receive(&mut stream, &authority, &mut session, Next::Combat)
                        .await
                        .map_err(|_| "pending corpse combat tick")?;
                    assert!(
                        session
                            .visibility
                            .as_ref()
                            .ok_or("visibility")?
                            .shows(DEMON)
                    );
                    assert_eq!(
                        session.sequence, sequence,
                        "the retained demon must defer the clear"
                    );
                    assert_eq!(session.admitted.continuity.combat, retained);

                    authority.corpse_pending.set(false);
                    receive(&mut stream, &authority, &mut session, Next::Combat)
                        .await
                        .map_err(|_| "combat retry")?;
                    assert_eq!(session.sequence, sequence + 2);
                    let spatial = read_frame(&mut client).await?;
                    let spatial = decode_state_delta(decode_wire_envelope(&spatial)?.payload())?;
                    assert_eq!(spatial.domain_id, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY);
                    let spatial = decode_world_spatial_entities_delta(spatial.payload)
                        .map_err(|err| format!("{err:?}"))?;
                    assert_eq!(spatial.leave, [DEMON]);
                    let combat = read_frame(&mut client).await?;
                    let combat = decode_state_delta(decode_wire_envelope(&combat)?.payload())?;
                    assert_eq!(combat.domain_id, STATE_DOMAIN_ACTOR_COMBAT_STATE);
                    assert_eq!(
                        decode_actor_combat_state(combat.payload)
                            .map_err(|err| format!("{err:?}"))?
                            .target,
                        None
                    );
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "combat visibility test thread panicked")?
}
