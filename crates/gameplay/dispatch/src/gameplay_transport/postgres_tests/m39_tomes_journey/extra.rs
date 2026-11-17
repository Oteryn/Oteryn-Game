//! Ordinary extra books are paid inputs, never seeded Tome progression or XP.
use super::*;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    cael: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let before = xp(h).await?;
    assert_eq!(
        grant(owner, admitted, cael, "l2-cael-tome-13").await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(xp(h).await?, before);
    for number in 1..=2 {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "extra Tome fence")?
            .ok_or("extra Tome fence")?;
        // Reuse the ordinary-input MINT owner. No quest track or XP is supplied.
        super::super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: world::start()?,
                item: "oteryn:item.tibia.i10217".into(),
                count: 1,
            },
            19_000 + u64::try_from(number)?,
            true,
        )
        .await?;
        assert_eq!(carried(h, 10217).await?, 1);
        let (result, command) = grant(owner, admitted, cael, "l2-cael-tome-13").await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(carried(h, 10217).await?, 0);
        assert_eq!(stage(h, COUNTER).await?, Some(12));
        assert_eq!(xp(h).await? - before, number * 5000);
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
            &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: cael,
                text: "yes".into(),
            },
        )
        .map_err(debug)?;
        wire(owner, admitted, command, 7, payload).await?;
        assert_eq!(
            xp(h).await? - before,
            number * 5000,
            "literal replay cannot repeat XP"
        );
        assert_eq!(
            grant(owner, admitted, cael, "l2-cael-tome-13").await?.0,
            NpcIntentDisposition::Rejected,
            "a fresh command still needs another real book"
        );
        assert_eq!(xp(h).await? - before, number * 5000);
    }
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_xp_obligations WHERE transition_key='oteryn:quest-reward/l2-cael-tome-13'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        pending, 0,
        "native XP consumer drains both paid obligations"
    );
    Ok(())
}

async fn xp(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?,
    )
}
