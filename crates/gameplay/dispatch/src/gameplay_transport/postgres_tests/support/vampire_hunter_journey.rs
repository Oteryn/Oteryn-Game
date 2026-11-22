use super::qengine_rewards_fixtures::{talk, talk_outcome};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const INQUISITION: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const RANK: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/rank";
const DOOR: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/door";
const TOKEN: &str = "oteryn:item.tibia.i8192";

pub(super) async fn loot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native loot")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(super::fixture::debug)?;
    super::qengine_kill_stories_fixtures::death(
        owner,
        admitted,
        "oteryn:creature.diblis_the_fair",
        60_000,
    )
    .await?;
    let tokens: Vec<i64> = sqlx::query_scalar(
        "SELECT quantity FROM game_boss_reward_items WHERE definition_production_key=$1 AND NOT expired AND collected_transaction_id IS NULL",
    ).bind(TOKEN).fetch_all(&h.pool).await?;
    assert_eq!(tokens, [1]);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::qengine_outfit_stories_fixtures::seed_quest_prerequisites;
    use super::{classic_quests_fixtures, quest_world_callbacks_fixtures};

    let [npc] = classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.storkus"]).await?;
    let first = admitted.continuity.next_command_id;
    assert_eq!(
        talk(owner, admitted, npc, first, "hi").await?,
        NpcIntentDisposition::Ok
    );
    seed_quest_prerequisites(h, owner, admitted, &[(INQUISITION.into(), 10)], 116).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let gated = talk_outcome(owner, admitted, npc, first + 1, "vampire lord token").await?;
    assert!(gated.conversation.is_none(), "no offer before Inquisition");
    seed_quest_prerequisites(h, owner, admitted, &[(INQUISITION.into(), 11)], 117).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    let base_xp: i64 =
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    let mut total_xp = 0;
    for (index, (tokens, xp)) in [
        (1, 1000),
        (4, 5000),
        (5, 10000),
        (10, 20000),
        (30, 50000),
        (50, 100000),
    ]
    .into_iter()
    .enumerate()
    {
        let rank = i64::try_from(index + 1)?;
        let command = first + 2 + u64::try_from(index)? * 4;
        let offer = talk_outcome(owner, admitted, npc, command, "vampire lord token").await?;
        assert_eq!(offer.disposition, NpcIntentDisposition::Ok);
        assert_eq!(
            offer.conversation.ok_or("Storkus exchange offer")?.lines,
            ["Would ye' like to give me vampire tokens?"]
        );
        assert_eq!(
            talk(owner, admitted, npc, command + 1, "yes").await?,
            NpcIntentDisposition::Rejected
        );
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "token fence")?
            .ok_or("token fence")?;
        quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: TOKEN.into(),
                count: tokens,
            },
            15_000 + u64::try_from(index)?,
            true,
        )
        .await?;
        assert_eq!(
            talk(owner, admitted, npc, command + 2, "vampire lord token").await?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            talk(owner, admitted, npc, command + 3, "yes").await?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT value::bigint FROM game_character_quest_tracks WHERE track_key=$1"
            )
            .bind(RANK)
            .fetch_one(&h.pool)
            .await?,
            rank
        );
        let door: Option<i64> = sqlx::query_scalar(
            "SELECT value::bigint FROM game_character_quest_tracks WHERE track_key=$1",
        )
        .bind(DOOR)
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(door, (rank == 6).then_some(1));
        let access = owner
            .states
            .quest_sessions
            .lock()
            .map_err(super::fixture::debug)?
            .get(&admitted.game_session_id)
            .and_then(|session| session.copy.as_ref())
            .ok_or("quest door session copy")?
            .meets_door_storage(41731);
        assert_eq!(access, rank == 6, "native quest door access at rank {rank}");
        total_xp += xp;
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT total_experience::bigint FROM game_character_progression_state"
            )
            .fetch_one(&h.pool)
            .await?,
            base_xp + total_xp
        );
        let taken: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1 AND before_item->>'definition_production_key'=$2")
            .bind(format!("oteryn:quest-reward/vampire-hunter-rank-{rank}")).bind(TOKEN).fetch_one(&h.pool).await?;
        assert_eq!(taken, i64::from(tokens));
    }
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE m.claim_production_key='oteryn:quest-reward/vampire-hunter-rank-6' AND i.definition_production_key='oteryn:item.tibia.i8191'")
        .fetch_one(&h.pool).await?, 1);
    assert!(sqlx::query_scalar::<_, bool>("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key='oteryn:quest.vampire_hunter_quest'")
        .fetch_one(&h.pool).await?);
    Ok(())
}
