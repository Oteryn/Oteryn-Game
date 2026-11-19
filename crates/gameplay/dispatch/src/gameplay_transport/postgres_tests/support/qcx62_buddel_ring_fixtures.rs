use super::fixture::{Harness, debug};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests_fixtures as classic_quests,
};
use crate::content::npc_catalogue::travel::DeclinedExchange;
use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    npc_rules::NpcRuleCatalog,
    world_callbacks::{Offering, Position},
};
use serde_json::json;

pub(super) const NAME: &str = "qcx62-buddel-ring-exchange";
const NPC: &str = "oteryn:npc.buddel";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const RING: &str = "oteryn:item.tibia.i3097";
const EXCHANGES: &str = "oteryn:quest-progress/native/barbarian_test/buddel_ring_crossings";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/buddel.lua:59-67";
const OKOLNIR: Position = super::qcx62_buddel_fixtures::ARRIVAL_CELLS[0];

pub(super) fn catalogue(native: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let program = json!({
        "schema":1, "quest":QUEST, "program_key":"oteryn:quest-npc-program.barbarian-test-buddel",
        "source_files":[{
            "path":"data-global/npc/buddel.lua",
            "revision":"201189f871d23597b82caf88fd611454a2ddefb8",
            "sha256":"8c5ad304be7253f3b728ce1daf354584d10032f3dbfcc270ecc015d815623485"
        }],
        "tracks":[{"key":EXCHANGES,"quest":QUEST,"initial":0,"min":0,"max":i64::MAX,"sources":[SOURCE]}],
        "rules":[
            {
                "key":"oteryn:quest-npc-rule.buddel.offer-ring-okolnir","quest":QUEST,"npc":NPC,
                "trigger":"say","keywords":["okolnir"],
                "guards":[{"kind":"item","item":RING,"count":1}],
                "reply":["Ohh, you got a nice ring there! Ya don't have to pay if you gimme the ring and I promise you I will bring you to the correct spot!*HICKS* Alright?"],
                "effects":[{"kind":"set_topic","value":1}],"sources":[SOURCE]
            },
            {
                "key":"oteryn:quest-npc-rule.buddel.take-ring-okolnir","quest":QUEST,"npc":NPC,
                "trigger":"say","keywords":["yes"],"topics":[1],
                "guards":[{"kind":"item","item":RING,"count":1}],
                "effects":[
                    {"kind":"take_item","item":RING,"count":1},
                    {"kind":"add","track":EXCHANGES,"value":1},
                    {"kind":"teleport","position":OKOLNIR},
                    {"kind":"magic_effect","effect":"CONST_ME_TELEPORT"},
                    {"kind":"set_topic","value":0}
                ],"sources":[SOURCE]
            }
        ]
    });
    NpcRuleCatalog::from_json_documents(&[&program.to_string()])
        .map_err(debug)?
        .attach_fixture(native.clone())
        .map_err(Into::into)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    at: Position,
) -> TestResult {
    let [buddel] = classic_quests::install(owner, admitted, [NPC]).await?;
    let mut route = super::qcx62_buddel_fixtures::route_fixture(
        "okolnir",
        OKOLNIR,
        "It's nice there. Except of the ice dragons which are not very companionable.",
    );
    let policy = route.conditional.as_mut().ok_or("Okolnir policy")?;
    policy.declined_exchange = Some(DeclinedExchange {
        topic: 1,
        reply: "Give me 50 gold and I bring you to okolnir. 'kay?".into(),
        progress: Vec::new(),
    });
    policy.cancel_reply = Some("You shouldn't miss the experience.".into());
    let mut book_entry = NpcTalkEntry::new("Buddel", None);
    book_entry.travel_routes.push(route);
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::default().with(NPC, book_entry),
    ));

    classic_quests::talk(owner, admitted, buddel, 20200, "hi").await?;
    classic_quests::talk(owner, admitted, buddel, 20201, "okolnir").await?;
    classic_quests::talk(owner, admitted, buddel, 20202, "yes").await?;
    assert_eq!(travel(h).await?, 0);

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for nonce in [993, 994] {
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                item: RING.into(),
                count: 1,
                position: at,
            },
            nonce,
            true,
        )
        .await?;
    }
    let offer = classic_quests::talk(owner, admitted, buddel, 20203, "okolnir").await?;
    assert!(offer.iter().any(|line| line.contains("nice ring")));
    let paid_offer = classic_quests::talk(owner, admitted, buddel, 20204, "no").await?;
    assert!(paid_offer.iter().any(|line| line.contains("50 gold")));
    let cancelled = classic_quests::talk(owner, admitted, buddel, 20205, "no").await?;
    assert!(cancelled.iter().any(|line| line.contains("shouldn't miss")));
    assert_eq!(travel(h).await?, 0);
    let offer = classic_quests::talk(owner, admitted, buddel, 20206, "okolnir").await?;
    assert!(offer.iter().any(|line| line.contains("nice ring")));
    classic_quests::talk(owner, admitted, buddel, 20207, "yes").await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let landed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        landed.position(),
        super::quest_world_callbacks_fixtures::local(OKOLNIR)
    );
    assert_eq!(travel(h).await?, 1);
    let rings: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(RING).fetch_one(&h.pool).await?;
    assert_eq!(rings, 1);
    let fare: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_newhaven_travel_receipts WHERE fare_gold_units>0",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(fare, 0);
    Ok(())
}

async fn travel(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
