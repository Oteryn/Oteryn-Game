// CHARM-2: the Bestiary kill as one more independent descendant of a
// committed creature death, next to loot and XP
// (`combat::settle_creature_death_rewards_with_bestiary`).

use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, context, debug, fence, id,
    runtime,
};
use crate::combat::{
    CombatBestiaryOutcome, CombatDeathRewardBestiaryError, CombatDeathRewardXpError,
    CreatureDeathBestiaryInput, CreatureDeathRewardInput, DeathGroundContext, DurabilitySession,
    RewardPrincipal, RewardProgressionBinding, settle_creature_death_rewards_with_bestiary,
};
use crate::combat::{
    LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry,
};
use crate::domain::bestiary::{BestiaryError, BestiaryKillCredit, BestiaryRace};
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::bestiary_progress::BestiaryKillOutcome;
use crate::durability::character_progression::{
    CharacterProgressionError, ExperienceCommitOutcome,
};
use crate::foundation::{ChannelId, CombatDeathFixture, ScopeOwnershipGeneration, WorldId};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

const RAT: &str = "oteryn:creature.rat";
const RAT_XP: i64 = 5;
const DEATH_AT_MS: u64 = 10_000_000;

fn death_fixture() -> TestResult<CombatDeathFixture> {
    CombatDeathFixture::new(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
    )
    .map_err(|error| debug(error).into())
}

fn definition(family: &str, key: &str) -> LootDefinitionRef {
    LootDefinitionRef::new(family, key, "VSL_COMBAT_FIXTURE_PROFILE/v1")
}

fn input(xp_amount: i64) -> TestResult<CreatureDeathRewardInput<2>> {
    let context = context();
    Ok(CreatureDeathRewardInput {
        corpse_item: definition("ItemType", "fixture:bestiary.item.rat_corpse"),
        loot_table_ref: definition("LootTable", "fixture:bestiary.loot.rat"),
        loot_table: LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![LootTableEntry {
                item: definition("ItemType", "fixture:bestiary.item.cheese"),
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(1_000_000),
            }],
        },
        ground: DeathGroundContext {
            map_revision: "fixture:bestiary.map.r1".into(),
            content_revision: "fixture:bestiary.content.r1".into(),
            ruleset_revision: "fixture:bestiary.ruleset.r1".into(),
            sim_revision: "fixture:bestiary.sim.r1".into(),
            native_room_placement_context: b"fixture:bestiary.room".to_vec(),
        },
        inflight_loot_mints_before_this_death: 0,
        reward_principals: vec![RewardPrincipal {
            gameplay_fence: fence(1)?,
        }],
        xp_amount: ExactI64::new(xp_amount),
        progression: RewardProgressionBinding {
            context: context.clone(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            policy: FiniteProgressionPolicy {
                context,
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                death_policy_revision: "death-1".into(),
                declared_difference_revision: "declaration-1".into(),
                thresholds: [
                    LevelThreshold {
                        level: 1,
                        minimum_experience: ExactI64::new(0),
                    },
                    LevelThreshold {
                        level: 2,
                        minimum_experience: ExactI64::new(1000),
                    },
                ],
                terminal_exclusive_experience: ExactI64::new(2000),
                death_loss_numerator: 1,
                death_loss_denominator: 10,
                death_loss_rounding: RoundingMode::Floor,
            },
        },
    })
}

fn bestiary(
    race: bool,
    last_damage_before_death_ms: i64,
) -> TestResult<CreatureDeathBestiaryInput> {
    let death_at_ms = DEATH_AT_MS;
    let principal_last_damage_at_ms =
        u64::try_from(i64::try_from(death_at_ms)? - last_damage_before_death_ms)?;
    Ok(CreatureDeathBestiaryInput {
        race: if race {
            Some(BestiaryRace::new(RAT, "definition-r1", vec![5, 50, 500]).map_err(debug)?)
        } else {
            None
        },
        credit: BestiaryKillCredit {
            principal_last_damage_at_ms,
            death_at_ms,
        },
    })
}

async fn progress_rows(harness: &Harness) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as(
        "SELECT race_key, kill_count FROM game_character_bestiary_progress \
          WHERE character_id = encode($1,'hex')::uuid ORDER BY race_key",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_all(&harness.pool)
    .await?)
}

#[test]
fn a_credited_death_counts_its_race_once_after_loot_and_xp_and_replays() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        // A bootstrap-only Character: the XP descendant initializes it first.
        let harness = Harness::create(admin, "credited", false).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:bestiary.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        let actor = fixture.actor();

        let mut settled = Vec::new();
        for attempt in 0..2 {
            fixture.project_death().map_err(debug)?;
            // A replay resends the exact same request (fence revision 1).
            let outcome = settle_creature_death_rewards_with_bestiary(
                actor,
                &mut fixture.borrow_combat_death(),
                &session,
                input(RAT_XP)?,
                bestiary(true, 300_000)?,
            )
            .await
            .map_err(debug)?;
            outcome.rewards.loot.map_err(debug)?;
            let xp = outcome.rewards.xp.map_err(debug)?;
            let kill = match outcome.bestiary.map_err(debug)? {
                CombatBestiaryOutcome::Recorded(kill) => kill,
                other => return Err(format!("kill was not recorded: {other:?}").into()),
            };
            match (attempt, &xp, &kill) {
                (0, ExperienceCommitOutcome::Committed(_), BestiaryKillOutcome::Committed(_))
                | (
                    1,
                    ExperienceCommitOutcome::AlreadyCommitted(_),
                    BestiaryKillOutcome::AlreadyCommitted(_),
                ) => {}
                outcomes => return Err(format!("unexpected outcomes: {outcomes:?}").into()),
            }
            settled.push((xp, kill));
        }
        let (BestiaryKillOutcome::Committed(first) | BestiaryKillOutcome::AlreadyCommitted(first)) =
            &settled[0].1
        else {
            return Err("missing kill".into());
        };
        let (ExperienceCommitOutcome::Committed(award)
        | ExperienceCommitOutcome::AlreadyCommitted(award)) = &settled[0].0;
        // XP advanced 1 -> 2; the kill, written after it, advanced 2 -> 3
        // under the same (death, character) occurrence.
        assert_eq!(award.committed_character_revision.get(), 2);
        assert_eq!(first.original_character_revision.get(), 2);
        assert_eq!(first.committed_character_revision.get(), 3);
        assert_eq!(first.occurrence.as_bytes(), award.occurrence.as_bytes());
        assert_eq!((first.kill_count_before, first.kill_count_after), (0, 1));
        assert_eq!(harness.root_revision().await?, "3");
        assert_eq!(harness.count("game_character_xp_receipts").await?, 1);
        assert_eq!(
            harness
                .count("game_character_bestiary_kill_receipts")
                .await?,
            1
        );
        assert_eq!(progress_rows(&harness).await?, vec![(RAT.to_owned(), 1)]);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_non_bestiary_or_uncredited_death_counts_nothing_and_never_touches_loot_or_xp() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for (tag, race, last_damage_before_death_ms) in [
            ("no_race", false, 0),
            ("late", true, 300_001),
            ("future", true, -1),
        ] {
            let harness = Harness::create(admin.clone(), tag, false).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let session = DurabilitySession {
                root: &harness.root,
                authority: &authority,
                node: &harness.node,
            };
            let mut fixture = death_fixture()?;
            fixture
                .strike("fixture:bestiary.strike.lethal", CombatDeathFixture::HEALTH)
                .map_err(debug)?;
            fixture.project_death().map_err(debug)?;
            let actor = fixture.actor();
            let outcome = settle_creature_death_rewards_with_bestiary(
                actor,
                &mut fixture.borrow_combat_death(),
                &session,
                input(RAT_XP)?,
                bestiary(race, last_damage_before_death_ms)?,
            )
            .await
            .map_err(debug)?;
            assert_eq!(outcome.rewards.loot.map_err(debug)?.entries.len(), 1);
            assert!(matches!(
                outcome.rewards.xp,
                Ok(ExperienceCommitOutcome::Committed(_))
            ));
            match (tag, outcome.bestiary) {
                ("no_race", Ok(CombatBestiaryOutcome::NotABestiaryRace))
                | ("late", Ok(CombatBestiaryOutcome::OutsideCreditWindow))
                | (
                    "future",
                    Err(CombatDeathRewardBestiaryError::Credit(
                        BestiaryError::InvalidCreditEvidence,
                    )),
                ) => {}
                (tag, other) => return Err(format!("{tag}: unexpected {other:?}").into()),
            }
            assert_eq!(harness.root_revision().await?, "2");
            assert_eq!(
                harness
                    .count("game_character_bestiary_kill_receipts")
                    .await?,
                0
            );
            assert!(progress_rows(&harness).await?.is_empty());
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn a_failed_xp_award_neither_blocks_the_kill_nor_is_rolled_back_by_it() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "xp_fail", false).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:bestiary.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();
        // Past the policy's terminal experience: the award is refused by the
        // calculator after initialization; the revision stays at one.
        let outcome = settle_creature_death_rewards_with_bestiary(
            actor,
            &mut fixture.borrow_combat_death(),
            &session,
            input(5_000)?,
            bestiary(true, 0)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(
            outcome.rewards.xp,
            Err(CombatDeathRewardXpError::Progression(
                CharacterProgressionError::Calculation(_)
            ))
        ));
        outcome.rewards.loot.map_err(debug)?;
        let Ok(CombatBestiaryOutcome::Recorded(BestiaryKillOutcome::Committed(kill))) =
            outcome.bestiary
        else {
            return Err("the kill must commit despite the XP failure".into());
        };
        assert_eq!(kill.original_character_revision.get(), 1);
        assert_eq!(kill.committed_character_revision.get(), 2);
        assert_eq!(harness.count("game_character_xp_receipts").await?, 0);
        assert_eq!(harness.root_revision().await?, "2");
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
