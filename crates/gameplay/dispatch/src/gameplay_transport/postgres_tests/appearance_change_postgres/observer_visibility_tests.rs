//! Two real admitted players use native VIS-2 membership and the same committed DB appearance.
use super::*;
#[path = "observer_admission_fixture.rs"]
mod observer_admission_fixture;
pub(in crate::gameplay_transport::monk_save::postgres_tests) use observer_admission_fixture::admit;

pub(in crate::gameplay_transport::monk_save::postgres_tests) async fn initialize(
    dependencies: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<ExactActorRef> {
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let actor = {
        let mut runtime = dependencies.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(101)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    dependencies.initialize_player(owner, actor, session).await;
    Ok(actor)
}

pub(super) async fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    observer: ExactActorRef,
    target: appearance::EntityRef,
) -> TestResult {
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let visible = owner
        .observe_visible_entities(observer)
        .await
        .ok_or("observer visibility")?;
    let entities = visible
        .entities
        .iter()
        .filter(|entity| {
            entity.kind == crate::gameplay_transport::world_spatial::VisibleKind::Player
        })
        .map(|entity| appearance::EntityRef {
            identity: entity.identity,
            generation: entity.generation,
        })
        .collect::<Vec<_>>();
    assert_eq!(entities.len(), 2);
    let observation = owner
        .appearance_observation(observer, session, entities)
        .await
        .ok_or("peer appearance")?;
    assert_eq!(observation.catalogue.root_revision, 1);
    let target = observation
        .bindings
        .iter()
        .find(|binding| binding.entity == target)
        .ok_or("visible target binding")?;
    assert_eq!(target.selection_revision, Some(2));
    assert!(matches!(
        target.look,
        appearance::Look::Outfit {
            colours: [1, 2, 3, 4],
            ..
        }
    ));
    Ok(())
}
