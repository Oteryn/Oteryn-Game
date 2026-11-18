//! Crystal 201189f8 data-global/npc/scutty.lua:80-87,114-121.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::authored_bindings::{
    Bindings, BoundQuest, Stage, Track, Trigger,
};
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, isle_of_evil};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-scutty-debt";
const NPC: &str = "oteryn:npc.scutty";
const DEBT: &str = "oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/mission04-mustpay";
const WORMS: &str = "oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/mission04-wormcount";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/scutty.lua:114-121";
const PLATINUM: u16 = 3035;

#[test]
fn scutty_confirmation_charges_carried_gold_and_clears_debt_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario(NAME)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let mission = format!("{}mission04", isle_of_evil::PREFIX);
    let callback: RewardCallback = serde_json::from_value(json!({
        "key": "oteryn:quest-reward/isle-of-evil/scutty-debt",
        "quest": isle_of_evil::QUEST,
        "npc": NPC,
        "keywords": ["yes"],
        "topic": 3,
        "next_topic": 3,
        "storage": [
            {"track":mission,"minimum":2,"maximum":2,"set":2,"preserve":true},
            {"track":DEBT,"minimum":1,"maximum":1,"set":0},
            {"track":WORMS,"minimum":0,"maximum":5,"set":0}
        ],
        "take": [],
        "fee": {"gold":500,"include_bank":false},
        "reply": ["There we go. Now get back out there and be more careful with my drilling worms this time!"],
        "failure_reply": ["You don't even have 500 gold on you..."],
        "sources": [SOURCE]
    }))?;
    let base = QuestStateCatalogue::empty(
        owner
            .quest_catalogue
            .as_ref()
            .ok_or("catalogue")?
            .content_revision(),
    )
    .ok_or("empty catalogue")?;
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: isle_of_evil::QUEST.into(),
            recipe_sha256: Sha256::digest(serde_json::to_vec(&callback)?)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            occurrence: None,
            admission: None,
            tracks: [(mission.as_str(), 5), (DEBT, 1), (WORMS, 5)]
                .into_iter()
                .map(|(key, maximum)| Track {
                    key: key.into(),
                    initial: 0,
                    minimum: 0,
                    maximum,
                })
                .collect(),
            stages: vec![Stage {
                key: "scutty-debt".into(),
                completes: false,
                trigger: Trigger::NpcDialogue {
                    callback: Box::new(callback.clone()),
                    alternatives: Vec::new(),
                },
            }],
            sources: vec![SOURCE.into()],
        }],
    };
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(mission, 2), (DEBT.into(), 1), (WORMS.into(), 4)],
        124,
    )
    .await?;
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    let mut ordinal = 51100;
    super::qengine_voucher_stories::mint(owner, admitted, PLATINUM, 4, &mut ordinal).await?;
    let counts = |expected_coins, expected_fees| async move {
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, PLATINUM).await?,
            expected_coins
        );
        let fees: (i64, i64) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(fee_gold_units),0)::bigint FROM game_quest_reward_fees WHERE transition_key='oteryn:quest-reward/isle-of-evil/scutty-debt'",
        ).fetch_one(&h.pool).await?;
        assert_eq!(fees, expected_fees);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (refused, _, _) = super::qengine_rewards::wire_conversation(
        owner,
        admitted,
        npc,
        &callback,
        false,
        vec!["hi", "mission", "yes"],
    )
    .await?;
    assert_eq!(refused, NpcIntentDisposition::Rejected);
    for (track, value) in [(DEBT, 1), (WORMS, 4)] {
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, track).await?,
            value
        );
    }
    counts(4, (0, 0)).await?;
    super::qengine_voucher_stories::mint(owner, admitted, PLATINUM, 2, &mut ordinal).await?;
    let (paid, _, command) = super::qengine_rewards::wire_conversation(
        owner,
        admitted,
        npc,
        &callback,
        false,
        vec!["hi", "mission", "yes"],
    )
    .await?;
    assert_eq!(paid, NpcIntentDisposition::Ok);
    for track in [DEBT, WORMS] {
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, track).await?,
            0
        );
    }
    counts(1, (1, 500)).await?;
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        7,
        encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: "yes".into(),
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    counts(1, (1, 500)).await
}
