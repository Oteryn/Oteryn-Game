//! ANIMUS-1 durable completion and real command 232 query, replay and current-session fencing.
use super::cyclopedia::support;
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::animus_mastery::{
    AnimusPage, CAPABILITY_ANIMUS_MASTERY_V1, COMMAND_TYPE_ANIMUS_MASTERY_QUERY, encode_page,
};

#[test]
fn animus_mastery_completion_replay_reload_and_fence_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("ANIMUS_COMPLETION POSTGRES_CASE_NOT_RUN: no configured PostgreSQL route");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("animus-mastery", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "animus_mastery", false, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("session")?;
            let dragon = "oteryn:creature.dragon";
            assert!(
                h.root
                    .complete_animus_mastery(&authority, &h.node, fence, dragon)
                    .await
                    .map_err(debug)?
            );
            assert!(
                !h.root
                    .complete_animus_mastery(&authority, &h.node, fence, dragon)
                    .await
                    .map_err(debug)?
            );
            assert_eq!(
                h.root
                    .read_animus_masteries(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                vec![dragon]
            );
            let mut stale = fence;
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
            assert!(
                h.root
                    .complete_animus_mastery(&authority, &h.node, stale, "oteryn:creature.rat")
                    .await
                    .is_err()
            );
            assert_eq!(
                h.root
                    .read_animus_masteries(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                vec![dragon]
            );
            eprintln!("ANIMUS_COMPLETION POSTGRES_CASE_COMPLETED");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn animus_mastery_postgres_real_dispatch_pages_replay_and_current_fence() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("ANIMUS_QUERY POSTGRES_CASE_NOT_RUN: no configured PostgreSQL route");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("animus-query", || {
        run(async |admin| {
            super::reset();
            let h = Harness::create_with_admission_seed(admin, "animus_query", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("session")?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(fence.item_fence());
            let page = [0, 0, 0, 2];

            // The real connection rejects an unnegotiated command before reading private rows.
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    1,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &page
                )
                .await?
                .0,
                CommandStatus::Rejected
            );
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                &crate::gameplay_transport::animus_mastery::offered(
                    owner.offered_capabilities(),
                    owner.animus_mastery_available(),
                ),
                &[CAPABILITY_ANIMUS_MASTERY_V1],
            )
            .ok_or("Animus capability")?;
            let (status, empty) = support::exchange(
                &owner,
                &mut admitted,
                2,
                COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                &page,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(empty, encode_page(0, &[]).ok_or("empty page")?);

            // The existing monotonic writer supplies committed query state. Soulpit entry
            // and completion qualification remain in that owner's separate atomicity cases.
            for species in [
                "oteryn:creature.rat",
                "oteryn:creature.dragon",
                "oteryn:creature.cyclops",
            ] {
                assert!(
                    h.root
                        .complete_animus_mastery(&authority, &h.node, fence, species)
                        .await
                        .map_err(debug)?
                );
            }
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    2,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &page
                )
                .await?,
                (CommandStatus::DuplicateReplay, empty)
            );
            let expected = [
                "oteryn:creature.cyclops",
                "oteryn:creature.dragon",
                "oteryn:creature.rat",
            ]
            .map(String::from);
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    3,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &page
                )
                .await?,
                (
                    CommandStatus::Accepted,
                    encode_page(3, &expected[..2]).ok_or("first page")?
                )
            );
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    4,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &[0, 2, 0, 2]
                )
                .await?,
                (
                    CommandStatus::Accepted,
                    encode_page(3, &expected[2..]).ok_or("last page")?
                )
            );
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    5,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &[0, 255, 255, 2]
                )
                .await?,
                (
                    CommandStatus::Accepted,
                    encode_page(3, &[]).ok_or("past-end page")?
                )
            );
            for (id, invalid) in [
                (6, &[0, 0, 0, 0][..]),
                (7, &[0, 0, 0, 51][..]),
                (8, &[0, 0, 0, 2, 0][..]),
                (9, &[1, 0, 0, 2][..]),
            ] {
                assert_eq!(
                    support::exchange(
                        &owner,
                        &mut admitted,
                        id,
                        COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                        invalid
                    )
                    .await?,
                    (CommandStatus::Rejected, Vec::new())
                );
            }

            let query = AnimusPage {
                offset: 0,
                limit: 2,
            };
            assert!(
                owner
                    .animus_mastery_query(
                        admitted,
                        AnimusPage {
                            offset: 0,
                            limit: 0
                        }
                    )
                    .await
                    .is_none()
            );
            assert!(
                owner
                    .animus_mastery_query(
                        admitted,
                        AnimusPage {
                            offset: 0,
                            limit: 51
                        }
                    )
                    .await
                    .is_none()
            );
            let mut missing_fence = admitted;
            missing_fence.item_fence = None;
            assert!(
                owner
                    .animus_mastery_query(missing_fence, query)
                    .await
                    .is_none()
            );
            // Matching stale cached fields are insufficient: the read proves the current
            // durable session/lease/scope, rather than comparing the cache with itself.
            let mut stale = admitted;
            stale.continuity.connection_generation = 999;
            stale
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .connection_generation =
                crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
            assert!(owner.animus_mastery_query(stale, query).await.is_none());
            let mut other_character = admitted;
            other_character
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .character_id =
                crate::domain::CharacterId::from_bytes(super::fixture::id(99)).map_err(debug)?;
            assert!(
                owner
                    .animus_mastery_query(other_character, query)
                    .await
                    .is_none()
            );
            let mut old_lease = admitted;
            old_lease
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .character_lease_generation += 1;
            assert!(owner.animus_mastery_query(old_lease, query).await.is_none());
            let mut stale_scope = admitted;
            stale_scope
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .scope_ownership_generation =
                crate::foundation::ScopeOwnershipGeneration::new(999).map_err(debug)?;
            assert!(
                owner
                    .animus_mastery_query(stale_scope, query)
                    .await
                    .is_none()
            );
            let mut absent_session = admitted;
            absent_session.game_session_id =
                crate::foundation::GameSessionId::decode(&super::fixture::id(99)).map_err(debug)?;
            absent_session
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .game_session_id = absent_session.game_session_id;
            assert!(
                owner
                    .animus_mastery_query(absent_session, query)
                    .await
                    .is_none()
            );

            // Reload the owner while retaining this session's independent ingress state.
            let commands = std::mem::take(
                &mut std::sync::Arc::get_mut(&mut owner.states)
                    .ok_or("isolated owner state")?
                    .command_ingress,
            );
            drop(owner);
            let mut owner = deps.owner(&h, &authority);
            std::sync::Arc::get_mut(&mut owner.states)
                .ok_or("isolated owner state")?
                .command_ingress = commands;
            deps.initialize(&owner).await;
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    10,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &page
                )
                .await?,
                (
                    CommandStatus::Accepted,
                    encode_page(3, &expected[..2]).ok_or("reloaded page")?
                )
            );
            assert_eq!(
                h.root
                    .read_animus_masteries(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                expected
            );
            // The wire uses byte ordering: punctuation must not be reordered by
            // the database locale (dragon_lord precedes dragonling).
            for species in ["oteryn:creature.dragonling", "oteryn:creature.dragon_lord"] {
                assert!(
                    h.root
                        .complete_animus_mastery(&authority, &h.node, fence, species)
                        .await
                        .map_err(debug)?
                );
            }
            let ordered = [
                "oteryn:creature.cyclops",
                "oteryn:creature.dragon",
                "oteryn:creature.dragon_lord",
                "oteryn:creature.dragonling",
                "oteryn:creature.rat",
            ]
            .map(String::from);
            assert_eq!(
                support::exchange(
                    &owner,
                    &mut admitted,
                    11,
                    COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
                    &[0, 0, 0, 50]
                )
                .await?,
                (
                    CommandStatus::Accepted,
                    encode_page(5, &ordered).ok_or("byte-ordered page")?
                )
            );
            assert_eq!(
                h.root
                    .read_animus_masteries(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                ordered
            );
            eprintln!("ANIMUS_QUERY POSTGRES_CASE_COMPLETED");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
