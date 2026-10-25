//! ADV-BLESS-VERIFY-1: real dispatch/death commits against PostgreSQL.
use super::fixture::{Harness, debug, fence, id, pvp_source_death_fixture, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::character_death::CharacterDeathRequest;
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;
use crate::spell::mana_training::CompiledTrainingFormula;
use oteryn_simulation_determinism::ExactI64;

fn request(tag: u8, pvp: bool) -> TestResult<CharacterDeathRequest<50>> {
    let mut request = death_request(tag)?;
    for threshold in &mut request.policy.thresholds {
        threshold.minimum_experience = ExactI64::new(i64::from(threshold.level - 1) * 100_000);
    }
    request.policy.terminal_exclusive_experience = ExactI64::new(10_000_000);
    request.held_blessings = vec!["spiritual_shielding".into()];
    if pvp {
        request.pvp_contributors = vec![PvpDeathContributor {
            character_id: CharacterId::from_bytes(id(60)).map_err(debug)?,
            kind: PvpContributorKind::Damage,
            unjustified: true,
        }];
    }
    Ok(request)
}

async fn seed(
    h: &Harness,
    level: u32,
    world_type: i16,
    forfeited: bool,
    red_skull: bool,
    regular_blessing: bool,
) -> TestResult {
    pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
    // Only initial state is a fixture. Every successor, loss and receipt below
    // is written by the production fenced owners using the runtime role.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$2,total_experience=$3 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(i64::from(level)).bind(i64::from(level-1)*100_000)
        .execute(&mut *tx).await?;
    if regular_blessing {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'spiritual_shielding','fixture')")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,adventurer_forfeited,skull,skull_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,CASE WHEN $4=0 THEN NULL ELSE clock_timestamp()+interval '1 day' END)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(forfeited).bind(if red_skull { 1_i16 } else { 0_i16 })
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'adventurer-blessing-fixture')")
        .bind(id(42).as_slice()).bind(world_type).execute(&h.pool).await?;
    Ok(())
}

async fn ledger(h: &Harness) -> TestResult {
    pvp_source_death_fixture::seed_ledger(&h.pool, 41, 60, true, Some(1), "0 seconds").await?;
    Ok(())
}

fn boundary(
    level: u32,
    pvp: bool,
    world_type: i16,
    forfeited: bool,
    red_skull: bool,
    regular_blessing: bool,
) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "adventurer_boundary", true, false)
            .await
            .map_err(|e| format!("adventurer harness: {e}"))?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("adventurer authority: {e:?}"))?;
        seed(&h, level, world_type, forfeited, red_skull, regular_blessing).await?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&h, &authority);
        if pvp {
            ledger(&h).await?;
        }
        let mut current = fence(1)?;
        current.character_lease_generation = 2;
        let formula = CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            "content-1",
        )
        .map_err(debug)?;
        let druid =
            convert_vocation(&formula, &DurableBuildState::default(), "druid").map_err(debug)?;
        let mut skills = druid.skills();
        skills[2] = (11, 1);
        let trained = DurableBuildState::new("druid", (1, 1), skills).map_err(debug)?;
        for (tag, cause, before, after) in [
            (
                87,
                BuildCause::VocationChoice,
                DurableBuildState::default(),
                druid.clone(),
            ),
            (88, BuildCause::Training, druid, trained.clone()),
        ] {
            let outcome = h
                .root
                .commit_character_build(
                    &authority,
                    &h.node,
                    current,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(id(tag)).map_err(debug)?,
                        cause,
                        before,
                        after,
                        pruned_stance: None,
                    },
                    &formula,
                )
                .await
                .map_err(debug)?;
            let BuildCommitOutcome::Committed(receipt) = outcome else {
                return Err("build unexpectedly replayed".into());
            };
            current.expected_character_revision = receipt.committed_character_revision;
        }
        let mut request = request(90, pvp)?;
        if !regular_blessing {
            request.held_blessings.clear();
        }
        let death = owner
            .settle_source_player_death(current, request.clone(), Some(&formula), None)
            .await
            .map_err(debug)?;
        let protected = level <= 20 && pvp && world_type == 2 && !forfeited && !red_skull;
        assert_eq!(death.level_before, level);
        if protected {
            assert_eq!(death.experience_lost.get(), 0, "level {level}");
            assert_eq!(death.experience_after, death.experience_before);
            assert_eq!(death.level_after, level);
            assert_eq!(death.blessings_after, request.held_blessings);
        } else {
            let expected_loss = i64::try_from(
                crate::domain::death::death_experience_loss(level, u8::from(regular_blessing), false)
                    .map_err(debug)?,
            )?.min(death.experience_before.get());
            assert!(expected_loss > 0);
            assert_eq!(death.experience_lost.get(), expected_loss, "level {level}");
            assert_eq!(death.experience_after.get(), death.experience_before.get() - expected_loss);
            assert!(death.blessings_after.is_empty());
        }
        assert!(death.build_loss.is_none());
        assert_eq!(
            h.root.read_character_build_state(&authority, current.character_id).await.map_err(debug)?,
            trained,
            "every PvE/PvP death preserves all build families and progress"
        );
        let held: Vec<String> = sqlx::query_scalar("SELECT blessing_key FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY blessing_key LIMIT 33")
            .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(held, death.blessings_after);
        assert_eq!(
            owner
                .settle_source_player_death(current, request, Some(&formula), None)
                .await
                .map_err(debug)?,
            death,
            "retry replays the retained outcome"
        );
        let recorded: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_death_receipts),(SELECT count(*) FROM game_character_pending_respawns),(SELECT count(*) FROM game_character_pvp_ledger)")
            .fetch_one(&h.pool).await?;
        assert_eq!(recorded, (1, 1, 0));
        eprintln!(
            "ADV_BLESS_POSTGRES_EVIDENCE: level={level} pvp={pvp} world={world_type} forfeited={forfeited} protected={protected}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn adventurer_blessing_pvp_level_20() -> TestResult {
    boundary(20, true, 2, false, false, true)
}
#[test]
fn adventurer_blessing_pvp_level_21() -> TestResult {
    boundary(21, true, 2, false, false, true)
}
#[test]
fn adventurer_blessing_pvp_level_22() -> TestResult {
    boundary(22, true, 2, false, false, true)
}
#[test]
fn adventurer_blessing_does_not_protect_pve() -> TestResult {
    boundary(20, false, 2, false, false, true)
}
#[test]
fn adventurer_blessing_stays_forfeited() -> TestResult {
    boundary(20, true, 2, true, false, true)
}
#[test]
fn adventurer_blessing_is_open_only() -> TestResult {
    boundary(20, true, 3, false, false, true)
}

#[test]
fn adventurer_blessing_red_skull_does_not_override_loss() -> TestResult {
    boundary(20, true, 2, false, true, true)
}

#[test]
fn adventurer_blessing_unprotected_pvp_preserves_trained_build_and_d58_xp_loss() -> TestResult {
    boundary(22, true, 2, false, false, false)
}

#[test]
fn adventurer_blessing_first_level_21_is_permanent_after_delevel() -> TestResult {
    permanence(true)
}

#[test]
fn adventurer_blessing_death_history_keeps_it_lost_after_delevel() -> TestResult {
    permanence(false)
}

fn permanence(award: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "adventurer_level_up", true, false)
            .await
            .map_err(|e| format!("adventurer harness: {e}"))?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("adventurer authority: {e:?}"))?;
        seed(&h, if award { 20 } else { 21 }, 2, false, false, true).await?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&h, &authority);
        let mut current = fence(1)?;
        current.character_lease_generation = 2;
        let request = request(90, true)?;
        if award {
            h.root
                .commit_character_experience(
                    &authority,
                    &h.node,
                    current,
                    ExperienceAwardRequest {
                        occurrence: ExperienceRewardOccurrence::from_bytes(id(89))
                            .map_err(debug)?,
                        context: request.context.clone(),
                        policy_revision: request.policy_revision.clone(),
                        reward_revision: request.reward_revision.clone(),
                        policy: request.policy.clone(),
                        amount: ExactI64::new(100_000),
                    },
                )
                .await
                .map_err(debug)?;
        }
        ledger(&h).await?;
        let death = owner
            .settle_source_player_death(current, request.clone(), None, None)
            .await
            .map_err(debug)?;
        assert_eq!((death.level_before, death.level_after), (21, 20));
        assert!(death.experience_lost.get() > 0);
        current.expected_character_revision = death.committed_character_revision;
        h.root
            .consume_pending_respawn(&authority, &h.node, current, Some(death.occurrence))
            .await
            .map_err(debug)?;
        ledger(&h).await?;
        let mut again = request;
        again.occurrence =
            crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(91))
                .map_err(debug)?;
        again.held_blessings.clear();
        let delevelled = owner
            .settle_source_player_death(current, again, None, None)
            .await
            .map_err(debug)?;
        assert_eq!(delevelled.level_before, 20);
        assert!(
            delevelled.experience_lost.get() > 0,
            "first reaching 21 permanently ended protection"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
