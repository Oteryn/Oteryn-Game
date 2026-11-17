//! Generic locked NPC level admission, with the selected Paradox start as its first consumer.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn l6_npc_minimum_level_renamed_stripped_binding_cannot_use_generic_writer() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED l6_npc_minimum_level_shadow");
    super::qengine_rewards::scenario("qengine-rewards-questpc4-paradox-shadow")
}

#[test]
fn l6_npc_minimum_level_uses_locked_progression_and_bound_terms() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED l6_npc_minimum_level");
    super::qengine_rewards::scenario("qengine-rewards-questpc4-paradox-level")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_paradox version={version}");
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let accepted = callbacks
        .iter()
        .find(|callback| callback.key == "oteryn:quest-reward/l6-paradox-oldrak")
        .ok_or("missing selected Paradox Oldrak acceptance callback")?;
    assert_eq!(accepted.quest, "oteryn:quest.the_paradox_tower_quest");
    assert_eq!(accepted.minimum_level, Some(30));
    let [npc] = super::classic_quests::install(owner, admitted, [accepted.npc.as_str()]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state")
        .fetch_one(&h.pool)
        .await?;
    assert!(level < 30, "low-level fixture: {level}");
    let before = counts(h).await?;
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, accepted, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        before,
        "below-level request writes nothing"
    );
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone();
    let transition = canonical.transition(&accepted.key).ok_or("transition")?;
    let mut alias = transition.clone();
    alias.key = "oteryn:quest-transition/fixture-level-renamed".into();
    if name.ends_with("-shadow") {
        for effect in &mut alias.effects {
            if let oteryn_gameplay_quests::quest::QuestEffectKind::SetWithBinding {
                value, ..
            } = effect.effect
            {
                effect.effect = oteryn_gameplay_quests::quest::QuestEffectKind::Set(value);
            }
        }
    }

    let catalogue = oteryn_gameplay_quests::quest::QuestStateCatalogue::new(
        canonical.content_revision(),
        transition
            .effects
            .iter()
            .map(|e| canonical.track(&e.track).cloned().ok_or("bound track"))
            .collect::<Result<Vec<_>, _>>()?,
        vec![transition.clone(), alias.clone()],
    )
    .map_err(debug)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let bypass = slot
        .commit_quest_transition(
            owner.root,
            authority,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: alias.key,
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(9000).map_err(debug)?,
                    ),
                ),
            },
            std::sync::Arc::new(catalogue),
        )
        .await;
    assert!(
        matches!(
            bypass,
            Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
        ),
        "weaker generic writer result: {bypass:?}"
    );
    drop(slot);
    assert_eq!(
        counts(h).await?,
        before,
        "renaming a bound effect cannot bypass admission"
    );
    raise_level(owner, admitted, authority, h, 30).await?;
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, accepted, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    assert_eq!(
        super::classic_quests::progress(
            owner,
            admitted,
            authority,
            "oteryn:quest-progress/l6/paradox/story"
        )
        .await?,
        1
    );
    if name.ends_with("-level") || name.ends_with("-shadow") {
        return Ok(());
    }
    let route = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    route
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/l6/paradox-climb-complete")
        .ok_or("missing selected Paradox complete climb binding")?;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(r)) FROM game_character_roots r),'quests',(SELECT jsonb_agg(to_jsonb(q)) FROM game_character_quest_states q),'tracks',(SELECT jsonb_agg(to_jsonb(t)) FROM game_character_quest_tracks t),'receipts',(SELECT count(*) FROM game_character_quest_receipts),'sources',(SELECT count(*) FROM game_quest_reward_sources),'mints',(SELECT count(*) FROM game_reward_claim_mint_receipts),'outfits',(SELECT count(*) FROM game_account_appearance_unlock_receipts))").fetch_one(&h.pool).await?)
}

/// Arrange the level prerequisite through the existing typed XP/revision owner.
/// The bounded fixture curve starts at its stored level/XP; no guard is disabled.
pub(super) async fn raise_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    h: &Harness,
    target: u32,
) -> TestResult {
    use crate::domain::progression::{
        FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    use sqlx::Row;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let row = sqlx::query(
        "SELECT * FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let level = u32::try_from(row.try_get::<i64, _>("level")?)?;
    let xp: i64 = row.try_get("total_experience")?;
    assert!(level < target && target - level < 128);
    let context = ProgressionRevisionContext {
        profile: row.try_get("profile_revision")?,
        ruleset: row.try_get("ruleset_revision")?,
        content: row.try_get("content_revision")?,
        simulation: row.try_get("simulation_revision")?,
        evidence: row.try_get("evidence_revision")?,
        declaration: row.try_get("declaration_revision")?,
    };
    let mut thresholds = Vec::new();
    for offset in 0..128_u32 {
        thresholds.push(LevelThreshold {
            level: level.checked_add(offset).ok_or("level overflow")?,
            minimum_experience: ExactI64::new(
                xp.checked_add(i64::from(offset) * 1000)
                    .ok_or("XP overflow")?,
            ),
        });
    }
    let policy: FiniteProgressionPolicy<String, 128> = FiniteProgressionPolicy {
        context: context.clone(),
        policy_revision: row.try_get("policy_revision")?,
        reward_revision: row.try_get("reward_revision")?,
        death_policy_revision: "fixture-level-death-v1".into(),
        declared_difference_revision: context.declaration.clone(),
        thresholds: thresholds.try_into().map_err(|_| "threshold count")?,
        terminal_exclusive_experience: ExactI64::new(xp.checked_add(128000).ok_or("XP terminal")?),
        death_loss_numerator: 1,
        death_loss_denominator: 10,
        death_loss_rounding: RoundingMode::Floor,
    };
    let request = ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(super::fixture::id(196))
            .map_err(debug)?,
        amount: ExactI64::new(i64::from(target - level) * 1000),
        context,
        policy_revision: policy.policy_revision.clone(),
        reward_revision: policy.reward_revision.clone(),
        policy,
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_experience(owner.root, authority, owner.holder, fence, request, None)
        .await
        .map_err(debug)?;
    assert!(
        matches!(result, ExperienceCommitOutcome::Committed(_)),
        "typed level prerequisite: {result:?}"
    );
    let now:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(now, i64::from(target));
    Ok(())
}
