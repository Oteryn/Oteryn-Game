//! A second real admitted and authorized living actor observes the original death root.
use super::*;

pub(super) async fn observer(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    _deps: &Dependencies,
    victim: AdmittedSession,
) -> TestResult<AdmittedSession> {
    // This is the shared real source-publication/admission fixture, not a forged
    // session or a replay of the victim's lease. Character101 must be reserved by
    // the scenario, with its real guest ACL installed before the gameplay phase.
    h.seed_pvp_target().await?;
    // Initial inventory of the separately living observer, never a replacement
    // granted to the victim. Gameplay below runs with every native writer guard.
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(id(145).as_slice()).bind(id(42).as_slice()).bind(id(146).as_slice()).execute(&mut *setup).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,world_id,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(101).as_slice()).bind(id(42).as_slice()).bind(id(145).as_slice()).bind(id(146).as_slice()).execute(&mut *setup).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(101).as_slice()).execute(&mut *setup).await?;
    setup.commit().await?;
    admit_composed_observer(h).await?;
    let session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "observer current fence")?
        .ok_or("observer fresh fence")?;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?
        .0;
    let transport = current
        .current_transport()
        .ok_or("actual observer transport")?;
    let actor = {
        let _lane = owner.spell_lane_permit().await.ok_or("observer lane")?;
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: victim.world_id,
        channel_id: victim.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport,
            account_id: account.try_into().map_err(|_| "observer account")?,
        }),
        continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    admitted.continuity.selected_capabilities = victim.continuity.selected_capabilities;
    owner.admit_progression_session(&mut admitted).await;
    let facts = match crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        &h.root,
        owner.character,
        fence.character_id,
    )
    .await
    {
        crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        } => facts,
        crate::gameplay_transport::spell_character_facts::CastFactsLoad::Unavailable => {
            return Err("actual living observer build facts".into());
        }
    };
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner
            .states
            .spell_states
            .try_lock()
            .map_err(|_| "idle observer spell state")?;
        assert!(
            states
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some(),
            "fresh observer uses its actual Root class and resources"
        );
    }

    owner.admit_quest_session(&admitted).await;
    assert!(owner.track_restored(
        session,
        actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));
    let role: String = sqlx::query_scalar(
        "SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(101).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        role, "GUEST",
        "real observer retains exact guest ACL after victim death"
    );
    let at = {
        let runtime = owner.runtime.lock().await;
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
    };
    assert_eq!(
        (at.x, at.y, at.floor),
        (32309, 32805, 7),
        "actual observer stands on the published House entrance"
    );
    let mut transfer = vec![8, 1, 18, HOUSE.len() as u8];
    transfer.extend(HOUSE.as_bytes());
    let (inside, refused_payload) = owner
        .transfer_house(admitted, 1, &transfer)
        .await
        .ok_or("real observer House enter")?;
    let inside = inside.ok_or_else(||format!("authorized observer House admission refused; payload={refused_payload:?}; current guest ACL and published entrance independently verified"))?;
    assert_ne!(inside.game_session_id, victim.game_session_id);
    assert_ne!(
        inside
            .item_fence
            .ok_or("observer House fence")?
            .character_id,
        victim.item_fence.ok_or("victim fence")?.character_id
    );
    Ok(inside)
}

async fn admit_composed_observer(harness: &Harness) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
        AdmissionAuthorityPublicationV1,
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let (account, character): (String, String) = sqlx::query_as("SELECT account_id::text, character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(101).as_slice()).fetch_one(&harness.pool).await?;
    let mut source = crate::gameplay_transport::monk_save::postgres_tests::fresh_fixture::fresh::Source::appearance_observer(
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
    source = crate::gameplay_transport::monk_save::postgres_tests::fresh_fixture::fresh::Source::appearance_observer(
        now,
        source.current.account_id,
        source.current.character_id,
    )?;
    // The new observer belongs to the same mature native House composition
    // publisher from its FIRST Account/Character publication. The APP3-only
    // independent publisher cannot be impersonated by a House claim transfer.
    for row in &mut source.rows {
        if matches!(
            row.key,
            AdmissionAuthorityGuardKeyV1::Account { .. }
                | AdmissionAuthorityGuardKeyV1::Character(_)
        ) {
            row.source.authority =
                crate::durability::fresh_admission_composition::COMPOSITION_SOURCE_AUTHORITY.into();
            if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                security.provenance.source_authority = "platform-test".into();
            }
        }
    }
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
