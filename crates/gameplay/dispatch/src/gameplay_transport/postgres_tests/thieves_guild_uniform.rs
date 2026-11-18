//! Crystal 201189f8 data-global/npc/chantalle.lua:82-88, through NPC dialogue.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, npc_rules::NpcRuleCatalog};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const NAME: &str = "thieves-guild-uniform";
const NPC: &str = "oteryn:npc.chantalle";
const QUEST: &str = "oteryn:quest.the_thieves_guild_quest";
const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/mission06";
const RANK: &str = "oteryn:quest-progress/quest/u7_24/the_postman_missions/rank";
const HAT: &str = "oteryn:item.tibia.i3576";
const KEY: &str = "oteryn:item.tibia.i7934";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/chantalle.lua:82-88";
const REPLY: &str = "Oh my! You look so great in your uniform! You archpostmen are not only daring but also handsome. Here take it, that's the key you wanted. Just promise to visit me now and then!";

#[test]
fn fishnapping_grants_key_only_with_the_equipped_officer_hat_rank_and_sex() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, Position { x: 33294, y: 31677, floor: -15 }, false)
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use serde_json::json;
    let postman = "oteryn:quest.the_postman_missions_quest";
    let catalogue = QuestStateCatalogue::new(
        owner.quest_catalogue.as_ref().ok_or("catalogue")?.content_revision(),
        vec![oteryn_gameplay_quests::quest::QuestTrack {
            key: RANK.into(), quest: postman.into(), initial: 5, min: -1, max: 5
        }],
        vec![],
    ).map_err(super::fixture::debug)?;
    let program = json!({
        "schema": 1,
        "quest": QUEST,
        "tracks": [{"key": MISSION,"quest": QUEST,"initial": 1,"min": -1,"max": 2,"sources": [SOURCE]}],
        "rules": [{
            "key": "oteryn:quest-npc/thieves-guild/chantalle/officer-key",
            "quest": QUEST, "npc": NPC, "trigger": "say", "keywords": ["key"],
            "guards": [
                {"kind": "storage", "track": MISSION, "op": "eq", "value": 1},
                {"kind": "equipped", "slot": "head", "item": HAT},
                {"kind": "storage", "track": RANK, "op": "eq", "value": 5},
                {"kind": "sex", "value": 1}
            ],
            "effects": [{"kind": "give_item", "item": KEY, "count": 1}, {"kind": "set", "track": MISSION,"value": 2}],
            "reply": [REPLY], "sources": [SOURCE]
        }]
    }).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        NpcRuleCatalog::from_json_documents(&[&program])?.bind_fixture(catalogue)?
    ));
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let character = id(41);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_roots SET sex=2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
        .bind(character.as_slice()).execute(&mut *tx).await?;
    let hat = id(113);
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(hat.as_slice()).bind(id(42).as_slice()).bind(HAT).execute(&mut *tx).await?;
    tx.commit().await?;

    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    super::classic_quests::talk(owner, admitted, npc, 6400, "hi").await?;
    let reply = super::classic_quests::talk(owner, admitted, npc, 6401, "key").await?;
    assert!(!reply.iter().any(|line| line == REPLY));
    assert_eq!(key_count(h).await?, 0);

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(character.as_slice()).bind(id(42).as_slice()).bind(hat.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    let reply = super::classic_quests::talk(owner, admitted, npc, 6402, "key").await?;
    assert!(reply.iter().any(|line| line == REPLY));
    assert_eq!(key_count(h).await?, 1);
    let stage: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(character.as_slice()).bind(MISSION).fetch_one(&h.pool).await?;
    assert_eq!(stage, 2);
    super::classic_quests::talk(owner, admitted, npc, 6403, "key").await?;
    assert_eq!(key_count(h).await?, 1);
    Ok(())
}

async fn key_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(KEY).fetch_one(&h.pool).await?)
}
