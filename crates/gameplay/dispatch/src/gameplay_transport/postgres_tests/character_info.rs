//! CHARACTER-INFO-1 fenced durable unlock/select, replay and stale generation.
use super::fixture::{Harness, run};
use super::{TestResult, debug, prepare_current_session};
use oteryn_protocol_oteryn::character_info::CharacterTitle;

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
                    CharacterTitle::Cartographer,
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
    use super::cyclopedia::support::exchange;
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
        assert_eq!(
            &payload[payload.len() - 3..],
            &[1, 1, 1],
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
            &payload[payload.len() - 3..],
            &[1, 1, 0],
            "conditional title and its selection are revoked"
        );
        let revoked: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=2)")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(revoked);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
