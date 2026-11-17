//! Selected M01a briefings through the production registry and NPC transport.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::authored_bindings::Trigger;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

#[test]
fn m01a_rafzan_baxter_emilie_production_talk_routes_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m01a-starter-routes")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M01A_STARTER_ROUTES_POSTGRES {version}");
    let names = [
        "oteryn:npc.rafzan",
        "oteryn:npc.baxter",
        "oteryn:npc.emilie",
    ];
    let quests = [
        "oteryn:quest.goblin_merchant_quest",
        "oteryn:quest.machinery_of_war_quest",
        "oteryn:quest.authored.annual_autumn_vintage",
    ];
    let callbacks = {
        let bindings = owner
            .quest_catalogue
            .as_ref()
            .ok_or("production catalogue")?
            .authored_bindings()
            .ok_or("production registry")?;
        let mut callbacks = Vec::new();
        for (key, npc) in quests.iter().zip(names) {
            let quest = bindings
                .quests
                .iter()
                .find(|quest| quest.quest == *key)
                .ok_or("selected production route is missing")?;
            let stage = quest
                .stages
                .iter()
                .find(|stage| stage.key == "s1")
                .ok_or("briefing")?;
            assert!(!stage.completes);
            let Trigger::NpcDialogue { callback, .. } = &stage.trigger else {
                return Err("TALK route".into());
            };
            assert_eq!(callback.npc, npc);
            callbacks.push(callback.as_ref().clone());
        }
        callbacks
    };
    let initial: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=ANY($1)",
    )
    .bind(quests.map(str::to_owned).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(initial, 0, "no chosen progress is seeded");
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let actors = super::classic_quests::install(owner, admitted, names).await?;
    for (index, callback) in callbacks.iter().enumerate() {
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let before_tracks: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT quest_key, track_key, value FROM game_character_quest_tracks ORDER BY quest_key, track_key")
            .fetch_all(&h.pool).await?;
        let before_receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        let (wrong, _, _) = super::qengine_rewards::wire_reward(
            owner,
            admitted,
            actors[(index + 1) % actors.len()],
            callback,
            false,
        )
        .await?;
        // NPC-0 accepts unrecognised text in an engaged conversation. Its Ok
        // is not a quest grant; the complete durable footprint must stay put.
        assert_eq!(wrong, NpcIntentDisposition::Ok);
        let after_tracks: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT quest_key, track_key, value FROM game_character_quest_tracks ORDER BY quest_key, track_key")
            .fetch_all(&h.pool).await?;
        let after_receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            after_tracks, before_tracks,
            "another NPC cannot advance any quest track"
        );
        assert_eq!(
            after_receipts, before_receipts,
            "another NPC cannot issue a quest receipt"
        );
        if callback.minimum_level == Some(50) {
            let (low, _, _) = super::qengine_rewards::wire_reward(
                owner,
                admitted,
                actors[index],
                callback,
                false,
            )
            .await?;
            assert_eq!(
                low,
                NpcIntentDisposition::Rejected,
                "Baxter's chosen level gate is live"
            );
            let rows: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
            )
            .bind(&callback.quest)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(rows, 0, "refusals cannot advance the route");
            super::qengine_outfit_stories::earn_level(owner, admitted, authority, 50).await?;
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
        }
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, actors[index], callback, false)
                .await?;
        assert_eq!(
            result,
            NpcIntentDisposition::Ok,
            "selected briefing {}",
            callback.quest
        );
        let value: i64 = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
        )
        .bind(&callback.quest)
        .bind(&callback.storage[0].track)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(value, 1);
        let (again, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, actors[index], callback, false)
                .await?;
        assert_eq!(
            again,
            NpcIntentDisposition::Rejected,
            "a fresh command cannot repeat the briefing"
        );
        let receipts: (i64, i64) = sqlx::query_as("SELECT count(*), count(*) FILTER (WHERE completes) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(&callback.quest).fetch_one(&h.pool).await?;
        assert_eq!(
            receipts,
            (1, 0),
            "literal replay and fresh repeats never duplicate or complete the campaign"
        );
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let persisted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=ANY($1) AND value=1",
    )
    .bind(quests.map(str::to_owned).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(persisted, 3);
    Ok(())
}
