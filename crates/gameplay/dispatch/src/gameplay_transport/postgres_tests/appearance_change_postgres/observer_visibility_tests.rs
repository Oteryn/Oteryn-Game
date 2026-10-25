//! Two real admitted players use native VIS-2 membership and the same committed DB appearance.
use super::*;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
};

pub(in crate::gameplay_transport::monk_save::postgres_tests) async fn admit(
    harness: &Harness,
) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let (account, character): (String, String) = sqlx::query_as("SELECT account_id::text, character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(101).as_slice()).fetch_one(&harness.pool).await?;
    let mut source = fresh_fixture::fresh::Source::appearance_observer(
        now,
        account,
        crate::foundation::CharacterId::decode(&id(101)).map_err(debug)?,
    )?;
    let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        harness.root.clone(),
    );
    let keys = source
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.key,
                AdmissionAuthorityGuardKeyV1::Runtime(_)
                    | AdmissionAuthorityGuardKeyV1::SigningTrust { .. }
            )
        })
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    let current = guards.load(&keys).await.map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    source = fresh_fixture::fresh::Source::appearance_observer(
        now,
        source.current.account_id,
        source.current.character_id,
    )?;
    for prior in current {
        let prior = prior.ok_or("current global publication")?;
        let row = source
            .rows
            .iter_mut()
            .find(|row| row.key == prior.key)
            .ok_or("global key")?;
        if matches!(row.key, AdmissionAuthorityGuardKeyV1::SigningTrust { .. }) {
            row.precondition = crate::foundation::admission_authority_publication::AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision: prior.publication_revision };
            row.publication_revision = prior
                .publication_revision
                .checked_add(1)
                .ok_or("publication revision")?;
            row.source.source_revision = prior
                .source
                .source_revision
                .checked_add(1)
                .ok_or("source revision")?;
            row.source.decision_identity =
                format!("platform-observation-{}", row.source.source_revision);
        } else {
            *row = prior;
        }
    }
    let rows = source.rows.clone();
    source.rows.retain(|row| {
        matches!(
            row.key,
            AdmissionAuthorityGuardKeyV1::Account { .. }
                | AdmissionAuthorityGuardKeyV1::Character(_)
                | AdmissionAuthorityGuardKeyV1::SigningTrust { .. }
        )
    });
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows = rows;
    // Issue the grant after publication work; its ten-second TTL starts at issuance,
    // independently of the earlier owner observations retained in publication rows.
    let issued_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let request = source.appearance_observer_request(&character, issued_at)?;
    let store =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone());
    let outcome = store.commit(&request).await.map_err(debug)?;
    assert!(
        matches!(outcome,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ),
        "observer admission: {outcome:?}"
    );
    Ok(())
}

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
