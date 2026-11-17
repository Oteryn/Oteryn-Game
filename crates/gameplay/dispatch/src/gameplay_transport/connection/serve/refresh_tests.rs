//! NBPROFILE: a named visibility read that only met a busy holder skips the tick; a hard
//! failure still ends the session.

use super::*;
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use crate::gameplay_transport::world_spatial::{
    ActorPosition, ChannelEntities, ChannelEntity, VisibleKind,
};
use oteryn_protocol_oteryn::entity_names::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES_V2, EntityDisplayName,
};
use std::cell::Cell;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn observation() -> WorldSpatialObservation {
    WorldSpatialObservation {
        content_generation: [7; 32],
        actor_position: ActorPosition {
            x: 300,
            y: 300,
            floor: 7,
        },
    }
}

struct NamedOwner {
    failure: Cell<Option<ObservationError>>,
    reads: Cell<usize>,
}

impl FreshAdmissionAuthority for NamedOwner {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(observation())
    }

    async fn observe_named_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> Result<ChannelEntities, ObservationError> {
        self.reads.set(self.reads.get() + 1);
        if let Some(failure) = self.failure.get() {
            return Err(failure);
        }
        Ok(ChannelEntities {
            content_generation: observation().content_generation,
            observer: actor.placement_identity(),
            entities: vec![ChannelEntity {
                display_name: Some(
                    EntityDisplayName::new("Observer")
                        .map_err(|_| ObservationError::Unavailable)?,
                ),
                health_percent: 100,
                direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                kind: VisibleKind::Player,
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
                position: observation().actor_position,
                revision: 1,
            }],
            lights: Default::default(),
            observed_at_ms: 0,
        })
    }
}

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

fn admitted() -> TestResult<AdmittedSession> {
    let world_id = WorldId::decode(&uuid_v7(0x33))?;
    let channel_id = ChannelId::decode(&uuid_v7(0x44))?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22))?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[6, CAPABILITY_WORLD_SPATIAL_ENTITIES_V2],
            )
            .ok_or("select named visibility")?,
            ..SessionContinuity::FRESH
        },
        item_fence: None,
    })
}

async fn scenario() -> TestResult {
    let authority = NamedOwner {
        failure: Cell::new(None),
        reads: Cell::new(0),
    };
    let (mut server, _client) = tokio::io::duplex(1 << 20);
    let mut admitted = admitted()?;
    let actor = admitted.runtime_actor.ok_or("fixture actor")?;
    let mut session = setup::prepare(
        &mut server,
        &mut admitted,
        &authority,
        IDLE_LIVENESS,
        actor,
        observation(),
        integrity::Integrity::default(),
    )
    .await
    .map_err(|_| "prepare connection")?;
    assert_eq!(authority.reads.get(), 1, "the join snapshot reads names");
    let (sequence, revision) = (session.sequence, session.revision);

    // A holder that stayed busy for the whole read budget skips this tick only.
    authority.failure.set(Some(ObservationError::HolderBusy));
    refresh::visibility(&mut server, &authority, &mut session)
        .await
        .map_err(|_| "busy named visibility disconnected the observer")?;
    assert_eq!(authority.reads.get(), 2);
    assert_eq!((session.sequence, session.revision), (sequence, revision));
    assert!(session.visibility.is_some());

    // The next tick observes again with the same view.
    authority.failure.set(None);
    refresh::visibility(&mut server, &authority, &mut session)
        .await
        .map_err(|_| "named visibility after the busy tick")?;
    assert_eq!(authority.reads.get(), 3);
    assert_eq!((session.sequence, session.revision), (sequence, revision));

    // A hard failure still ends the session.
    authority.failure.set(Some(ObservationError::Unavailable));
    assert!(matches!(
        refresh::visibility(&mut server, &authority, &mut session).await,
        Err(ServeEnd::Disconnected)
    ));
    Ok(())
}

#[test]
fn named_visibility_busy_refresh_keeps_observer_and_hard_failure_disconnects() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()?
                .block_on(scenario())
        })?
        .join()
        .map_err(|_| "named visibility refresh test thread panicked")?
}
