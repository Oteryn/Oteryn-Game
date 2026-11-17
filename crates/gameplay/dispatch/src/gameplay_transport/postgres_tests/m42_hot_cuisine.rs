//! M42 proves the existing cooking writer against exact native fluid custody.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::quest_state::npc_program::{NpcProgramCommit, NpcProgramRequest};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::{RewardClaimMintContent, RewardClaimMintRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    npc_rules::NpcRuleTrigger,
    world_callbacks::{Offering, Position},
};
use std::sync::Arc;

#[test]
fn m42_utc_cooking_refuses_wrong_fluid_and_commits_exact_typed_recipe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m42-hot-typed", || {
        super::quest_world_callbacks::scenario(
            "m42-hot-typed",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "M42_HOT POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn m42_final_menu_and_culinary_master_use_real_typed_receipts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m42-hot-final", || {
        super::quest_world_callbacks::scenario(
            "m42-hot-final",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "M42_HOT_FINAL POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn cook(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    catalogue: Arc<QuestStateCatalogue>,
    request: NpcProgramRequest,
) -> TestResult<NpcProgramCommit> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_npc_program(
        owner.root,
        authority,
        owner.holder,
        &owner.spell_lane_permit().await.ok_or("NPC permit")?,
        fence,
        request,
        catalogue,
        &NativeItemWeights(native).with_wheel(owner.premium_coordinator),
        &owner.premium_coordinator,
    )
    .await
    .map_err(debug)
}

async fn fluids(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    subtype: u8,
    occurrence: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let facts = |id| {
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(9000 + u64::from(subtype) + 100 * occurrence).map_err(debug)?,
        ),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: format!("fixture:reward.m42-fluid-{subtype}-{occurrence}"),
            revision_ref: "definition-r1".into(),
        },
        source_placement: format!("fixture:placement.m42-fluid-{subtype}-{occurrence}"),
        item: facts(2853)?,
        quantity: 1,
        contents: (0..2)
            .map(|_| {
                Ok(RewardClaimMintContent {
                    charges_remaining: None,
                    item: facts(2874)?,
                    quantity: 1,
                    fluid_subtype: Some(subtype),
                    key_number: None,
                    text: None,
                })
            })
            .collect::<TestResult<Vec<_>>>()?,
        additional_items: Vec::new(),
        root_text: None,
        backpack: facts(2854)?,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let weights = NativeItemWeights(native);
    let mut frozen = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request, &weights)
        .await
        .map_err(debug)?;
    h.root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut frozen,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('achievement_facts',(SELECT count(*) FROM game_account_achievements),'achievement_requests',(SELECT count(*) FROM game_account_achievement_grant_requests),'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_roots r),'states',(SELECT jsonb_agg(to_jsonb(q) ORDER BY character_id,quest_key) FROM game_character_quest_states q),'npc_receipts',(SELECT count(*) FROM game_quest_npc_program_receipts),'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'fluids',(SELECT jsonb_agg(to_jsonb(f) ORDER BY item_instance_id) FROM game_item_fluid_states f),'tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT count(*) FROM game_character_quest_receipts),'mints',(SELECT count(*) FROM game_reward_claim_mint_receipts),'sources',(SELECT count(*) FROM game_quest_npc_item_sources),'consumptions',(SELECT count(*) FROM game_quest_callback_consumptions),'audits',(SELECT count(*) FROM game_item_audit_outbox))").fetch_one(&h.pool).await?)
}

async fn exercise_recipe(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    recipe_index: u64,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M42_HOT POSTGRES_STARTED version={version}");
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .premium_gate(super::fixture::id(40))
            .await
            .premium_current(),
        "resumed cook requires delivered live Premium"
    );
    let catalogue = super::npc_item_exchanges::resumed_catalogue(owner, h).await?;
    owner.quest_catalogue = Some(catalogue.clone());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            authority,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: format!(
                    "oteryn:quest-transition/fixture/resumed-cook-{recipe_index}"
                ),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(8000).map_err(debug)?,
                )),
            },
            catalogue.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/hot-cuisine-batch3.json"
    ))?;
    let recipe = &document["recipes"][usize::try_from(recipe_index - 1)?];
    let fluid = recipe["ingredients"]
        .as_array()
        .ok_or("ingredients")?
        .iter()
        .find(|i| i.get("subtype").is_some_and(|v| !v.is_null()))
        .ok_or("fluid ingredient")?;
    assert_eq!(fluid["count"].as_u64(), Some(2));
    let subtype = u8::try_from(fluid["subtype"].as_u64().ok_or("subtype")?)?;
    let mut ordinal = 700;
    for ingredient in recipe["ingredients"].as_array().ok_or("ingredients")? {
        if ingredient.get("subtype").is_some_and(|v| !v.is_null()) {
            continue;
        }
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: ingredient["item"].as_str().ok_or("item")?.into(),
                count: u16::try_from(ingredient["count"].as_u64().ok_or("count")?)?,
            },
            ordinal,
            true,
        )
        .await?;
        ordinal += 1;
    }
    fluids(owner, admitted, h, authority, 3, 0).await?; // exact milk; this recipe requires two waters
    let request = NpcProgramRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(8100).map_err(debug)?,
        ),
        npc: "oteryn:npc.jean_pierre".into(),
        trigger: NpcRuleTrigger::Say,
        message: "yes".into(),
        topic: i32::try_from(recipe["topic"].as_i64().ok_or("topic")?)?,
        transition_key: format!(
            "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-{recipe_index}"
        ),
    };
    let before = snapshot(h).await?;
    let refused = cook(
        owner,
        admitted,
        authority,
        catalogue.clone(),
        request.clone(),
    )
    .await?;
    assert!(matches!(
        refused.transition,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(
        snapshot(h).await?,
        before,
        "wrong fluid must change no Item, receipt, progress or audit"
    );
    fluids(owner, admitted, h, authority, subtype, 0).await?;
    let before = snapshot(h).await?;
    let mut mismatched = request.clone();
    mismatched.command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(8101).map_err(debug)?,
    );
    mismatched.topic = 15;
    mismatched.transition_key =
        "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-6".into();
    let refused = cook(owner, admitted, authority, catalogue.clone(), mismatched).await?;
    assert!(matches!(
        refused.transition,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(
        snapshot(h).await?,
        before,
        "mismatched recipe must neither charge nor grant"
    );
    let committed = cook(
        owner,
        admitted,
        authority,
        catalogue.clone(),
        request.clone(),
    )
    .await?;
    assert!(
        matches!(committed.transition, QuestTransitionOutcome::Committed(_)),
        "{committed:?}"
    );
    let live:Vec<(i16,i64)>=sqlx::query_as("SELECT f.fluid_subtype,count(*) FROM game_item_fluid_states f JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2874' GROUP BY f.fluid_subtype ORDER BY f.fluid_subtype").fetch_all(&h.pool).await?;
    assert_eq!(
        live,
        [(3, 2)],
        "only the two exact waters are consumed; milk remains carried"
    );
    let quantity:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(recipe["reward"]["item"].as_str().ok_or("reward")?).fetch_one(&h.pool).await?;
    assert_eq!(quantity, 1);
    let before = snapshot(h).await?;
    let replay = cook(
        owner,
        admitted,
        authority,
        catalogue.clone(),
        request.clone(),
    )
    .await?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        snapshot(h).await?,
        before,
        "same command must not consume or mint twice"
    );
    if recipe_index == 15 {
        let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/quest_start'").fetch_one(&h.pool).await?;
        assert_eq!(stage, 2, "final sauce commits menu completion");
        let canonical: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key='oteryn:quest.hot_cuisine_quest'")
            .fetch_one(&h.pool).await?;
        assert!(
            canonical,
            "the same item/progress transaction completes the canonical quest state"
        );
        let completions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.hot_cuisine_quest' AND completes")
            .fetch_one(&h.pool).await?;
        assert_eq!(completions, 1);
    }
    for (offset, name) in ["prior", "bonus"].into_iter().enumerate() {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seeded = slot
            .commit_quest_transition(
                owner.root,
                authority,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/fixture/m42-{name}-{recipe_index}"
                    ),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(12000 + u64::try_from(offset)?).map_err(debug)?,
                    )),
                },
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    }
    for ingredient in recipe["ingredients"].as_array().ok_or("ingredients")? {
        if ingredient.get("subtype").is_some_and(|v| !v.is_null()) {
            continue;
        }
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: ingredient["item"].as_str().ok_or("item")?.into(),
                count: u16::try_from(ingredient["count"].as_u64().ok_or("count")?)?,
            },
            ordinal,
            true,
        )
        .await?;
        ordinal += 1;
    }
    fluids(owner, admitted, h, authority, subtype, 1).await?;
    let mut bonus = request.clone();
    bonus.command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(8200).map_err(debug)?,
    );
    let second = cook(owner, admitted, authority, catalogue.clone(), bonus.clone()).await?;
    assert!(
        matches!(second.transition, QuestTransitionOutcome::Committed(_)),
        "{second:?}"
    );
    if recipe_index == 15 {
        let completions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.hot_cuisine_quest' AND completes")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            completions, 1,
            "later valid cooking must not complete twice"
        );
    }
    let awards:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/culinary_master'").fetch_one(&h.pool).await?;
    assert_eq!(
        awards, 1,
        "terminal cookbook award shares the fenced cooking transaction"
    );
    let before = snapshot(h).await?;
    let repeat = cook(owner, admitted, authority, catalogue.clone(), bonus).await?;
    assert!(matches!(
        repeat.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(snapshot(h).await?, before);
    let mut exhausted = request.clone();
    exhausted.command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(8201).map_err(debug)?,
    );
    let refused = cook(owner, admitted, authority, catalogue, exhausted).await?;
    assert!(matches!(
        refused.transition,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(
        snapshot(h).await?,
        before,
        "used bonus cannot regrant or change progress"
    );
    let mut plan:serde_json::Value=sqlx::query_scalar("SELECT plan FROM game_quest_npc_item_sources WHERE transition_key=$1 ORDER BY cause_ordinal DESC LIMIT 1").bind(&request.transition_key).fetch_one(&h.pool).await?;
    let suffix = format!("/kv/cookcount-{}", recipe_index + 1);
    let counter = plan["tracks"]
        .as_array_mut()
        .ok_or("tracks")?
        .iter_mut()
        .find(|t| t["key"].as_str().is_some_and(|k| k.ends_with(&suffix)))
        .ok_or("cook counter")?;
    counter["after"] = counter["before"].clone();
    assert!(
        sqlx::query_scalar::<_, String>("SELECT game_quest_npc_reward_claim_key($1,$2)")
            .bind(plan)
            .bind(&request.transition_key)
            .fetch_one(&h.pool)
            .await
            .is_err(),
        "non-advancing occurrence cannot authorize another reward"
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    exercise_recipe(owner, admitted, h, authority, 12).await
}
pub(super) async fn exercise_final(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    exercise_recipe(owner, admitted, h, authority, 15).await
}
