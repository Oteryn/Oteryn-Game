use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.a_prisoner"]).await?;
    let mut mint_command = 900;
    super::qengine_voucher_stories_fixtures::mint(owner, admitted, 3585, 7, &mut mint_command)
        .await?;
    for (word, topic, apples, key) in [
        ("hi", 0, 7, None),
        ("riddle", 1, 7, None),
        ("PD-D-KS-P-PD", 2, 7, None),
        ("yes", 3, 0, None),
        ("bye", 0, 0, None),
        ("hi", 0, 0, None),
        ("key", 3, 0, None),
        ("yes", 4, 0, None),
        ("yes", 5, 0, None),
        ("yes", 0, 0, Some(3666)),
    ] {
        let command = admitted.continuity.next_command_id;
        admitted.continuity.next_command_id += 1;
        assert_eq!(
            super::qengine_rewards_fixtures::talk(owner, admitted, npc, command, word).await?,
            NpcIntentDisposition::Ok,
            "{word}, expected topic {topic}",
        );
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            topic,
            "{word}"
        );
        let actual: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
             WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3585'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(actual, apples);
        let number: Option<i32> = sqlx::query_scalar(
            "SELECT k.key_number FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key='oteryn:item.tibia.i2969'",
        )
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(number, key);
    }
    Ok(())
}
