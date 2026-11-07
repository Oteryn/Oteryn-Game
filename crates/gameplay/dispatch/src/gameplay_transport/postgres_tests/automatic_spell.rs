//! Automatic spells through the live level refresh, with durable replay and stale fences.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{ExperienceAwardRequest, ExperienceRewardOccurrence};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::spell::Vocation;
use oteryn_simulation_determinism::ExactI64;

#[test]
fn automatic_spell_known_spell_level_up_replay_and_fence_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("automatic-spell", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "automatic_spell", false, false).await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let session = deps.admitted.game_session_id;
            let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id, "content-1",
            )?;
            let active = controller.active().ok_or("active generation")?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let mut admitted = deps.admitted;
            Box::pin(owner.admit_progression_session(&mut admitted)).await;
            let progression = owner.progression_binding(session).ok_or("progression")?;
            let formula = active.native_gameplay().and_then(|n| n.training_formula()).ok_or("formula")?;
            let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("session")?;
            h.root.commit_character_build(&authority, &h.node, fence, BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?,
                pruned_stance: None,
            }, formula).await.map_err(debug)?;
            // Match the existing vitals fixture: it has no activated equipment catalogue.
            owner.active_generation = None;
            admit(&owner, &deps, &request).await?;
            owner.active_generation = Some(active);
            let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("session")?;
            let spell = deps.spells.spells.iter().find(|s| !s.learning_required && s.level > 1 && s.level <= 20 && s.vocations.contains(&Vocation::Druid)).ok_or("level gated druid spell")?;
            let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_known_spells").fetch_one(&h.pool).await?;
            assert_eq!(keys, 0);
            let award = ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: ExactI64::new(98_800),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            };
            let outcome = h.root.commit_character_experience(&authority, &h.node, fence, award.clone()).await.map_err(debug)?;
            owner.states.level_refresh.mark(crate::gameplay_transport::level_up_live::leveled_character(&outcome));
            Box::pin(owner.refresh_level_up_vitals(deps.actor, session)).await;
            let first: Vec<(String, String)> = sqlx::query_as("SELECT spell_key, learned_at::text FROM game_character_known_spells ORDER BY spell_key").fetch_all(&h.pool).await?;
            assert!(first.iter().any(|(key, _)| key == &spell.key));
            let CastFactsLoad::Ready { facts, character_revision } = load_character_cast_facts(&h.root, &authority, fence.character_id).await else { return Err("cast facts".into()); };
            for definition in &deps.spells.spells {
                assert_eq!(
                    first.iter().any(|(key, _)| key == &definition.key),
                    !definition.learning_required && definition.level <= facts.level
                        && definition.vocations.contains(&facts.vocation),
                    "catalogue eligibility: {}", definition.key,
                );
            }
            let replay = h.root.commit_character_experience(&authority, &h.node, fence, award).await.map_err(debug)?;
            owner.states.level_refresh.mark(crate::gameplay_transport::level_up_live::leveled_character(&replay));
            Box::pin(owner.refresh_level_up_vitals(deps.actor, session)).await;
            assert!(owner.reconcile_automatic_spells(fence.character_id, session, character_revision, facts.level, facts.vocation).await);
            // A stale snapshot or connection must never persist another grant.
            assert!(h.root.acquire_automatic_spells(&authority, &h.node, fence, &std::collections::BTreeSet::from(["stale".into()])).await.is_err());
            let mut current = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("session")?;
            current.connection_generation = crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
            assert!(h.root.acquire_automatic_spells(&authority, &h.node, current, &std::collections::BTreeSet::from(["stale-session".into()])).await.is_err());
            let after: Vec<(String, String)> = sqlx::query_as("SELECT spell_key, learned_at::text FROM game_character_known_spells ORDER BY spell_key").fetch_all(&h.pool).await?;
            assert_eq!(after, first, "replay keeps both identities and original timestamps");
            eprintln!("AUTOSPELL PostgreSQL live refresh, replay and stale fences verified");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
