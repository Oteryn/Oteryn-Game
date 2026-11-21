use super::fixture::Harness;
use super::qengine_outfit_stories_fixtures::{
    seed_quest_prerequisites, select_outfit_capabilities, supply,
};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const OUTFIT: &str = "oteryn:quest.outfit_and_addon_quests";
const MAIN_PIN: &str = "f03faa35abd9af89c281d3b4f7db2dd569ae32eadd9187727e7882e4c90e3ccf";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use oteryn_gameplay_quests::quest::{npc_rules, reward_callbacks};

    let (npc_key, key, track) = match name {
        "qengine-rewards-l6-amber-main-outfit-pin" => (
            "oteryn:npc.amber",
            "oteryn:quest-reward/l6-citizen-rook-backpack-start",
            "oteryn:quest-progress/outfit_quest/ref",
        ),
        "qengine-rewards-l5-hjaern-main-outfit-pin" => (
            "oteryn:npc.hjaern",
            "oteryn:quest-reward/l5-norseman-base",
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/norseman_outfit",
        ),
        _ => return Err("unknown participant pin case".into()),
    };
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [npc_key]).await?;
    select_outfit_capabilities(owner, admitted)?;
    let callbacks = reward_callbacks::load()?;
    let reward = callbacks.iter().find(|row| row.key == key).ok_or("reward")?;
    let prerequisites = if npc_key == "oteryn:npc.hjaern" {
        let ice = "oteryn:quest-progress/quest/u8_0/the_ice_islands/";
        std::iter::once((format!("{ice}questline"), 39))
            .chain((1..=4).map(|n| (format!("{ice}obelisk0{n}"), 5)))
            .collect::<Vec<_>>()
    } else {
        vec![(
            "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/mission_backpack_rook".into(),
            0,
        )]
    };
    seed_quest_prerequisites(h, owner, admitted, &prerequisites, 106).await?;
    if npc_key == "oteryn:npc.hjaern" {
        let charm = reward.take.first().ok_or("frost charm")?;
        let mut bags = Vec::new();
        let mut occurrence = 500;
        supply(owner, admitted, h, charm, &mut bags, &mut occurrence).await?;
    }

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let pin: Vec<u8> = sqlx::query_scalar("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$1,'content-1',decode($2,'hex'),character_revision,character_revision FROM game_character_roots RETURNING definition_hash")
        .bind(OUTFIT)
        .bind(MAIN_PIN)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let stored_hash = pin.as_slice().try_into()?;
    assert_ne!(catalogue.definition_hash(OUTFIT), Some(stored_hash));
    assert!(npc_rules::legacy_defaults(catalogue, OUTFIT, &stored_hash).is_some());
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    let (result, _, _) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, reward, reward.outfit.is_some())
            .await?;
    assert_eq!(result, NpcIntentDisposition::Ok, "{key}");
    let receipt: (Vec<u8>, String) = sqlx::query_as("SELECT definition_hash,pinned_content_revision FROM game_quest_reward_participants WHERE quest_key=$1 AND transition_key=$2")
        .bind(OUTFIT)
        .bind(key)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipt, (pin.clone(), "content-1".to_owned()));
    let state: (Vec<u8>, bool) = sqlx::query_as("SELECT definition_hash,completed_character_revision IS NULL FROM game_character_quest_states WHERE quest_key=$1")
        .bind(OUTFIT)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(state, (pin, true));
    let value: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(OUTFIT)
    .bind(track)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(value, 1);
    Ok(())
}
