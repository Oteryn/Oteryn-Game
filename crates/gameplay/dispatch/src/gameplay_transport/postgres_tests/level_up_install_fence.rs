//! A committed Character revision can advance between spell reconciliation and live installation.
use std::time::Duration;

use super::fixture::{Harness, debug, id, join_two};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::level_up_live::{install_pause, leveled_character};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::spell::cast::PlayerSpellState;

#[test]
fn level_up_vitals_postgres_defers_stale_facts_after_revision_advance() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "level_up_install_defers", false)
}

#[test]
fn level_up_vitals_postgres_retries_and_installs_latest_level_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "level_up_install_retries", true)
}

fn run(admin: String, name: &'static str, retry: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("level-up-install-fence", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(Box::pin(scenario(admin, name, retry)))
    })
}

async fn scenario(admin: String, name: &'static str, retry: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
    let result = exercise(&h, retry).await;
    result.and(h.cleanup().await)
}

async fn exercise(h: &Harness, retry: bool) -> TestResult {
    let request = prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let (controller, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active generation")?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    let mut admitted = deps.admitted;
    Box::pin(owner.admit_progression_session(&mut admitted)).await;
    let formula = active
        .native_gameplay()
        .and_then(|n| n.training_formula())
        .ok_or("formula")?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("session")?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: convert_vocation(formula, &DurableBuildState::default(), "druid")
                    .map_err(debug)?,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    // The tutorial vitals fixture does not mount the native equipment catalogue.
    owner.active_generation = None;
    admit(&owner, &deps, &request).await?;
    owner.active_generation = Some(active);
    let before = live(&owner, actor, session).await?;

    let first = award(&owner, session, 91).await?;
    assert!(first > before.character_facts().level);
    let pause = install_pause::install();
    let refresh = Box::pin(owner.refresh_level_up_vitals(actor, session));
    let advance = async {
        pause.reached.notified().await;
        let latest = award(&owner, session, 92).await;
        pause.resume.notify_one();
        latest
    };
    let (_, latest) =
        tokio::time::timeout(Duration::from_secs(30), join_two(refresh, advance)).await?;
    let latest = latest?;
    assert!(latest > first);

    if !retry {
        assert_eq!(
            live(&owner, actor, session).await?,
            before,
            "stale cast facts must leave live vitals untouched"
        );
        assert!(owner.states.level_refresh.is_marked(fence.character_id));
        return Ok(());
    }

    let CastFactsLoad::Ready { facts, .. } =
        load_character_cast_facts(&h.root, &authority, fence.character_id).await
    else {
        return Err("latest cast facts unavailable".into());
    };
    assert_eq!(facts.level, latest);
    let expected = before
        .after_level_up(facts)
        .ok_or("latest level-up vitals")?;
    Box::pin(owner.refresh_level_up_vitals(actor, session)).await;
    let installed = live(&owner, actor, session).await?;
    assert_eq!(
        installed, expected,
        "the retry installs only the latest level"
    );
    assert!(!owner.states.level_refresh.is_marked(fence.character_id));
    Box::pin(owner.refresh_level_up_vitals(actor, session)).await;
    assert_eq!(live(&owner, actor, session).await?, installed);
    Ok(())
}

async fn award(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    occurrence: u8,
) -> TestResult<u32> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("award session")?;
    let binding = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("current progression")?;
    let next = binding
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level > before.level)
        .ok_or("next level threshold")?;
    let amount = next
        .minimum_experience
        .checked_sub(before.total_experience)
        .map_err(debug)?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(occurrence))
                    .map_err(debug)?,
                amount,
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    let ExperienceCommitOutcome::Committed(receipt) = &outcome else {
        return Err("expected a new experience award".into());
    };
    assert_eq!(
        receipt.original_character_revision,
        fence.expected_character_revision
    );
    assert!(receipt.committed_character_revision > receipt.original_character_revision);
    assert_eq!(receipt.level_after, next.level);
    let leveled = leveled_character(&outcome);
    assert_eq!(leveled, Some(fence.character_id));
    owner.states.level_refresh.mark(leveled);
    Ok(receipt.level_after)
}

async fn live(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<PlayerSpellState> {
    let runtime = owner.runtime.lock().await;
    owner
        .states
        .spell_states
        .lock()
        .await
        .get(&runtime, actor, session)
        .cloned()
        .ok_or_else(|| "live spell state unavailable".into())
}
