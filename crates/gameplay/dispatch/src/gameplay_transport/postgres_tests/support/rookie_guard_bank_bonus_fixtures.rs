//! Crystal 201189f8 data-global/npc/paulie.lua:160-169.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, npc_rules::NpcRuleCatalog,
};

pub(super) const NAME: &str = "rookie-guard-bank-bonus";
const ENTRY: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const NPC: &str = "oteryn:npc.paulie";
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission08";
const BEGIN: &str = "oteryn:quest-transition/rookie-guard-bank-fixture";
const BONUS: &str = "oteryn:quest-transition/bank-deposit/rookie-guard";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/paulie.lua:160-169";

pub(super) fn run_case() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, ENTRY, false)
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let catalogue = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 2,
        }],
        vec![QuestTransition {
            key: BEGIN.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from: QuestComparison::Eq(-1),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(debug)?;
    let program = serde_json::json!({
        "schema":1, "quest":QUEST, "rules":[],
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":-1,"min":-1,"max":2,"sources":[SOURCE]}],
        "bank_deposit_bonus":{"key":BONUS,"npc":NPC,"track":TRACK,"before":1,"after":2,"gold":50,
            "reply":"Alright, we have added the amount of {amount} +{bonus} gold to your {balance} - that is the money you deposited plus a bonus of {bonus} gold. Thank you! You can withdraw your money anytime.",
            "sources":[SOURCE]}
    }).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        NpcRuleCatalog::from_json_documents(&[&program])?.bind_fixture(catalogue)?,
    ));
    super::qengine_outfit_stories_fixtures::commit_fixture(owner, admitted, BEGIN, 6470).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    let talk = super::classic_quests_fixtures::talk;
    talk(owner, admitted, npc, 6471, "hi").await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current quest fence")?
        .ok_or("current quest fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: ENTRY,
            item: "oteryn:item.tibia.i3031".into(),
            count: 20,
        },
        6472,
        true,
    )
    .await?;

    talk(owner, admitted, npc, 6473, "deposit 21").await?;
    let refused = talk(owner, admitted, npc, 6474, "yes").await?;
    assert!(
        refused
            .iter()
            .any(|line| line.to_ascii_lowercase().contains("enough"))
    );
    assert_eq!(snapshot(h).await?, (1, 0, 0));

    talk(owner, admitted, npc, 6475, "deposit 10").await?;
    let accepted = talk(owner, admitted, npc, 6476, "yes").await?;
    assert!(accepted.iter().any(|line| line.contains("10 +50 gold")));
    assert_eq!(snapshot(h).await?, (2, 60, 1));
    let replay = owner
        .resume_npc_bank(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 6476,
                item_fence: Some(fence.item_fence()),
            },
        )
        .await
        .ok_or("retained deposit")?;
    assert_eq!(
        replay.conversation.ok_or("historical reply")?.lines,
        accepted
    );
    assert_eq!(snapshot(h).await?, (2, 60, 1));

    talk(owner, admitted, npc, 6477, "deposit 5").await?;
    let ordinary = talk(owner, admitted, npc, 6478, "yes").await?;
    assert!(ordinary.iter().all(|line| !line.contains("+50")));
    assert_eq!(snapshot(h).await?, (2, 65, 1));
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1), \
                (SELECT coalesce(sum(balance),0)::bigint FROM game_account_bank_balances), \
                (SELECT count(*) FROM game_account_bank_entries WHERE quest_deposit_transaction_id IS NOT NULL)",
    ).bind(TRACK).fetch_one(&h.pool).await?)
}
