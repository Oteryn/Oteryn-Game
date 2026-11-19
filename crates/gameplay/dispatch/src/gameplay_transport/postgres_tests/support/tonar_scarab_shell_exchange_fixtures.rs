//! Crystal 201189f8 data-global/npc/tonar.lua:66-81, data/items/items.xml:13001-13016.
use std::sync::Arc;

use super::archivist_vote_fixtures::{FixtureItem, with_npc_items};
use super::classic_quests_fixtures::{talk, talk_with_disposition};
use super::fixture::{Harness, debug, id};
use super::full_world_support::{mint_carried, quantity};
use super::{ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use serde_json::json;

const QUEST: &str = "oteryn:quest.grimvale_quest";
const NPC: &str = "oteryn:npc.tonar";
const CREDIT: &str = "oteryn:quest-progress/quest/u12_00/grimvale_quest/scarab_ocarina";
const FRESH: &str = "oteryn:item.tibia.i4194";
const MATURE: &str = "oteryn:item.tibia.i4195";
const OCARINA: &str = "oteryn:item.tibia.i43740";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/tonar.lua:66-81";

pub(super) fn catalogue() -> TestResult<QuestStateCatalogue> {
    let binding = Bindings::parse(json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "tracks":[{"key":CREDIT,"initial":-1,"minimum":-1,"maximum":1}],
        "stages":[{"key":"s1","completes":false,"trigger":{"kind":"npc_dialogue","callback":{
            "key":"oteryn:quest-reward/fixture/tonar-ocarina","quest":QUEST,"npc":NPC,
            "topic":1,"next_topic":0,"keywords":["yes"],"repeatable":true,"per_occurrence_mint":true,
            "storage":[{"track":CREDIT,"minimum":-1,"maximum":1,"set":1}],
            "inventory":[{"item":MATURE,"minimum":1,"maximum":u32::MAX}],
            "take":[{"item":MATURE,"count":1}],"give":{"item":OCARINA,"count":1},
            "reply":["Here is your scarab ocarina."],"failure_reply":["Wait for it to decompose."],"failure_next_topic":0,"sources":[SOURCE]
        }}}],"sources":[SOURCE]
    }]}).to_string().as_bytes())?;
    let base = QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)?;
    Ok(binding.attach(&base)?)
}

pub(super) fn install_offer(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.states.npc_conversations.fixture_reward_offers(
        &json!({"schema":1,"offers":[{
            "npc":NPC,"keywords":["ocarina","scarab"],"topic":0,"any_topic":true,"next_topic":1,
            "storage":[],"reply":["Did you bring an intact shell?"]
        }]})
        .to_string(),
    )?;
    Ok(())
}

pub(super) async fn exercise(h: &Harness) -> TestResult {
    let profiles = [
        FixtureItem {
            id: 4194,
            name: "dead scarab",
            weight: 2000,
            stackable: false,
        },
        FixtureItem {
            id: 4195,
            name: "dead scarab",
            weight: 2000,
            stackable: false,
        },
        FixtureItem {
            id: 43740,
            name: "scarab ocarina",
            weight: 120,
            stackable: false,
        },
    ];
    with_npc_items(h, Arc::new(catalogue()?), NPC, &profiles, async |owner, admitted, npc| {
        install_offer(owner)?;
        mint_carried(h, owner, *admitted, FRESH, 1, 92).await?;
        talk(owner, admitted, npc, 1, "hi").await?;
        talk(owner, admitted, npc, 2, "ocarina").await?;
        assert_eq!(talk_with_disposition(owner, admitted, npc, 3, "yes", NpcIntentDisposition::Rejected).await?, ["Wait for it to decompose."]);
        assert_eq!(owner.states.npc_conversations.fixture_quest_topic(admitted.game_session_id)?, 0);
        assert_eq!(quantity(h, FRESH).await?, 1);
        assert_eq!(quantity(h, OCARINA).await?, 0);
        for (seed, command) in [(93, 4), (94, 6)] {
            mint_carried(h, owner, *admitted, MATURE, 1, seed).await?;
            talk(owner, admitted, npc, command, "scarab").await?;
            assert_eq!(talk(owner, admitted, npc, command + 1, "yes").await?, ["Here is your scarab ocarina."]);
            assert_eq!(quantity(h, MATURE).await?, 0);
        }
        assert_eq!(quantity(h, FRESH).await?, 1);
        assert_eq!(quantity(h, OCARINA).await?, 2);
        let credit: i64 = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2"
        ).bind(id(41).as_slice()).bind(CREDIT).fetch_one(&h.pool).await?;
        assert_eq!(credit, 1);
        Ok(())
    }).await
}
