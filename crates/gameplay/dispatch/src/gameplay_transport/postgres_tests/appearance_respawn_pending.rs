//! A pending respawn fences a dead Character's appearance only until its placement. The
//! periodic projection must defer, not report a lost observer: node boot S08 was disconnected
//! (`serve_end phase=visibility`) two seconds after its durable death.
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::appearance_change::observation::{ObservationError, observe_result};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::appearance;

#[test]
fn appearance_projection_defers_while_the_observer_respawn_is_pending_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "appearance_respawn_pending").await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = controller.active();
        let actor = dependencies.actor;
        let session = dependencies.admitted.game_session_id;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        dependencies.initialize(&owner).await;
        let entity = appearance::EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        };
        observe_result(&owner, actor, session, false, vec![entity])
            .await
            .map_err(|error| format!("initial appearance projection: {error:?}"))?;

        // The durable death leaves its pending respawn until the placement consumes it.
        // Only the fence row matters here: the receipt foreign key and guards are bypassed
        // for this fixture row alone.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_pending_respawns (character_id, death_occurrence_id, respawn_position) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '\\x01'::bytea)")
            .bind(id(41).as_slice())
            .bind(id(230).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let deferred = observe_result(&owner, actor, session, false, vec![entity]).await;
        assert!(
            matches!(deferred, Err(ObservationError::RespawnPending)),
            "a pending respawn defers the projection: {:?}",
            deferred.err()
        );

        sqlx::query(
            "DELETE FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .execute(&harness.pool)
        .await?;
        observe_result(&owner, actor, session, false, vec![entity])
            .await
            .map_err(|error| format!("appearance after placement: {error:?}"))?;

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Node boot world=1: the lost S08 Character stayed dead in town, and every later join that
/// saw it failed `join_prepare_failed phase=appearances` with the target's RespawnPending.
#[test]
fn appearance_projection_skips_a_peer_whose_respawn_is_pending_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(peer_scenario)
}

fn peer_scenario() -> TestResult {
    use super::appearance_change_postgres::observer_visibility_tests;
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "appearance_peer_respawn").await?;
        prepare_current_session(&harness).await?;
        harness.seed_appearance_observer().await?;
        observer_visibility_tests::admit(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = controller.active();
        {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    dependencies.actor,
                    dependencies.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        dependencies.initialize(&owner).await;
        let observer = observer_visibility_tests::initialize(&dependencies, &owner).await?;
        let observer_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
        let visible = owner
            .observe_visible_entities(observer)
            .await
            .ok_or("observer visibility")?;
        let players = visible
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
        assert_eq!(players.len(), 2);
        let dead = appearance::EntityRef {
            identity: dependencies.actor.placement_identity(),
            generation: dependencies.actor.actor_local_generation(),
        };
        let before = observe_result(&owner, observer, observer_session, false, players.clone())
            .await
            .map_err(|error| format!("peer appearance: {error:?}"))?;
        assert!(before.bindings.iter().any(|binding| binding.entity == dead));

        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_pending_respawns (character_id, death_occurrence_id, respawn_position) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '\\x01'::bytea)")
            .bind(id(41).as_slice())
            .bind(id(231).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let during = observe_result(&owner, observer, observer_session, false, players).await;
        let during = during.map_err(|error| format!("observer lost to a dead peer: {error:?}"))?;
        assert!(
            during.bindings.iter().all(|binding| binding.entity != dead),
            "the dead peer awaiting its placement is not bound"
        );
        assert_eq!(during.bindings.len(), before.bindings.len() - 1);

        // Exercise the connection's upserts and spatial attachment, not only the owner read.
        let mut admitted = dependencies.admitted;
        admitted.game_session_id = observer_session;
        admitted.runtime_actor = Some(observer);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(observer_session)
            .await?;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                &offered,
                &[4, 6, 19, 25],
            )
            .ok_or("appearance capabilities")?;
        {
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let serving = crate::gameplay_transport::connection::serve_admitted(
                &mut server,
                admitted,
                &owner,
                crate::gameplay_transport::connection::IDLE_LIVENESS,
            );
            tokio::pin!(serving);
            let snapshot = tokio::select! {
                end = &mut serving => return Err(format!("pending peer prevented initial mappings: {end:?}").into()),
                snapshot = read_appearance_snapshot(&mut client) => snapshot?,
            };
            assert_eq!(snapshot.bindings.len(), 1);
            assert!(
                snapshot
                    .bindings
                    .iter()
                    .all(|binding| binding.entity != dead)
            );

            // The same connection must restore the peer, then forget it on a later death.
            sqlx::query("DELETE FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).execute(&harness.pool).await?;
            let restored = tokio::select! {
                end = &mut serving => return Err(format!("observer disconnected on peer respawn: {end:?}").into()),
                mappings = read_appearance_mappings(&mut client) => mappings?,
            };
            assert_eq!(restored.upserts.len(), 1);
            assert_eq!(restored.upserts[0].entity, dead);
            assert!(restored.forgets.is_empty());
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_pending_respawns (character_id, death_occurrence_id, respawn_position) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, '\\x01'::bytea)")
                .bind(id(41).as_slice()).bind(id(232).as_slice())
                .execute(&mut *tx).await?;
            tx.commit().await?;
            let forgotten = tokio::select! {
                end = &mut serving => return Err(format!("observer disconnected on pending peer refresh: {end:?}").into()),
                mappings = read_appearance_mappings(&mut client) => mappings?,
            };
            assert!(forgotten.upserts.is_empty());
            assert_eq!(forgotten.forgets, vec![dead]);
        }

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn read_appearance_snapshot(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<appearance::Snapshot> {
    use tokio::io::AsyncReadExt;
    let mut appearance = None;
    loop {
        let length = usize::try_from(client.read_u32().await?)?;
        assert!(length <= 1 << 20);
        let mut bytes = vec![0; length];
        client.read_exact(&mut bytes).await?;
        let envelope = crate::foundation::decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            crate::foundation::MessageType::SnapshotChunk => {
                let (_, body) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())
                        .map_err(debug)?;
                for domain in oteryn_protocol_oteryn::decode_snapshot_body(body).map_err(debug)? {
                    if domain.domain_id == 21 {
                        appearance =
                            Some(appearance::Snapshot::decode(domain.payload).map_err(debug)?);
                    }
                }
            }
            crate::foundation::MessageType::SnapshotCommit => {
                return appearance.ok_or_else(|| "appearance snapshot missing".into());
            }
            _ => {}
        }
    }
}

async fn read_appearance_mappings(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<appearance::Mappings> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    loop {
        let length = usize::try_from(client.read_u32().await?)?;
        assert!(length <= 1 << 20);
        let mut bytes = vec![0; length];
        client.read_exact(&mut bytes).await?;
        let envelope = crate::foundation::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == crate::foundation::MessageType::LivenessProbe {
            let probe =
                oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
            let ack =
                oteryn_protocol_oteryn::encode_liveness_ack(1, probe, envelope.server_sequence())
                    .map_err(debug)?;
            client.write_u32(u32::try_from(ack.len())?).await?;
            client.write_all(&ack).await?;
        } else if envelope.message_type() == crate::foundation::MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 21
                && delta.delta_type == appearance::DELTA_TYPE_APPEARANCE_MAPPINGS
            {
                return appearance::Mappings::decode(delta.payload).map_err(debug);
            }
        }
    }
}
