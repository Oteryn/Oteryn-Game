//! WU26 T188/T197 (capability 492): the boss reward settlement transaction moves the
//! Moonsilver first-drop pity, and the real command 431 router reports it.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::boss_encounter::reward_chest::{
    BossRewardChestPlan, BossRewardContributor, BossRewardDeath, BossRewardItem,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_gameplay_encounters::bosses::reward_chest::RewardParticipation;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::boss_difficulty::{self as wire, PityTracker, Request};

const BOSS: &str = "oteryn:boss.mimar_haffar";
const GOLD: &str = "oteryn:item.tibia.i3031";
const MOONSILVER_HELMET: &str = "oteryn:item.tibia.i53229";

#[test]
fn boss_pity_postgres_victory_counts_and_first_drop_ends_the_tracker() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BOSS_PITY POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss pity PostgreSQL scenario panicked")??;
    Ok(())
}

fn death(seed: u8, fence: CurrentCharacterItemFence, minted: u64, item: &str) -> BossRewardDeath {
    let contribution = RewardParticipation {
        damage_dealt: 1,
        damage_received: 0,
        healing_others: 0,
    };
    BossRewardDeath {
        death: [seed; 16],
        boss_key: BOSS.into(),
        minted_at_unix_ms: minted,
        contributors: vec![BossRewardContributor {
            character_id: fence.character_id,
            session_id: fence.game_session_id,
            lease_generation: fence.character_lease_generation,
            contribution,
        }],
        chests: vec![BossRewardChestPlan {
            character_id: fence.character_id,
            session_id: fence.game_session_id,
            lease_generation: fence.character_lease_generation,
            contribution,
            items: vec![BossRewardItem {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: item.into(),
                    revision_ref: "definition-r1".into(),
                },
                quantity: 1,
            }],
        }],
    }
}

async fn stored(h: &Harness, fence: CurrentCharacterItemFence) -> TestResult<(i16, bool)> {
    Ok(sqlx::query_as("SELECT kills_without_drop,first_drop_received FROM game_character_boss_pity WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(BOSS).fetch_one(&h.pool).await?)
}

async fn window_tracker(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<Option<PityTracker>> {
    let request = Request::OpenWithProgress {
        boss_key: BOSS.into(),
    };
    let id = admitted.continuity.next_command_id;
    let (status, payload) = super::boss_difficulty::exchange(
        owner,
        admitted,
        id,
        &wire::encode_request(&request).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    Ok(wire::decode_reply_with_pity(&payload).map_err(debug)?.1)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "boss_pity", true, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                wire::CAPABILITY_BOSS_DIFFICULTY_V1,
                wire::CAPABILITY_BOSS_DIFFICULTY_PITY_V1,
            ],
        )
        .ok_or("pity bundle capability not offered")?;
        let fence = admitted.item_fence.ok_or("pity fence missing")?;
        // Catalogue values are isolated fixture data, never a game room profile.
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled) VALUES($1,'fixture-v1','oteryn:creature.mimar_haffar',1,1,1,1,1,'[]','oteryn:room.fixture','oteryn:reward.fixture',true)")
            .bind(BOSS).execute(&h.pool).await?;
        let fresh = PityTracker {
            kills_without_drop: 0,
            bonus_permille: 0,
            tracker_active: true,
        };
        assert_eq!(window_tracker(&owner, &mut admitted).await?, Some(fresh));

        let minted: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        let minted = u64::try_from(minted)?;
        let settle = |plan: BossRewardDeath| {
            h.root.settle_boss_reward_chests(
                &authority,
                &h.node,
                fence.runtime_scope,
                fence.scope_ownership_generation,
                plan,
            )
        };
        // A victory without Moonsilver counts one miss; its replay counts nothing.
        assert!(settle(death(201, fence, minted, GOLD)).await?);
        assert_eq!(stored(&h, fence).await?, (1, false));
        assert!(settle(death(201, fence, minted, GOLD)).await?);
        assert_eq!(stored(&h, fence).await?, (1, false));
        assert_eq!(
            h.root
                .boss_pity_bonuses(
                    &authority,
                    &h.node,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                    BOSS.into(),
                    vec![*fence.character_id.as_bytes()],
                )
                .await?,
            vec![(*fence.character_id.as_bytes(), 100)]
        );
        assert_eq!(
            window_tracker(&owner, &mut admitted).await?,
            Some(PityTracker {
                kills_without_drop: 1,
                bonus_permille: 100,
                tracker_active: true,
            })
        );
        eprintln!("BOSS_PITY_MISS POSTGRES_CASE_COMPLETED");

        // The first Moonsilver drop ends the tracker; later victories leave it off.
        assert!(settle(death(202, fence, minted, MOONSILVER_HELMET)).await?);
        assert_eq!(stored(&h, fence).await?, (1, true));
        assert!(settle(death(203, fence, minted, GOLD)).await?);
        assert_eq!(stored(&h, fence).await?, (1, true));
        let ended = PityTracker {
            kills_without_drop: 1,
            bonus_permille: 0,
            tracker_active: false,
        };
        assert_eq!(window_tracker(&owner, &mut admitted).await?, Some(ended));

        // A session without capability 492 still receives the V1 window bytes.
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                wire::CAPABILITY_BOSS_DIFFICULTY_V1,
            ],
        )
        .ok_or("difficulty bundle capability not offered")?;
        assert_eq!(window_tracker(&owner, &mut admitted).await?, None);
        eprintln!("BOSS_PITY_FIRST_DROP POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}
