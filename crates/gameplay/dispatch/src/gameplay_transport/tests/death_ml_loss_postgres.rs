use crate::bestiary_postgres_harness::{self as harness, Harness, TestResult, debug, fence, id};
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
    DurableBuildState, convert_vocation,
};
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::foundation::{ChannelId, WorldId};
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
use crate::spell::mana_training::CompiledTrainingFormula;
use oteryn_gameplay_progression::hit_training::SkillFamily;
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

pub(super) fn death(tag: u8) -> TestResult<CharacterDeathRequest<50>> {
    let thresholds = std::array::from_fn(|index| LevelThreshold {
        level: u32::try_from(index + 1).unwrap_or(u32::MAX),
        minimum_experience: ExactI64::new(i64::try_from(index).unwrap_or(i64::MAX) * 20),
    });
    let context = harness::context();
    Ok(CharacterDeathRequest {
        pvp_contributors: Vec::new(),
        occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
        policy: FiniteProgressionPolicy {
            context: context.clone(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(1_000_000),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
        context,
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        held_blessings: vec![],
        death_cell: DeathCell {
            world_id: WorldId::decode(&id(harness::WORLD)).map_err(debug)?,
            channel_id: ChannelId::decode(&id(harness::CHANNEL)).map_err(debug)?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
    })
}

#[test]
fn train_preserving_death_respawn_then_train_save_commits() -> TestResult {
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, "death_ml_rebase", true).await?;
        let result = train_die_respawn_train(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn train_die_respawn_train(h: &Harness) -> TestResult {
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        &harness::context().content,
    )
    .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let occurrence = |tag| BuildOccurrence::from_bytes(id(tag)).map_err(debug);
    // The vocation choice the actor trains under: revision 1 to 2.
    let druid =
        convert_vocation(&formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    let chosen = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fence(1)?,
            BuildChangeRequest {
                occurrence: occurrence(80)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: druid.clone(),
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
    let mut actor = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: crate::spell::Vocation::Druid,
            level: 8,
            magic_level: u32::from(druid.magic().0),
            max_health: 185,
            max_mana: 90,
            max_soul: 100,
        },
        0,
        0,
    )
    .ok_or("actor")?;
    actor
        .load_owned_build(&druid)
        .map_err(|_| "load_owned_build")?;
    actor
        .enable_owned_training(&druid, &formula)
        .map_err(debug)?;

    // Train: the 50 druid sword tries reach sword 11, saved at revision 2 to 3.
    assert!(
        actor
            .train_skill(SkillFamily::Sword, 1, &formula, |_| 50)
            .map_err(debug)?
            .checkpoint_due
    );
    let prepared = actor
        .prepare_training_checkpoint(2, &formula, occurrence(81)?, true)
        .map_err(debug)?
        .ok_or("no training to save")?;
    let BuildCommitOutcome::Committed(receipt) = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fence(2)?,
            prepared.request.clone(),
            &formula,
        )
        .await
        .map_err(debug)?
    else {
        return Err("training save replayed".into());
    };
    let trained = receipt.after.clone();
    actor
        .commit_training_checkpoint(prepared, &receipt)
        .map_err(debug)?;
    assert_eq!(trained.skills()[2].0, 11);

    // A death that preserves the build: revision 3 to 4, then its respawn is consumed.
    let request = death(82)?;
    let committed = h
        .root
        .settle_runtime_player_death(
            &CharacterRevisionSequencer::new(),
            &authority,
            &h.node,
            fence(3)?,
            request.clone(),
            Some(&formula as &dyn BuildFormula),
            None,
        )
        .await
        .map_err(debug)?;
    assert!(committed.build_loss.is_none());
    assert_eq!(committed.experience_lost.get(), 1_000);
    assert_eq!(
        h.root
            .read_character_build_state(&authority, fence(4)?.character_id)
            .await
            .map_err(debug)?,
        trained
    );
    h.root
        .consume_pending_respawn(&authority, &h.node, fence(4)?, Some(request.occurrence))
        .await
        .map_err(debug)?
        .ok_or("missing pending respawn")?;
    assert_eq!(h.root_revision().await?, "4");

    // A checkpoint based on a different build remains refused, writing nothing.
    let mut stale = actor.clone();
    stale
        .train_skill(SkillFamily::Sword, 3, &formula, |_| 1)
        .map_err(debug)?;
    let mut refused = stale
        .prepare_training_checkpoint(4, &formula, occurrence(83)?, true)
        .map_err(debug)?
        .ok_or("no stale training to save")?;
    assert_eq!(refused.request.before, trained);
    refused.request.before = druid;
    assert!(
        h.root
            .commit_character_build(&authority, &h.node, fence(4)?, refused.request, &formula)
            .await
            .is_err()
    );
    assert_eq!(h.root_revision().await?, "4");

    // A respawn rebase on the unchanged build still permits the next training save.
    actor.rebase_after_death(&trained);
    assert_eq!(
        actor.character_facts().magic_level,
        u32::from(trained.magic().0)
    );
    assert!(!actor.has_training_to_flush());
    actor
        .train_skill(SkillFamily::Sword, 3, &formula, |_| 1)
        .map_err(debug)?;
    let prepared = actor
        .prepare_training_checkpoint(4, &formula, occurrence(84)?, true)
        .map_err(debug)?
        .ok_or("no training to save after respawn")?;
    assert_eq!(prepared.request.before, trained);
    let BuildCommitOutcome::Committed(receipt) = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fence(4)?,
            prepared.request.clone(),
            &formula,
        )
        .await
        .map_err(debug)?
    else {
        return Err("training save after respawn replayed".into());
    };
    actor
        .commit_training_checkpoint(prepared, &receipt)
        .map_err(debug)?;
    assert_eq!(h.root_revision().await?, "5");
    Ok(())
}

/// D58 / NIGHT-W1 decision 19: XP drops by the same amount while every trained
/// level and progress accumulator survives respawn, replay and restarted admission.
#[test]
fn trained_death_preserves_magic_and_skills_through_respawn_and_relog() -> TestResult {
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, "death_ml_relog", true).await?;
        let result = die_respawn_relog(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn die_respawn_relog(h: &Harness) -> TestResult {
    use crate::domain::CharacterId;
    use crate::durability::DurabilityRoot;
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    use crate::gameplay_transport::{CastFactsLoad, load_character_cast_facts};
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        &harness::context().content,
    )
    .map_err(debug)?;
    let table: &dyn BuildFormula = &formula;
    let character = CharacterId::from_bytes(id(harness::CHARACTER)).map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let occurrence = |tag| BuildOccurrence::from_bytes(id(tag)).map_err(debug);

    // Druid at revision 1 to 2.
    let druid =
        convert_vocation(&formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    let chosen = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fence(1)?,
            BuildChangeRequest {
                occurrence: occurrence(90)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: druid.clone(),
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));

    // Experience to three level 50 losses (L = 112,900 unblessed, unpromoted): 2 to 3.
    let loss =
        u64::try_from(crate::domain::death::death_experience_loss(50, 0, false).map_err(debug)?)?;
    assert_eq!(loss, 112_900);
    let total = 3 * loss;
    let award = h
        .root
        .commit_character_experience(
            &authority,
            &h.node,
            fence(2)?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: ExactI64::new(i64::try_from(total - 1_000)?),
                context: harness::context(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: death(92)?.policy,
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));

    // A trained build: magic level 3 and sword 14, each half-way to the next: 3 to 4.
    let half = |family, level| {
        table
            .required("druid", family, level)
            .map(|required| required / 2)
            .ok_or("requirement")
    };
    let mut skills = druid.skills();
    skills[2] = (14, half(3, 15)?);
    let trained = DurableBuildState::new("druid", (3, half(0, 4)?), skills).map_err(debug)?;
    let BuildCommitOutcome::Committed(_) = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fence(3)?,
            BuildChangeRequest {
                occurrence: occurrence(93)?,
                cause: BuildCause::Training,
                before: druid.clone(),
                after: trained.clone(),
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?
    else {
        return Err("trained build replayed".into());
    };

    // The live actor trains one more sword try, and the death flush saves it: 4 to 5.
    let mut actor = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: crate::spell::Vocation::Druid,
            level: 50,
            magic_level: u32::from(trained.magic().0),
            max_health: 185,
            max_mana: 90,
            max_soul: 100,
        },
        0,
        0,
    )
    .ok_or("actor")?;
    actor
        .load_owned_build(&trained)
        .map_err(|_| "load_owned_build")?;
    actor
        .enable_owned_training(&trained, &formula)
        .map_err(debug)?;
    actor
        .train_skill(SkillFamily::Sword, 1, &formula, |_| 1)
        .map_err(debug)?;
    let prepared = actor
        .prepare_training_checkpoint(2, &formula, occurrence(94)?, true)
        .map_err(debug)?
        .ok_or("no training to flush")?;
    let BuildCommitOutcome::Committed(receipt) = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fence(4)?,
            prepared.request.clone(),
            &formula,
        )
        .await
        .map_err(debug)?
    else {
        return Err("death flush replayed".into());
    };
    let flushed = receipt.after.clone();
    actor
        .commit_training_checkpoint(prepared, &receipt)
        .map_err(debug)?;
    assert_eq!(flushed.skills()[2].1, trained.skills()[2].1 + 1);

    // The death: XP loss with build preserved, 5 to 6.
    let request = death(95)?;
    let committed = h
        .root
        .settle_runtime_player_death(
            &CharacterRevisionSequencer::new(),
            &authority,
            &h.node,
            fence(5)?,
            request.clone(),
            Some(table),
            None,
        )
        .await
        .map_err(debug)?;
    assert_eq!(committed.experience_before.get(), i64::try_from(total)?);
    assert_eq!(committed.experience_lost.get(), i64::try_from(loss)?);
    assert_eq!(h.root_revision().await?, "6");
    assert!(committed.build_loss.is_none());
    assert_eq!(
        committed.experience_after.get(),
        i64::try_from(total - loss)?
    );
    let expected = flushed;
    assert_eq!(expected.vocation(), "druid");
    assert_eq!(
        h.root
            .read_character_build_state(&authority, character)
            .await
            .map_err(debug)?,
        expected
    );

    // Unchanged builds use the existing v1 binding and NULL build payload.
    let row: (i32, bool) = sqlx::query_as(
        "SELECT get_byte(command_binding, 0), vocation IS NULL AND \
                magic_level_before IS NULL AND mana_spent_before IS NULL AND \
                magic_level_after IS NULL AND mana_spent_after IS NULL AND \
                sword_level_before IS NULL AND sword_tries_before IS NULL AND \
                sword_level_after IS NULL AND sword_tries_after IS NULL \
           FROM game_character_death_receipts \
          WHERE death_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(id(95).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(row, (1, true));

    // Respawn: the pending respawn is consumed and the actor rebased on the preserved build.
    h.root
        .consume_pending_respawn(&authority, &h.node, fence(6)?, Some(request.occurrence))
        .await
        .map_err(debug)?
        .ok_or("missing pending respawn")?;
    actor.rebase_after_death(&expected);
    assert_eq!(
        actor.character_facts().magic_level,
        u32::from(expected.magic().0)
    );
    assert!(!actor.has_training_to_flush());

    // Relog through a restarted root: the death replays the same loss, admission reads the
    // preserved magic level and build, and a new actor trains on them.
    let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(restarted.maintain_ready_once().await?);
    let relogged = restarted
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let replayed = restarted
        .reconcile_character_death(&relogged, request.occurrence)
        .await
        .map_err(debug)?
        .ok_or("death not reconciled after relog")?;
    assert_eq!(replayed.build_loss, committed.build_loss);
    let CastFactsLoad::Ready { facts, .. } =
        load_character_cast_facts(&restarted, &relogged, character).await
    else {
        return Err("relogged facts unavailable".into());
    };
    assert_eq!(facts.magic_level, u32::from(expected.magic().0));
    let build = restarted
        .read_character_build_state(&relogged, character)
        .await
        .map_err(debug)?;
    assert_eq!(build, expected);
    let mut relog_actor = PlayerSpellState::new(facts, 0, 0).ok_or("relogged actor")?;
    relog_actor
        .load_owned_build(&build)
        .map_err(|_| "relogged load_owned_build")?;
    relog_actor
        .enable_owned_training(&build, &formula)
        .map_err(debug)?;
    relog_actor
        .train_skill(SkillFamily::Sword, 3, &formula, |_| 1)
        .map_err(debug)?;
    let prepared = relog_actor
        .prepare_training_checkpoint(4, &formula, occurrence(96)?, true)
        .map_err(debug)?
        .ok_or("no training to save after relog")?;
    assert_eq!(prepared.request.before, expected);
    let BuildCommitOutcome::Committed(_) = restarted
        .commit_character_build(&relogged, &h.node, fence(6)?, prepared.request, &formula)
        .await
        .map_err(debug)?
    else {
        return Err("training save after relog replayed".into());
    };
    assert_eq!(h.root_revision().await?, "7");
    Ok(())
}
