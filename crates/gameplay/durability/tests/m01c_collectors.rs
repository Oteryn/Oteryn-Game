//! Collector delivery uses the production revision slot and reward/MINT transaction.
//! Native movement/death capture is supplied by registered receiver fixtures.
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

// The reward owner independently checks the locked account's premium entitlement.
// Supply the same account-specific fixture authority to both production interfaces.
struct PremiumWeights<'a> {
    inner: reward_fixture::Weights<'a>,
    account: [u8; 16],
}
impl durability::item_transfer::ItemWeightLookup for PremiumWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.inner.generation_digest()
    }
    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        account == self.account
    }
    fn weight_of(
        &self,
        definition: &durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, durability::item_transfer::ItemWeightRefused> {
        self.inner.weight_of(definition)
    }
}

#[test]
fn m01c_pilgrimage_collector_delivers_twenty_platinum_once_postgres() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc3_m01c_collectors", true).await?;
                    let result = pilgrimage(&h).await;
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

async fn pilgrimage(h: &f::Harness) -> f::TestResult {
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
            .transition("oteryn:quest-reward/m01c-pilgrimage-s8")
            .is_some(),
        "production collector must be registered"
    );
    let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let premium = Premium(account.try_into().map_err(|_| "account UUID")?);
    let weights = PremiumWeights {
        inner: weights,
        account: premium.0,
    };
    let sequencer = durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(f::fence(1)?.character_id).await;
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let request = |stage: u8, command: u64| -> f::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: format!("oteryn:quest-reward/m01c-pilgrimage-s{stage}"),
            cause: QuestCause::Command(reward_fixture::command(command)?),
        })
    };
    let early = slot
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            request(8, 20000)?,
            Arc::clone(&catalogue),
            admission,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(early, QuestTransitionOutcome::Refused(_)),
        "early collector must refuse"
    );
    let out_of_order = slot
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            request(3, 20001)?,
            Arc::clone(&catalogue),
            admission,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(out_of_order, QuestTransitionOutcome::Refused(_)));
    for stage in 1..=8_u8 {
        let req = request(stage, 20010 + u64::from(stage))?;
        let result = slot
            .commit_quest_reward(
                &h.root,
                &authority,
                &h.node,
                f::fence(u64::from(stage))?,
                req.clone(),
                Arc::clone(&catalogue),
                admission,
                &premium,
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(result, QuestTransitionOutcome::Committed(_)),
            "stage {stage}: {result:?}"
        );
        let replay = slot
            .commit_quest_reward(
                &h.root,
                &authority,
                &h.node,
                f::fence(u64::from(stage))?,
                req,
                Arc::clone(&catalogue),
                admission,
                &premium,
            )
            .await
            .map_err(f::debug)?;
        assert!(matches!(
            replay,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        let coins:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key='oteryn:item.tibia.i3035' AND i.lifecycle=1")
            .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            coins,
            if stage == 8 { 20 } else { 0 },
            "value only at collector completion"
        );
    }
    let again = slot
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(9)?,
            request(8, 20030)?,
            Arc::clone(&catalogue),
            admission,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(again, QuestTransitionOutcome::Refused(_)),
        "new command cannot claim a second reward"
    );
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision=9 FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.pilgrimage_of_ashes'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    eprintln!(
        "M01C_PILGRIMAGE ordered8 early_refused reordered_refused platinum20 completion9 replay_once"
    );
    Ok(())
}

#[test]
fn m01c_stampede_collector_requires_five_native_kills_and_grants_once_postgres() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc3_m01c_stampede", true).await?;
                    let result = stampede(&h).await;
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

async fn stampede(h: &f::Harness) -> f::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M01C_COLLECTOR POSTGRES_CONNECTED version={version}");
    let (controller, pin) =
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
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice()).bind([2_u8;32].as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
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
            .transition("oteryn:quest-reward/m01c-stampede-collector")
            .is_some(),
        "production collector must be registered"
    );
    let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let premium = Premium(account.try_into().map_err(|_| "account UUID")?);
    let weights = PremiumWeights {
        inner: weights,
        account: premium.0,
    };
    let sequencer = durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(f::fence(1)?.character_id).await;
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };

    let finish = |id| -> f::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: "oteryn:quest-reward/m01c-stampede-collector".into(),
            cause: QuestCause::Command(reward_fixture::command(id)?),
        })
    };
    let early = slot
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            finish(21000)?,
            Arc::clone(&catalogue),
            admission,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(early, QuestTransitionOutcome::Refused(_)));
    let lane = durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let context = durability::quest_world_callback::QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: pin.frame_binding().digest().to_vec(),
        reset_epoch: 0,
        actor_placement: f::id(17),
        movement_revision: 1,
    };
    let entry = QuestTransitionRequest {
        transition_key: "oteryn:quest-world/m01c-stampede-entry".into(),
        cause: QuestCause::Step(reward_fixture::command(21001)?),
    };
    for replay in [false, true] {
        let outcome = slot
            .commit_world_callback_with_items(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                entry.clone(),
                Arc::clone(&catalogue),
                &lane,
                context.clone(),
                Some(native),
                None,
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
            "trial entry replay={replay}: {outcome:?}"
        );
    }
    let observed_database_micros: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let party = Arc::new(foundation::PartySnapshot {
        world: f::id(f::WORLD),
        character: f::id(f::CHARACTER),
        party_id: None,
        revision: None,
        leader: None,
        shared_xp_enabled: false,
        members: Vec::new(),
        observed_database_micros,
    });
    let hash = catalogue
        .definition_hash("oteryn:quest.stampede_mini_world_change")
        .ok_or("Stampede definition")?;
    for n in 0..5_u8 {
        use durability::quest_state::native_kills::{NativeDeathSource, NativeQuestDeath};
        let death = NativeQuestDeath {
            transition: "oteryn:quest-transition/native-kills/m01c-stampede-elephant".into(),
            creature: "oteryn:creature.terrified_elephant".into(),
            observed_revision: 2 + u64::from(n),
            definition_hash: Some(hash),
            source: NativeDeathSource::Party(Arc::clone(&party)),
        };
        let cause = QuestCause::CreatureDeath(
            durability::character_progression::ExperienceRewardOccurrence::from_bytes(f::id(
                170 + n,
            ))
            .map_err(f::debug)?,
        );
        for replay in [false, true] {
            let outcome = slot
                .commit_native_quest_kill_with_admission(
                    &h.root,
                    &authority,
                    &h.node,
                    f::fence(2 + u64::from(n))?,
                    cause,
                    &death,
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
                "elephant {n} replay={replay}: {outcome:?}"
            );
        }
        if n < 4 {
            let early = slot
                .commit_quest_reward(
                    &h.root,
                    &authority,
                    &h.node,
                    f::fence(3 + u64::from(n))?,
                    finish(21010 + u64::from(n))?,
                    Arc::clone(&catalogue),
                    admission,
                    &premium,
                )
                .await
                .map_err(f::debug)?;
            assert!(
                matches!(early, QuestTransitionOutcome::Refused(_)),
                "collector before five kills: {early:?}"
            );
        }
    }
    for replay in [false, true] {
        let outcome = slot
            .commit_quest_reward(
                &h.root,
                &authority,
                &h.node,
                f::fence(7)?,
                finish(21020)?,
                Arc::clone(&catalogue),
                admission,
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
            "collector replay={replay}: {outcome:?}"
        );
    }
    let duplicate = slot
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(8)?,
            finish(21021)?,
            Arc::clone(&catalogue),
            admission,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(duplicate, QuestTransitionOutcome::Refused(_)));
    let (grants, completed):(i64,bool)=sqlx::query_as("SELECT (SELECT count(*) FROM game_account_achievements WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND achievement_key='oteryn:achievement/trail_of_the_ape_god'), completed_character_revision=8 FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.stampede_mini_world_change'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(grants, 1);
    assert!(completed);
    eprintln!(
        "M01C_STAMPEDE native_entry kills5 early_refused achievement_once completed8 replay_stable"
    );
    Ok(())
}
