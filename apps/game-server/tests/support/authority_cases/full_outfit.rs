//! FULLOUT: the real fenced quest reward owner, with admitted production cosmetics.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::sync::Arc;

struct NoItems<'a>(&'a NativeGameplayState);
impl ItemWeightLookup for NoItems<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
}

fn run(
    admin: String,
    outfit: &'static str,
    sex: i16,
    achievement: Option<&'static str>,
    reverse: bool,
) -> f::TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(
                        admin,
                        &format!("full_outfit_{outfit}_{sex}_{reverse}"),
                        true,
                    )
                    .await?;
                    let result = exercise(&h, outfit, sex, achievement, reverse).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "FULLOUT_POSTGRES_CASE_COMPLETED {outfit} sex={sex} duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "full outfit test thread")?
        .map_err(Into::into)
}

async fn exercise(
    h: &f::Harness,
    outfit: &str,
    sex: i16,
    achievement: Option<&str>,
    reverse: bool,
) -> f::TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_roots SET sex=$1")
        .bind(sex)
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_operation_receipts SET command_binding=set_byte(command_binding,octet_length(command_binding)-1,$1)")
        .bind(i32::from(sex)).execute(&mut *fixture).await?;
    sqlx::query("UPDATE game_character_bootstrap_intent_floors SET intent_binding=set_byte(intent_binding,octet_length(intent_binding)-1,$1)")
        .bind(i32::from(sex)).execute(&mut *fixture).await?;
    fixture.commit().await?;
    let (controller, _) = oteryn_gameplay_content::content::native_gameplay::test_support::cosmetics_fixture::activated_with_optional_cosmetics(f::scope()?.world_id(), None, None);
    eprintln!("FULLOUT stage=activated");
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = NoItems(native);
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    eprintln!("FULLOUT stage=seal");
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    eprintln!("FULLOUT stage=authority");
    let quest = "oteryn:quest.full_outfit_proof";
    let callbacks: Vec<RewardCallback> = (0..2).map(|index| {
        serde_json::from_value(serde_json::json!({
            "key":format!("oteryn:quest-reward/full-outfit/{index}"), "quest":quest,
            "npc":"oteryn:npc.lynda", "keywords":["addon"], "topic":0,"next_topic":0,
            "storage":[{"track":"oteryn:quest-progress/full-outfit/addons","minimum":index,"maximum":index,"set":index+1}], "take":[], "give":null,
            "outfit":{"outfit":format!("oteryn:outfit.{outfit}"),"addons":if (index==0)^reverse {1} else {2}},
            "achievement":null,"reply":["Your addon."],"sources":["TibiaWiki outfit achievement spoiler, fetched 2026-11-17"]
        }))
    }).collect::<Result<_,_>>()?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: "oteryn:quest-progress/full-outfit/addons".into(),
                quest: quest.into(),
                initial: 0,
                min: 0,
                max: 2,
            }],
            callbacks
                .iter()
                .map(|callback| QuestTransition {
                    key: callback.key.clone(),
                    quest: quest.into(),
                    effects: vec![QuestEffect {
                        track: "oteryn:quest-progress/full-outfit/addons".into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Add(1),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                })
                .collect(),
        )
        .map_err(f::debug)?,
    );
    eprintln!("FULLOUT stage=catalogue");
    for (index, callback) in callbacks.iter().enumerate() {
        let request = QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Command(CommandRef::new(
                GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
                CommandId::new(9800 + index as u64).map_err(f::debug)?,
            )),
        };
        let fence = f::fence(1 + index as u64)?;
        assert!(matches!(
            h.root
                .commit_fixture_quest_reward(
                    &authority,
                    &h.node,
                    fence,
                    request.clone(),
                    catalogue.clone(),
                    admission,
                    callback.clone()
                )
                .await
                .map_err(f::debug)?,
            QuestTransitionOutcome::Committed(_)
        ));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key LIKE 'oteryn:achievement/%'").fetch_one(&h.pool).await?;
        assert_eq!(count, i64::from(index == 1 && achievement.is_some()));
        assert!(matches!(
            h.root
                .commit_fixture_quest_reward(
                    &authority,
                    &h.node,
                    fence,
                    request,
                    catalogue.clone(),
                    admission,
                    callback.clone()
                )
                .await
                .map_err(f::debug)?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
    }
    let requests: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(requests, i64::from(achievement.is_some()));
    if let Some(achievement) = achievement {
        let keys: Vec<String> =
            sqlx::query_scalar("SELECT achievement_key FROM game_account_achievements")
                .fetch_all(&h.pool)
                .await?;
        assert_eq!(keys, vec![format!("oteryn:achievement/{achievement}")]);
    }
    Ok(())
}

#[test]
fn full_outfit_quest_addons_and_replay() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "barbarian", 2, Some("brutal_politeness"), false)
}
#[test]
fn full_outfit_quest_reverse_addons() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "barbarian", 2, Some("brutal_politeness"), true)
}
#[test]
fn full_outfit_male_mage_alumni() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "mage", 2, Some("alumni"), false)
}
#[test]
fn full_outfit_female_mage_ritualist() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "mage", 1, Some("ritualist"), false)
}
#[test]
fn full_outfit_male_summoner_ritualist() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "summoner", 2, Some("ritualist"), false)
}
#[test]
fn full_outfit_female_summoner_alumni() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "summoner", 1, Some("alumni"), false)
}
#[test]
fn full_outfit_unlisted_outfit_grants_nothing() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "beggar", 2, None, false)
}
