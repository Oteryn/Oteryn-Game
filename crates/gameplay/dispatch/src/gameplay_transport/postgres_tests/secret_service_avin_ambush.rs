//! Crystal 201189f8, data-global/npc/gamel.lua:59-74.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::NpcRuleCatalog,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::{
    npc_service::{NpcIntentDisposition, NpcTalkIntent},
    world_spatial_entities::EntityRef,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "secret-service-avin-ambush";
const QUEST: &str = "oteryn:quest.secret_service_quest";
const GAMEL: &str = "oteryn:npc.gamel";
const BANDIT: &str = "oteryn:creature.bandit";
const LETTER: &str = "oteryn:item.tibia.i402";
const TRACK: &str = "oteryn:quest-progress/avin-fixture/mission";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gamel.lua:59-74";
const AT: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};

#[test]
fn avin_greeting_requires_the_letter_and_spawns_both_bandits_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, AT, false)
    })
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let program = json!({"schema":1,"quest":QUEST,"tracks":[{
        "key":TRACK,"quest":QUEST,"initial":1,"min":1,"max":2,"sources":[SOURCE]
    }],"rules":[{
        "key":"oteryn:quest-npc/avin-fixture/ambush","quest":QUEST,"npc":GAMEL,"trigger":"greet",
        "guards":[{"kind":"storage","track":TRACK,"op":"eq","value":1},{"kind":"item","item":LETTER,"count":1}],
        "effects":[{"kind":"set","track":TRACK,"value":2},{"kind":"spawn_creatures","creatures":[{"creature":BANDIT,"count":2}]},{"kind":"set_topic","value":0}],
        "reply":["Guards, help me!"],"sources":[SOURCE]
    },{
        "key":"oteryn:quest-npc/avin-fixture/greet","quest":QUEST,"npc":GAMEL,"trigger":"greet",
        "effects":[],"reply":["Keep your voice down."],"sources":[SOURCE]
    }]});
    let mut invalid = program.clone();
    invalid["rules"][0]["effects"]
        .as_array_mut()
        .ok_or("ambush effects")?
        .push(json!({"kind":"teleport","position":AT}));
    assert!(NpcRuleCatalog::from_json_documents(&[&invalid.to_string()]).is_err());
    let source = NpcRuleCatalog::from_json_documents(&[&program.to_string()]).map_err(debug)?;
    let digest: String = source.programs()[0]
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 1,
            min: 1,
            max: 2,
        }],
        vec![QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(debug)?
    .with_fixture_npc_rules(source))
}

async fn greet(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    Ok(owner
        .npc_talk(
            admitted.runtime_actor.ok_or("player")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "hi".into(),
            },
        )
        .await)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = super::classic_quests::install(owner, admitted, [GAMEL]).await?;
    let greeting = greet(owner, admitted, npc, 9550).await?;
    assert_eq!(greeting.disposition, NpcIntentDisposition::Ok);
    assert_eq!(
        greeting.conversation.ok_or("greeting")?.lines,
        ["Keep your voice down."]
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            item: LETTER.into(),
            position: AT,
            count: 1,
        },
        4650,
        true,
    )
    .await?;
    let blockers = {
        let mut runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(admitted.runtime_actor.ok_or("player")?)
            .map_err(debug)?
            .position();
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        [7, 8]
            .into_iter()
            .map(|dx| {
                runtime
                    .admit_encounter_actor(
                        "oteryn:creature.rat",
                        20,
                        MovementLocalPosition { x: at.x + dx, ..at },
                    )
                    .map_err(debug)
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let refusal = greet(owner, admitted, npc, 9551).await?;
    assert_eq!(refusal.disposition, NpcIntentDisposition::Rejected);
    let stored = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),1)",
        )
        .bind(TRACK)
        .fetch_one(&h.pool)
    };
    assert_eq!(stored().await?, 1);
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime.current_live_creature_census().map_err(debug)?.len(),
            blockers.len()
        );
        for blocker in blockers {
            runtime.remove_encounter_actor(blocker).map_err(debug)?;
        }
    }
    let ambush = greet(owner, admitted, npc, 9552).await?;
    assert_eq!(ambush.disposition, NpcIntentDisposition::Ok);
    assert_eq!(
        ambush.conversation.ok_or("ambush")?.lines,
        ["Guards, help me!"]
    );
    assert_eq!(stored().await?, 2);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 402).await?, 1);
    let retry = greet(owner, admitted, npc, 9553).await?;
    assert_eq!(retry.disposition, NpcIntentDisposition::Ok);
    assert_eq!(
        retry.conversation.ok_or("repeat greeting")?.lines,
        ["Keep your voice down."]
    );
    let runtime = owner.runtime.lock().await;
    let bandits = runtime.current_live_creature_census().map_err(debug)?;
    assert_eq!(bandits.len(), 2);
    for bandit in bandits {
        assert_eq!(
            runtime.creature_spell_target_atom(bandit).map_err(debug)?,
            BANDIT
        );
    }
    Ok(())
}
