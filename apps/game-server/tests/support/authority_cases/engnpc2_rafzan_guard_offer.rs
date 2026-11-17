//! Registered Rafzan offer -> confirmation -> one native Guardcatcher.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, reward_callbacks};
use std::sync::Arc;

#[test]
fn engnpc2_rafzan_postgres_registered_guard_offer_and_grant() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_rafzan_guards", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_rafzan_postgres_registered_guard_offer_and_grant duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "Rafzan PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let callbacks = reward_callbacks::load().map_err(f::debug)?;
    let offer = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/goblin-merchant/guards-offer")
        .ok_or("registered guards offer")?;
    let grant = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/goblin-merchant/guardcatcher-grant")
        .ok_or("registered Guardcatcher grant")?;
    assert_eq!(offer.keywords, ["guards"]);
    assert_eq!(offer.topic, 0);
    assert_eq!(grant.keywords, ["yes"]);
    assert_eq!(offer.next_topic, grant.topic);
    assert_eq!(grant.next_topic, 0);
    assert_eq!(offer.npc, "oteryn:npc.rafzan");
    assert_eq!(grant.npc, offer.npc);
    assert!(offer.give.is_none());
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: grant.storage[0].track.clone(),
                quest: grant.quest.clone(),
                initial: 0,
                min: 0,
                max: 2,
            }],
            [offer, grant]
                .into_iter()
                .map(reward_callbacks::primary_transition)
                .collect::<Result<Vec<_>, _>>()
                .map_err(f::debug)?,
        )
        .map_err(f::debug)?,
    );
    let (controller, _pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native content")?;
    let weights = super::engitem_reward_text::Weights(native);
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    super::engitem_reward_text::equip_backpack(h, &authority, native, &weights).await?;
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let request = |callback: &reward_callbacks::RewardCallback, ordinal| -> f::TestResult<_> {
        Ok(QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Command(super::engitem_reward_text::command(ordinal)?),
        })
    };
    let apply = |revision, callback: &reward_callbacks::RewardCallback, request| {
        let fence = f::fence(revision);
        let catalogue = catalogue.clone();
        let callback = callback.clone();
        let authority = &authority;
        async move {
            let outcome = h
                .root
                .commit_fixture_quest_reward(
                    authority, &h.node, fence?, request, catalogue, admission, callback,
                )
                .await
                .map_err(f::debug)?;
            Ok::<_, Box<dyn std::error::Error>>(outcome)
        }
    };
    let offered = request(offer, 13300)?;
    let accepted = request(grant, 13301)?;
    assert!(
        matches!(
            apply(1, grant, accepted.clone()).await?,
            QuestTransitionOutcome::Refused(_)
        ),
        "yes cannot bypass the offer"
    );
    assert_eq!(h.count("game_character_quest_receipts").await?, 0);
    assert!(matches!(
        apply(1, offer, offered.clone()).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(
        h.count("game_reward_claim_mint_receipts").await?,
        0,
        "the question must not issue the bow"
    );
    let tracks: Vec<(String, i64)> =
        sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks")
            .fetch_all(&h.pool)
            .await?;
    let tracks = tracks.into_iter().collect();
    let resumed =
        oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::rafzan_resume_offer(
            &offer.npc, 0, "guards", &catalogue, &tracks,
        )
        .ok_or("registered read-only resume")?;
    assert_eq!(resumed, (grant.topic, offer.reply.clone()));
    assert!(
        oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::rafzan_resume_offer(
            &offer.npc, 0, "yes", &catalogue, &tracks
        )
        .is_none()
    );
    assert_eq!(
        h.root_revision().await?,
        "2",
        "resuming a question has no durable effect"
    );
    assert!(matches!(
        apply(2, grant, accepted.clone()).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let quantity:i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i17669' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(quantity, 1);
    let atomic:i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON m.xmin=q.xmin WHERE q.transition_key=$1").bind(&grant.key).fetch_one(&h.pool).await?;
    assert_eq!(atomic, 1);
    assert!(matches!(
        apply(2, grant, accepted).await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(
        matches!(
            apply(3, grant, request(grant, 13302)?).await?,
            QuestTransitionOutcome::Refused(_)
        ),
        "fresh confirmations cannot duplicate the tool"
    );
    assert!(matches!(
        apply(3, offer, request(offer, 13303)?).await?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.root_revision().await?, "3");
    assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);
    Ok(())
}
