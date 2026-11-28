//! CHARACTER-INFO-1 fenced durable unlock/select, replay and stale generation.
use super::fixture::{Harness, run};
use super::{TestResult, debug, prepare_current_session};
use oteryn_protocol_oteryn::character_info::CharacterTitle;

#[test]
fn first_character_info_self_query_waits_for_holder_and_awards_cyclopscamper() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        use crate::durability::DurabilityError;
        use crate::foundation::AuthenticatedTransportRefV1;
        use crate::gameplay_transport::capabilities::SelectedCapabilities;
        use crate::gameplay_transport::character_info::{CharacterInfoReply, decode, offered};
        use crate::gameplay_transport::connection::FirstEntryOutcome;
        use oteryn_protocol_oteryn::character_info::CAPABILITY_CHARACTER_INFO_V1;

        let h = Harness::create_with_admission_seed(admin, "first_character_info", true, false)
            .await?;
        let level = 100;
        super::m01b_explore_fixtures::koshei::seed_level(&h, level).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = super::Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let mut admitted = deps.admitted;
        let (first_entry, fence, _) = Box::pin(owner.initialize_first_entry(
            &request,
            admitted.game_session_id,
            AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
            deps.actor,
        ))
        .await;
        assert_eq!(first_entry, FirstEntryOutcome::Positioned);
        admitted.first_entry = first_entry;
        admitted.item_fence = Some(fence.ok_or("first-entry item fence")?);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered(&[], true), &[CAPABILITY_CHARACTER_INFO_V1])
                .ok_or("Character Info capability")?;
        let operation = decode(&[0, 0, 0, 50]).map_err(debug)?;
        let reply = {
            // Keep the holder busy through the automatic title refresh's first checkout.
            let held = h.root.try_issue_background_semantic_pass()?;
            assert!(matches!(
                h.root.try_issue_semantic_pass(),
                Err(DurabilityError::RootUnavailable)
            ));
            let mut query = std::pin::pin!(owner.character_info_command(admitted, operation));
            let first = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(query.as_mut(), cx))
            })
            .await;
            assert!(first.is_pending(), "first QuerySelf returned {first:?} while the holder was busy");
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            query.await
        };
        let CharacterInfoReply::Payload(bytes) = reply else {
            return Err(format!("first QuerySelf returned {reply:?}").into());
        };
        let (summary, title_block) = bytes.split_at_checked(1 + 8 * 8 + 2 * 2).ok_or("summary")?;
        assert_eq!(summary[0], 0);
        assert_eq!(i64::from_le_bytes(summary[1..9].try_into()?), level);
        let (&count, titles) = title_block.split_first().ok_or("title count")?;
        let (&selected, titles) = titles.split_last().ok_or("selected title")?;
        assert_eq!(titles.len(), usize::from(count));
        assert!(titles.contains(&CharacterTitle::Cyclopscamper.id()));
        assert_eq!(selected, 0);
        eprintln!("POSTGRES_CASE_COMPLETED first_character_info");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn character_title_unlock_and_selection_are_durable_and_referential() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "character_title", false, false).await?;
        prepare_current_session(&harness).await?;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(
            crate::foundation::GameSessionId::decode(&super::fixture::id(50)).map_err(debug)?,
        )
        .await?
        .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            harness
                .root
                .unlock_character_title(
                    &authority,
                    &harness.node,
                    fence,
                    CharacterTitle::AdeptArmorer,
                    100,
                    None
                )
                .await
                .map_err(debug)?
        );
        assert!(
            !harness
                .root
                .unlock_character_title(
                    &authority,
                    &harness.node,
                    fence,
                    CharacterTitle::AdeptArmorer,
                    100,
                    None
                )
                .await
                .map_err(debug)?,
            "replay is idempotent"
        );
        let revision = harness
            .root
            .select_character_title(
                &authority,
                &harness.node,
                fence,
                Some(CharacterTitle::AdeptArmorer),
            )
            .await
            .map_err(debug)?;
        assert_eq!(revision, 1);
        let mut stale = fence;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        assert!(
            harness
                .root
                .select_character_title(&authority, &harness.node, stale, None)
                .await
                .is_err()
        );
        assert!(
            harness
                .root
                .unlock_character_title(
                    &authority,
                    &harness.node,
                    stale,
                    CharacterTitle::Globetrotter,
                    100,
                    None
                )
                .await
                .is_err()
        );

        assert!(
            harness
                .root
                .select_character_title(
                    &authority,
                    &harness.node,
                    fence,
                    Some(CharacterTitle::AdeptEnhancer)
                )
                .await
                .is_err(),
            "unearned selection is rejected"
        );
        Ok(())
    })
}

#[test]
fn character_info_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| dispatch_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "character info scenario panicked")??;
    Ok(())
}

fn dispatch_scenario() -> TestResult {
    use super::cyclopedia_fixtures::support::exchange;
    use super::fixture::id;
    use super::{Dependencies, reset};
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use oteryn_protocol_oteryn::CommandStatus;
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "character_info_wire", true, false).await?;
        eprintln!("POSTGRES_STARTED CHARACTER-INFO-1 character_info_postgres_real_dispatch");
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
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        // The unnegotiated request must not reach the title write.
        assert_eq!(
            exchange(&owner, &mut admitted, 1, 113, &[2, 1]).await?.0,
            CommandStatus::Rejected
        );
        let offers = crate::gameplay_transport::character_info::offered(&[], true);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[97]).ok_or("character info capability")?;
        assert_eq!(
            exchange(&owner, &mut admitted, 2, 113, &[2, 1]).await?.0,
            CommandStatus::Rejected,
            "unqualified title"
        );
        use crate::durability::character_build::{
            BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
            convert_vocation,
        };
        let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            "content-1",
        )
        .map_err(debug)?;
        let knight =
            convert_vocation(&formula, &DurableBuildState::default(), "knight").map_err(debug)?;
        let mut skills = knight.skills();
        skills[2] = (100, 1);
        let trained = DurableBuildState::new("knight", (0, 0), skills).map_err(debug)?;
        let mut fence = super::fence(1)?;
        fence.character_lease_generation = 2;
        for (tag, cause, before, after) in [
            (
                90,
                BuildCause::VocationChoice,
                DurableBuildState::default(),
                knight.clone(),
            ),
            (91, BuildCause::Training, knight, trained),
        ] {
            let outcome = h
                .root
                .commit_character_build(
                    &authority,
                    &h.node,
                    fence,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(id(tag)).map_err(debug)?,
                        cause,
                        before,
                        after,
                        pruned_stance: None,
                    },
                    &formula,
                )
                .await
                .map_err(|error| format!("build {tag}: {error:?}"))?;
            let BuildCommitOutcome::Committed(receipt) = outcome else {
                return Err("build replay".into());
            };
            fence.expected_character_revision = receipt.committed_character_revision;
        }
        super::reaudit_title_predicates_fixtures::seed_mastery(&h).await?;
        let (status, payload) = exchange(&owner, &mut admitted, 3, 113, &[2, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(payload, [2, 1]);
        assert_eq!(
            exchange(&owner, &mut admitted, 4, 113, &[2, 1]).await?.1,
            [2, 1],
            "unlock replay"
        );
        let (status, payload) = exchange(&owner, &mut admitted, 5, 113, &[3, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(payload, [3, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            exchange(&owner, &mut admitted, 6, 113, &[3, 2]).await?.0,
            CommandStatus::Rejected,
            "unearned selection"
        );
        let (status, payload) = exchange(&owner, &mut admitted, 7, 113, &[0, 0, 0, 50]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        // PAR-P12a: the self query runs the automatic unlock first. The fixture Character was
        // created more than ten years before now (Aeternal, 6) and is at least level 50
        // (Trolltrasher, 42).
        assert_eq!(
            &payload[payload.len() - 5..],
            &[3, 1, 6, 42, 1],
            "earned and selected title survive reconnect"
        );
        for (command, tag) in [(8, 1), (9, 4)] {
            let mut request = vec![tag];
            request.extend_from_slice(&id(41));
            if tag == 1 {
                request.extend_from_slice(&[0, 0, 50]);
            }
            assert_eq!(
                exchange(&owner, &mut admitted, command, 113, &request)
                    .await?
                    .0,
                CommandStatus::Accepted
            );
        }
        assert_eq!(
            exchange(&owner, &mut admitted, 10, 113, &[3, 0]).await?.0,
            CommandStatus::Accepted
        );
        let selected: Option<i16> = sqlx::query_scalar("SELECT title FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(selected, None);
        h.root
            .unlock_character_title(
                &authority,
                &h.node,
                admitted.item_fence.ok_or("current fence")?,
                CharacterTitle::AdeptEnhancer,
                100,
                None,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            exchange(&owner, &mut admitted, 11, 113, &[3, 2]).await?.0,
            CommandStatus::Accepted
        );
        let (status, payload) = exchange(&owner, &mut admitted, 12, 113, &[0, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            &payload[payload.len() - 5..],
            &[3, 1, 6, 42, 0],
            "conditional title and its selection are revoked"
        );
        let revoked: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=2)")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(revoked);
        // PAR-P12: Global Globetrotter and Reigning Drome Champion are not permanent. Without
        // their native facts the wire unlock is refused before any title write.
        for (command, title) in [(13, 3), (14, 4)] {
            assert_eq!(
                exchange(&owner, &mut admitted, command, 113, &[2, title])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "unqualified non-permanent title {title}"
            );
        }
        // A stored Globetrotter row (earned while every map area was explored) and its
        // selection are revoked together once the map predicate no longer holds.
        assert!(
            h.root
                .unlock_character_title(
                    &authority,
                    &h.node,
                    admitted.item_fence.ok_or("current fence")?,
                    CharacterTitle::Globetrotter,
                    100,
                    None,
                )
                .await
                .map_err(debug)?
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 15, 113, &[3, 3]).await?.0,
            CommandStatus::Accepted
        );
        let (status, payload) = exchange(&owner, &mut admitted, 16, 113, &[0, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            &payload[payload.len() - 5..],
            &[3, 1, 6, 42, 0],
            "Globetrotter and its selection are revoked like Global"
        );
        let (globetrotter, selected): (bool, Option<i16>) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=3),(SELECT title FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid)")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(!globetrotter);
        assert_eq!(selected, None);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

/// PAR-P12a: the automatic refresh grants a title from its native owner (a 7-day reward
/// streak) and, in the same transaction, revokes a stored conditional title whose predicate
/// fails (Cyclopscamper below level 100) together with its selection.
#[test]
fn character_titles_refresh_grants_and_revokes_from_native_facts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    use super::fixture::id;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "character_title_refresh", false, false)
            .await?;
        eprintln!(
            "POSTGRES_STARTED CHARACTER-INFO-1 character_titles_refresh_grants_and_revokes_from_native_facts"
        );
        prepare_current_session(&h).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(
                    crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                )
                .await?
                .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            h.root
                .unlock_character_title(
                    &authority,
                    &h.node,
                    fence,
                    CharacterTitle::Cyclopscamper,
                    100,
                    None
                )
                .await
                .map_err(debug)?
        );
        h.root
            .select_character_title(
                &authority,
                &h.node,
                fence,
                Some(CharacterTitle::Cyclopscamper),
            )
            .await
            .map_err(debug)?;
        super::daily_reward_fixtures::epoch(&h, 1).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_character_daily_rewards(character_id,world_id,lane_day,streak,last_claim_epoch,last_claim_day) SELECT encode($1,'hex')::uuid,world_id,2,7,reset_epoch,reset_day FROM game_world_reset_history WHERE world_id=encode($2,'hex')::uuid AND reset_epoch=1")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_daily_reward_claims(character_id,world_id,reset_epoch,reset_day,transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium) SELECT character_id,world_id,last_claim_epoch,last_claim_day,encode($1,'hex')::uuid,$2,1,6,7,0,false FROM game_character_daily_rewards WHERE character_id=encode($3,'hex')::uuid")
            .bind(id(91).as_slice()).bind([3_u8;32].as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let now = 1_800_000_000_000;
        let granted = h
            .root
            .refresh_character_titles(&authority, &h.node, fence, now, None, None)
            .await
            .map_err(debug)?;
        // The fixture's creation receipt is more than ten years before `now` (Aeternal).
        assert_eq!(
            granted,
            [
                CharacterTitle::Aeternal,
                CharacterTitle::CreatureOfHabitGrade1
            ]
        );
        let (titles, selected): (Vec<i16>, Option<i16>) = sqlx::query_as("SELECT ARRAY(SELECT title FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid ORDER BY title),(SELECT title FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid)")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            titles,
            [6, 14],
            "streak title granted, Cyclopscamper revoked"
        );
        assert_eq!(selected, None, "the revoked selection is cleared");
        assert!(
            h.root
                .refresh_character_titles(&authority, &h.node, fence, now, None, None)
                .await
                .map_err(debug)?
                .is_empty(),
            "replay grants nothing"
        );
        let mut stale = fence;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        assert!(
            h.root
                .refresh_character_titles(&authority, &h.node, stale, now, None, None)
                .await
                .is_err()
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
