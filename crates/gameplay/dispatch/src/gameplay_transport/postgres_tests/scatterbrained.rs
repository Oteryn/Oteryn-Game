//! The authored Eclesius turn-in uses the production NPC and reward owners.
use super::fixture::{Harness, debug};
use super::qengine_rewards::{scenario, wire_conversation};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(super) const CASE: &str = "qengine-rewards-scatterbrained-feed";
const KEY: &str = "oteryn:quest-reward/chosen/scatterbrained/feed";

#[test]
fn scatterbrained_feed_reopens_at_server_save() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(CASE)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == KEY)
        .ok_or("authored Eclesius reward")?;
    let [npc] = super::classic_quests::install(owner, admitted, [callback.npc.as_str()]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC quest capabilities")?;
    for ordinal in [801, 802] {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("current quest fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33294,
                    y: 31677,
                    floor: -15,
                },
                item: callback.take[0].item.clone(),
                count: 1,
            },
            ordinal,
            true,
        )
        .await?;
    }
    sqlx::raw_sql(
        "CREATE TABLE repeatcd_test_clock (now timestamptz NOT NULL);
         INSERT INTO repeatcd_test_clock VALUES (statement_timestamp());
         GRANT SELECT ON repeatcd_test_clock TO oteryn_game_runtime;
         CREATE OR REPLACE FUNCTION game_native_daily_reward_clock() RETURNS timestamptz
         LANGUAGE sql STABLE SET search_path = pg_catalog AS
         $$ SELECT now FROM public.repeatcd_test_clock $$;",
    )
    .execute(&h.pool)
    .await?;
    let before = experience(h).await?;
    for (clock, expected, reports) in [
        ("2026-01-10 05:01:00+00", NpcIntentDisposition::Ok, 1),
        ("2026-01-11 04:59:59+00", NpcIntentDisposition::Rejected, 1),
        ("2026-01-11 05:00:00+00", NpcIntentDisposition::Ok, 2),
    ] {
        sqlx::query("UPDATE repeatcd_test_clock SET now=$1::text::timestamptz")
            .bind(clock)
            .execute(&h.pool)
            .await?;
        let (result, _, _) = wire_conversation(
            owner,
            admitted,
            npc,
            &callback,
            false,
            vec!["hi", "feed", "yes"],
        )
        .await?;
        let sources: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_reward_sources
             WHERE transition_key=$1 AND mint_transaction_id IS NOT NULL AND plan->>'experience'='250'"
        ).bind(KEY).fetch_one(&h.pool).await?;
        assert_eq!(sources, reports);
        assert_eq!(experience(h).await?, before + 250 * reports);
        let contents: (i64, i64, i64) = sqlx::query_as(
            "SELECT coalesce(sum(i.quantity) FILTER (WHERE i.definition_production_key='oteryn:item.tibia.i9109'),0)::bigint,
                    coalesce(sum(i.quantity) FILTER (WHERE i.definition_production_key='oteryn:item.tibia.i3031'),0)::bigint,
                    coalesce(sum(i.quantity) FILTER (WHERE i.definition_production_key='oteryn:item.tibia.i3035'),0)::bigint
             FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)"
        ).fetch_one(&h.pool).await?;
        assert_eq!(contents.0, 2 - reports);
        assert_eq!(contents.1 + 100 * contents.2, 150 * reports);
        assert_eq!(result, expected, "clock {clock}");
    }
    Ok(())
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience + coalesce((SELECT sum(amount)::bigint
           FROM game_character_quest_xp_obligations WHERE transition_key=$1),0)
         FROM game_character_progression_state",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?)
}
