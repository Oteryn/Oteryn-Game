use super::fixture::{Harness, debug, id};

use super::{Dependencies, TestResult};
use crate::foundation::{CharacterLease, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::last_position::AdmissionPosition;

use oteryn_gameplay_quests::quest::grizzly_adams::{self as quest};

#[path = "grizzly_boss/world.rs"]
pub(super) mod world;

pub(super) async fn second_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    h: &Harness,
    origin: MovementLocalPosition,
) -> TestResult<(ExactActorRef, AdmittedSession)> {
    h.seed_pvp_target().await?;
    super::appearance_change_postgres_fixtures::observer_visibility_tests::admit(h).await?;
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "second fence")?
        .ok_or("second current fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, origin)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: deps.admitted.world_id,
        channel_id: deps.admitted.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])
                .map_err(debug)?,
            account_id: id(100),
        }),
        continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize_player(owner, actor, session).await;
    owner.admit_quest_session(&admitted).await;
    assert!(owner.track_restored(session, actor, &AdmissionPosition::FirstLogin, fence));
    Ok((actor, admitted))
}
