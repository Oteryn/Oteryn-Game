use super::fixture::{Harness, debug, id};
use super::{ComposedFreshAdmission, TestResult};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CharacterLease, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority, SessionContinuity};
use crate::gameplay_transport::last_position::AdmissionPosition;

/// A second historical Character root receives a genuine independently verified Fresh session.
/// The shared verifier and publications own every claim; commands retain all production guards.
pub(super) async fn peer_at(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    base: AdmittedSession,
    origin: MovementLocalPosition,
) -> TestResult<AdmittedSession> {
    use crate::durability::guild_chat_fixture::{historical_character, sid};
    let character = crate::domain::CharacterId::from_bytes(sid(90, 1)).map_err(debug)?;
    historical_character(&h.pool, 90, character, sid(90, 2), id(42), "Scenario Peer").await?;
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityPublicationV1,
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let template = super::fresh_fixture::fresh::Source::new(now)?;
    let current = crate::foundation::fnd04_verifier::FreshCurrentEvidence {
        account_id: crate::gameplay_transport::canonical_uuid(&sid(90, 2)),
        character_id: crate::foundation::CharacterId::decode(character.as_bytes())
            .map_err(debug)?,
        ..template.current
    };
    let mut source = super::fresh_fixture::fresh::Source::with_current(now, current.clone())?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let mut publications = super::fresh_fixture::fresh::Source::with_current(now, current)?;
    publications
        .rows
        .retain(|r| matches!(r.key, Key::Account { .. } | Key::Character(_)));
    for row in &mut source.rows {
        if matches!(row.key, Key::Runtime(_) | Key::SigningTrust { .. }) {
            *row = guards
                .load(std::slice::from_ref(&row.key))
                .await?
                .into_iter()
                .next()
                .flatten()
                .ok_or("shared publication")?;
            if matches!(row.key, Key::SigningTrust { .. }) {
                row.precondition=crate::foundation::admission_authority_publication::AdmissionPublicationPreconditionV1::CompareAndSet{expected_publication_revision:row.publication_revision};
                row.publication_revision += 1;
                row.source.source_revision += 1;
                row.source.decision_identity =
                    format!("xmech-peer-trust-{}", row.source.source_revision);
                row.source.source_observed_at = now;
                publications.rows.push(row.clone());
            }
        }
    }
    guards
        .publish(&AdmissionAuthorityPublicationV1::prepare(&publications, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    let peer_session = GameSessionId::decode(&id(70)).map_err(debug)?;
    let transport =
        crate::foundation::AuthenticatedTransportRefV1::decode(&id(71)).map_err(debug)?;
    let grant = source.request_with_nonce(
        peer_session,
        transport,
        Some((
            &source.current.account_id,
            &crate::gameplay_transport::canonical_uuid(character.as_bytes()),
        )),
        now,
        Some([90; 32]),
    )?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&grant).await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let current = store.current_session(peer_session).await?;
    assert_eq!(current.current_character_lease().generation(), 2);
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(character.as_bytes().as_slice()).execute(&h.pool).await?;

    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, origin)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                fence.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(character.as_bytes()).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    fence.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Knight,
                        // Match the leader's level-50 combat fixture: the weakest
                        // fist hit must penetrate Rat defense/armor/mitigation.
                        // Durable progression remains owned by admission below.
                        level: 50,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: fence.game_session_id,
        runtime_actor: Some(actor),
        item_fence: Some(fence),
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport,
            account_id: sid(90, 2),
        }),
        continuity: SessionContinuity::FRESH,
        ..base
    };
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_loop_training(&mut admitted).await;
    let gameplay = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "peer fence")?
        .ok_or("peer current")?;
    assert!(owner.track_restored(
        admitted.game_session_id,
        actor,
        &AdmissionPosition::FirstLogin,
        gameplay
    ));
    Ok(admitted)
}
