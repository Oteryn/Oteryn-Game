use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
};
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-thalas-greeting";
const QUEST: &str = "oteryn:quest.the_ancient_tombs_quest";
const TRACK: &str = "oteryn:quest-track/ancient-tombs/thalas-treasure";
const REFUSAL: &str = "Begone! Hissssss! You bear not the mark of the cobra!";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/cobra.lua:48-69";

pub(super) fn thalas_greeting_requires_current_poison_and_advances_treasure_with_teleport_scenario()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32860,
                y: 31303,
                floor: -7,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let track = catalogue.track(TRACK).ok_or("Thalas treasure")?;
    // Crystal cobra.lua:52-65 advances an early track and still teleports an advanced character.
    // The fixture crops substitute two adjacent native cells for the source NPC and landing.
    let dst = serde_json::json!({"x":32859,"y":31302,"floor":-7});
    let effects = [
        serde_json::json!({"kind":"teleport","position":dst,"gate":{
            "condition":"poison", "refusal":REFUSAL, "success":"Venture the path of decay!", "origin_effect":true
        }}),
        serde_json::json!({"kind":"magic_effect","effect":"CONST_ME_TELEPORT"}),
        serde_json::json!({"kind":"end_conversation","delay_ms":0}),
    ];
    let rules: Vec<_> = [("early", "le", 1), ("advanced", "gt", 1)]
        .into_iter()
        .map(|(suffix, op, value)| {
            let mut effects = effects.to_vec();
            if suffix == "early" {
                effects.push(serde_json::json!({"kind":"set","track":TRACK,"value":2}));
            }
            serde_json::json!({
                "key":format!("oteryn:quest-npc/ancient-tombs/cobra-fixture-{suffix}"),
                "quest":QUEST, "npc":"oteryn:npc.cobra", "trigger":"greet",
                "guards":[{"kind":"storage","track":TRACK,"op":op,"value":value}],
                "effects":effects, "sources":[SOURCE]
            })
        })
        .collect();
    let source = serde_json::json!({
        "schema":1, "quest":QUEST, "program_key":"oteryn:quest-npc-program.ancient-tombs-cobra-greeting",
        "source_files":[{"path":"data-global/npc/cobra.lua","revision":"201189f871d23597b82caf88fd611454a2ddefb8","sha256":"8cd8398dd6676af4aaf652156a97cba0ae39cd198c738eb5ae8f390e18b9cad2"}],
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":track.initial,"min":track.min,"max":track.max,"sources":[SOURCE]}],
        "rules":rules
    })
    .to_string();
    let prior = catalogue
        .definition_hash(QUEST)
        .ok_or("prior quest definition")?;
    let catalogue = NpcRuleCatalog::from_json_documents(&[&source])?.attach_fixture(catalogue)?;
    assert_ne!(catalogue.definition_hash(QUEST), Some(prior));
    assert!(
        oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(&catalogue, QUEST, &prior)
            .is_some()
    );
    owner.quest_catalogue = Some(Arc::new(catalogue));
    let [cobra] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.cobra"]).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let lines = super::classic_quests_fixtures::talk(owner, admitted, cobra, 5410, "hi").await?;
    assert!(lines.iter().any(|line| line == REFUSAL));
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        before
    );

    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
        let definition = ConditionDefinition::new(
            "oteryn:test.thalas_poison",
            1,
            ConditionValues::DamageOverTime {
                element: crate::domain::condition::DotElement::Poison,
                total_min: 100,
                total_max: 100,
                per_tick: 1,
                interval_ms: 10_000,
                delayed: true,
            },
        )
        .ok_or("poison condition")?;
        state
            .conditions
            .apply(
                &definition,
                None,
                ConditionSourceKind::SelfUse,
                &[],
                &ApplicationFacts {
                    now: owner.owner_now().get(),
                    base_speed: 0,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &GameplayDecisionRoot::from_bytes([9; 32]),
                    occurrence: DecisionOccurrenceId::from_bytes(id(91)),
                },
            )
            .map_err(debug)?;
    }
    super::classic_quests_fixtures::talk(owner, admitted, cobra, 5411, "hi").await?;
    let landed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((landed.x, landed.y, landed.floor), (32859, 31302, 7));
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    Ok(())
}
