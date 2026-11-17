//! NBPROFILE: node boot M01 disconnected the follower (`appearance_observation_failed
//! class=visibility`, then `serve_end phase=visibility`) right after the followed peer logged
//! out. The serve refresh reads its channel entities first and the appearances after the
//! runtime lock was released; a peer that logs out in between is a normal removal, not a
//! lost observer.
use super::appearance_change_postgres::observer_visibility_tests;
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::appearance_change::observation::observe_result;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::world_spatial::VisibleKind;
use oteryn_protocol_oteryn::appearance;

#[test]
fn appearance_observation_skips_a_peer_that_logged_out_after_the_visibility_read_postgres()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "appearance_vanished_peer").await?;
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
        let peer = dependencies.actor;
        let peer_session = dependencies.admitted.game_session_id;
        {
            let mut runtime = dependencies.runtime.lock().await;
            runtime.initialize_first_entry_position(peer).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    peer,
                    peer_session,
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
        let players = |channel: &crate::gameplay_transport::world_spatial::ChannelEntities| {
            channel
                .entities
                .iter()
                .filter(|entity| entity.kind == VisibleKind::Player)
                .map(|entity| appearance::EntityRef {
                    identity: entity.identity,
                    generation: entity.generation,
                })
                .collect::<Vec<_>>()
        };
        let own = appearance::EntityRef {
            identity: observer.placement_identity(),
            generation: observer.actor_local_generation(),
        };
        let gone = appearance::EntityRef {
            identity: peer.placement_identity(),
            generation: peer.actor_local_generation(),
        };

        // The refresh's channel read: both players are in view.
        let visible = owner
            .observe_visible_entities(observer)
            .await
            .ok_or("observer visibility")?;
        let requested = players(&visible);
        assert_eq!(requested.len(), 2);
        assert!(requested.contains(&own) && requested.contains(&gone));

        // The peer logs out before the appearance read takes the runtime lock again.
        owner
            .runtime
            .lock()
            .await
            .remove_terminal_session(peer_session, peer)
            .map_err(debug)?;

        let observed = observe_result(&owner, observer, observer_session, false, requested)
            .await
            .map_err(|error| format!("observer lost to a peer that logged out: {error:?}"))?;
        assert_eq!(observed.skipped, vec![gone]);
        assert_eq!(
            observed
                .bindings
                .iter()
                .map(|binding| binding.entity)
                .collect::<Vec<_>>(),
            vec![own]
        );

        // The next refresh no longer sees the peer, so its view sends the leave.
        let after = owner
            .observe_visible_entities(observer)
            .await
            .ok_or("observer visibility after the logout")?;
        assert_eq!(players(&after), vec![own]);

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
