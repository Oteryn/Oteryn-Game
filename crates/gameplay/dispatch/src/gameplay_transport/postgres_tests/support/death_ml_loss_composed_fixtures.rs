use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::content::ContentActivationController;
use crate::content::native_gameplay::NativeGameplayInput;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
    DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceCommitOutcome,
    ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::foundation::{CharacterId, CharacterLease, GameSessionId, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::actor_spell::TrainingSave;
use crate::spell::cast::CharacterCastFacts;
use crate::spell::mana_training::RetainedTrainingCheckpoint;
use oteryn_gameplay_progression::hit_training::SkillFamily;
use oteryn_simulation_determinism::ExactI64;

/// The debug-build death settle chain is deeper than the default test stack (as in the
/// kill-reward end-to-end case).
pub(super) const SCENARIO_STACK_BYTES: usize = 64 << 20;
/// The production experience table's level 20 floor.
const LEVEL_20_EXPERIENCE: u64 = 98_800;

/// Which death flush a scenario composes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    /// The flush saves the pending try under the current fence.
    Clean,
    /// The flush's checkpoint committed, but under a fence the session has since moved past.
    CommittedFenced,
    /// The flush's checkpoint is refused before any write, and the actor-end release settles.
    ReleaseRejected,
    /// The flush's checkpoint is refused before any write, and the live cadence respawns.
    LiveRejected,
}

/// The production manifest's generation, activated for `world`, with its training section
/// interpreted under `content` (the Character root's content revision).
fn production_generation(
    world: WorldId,
    content: &str,
) -> TestResult<ContentActivationController> {
    production_generation_with_pin(world, content).map(|(controller, _)| controller)
}

pub(super) fn production_generation_with_pin(
    world: WorldId,
    content: &str,
) -> TestResult<(
    ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    production_generation_with_source(world, content, None)
}

pub(super) fn production_generation_with_source(
    world: WorldId,
    content: &str,
    source: Option<&[u8]>,
) -> TestResult<(
    ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    let supplied = production_input_with_source(content, source)?;
    crate::content::native_gameplay::test_support::try_activate_input(world, &supplied)
        .map_err(debug)
}

/// Keep initial and successor generations on the same fixture revision.
pub(super) fn production_input_with_source(
    content: &str,
    source: Option<&[u8]>,
) -> TestResult<NativeGameplayInput> {
    use sha2::Digest;
    let manifest = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server"
    ))
    .join("../../content/spells.manifest.json");
    let mut supplied = NativeGameplayInput::from_manifest(&manifest).map_err(debug)?;
    supplied.source_world =
        source.map(
            |bytes| crate::content::native_gameplay::PinnedGameplayBytes {
                bytes: bytes.to_vec(),
                sha256: sha2::Sha256::digest(bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
            },
        );
    supplied
        .build_training
        .as_mut()
        .ok_or("the manifest pins build training")?
        .content_revision = content.into();
    {
        let cosmetics = &mut supplied.cosmetics;
        let mut packet: serde_json::Value = serde_json::from_slice(&cosmetics.bytes)?;
        if packet["availability"] == "authored" {
            // The fixture rebinds the Character content revision above;
            // its admitted appearance packet must use that same revision.
            packet["authoring"]["document"]["content_revision"] = content.into();
            cosmetics.bytes = serde_json::to_vec(&packet)?;
            cosmetics.sha256 = sha2::Sha256::digest(&cosmetics.bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
        }
    }
    Ok(supplied)
}

pub(super) async fn current_fence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<CurrentCharacterGameplayFence> {
    // Setup needs the same bounded holder recovery as fresh admission:
    // a completed durable pass can retire its holder before the next read.
    let budget =
        crate::gameplay_transport::AdmissionRetryBudget::for_root(&session, owner.root);
    Ok(owner
        .current_quest_fence_within(&budget, session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("terminal session")?)
}

/// The durable build row, or the death receipt's `before`/`after` columns, as a build.
async fn build_row(h: &Harness, sql: &str) -> TestResult<DurableBuildState> {
    let row: (String, i32, i64, Vec<i32>, Vec<i64>) =
        sqlx::query_as(sqlx::AssertSqlSafe(sql.to_owned()))
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
    let mut skills = [(0, 0); 7];
    for (skill, (level, tries)) in skills.iter_mut().zip(row.3.iter().zip(&row.4)) {
        *skill = (u16::try_from(*level)?, u64::try_from(*tries)?);
    }
    DurableBuildState::new(
        row.0,
        (u16::try_from(row.1)?, u64::try_from(row.2)?),
        skills,
    )
    .map_err(debug)
}

fn columns(suffix: &str) -> String {
    let families = [
        "fist",
        "club",
        "sword",
        "axe",
        "distance",
        "shielding",
        "fishing",
    ];
    let levels: Vec<String> = families
        .iter()
        .map(|family| format!("{family}_level{suffix}"))
        .collect();
    let tries: Vec<String> = families
        .iter()
        .map(|family| format!("{family}_tries{suffix}"))
        .collect();
    format!(
        "vocation, magic_level{suffix}, mana_spent{suffix}, ARRAY[{}], ARRAY[{}]",
        levels.join(","),
        tries.join(",")
    )
}

/// The Character's death receipts.
async fn death_receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_death_receipts \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

#[allow(
    clippy::too_many_lines,
    reason = "one end-to-end scenario: setup, the lethal hit, the respawn and its checks"
)]
pub(super) fn scenario(case: Case) -> TestResult {
    run(async move |admin| {
        let name = match case {
            Case::Clean => "death_ml_composed",
            Case::CommittedFenced => "death_ml_fenced",
            Case::ReleaseRejected => "death_ml_rejected",
            Case::LiveRejected => "death_ml_live_rejected",
        };
        let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
        let controller = production_generation(dependencies.admitted.world_id, "content-1")?;
        let active = controller.active().ok_or("activated generation")?;
        let formula = active
            .native_gameplay()
            .and_then(|native| native.training_formula())
            .ok_or("the generation's training formula")?;
        let table: &dyn BuildFormula = formula;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = Some(active);

        // ARCH-PROGRESSION-SOURCE-0 §1.5: the production admission step binds (and
        // initializes) the session's progression from the Character root's revisions.
        let mut admitted = dependencies.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let progression = owner
            .progression_binding(session)
            .ok_or("the session binds the generation's progression")?;

        // Setup through the durable root under the session's current fences: the druid
        // choice, level 20 experience and a trained build (magic 3 and sword 14, each half-way
        // to its next level).
        let occurrence = |tag| BuildOccurrence::from_bytes(id(tag)).map_err(debug);
        let druid =
            convert_vocation(table, &DurableBuildState::default(), "druid").map_err(debug)?;
        let chosen = harness
            .root
            .commit_character_build(
                &authority,
                &harness.node,
                current_fence(&owner, session).await?,
                BuildChangeRequest {
                    occurrence: occurrence(90)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                table,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
        let award = harness
            .root
            .commit_character_experience(
                &authority,
                &harness.node,
                current_fence(&owner, session).await?,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91))
                        .map_err(debug)?,
                    amount: ExactI64::new(i64::try_from(LEVEL_20_EXPERIENCE)?),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));
        let half = |family, level| {
            table
                .required("druid", family, level)
                .map(|required| required / 2)
                .ok_or("requirement")
        };
        let mut skills = druid.skills();
        skills[2] = (14, half(3, 15)?);
        let trained =
            DurableBuildState::new("druid", (3, half(0, 4)?), skills).map_err(debug)?;
        let BuildCommitOutcome::Committed(_) = harness
            .root
            .commit_character_build(
                &authority,
                &harness.node,
                current_fence(&owner, session).await?,
                BuildChangeRequest {
                    occurrence: occurrence(92)?,
                    cause: BuildCause::Training,
                    before: druid.clone(),
                    after: trained.clone(),
                    pruned_stance: None,
                },
                table,
            )
            .await
            .map_err(debug)?
        else {
            return Err("trained build replayed".into());
        };
        // The owner's revision writer starts after the setup writes, as after an admission.
        // W02 shares the sequencer across native scope views; this fixture still resets it.
        owner.revision_sequencer = CharacterRevisionSequencer::new().into();
        // The fence a checkpoint prepared before the session's lease moved retains.
        let stale = CurrentCharacterGameplayFence {
            character_lease_generation: 1,
            ..current_fence(&owner, session).await?
        };
        let mut retained_request = None;

        // The admitted actor, as first entry leaves it: placed, its Character lease bound,
        // its spell state loaded from the durable build with training enabled.
        {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        CharacterCastFacts {
                            level: 20,
                            magic_level: u32::from(trained.magic().0),
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (0, 0),
                        owner.owner_now(),
                    )
                    .is_some()
            );
            assert!(states.load_owned_build(&runtime, actor, session, &trained));
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("live actor")?;
            state
                .enable_owned_training(&trained, formula)
                .map_err(debug)?;
            // One sword try the cadence has not saved yet: the death flush must save it.
            state
                .train_skill(SkillFamily::Sword, owner.owner_now().get(), formula, |_| 1)
                .map_err(debug)?;
            assert!(state.has_training_to_flush());
        }

        // A real lethal creature hit records the death.
        let death = {
            let mut runtime = dependencies.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                owner.owner_now().get(),
            );
            let hit = states
                .apply_creature_damage(&mut runtime, actor, session, 100_000, now)
                .ok_or("the hit lands")?;
            assert_eq!(hit.damage.health_after, 0);
            states
                .player_death(&runtime, actor, session)
                .ok_or("the lethal hit records the death")?
        };
        assert!(owner.states.spell_states.lock().await.is_dead(actor));
        if case != Case::Clean {
            // The death flush's own checkpoint for the try, retained under the stale fence
            // (a retained checkpoint refuses creature hits, so it follows the death).
            let runtime = dependencies.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("dead actor")?;
            let prepared = state
                .prepare_training_checkpoint(
                    owner.owner_now().get(),
                    formula,
                    occurrence(93)?,
                    true,
                )
                .map_err(debug)?
                .ok_or("a checkpoint for the try")?;
            if case == Case::CommittedFenced {
                retained_request = Some(prepared.request.clone());
            }
            state
                .retain_training_checkpoint(RetainedTrainingCheckpoint {
                    fence: stale,
                    prepared,
                    formula_digest: formula.digest(),
                })
                .map_err(debug)?;
        }
        if let Some(request) = retained_request {
            // Its write committed before the post-write authority check saw the moved fence.
            let committed = harness
                .root
                .commit_character_build(
                    &authority,
                    &harness.node,
                    current_fence(&owner, session).await?,
                    request,
                    table,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(committed, BuildCommitOutcome::Committed(_)));
            // W02 shares the sequencer across native scope views; this fixture still resets it.
            owner.revision_sequencer = CharacterRevisionSequencer::new().into();
            // Every forced replay is fenced out and leaves the checkpoint retained.
            for _ in 0..2 {
                assert_eq!(
                    owner.save_spell_training(actor, session, true).await,
                    TrainingSave::FencedOut
                );
            }
            let runtime = dependencies.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .get_mut(&runtime, actor, session)
                    .ok_or("dead actor")?
                    .pending_training_checkpoint()
                    .is_some()
            );
        }

        // The death flush's try, durable before the death is priced.
        let mut flushed_skills = trained.skills();
        flushed_skills[2].1 += 1;
        let flushed =
            DurableBuildState::new("druid", trained.magic(), flushed_skills).map_err(debug)?;
        let durable = format!(
            "SELECT {} FROM game_character_build_state \
             WHERE character_id = encode($1,'hex')::uuid",
            columns("")
        );
        if matches!(case, Case::ReleaseRejected | Case::LiveRejected) {
            // The store refuses the stale fence before any write. The first attempt keeps the
            // actor dead with nothing committed, and re-fences the proven-uncommitted
            // checkpoint (same occurrence) to the session's current authority.
            let mut settled = None;
            if case == Case::ReleaseRejected {
                assert!(
                    !owner
                        .settle_released_death(actor, session, &mut settled)
                        .await,
                    "a release must not settle a death whose flush was refused"
                );
                assert!(settled.is_none());
            } else {
                assert!(
                    owner
                        .respawn_after_death(actor, session, death)
                        .await
                        .is_none(),
                    "a live death must not settle a death whose flush was refused"
                );
            }
            assert_eq!(death_receipts(&harness).await?, 0);
            assert_eq!(build_row(&harness, &durable).await?, trained);
            assert!(owner.states.spell_states.lock().await.is_dead(actor));
            let current = current_fence(&owner, session).await?;
            {
                let runtime = dependencies.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("dead actor")?;
                let retained = state
                    .pending_training_checkpoint()
                    .ok_or("the refused checkpoint stays retained")?;
                assert_eq!(retained.fence, current);
                assert_eq!(retained.prepared.request.occurrence, occurrence(93)?);
                assert!(state.has_training_to_flush());
            }
            if case == Case::ReleaseRejected {
                // The retried release flushes the try under the current fence, then settles
                // the death priced from it.
                assert!(
                    owner
                        .settle_released_death(actor, session, &mut settled)
                        .await,
                    "the retried release settles once the re-fenced flush is durable"
                );
                assert_eq!(settled, Some(death));
                assert_eq!(death_receipts(&harness).await?, 1);
                assert_eq!(build_row(&harness, &durable).await?, flushed);
                let (lost, payload_absent): (i64, bool) = sqlx::query_as(
                    "SELECT experience_lost, vocation IS NULL FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid"
                ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(lost, 17_200);
                assert!(payload_absent);
                drop(owner);
                drop(authority);
                drop(seal);
                return harness.cleanup().await;
            }
        }

        // The composed respawn: flush, priced death, placement, rebase and refill.
        let (_, vitals) = owner
            .respawn_after_death(actor, session, death)
            .await
            .ok_or("the death settles and the actor respawns")?;
        assert_eq!(vitals.health, vitals.max_health);
        assert!(!owner.states.spell_states.lock().await.is_dead(actor));

        // D58 / NIGHT-W1 decision 19: the death loses XP and records no
        // build change; the preceding training receipt holds the flushed try.
        let (binding, experience_before, experience_lost): (i32, i64, i64) = sqlx::query_as(
            "SELECT get_byte(command_binding, 0), experience_before, experience_lost \
             FROM game_character_death_receipts WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(binding, 1);
        assert_eq!(u64::try_from(experience_before)?, LEVEL_20_EXPERIENCE);
        let lost = 17_200;
        assert_eq!(u64::try_from(experience_lost)?, lost);
        let payload_absent: bool = sqlx::query_scalar(
            "SELECT vocation IS NULL FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid"
        ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert!(payload_absent);
        let preserved = build_row(&harness, &durable).await?;
        assert_eq!(preserved, flushed);
        assert!(preserved.magic().0 > 0 && preserved.magic().1 > 0);
        assert!(preserved.skills()[2].0 > 10 && preserved.skills()[2].1 > 0);
        let experience_after: i64 = sqlx::query_scalar(
            "SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid"
        ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(u64::try_from(experience_after)?, LEVEL_20_EXPERIENCE - lost);
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_pending_respawns \
             WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(pending, 0);

        // The live actor continues from the preserved build: one more try saves on it.
        {
            let runtime = dependencies.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("respawned actor")?;
            assert_eq!(
                state.character_facts().magic_level,
                u32::from(preserved.magic().0)
            );
            assert!(!state.has_training_to_flush());
            state
                .train_skill(SkillFamily::Sword, owner.owner_now().get(), formula, |_| 1)
                .map_err(debug)?;
        }
        assert_eq!(
            owner.save_spell_training(actor, session, true).await,
            TrainingSave::Saved
        );
        let mut next_skills = preserved.skills();
        next_skills[2].1 += 1;
        assert_eq!(
            build_row(&harness, &durable).await?,
            DurableBuildState::new("druid", preserved.magic(), next_skills).map_err(debug)?
        );

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
