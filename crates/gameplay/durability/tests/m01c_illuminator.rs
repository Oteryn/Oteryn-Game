//! Illuminator entry uses authoritative XP, custody and prerequisite owners.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
#[allow(dead_code, reason = "reuse the existing canonical PostgreSQL harness")]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code, reason = "reuse the native backpack and weight fixture")]
#[path = "../../../../apps/game-server/tests/support/authority_cases/engitem_reward_text.rs"]
mod reward_fixture;
use bestiary_postgres_harness as f;
use durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use durability::reward_claim_mint::QuestRewardAdmission;
use std::sync::Arc;

#[path = "support/native_inventory.rs"]
mod native_inventory;
struct Premium([u8; 16]);
impl durability::quest_state::grizzly::GrizzlyPremiumGate for Premium {
    async fn task_permitted(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, durability::DurabilityError> {
        Ok(account == self.0)
    }
}
#[test]
fn m01c_illuminator_shell_requires_level_premium_and_completed_parent_postgres() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "questpc3_m01c_illuminator", true).await?;
                    let result = illuminator(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M01C_COLLECTOR POSTGRES_CASE_COMPLETED duration_ms={} success=true",
                        started.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "collector PostgreSQL thread")?
        .map_err(Into::into)
}

async fn illuminator(h: &f::Harness) -> f::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M01C_COLLECTOR POSTGRES_CONNECTED version={version}");
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|a| a.native_gameplay())
        .ok_or("native content")?;
    let weights = reward_fixture::Weights(native);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    reward_fixture::equip_backpack(h, &authority, native, &weights).await?;
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(f::debug)?
            .catalogue()
            .clone(),
    );
    assert!(
        catalogue
            .transition("oteryn:quest-collection/m01c-illuminator-shell")
            .is_some(),
        "production collector must be registered"
    );
    let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let premium = Premium(account.try_into().map_err(|_| "account UUID")?);

    let shell =
        native_inventory::mint_to_bag(h, &authority, native, &weights, 51656, 1, 312).await?;
    let sequencer = durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(f::fence(1)?.character_id).await;

    let request = |id| -> f::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: "oteryn:quest-collection/m01c-illuminator-shell".into(),
            cause: QuestCause::Command(reward_fixture::command(id)?),
        })
    };
    // Arrange an authoritative level through the XP owner, not a direct SQL state write.
    let levelled = slot
        .commit_experience(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            xp(220, 149000)?,
            None,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        levelled,
        durability::character_progression::ExperienceCommitOutcome::Committed(_)
    ));
    let before_parent = slot
        .commit_quest_transition_with_admission(
            &h.root,
            &authority,
            &h.node,
            f::fence(2)?,
            request(9300 + 312)?,
            Arc::clone(&catalogue),
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(before_parent, QuestTransitionOutcome::Refused(_)));
    drop(slot);
    complete_parent_fixture(h, &authority, native, &weights).await?;
    let mut slot = sequencer.acquire(f::fence(3)?.character_id).await;
    let below = slot
        .commit_quest_transition_with_admission(
            &h.root,
            &authority,
            &h.node,
            f::fence(3)?,
            request(9300 + 312)?,
            Arc::clone(&catalogue),
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(below, QuestTransitionOutcome::Refused(_)),
        "completed parent and shell cannot bypass level199"
    );
    let levelled = slot
        .commit_experience(
            &h.root,
            &authority,
            &h.node,
            f::fence(3)?,
            xp(221, 1000)?,
            None,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        levelled,
        durability::character_progression::ExperienceCommitOutcome::Committed(_)
    ));
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(level, 200);
    let free = slot
        .commit_quest_transition_with_admission(
            &h.root,
            &authority,
            &h.node,
            f::fence(4)?,
            request(9300 + 312)?,
            Arc::clone(&catalogue),
            &Premium([0; 16]),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(free, QuestTransitionOutcome::Refused(_)),
        "all other conditions do not bypass premium"
    );
    let forged = slot
        .commit_quest_transition_with_admission(
            &h.root,
            &authority,
            &h.node,
            f::fence(4)?,
            request(31003)?,
            Arc::clone(&catalogue),
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(forged, QuestTransitionOutcome::Refused(_)),
        "an unrecorded command cannot stand in for a native custody occurrence"
    );
    for replay in [false, true] {
        let outcome = slot
            .commit_quest_transition_with_admission(
                &h.root,
                &authority,
                &h.node,
                f::fence(4)?,
                request(9300 + 312)?,
                Arc::clone(&catalogue),
                &premium,
            )
            .await
            .map_err(f::debug)?;
        assert!(
            if replay {
                matches!(outcome, QuestTransitionOutcome::AlreadyCommitted(_))
            } else {
                matches!(outcome, QuestTransitionOutcome::Committed(_))
            },
            "shell collection replay={replay}: {outcome:?}"
        );
    }
    let (progress,incomplete):(i64,bool)=sqlx::query_as("SELECT t.value,s.completed_character_revision IS NULL FROM game_character_quest_tracks t JOIN game_character_quest_states s ON s.character_id=t.character_id AND s.quest_key='oteryn:quest.illuminator_outfits_quest' WHERE t.character_id=encode($1,'hex')::uuid AND t.track_key='oteryn:quest-progress/m01c-illuminator-route'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(progress, 1);
    assert!(incomplete, "an entry cannot finish or grant an outfit");
    let preserved:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) WHERE item=encode($3,'hex')::uuid)").bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(shell.as_slice()).fetch_one(&h.pool).await?;
    assert!(
        preserved,
        "COLLECT observes custody without consuming the shell"
    );
    eprintln!(
        "M01C_ILLUMINATOR native_shell51656 level199_refused level200_admitted premium_required parent_completed_fixture replay_safe completion_held"
    );
    Ok(())
}

fn xp(
    tag: u8,
    amount: i64,
) -> f::TestResult<durability::character_progression::ExperienceAwardRequest<151>> {
    use domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let context = f::context();
    let thresholds: [LevelThreshold; 151] = (50..=200_u32)
        .map(|level| LevelThreshold {
            level,
            minimum_experience: ExactI64::new(1000 + i64::from(level - 50) * 1000),
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "bounded fixture XP thresholds")?;
    Ok(durability::character_progression::ExperienceAwardRequest {
        occurrence: durability::character_progression::ExperienceRewardOccurrence::from_bytes(
            f::id(tag),
        )
        .map_err(f::debug)?,
        amount: ExactI64::new(amount),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(152000),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

async fn complete_parent_fixture(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    weights: &reward_fixture::Weights<'_>,
) -> f::TestResult {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, reward_callbacks};
    // Registered receiver fixture for the existing parent owner, not parent-route qualification.
    let callback: reward_callbacks::RewardCallback = serde_json::from_value(
        serde_json::json!({"key":"oteryn:quest-reward/m01c-illuminator-parent-fixture","quest":"oteryn:quest.between_the_lines_quest","npc":"oteryn:npc.fixture","keywords":["done"],"topic":0,"next_topic":0,"completes":true,"storage":[{"track":"oteryn:quest-progress/m01c-parent-fixture","minimum":0,"maximum":0,"set":1}],"take":[],"give":null,"outfit":null,"achievement":null,"reply":["done"],"sources":["M01c parent completion receiver fixture"]}),
    )?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: callback.storage[0].track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![reward_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(reward_fixture::command(31100)?),
    };
    let admission = QuestRewardAdmission {
        native,
        weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let outcome = h
        .root
        .commit_fixture_quest_reward(
            authority,
            &h.node,
            f::fence(2)?,
            request,
            catalogue,
            admission,
            callback,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    Ok(())
}
