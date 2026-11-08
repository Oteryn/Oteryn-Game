//! Rich Character achievement metadata through the production command and privacy boundary.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::CharacterId;
use crate::durability::{
    account_achievement::{
        AchievementGrantOutcome, AchievementGrantRequest, AchievementSourceEvent, EarnedAchievement,
    },
    character_progression::CurrentCharacterGameplayFence,
    fresh_admission::FreshAdmissionStore,
};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    cyclopedia_character::{
        CAPABILITY_CYCLOPEDIA_CHARACTER_V1 as CAP, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB as TAB,
        achievements::{AchievementPage, AchievementRequest},
    },
};
#[test]
fn character_achievement_metadata_postgres_real_dispatch_and_privacy() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Character achievements scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(
            admin,
            "character_achievement_metadata",
            true,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_STARTED character_achievement_metadata_postgres_real_dispatch_and_privacy"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let query = AchievementRequest {
            page: 0,
            target: None,
        }
        .encode();
        assert!(
            owner
                .cyclopedia_character_wire(admitted, TAB, &query)
                .await
                .is_err()
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability {
                id: CAP,
                requires: &[],
            }],
            &[CAP],
        )
        .ok_or("capability")?;
        let record = h
            .root
            .read_current_character(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let gameplay = CurrentCharacterGameplayFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
            expected_character_revision: record.revision,
        };
        let requests = ["here_fishy_fishy", "skin_deep"]
            .into_iter()
            .enumerate()
            .map(|(i, slug)| {
                let key = format!("oteryn:achievement/{slug}");
                AchievementGrantRequest {
                    catalogue: deps.achievements.lookup(&key),
                    achievement_key: key,
                    source: AchievementSourceEvent {
                        kind: "oteryn:character-achievement-test".into(),
                        event_id: vec![i as u8 + 1],
                    },
                }
            })
            .collect();
        let grants = h
            .root
            .commit_test_achievement_grants(&authority, &h.node, gameplay, requests)
            .await
            .map_err(debug)?;
        let mut facts = Vec::new();
        for grant in grants {
            let AchievementGrantOutcome::Granted(grant) = grant else {
                return Err("grant".into());
            };
            facts.push(EarnedAchievement {
                achievement_key: grant.achievement_key,
                earned_at_unix_ms: grant.earned_at_unix_ms,
            });
        }
        let (status, payload) = exchange(&owner, &mut admitted, 1, TAB, &query).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let result = AchievementPage::decode(&payload).map_err(debug)?;
        assert_eq!(result.character, id(41));
        assert_eq!(
            result.page,
            deps.achievements
                .account_achievements_page(&facts, 0)
                .map_err(debug)?
        );
        assert!(
            result.page.rows.iter().all(|row| !row.name.is_empty()
                && !row.description.is_empty()
                && row.earned_at > 0)
        );
        sqlx::query(
            "INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)",
        )
        .bind(id(61).as_slice())
        .execute(&h.pool)
        .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,lifecycle,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Peer Hero' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
        let foreign = AchievementRequest {
            page: 0,
            target: Some(id(60)),
        }
        .encode();
        assert!(
            owner
                .cyclopedia_character_wire(admitted, TAB, &foreign)
                .await
                .is_err()
        );
        sqlx::query("INSERT INTO game_vip_lists(account_id,world_id) SELECT account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        let permitted = owner
            .cyclopedia_character_wire(admitted, TAB, &foreign)
            .await
            .map_err(|_| "permitted profile")?;
        let permitted = AchievementPage::decode(&permitted).map_err(debug)?;
        assert_eq!(permitted.character, id(60));
        assert_eq!(permitted.name, "Peer Hero");
        assert_eq!(permitted.page.fact_count, 0);
        assert!(
            permitted.page.rows.is_empty(),
            "foreign profile must not inherit viewer facts"
        );
        let mut stale = admitted;
        let mut stale_fence = fence;
        stale_fence.character_lease_generation += 1;
        stale.item_fence = Some(stale_fence);
        assert!(
            owner
                .cyclopedia_character_wire(stale, TAB, &foreign)
                .await
                .is_err()
        );
        sqlx::query(
            "UPDATE game_vip_consents SET consent=1 WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(60).as_slice())
        .execute(&h.pool)
        .await?;
        assert!(
            owner
                .cyclopedia_character_wire(admitted, TAB, &foreign)
                .await
                .is_err()
        );
        let own = h
            .root
            .read_cyclopedia_achievement_facts(
                &authority,
                &h.node,
                fence,
                CharacterId::from_bytes(id(41)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(own.1.len(), 2);
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 2, "read-only pages must never create grant requests");
        eprintln!(
            "POSTGRES_EXECUTED character_achievement_metadata_postgres_real_dispatch_and_privacy"
        );
        h.pool.close().await;
        Ok(())
    })
}
