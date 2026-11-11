//! A pending respawn fences a dead Character's appearance only until its placement. The
//! periodic projection must defer, not report a lost observer: node boot S08 was disconnected
//! (`serve_end phase=visibility`) two seconds after its durable death.
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::appearance_change::observation::{ObservationError, observe_result};
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
