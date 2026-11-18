//! Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:8-9,20-32,
//! data/items/items.xml:28263-28265.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{num::NonZeroU64, sync::Arc};

pub(super) const NAME: &str = "qewrath-incarnation-remains";
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission10";
const STATUS: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/boss_status";
const COOLDOWN: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/sceptre_last_used";
const SCEPTRE: &str = "oteryn:item.tibia.i11362";
const CREATURE: &str = "oteryn:creature.wrath_of_the_emperor";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission10_a_message_of_freedom_sceptre.lua:20-32";
const BODY: Position = Position {
    x: 33050,
    y: 31085,
    floor: -15,
};

#[test]
fn banishing_native_incarnation_remains_credits_once_within_the_sceptre_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, BODY, false)
    })
}
fn fixture() -> TestResult<QuestStateCatalogue> {
    let callback = json!({
        "key":"oteryn:quest-reward/wrath/banish-presence","quest":QUEST,"npc":"oteryn:npc.zalamon",
        "keywords":["sceptre"],"topic":0,"next_topic":0,"take":[],"give":null,"outfit":null,"achievement":null,
        "reply":["The sceptre is almost torn from your hand as you banish the presence of the emperor."],"sources":[SOURCE], "carried_item":SCEPTRE,"creature_target":"oteryn:creature.fury_of_the_emperor",
        "present":[{"item":SCEPTRE,"count":1}],
        "corpse_target":{"appearance":11361,"retain_tool":true,
            "additional_creatures":[CREATURE,"oteryn:creature.scorn_of_the_emperor","oteryn:creature.spite_of_the_emperor"]},
        "storage":[{"track":QUESTLINE,"minimum":0,"maximum":34,"set":0,"preserve":true}],
        "state_ops":[
            {"track":STATUS,"from":{"kind":"between","minimum":0,"maximum":4},"effect":{"kind":"add","value":1}},
            {"track":MISSION,"from":{"kind":"between","minimum":0,"maximum":5},"effect":{"kind":"add_at_least","value":1,"minimum":2}},
            {"track":COOLDOWN,"from":{"kind":"elapsed_at_least","seconds":60},"effect":{"kind":"set_now"}},
        ],
    });
    let digest = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let binding = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
        "tracks":[
            {"key":QUESTLINE,"initial":30,"minimum":0,"maximum":34},
            {"key":MISSION,"initial":1,"minimum":0,"maximum":6},
            {"key":STATUS,"initial":0,"minimum":0,"maximum":5},
            {"key":COOLDOWN,"initial":0,"minimum":0,"maximum":i64::MAX},
        ],
        "stages":[{"key":"banish-presence","completes":false,"trigger":{"kind":"carried_use","callback":callback}}],
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&binding)?)?.attach(&base)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.quest_catalogue = Some(Arc::new(fixture()?));
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: BODY,
            item: SCEPTRE.into(),
            count: 1,
        },
        610,
        true,
    )
    .await?;
    super::questpc7_goblin::stalker_death(owner, admitted, CREATURE, BODY, 9190).await?;
    let source: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(SCEPTRE).fetch_one(&h.pool).await?;
    let corpse: sqlx::types::Uuid = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i11361' AND i.lifecycle=1")
        .fetch_one(&h.pool).await?;
    let corpse_handle = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|body| body.item.item_instance_id == *corpse.as_bytes())
        .ok_or("visible incarnation remains")?
        .identity;
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Item(NonZeroU64::new(2).ok_or("body handle")?)),
    };
    for (id, target, expected) in [
        (
            9200,
            Some(ItemKey::Entity(corpse_handle)),
            UseDisposition::Committed,
        ),
        (9200, None, UseDisposition::Committed),
        (
            9201,
            Some(ItemKey::Entity(corpse_handle)),
            UseDisposition::Rejected,
        ),
        (
            9202,
            Some(ItemKey::Instance(*corpse.as_bytes())),
            UseDisposition::Rejected,
        ),
    ] {
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .use_quest_tool(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: id,
                    item_fence: Some(fence.item_fence()),
                },
                intent.clone(),
                Some(ItemKey::Instance(*source.as_bytes())),
                target,
            )
            .await
            .map_err(|()| "sceptre USE")?
            .ok_or("sceptre USE not owned")?;
        assert_eq!(outcome.disposition, expected);
    }
    let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_corpse_samples WHERE source_item=$1 AND corpse_item=$2 AND maximum_age_ms IS NULL),(SELECT quantity FROM game_item_instances WHERE item_instance_id=$1 AND lifecycle=1),(SELECT count(*) FROM game_quest_callback_consumptions WHERE item_instance_id=$1)")
        .bind(source).bind(corpse).fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 0));
    for (track, value) in [(STATUS, 1), (MISSION, 2)] {
        let actual: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(track)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(actual, value);
    }
    Ok(())
}
