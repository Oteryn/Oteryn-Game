use super::*;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

pub(super) fn policy() -> FiniteProgressionPolicy<String, 2> {
    FiniteProgressionPolicy {
        context: ProgressionRevisionContext {
            profile: "profile-1".into(),
            ruleset: "ruleset-1".into(),
            content: "content-1".into(),
            simulation: "simulation-1".into(),
            evidence: "evidence-1".into(),
            declaration: "declaration-1".into(),
        },
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        death_policy_revision: "death-1".into(),
        declared_difference_revision: "declaration-1".into(),
        thresholds: [
            LevelThreshold {
                level: 6,
                minimum_experience: ExactI64::new(0),
            },
            LevelThreshold {
                level: 7,
                minimum_experience: ExactI64::new(1000),
            },
        ],
        terminal_exclusive_experience: ExactI64::new(100000),
        death_loss_numerator: 1,
        death_loss_denominator: 10,
        death_loss_rounding: RoundingMode::Floor,
    }
}

#[test]
fn grizzly_demon_reward_is_earned_atomically_once_and_survives_owner_restart() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Grizzly holy icon PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "grizzly_demon_reward", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let catalogue = catalogue()?;
        let yes = PremiumProbe::new(true);
        seed(&h, Some(130), &[]).await?;
        committed(
            apply(
                &h,
                &authority,
                &sequencer,
                &catalogue,
                command(1)?,
                Action::Join,
                &yes,
            )
            .await?,
        )?;
        seed(&h, None, &[(quest::POINTS_STORAGE, 100)]).await?;
        committed(
            apply(
                &h,
                &authority,
                &sequencer,
                &catalogue,
                command(2)?,
                Action::StartTask(42),
                &yes,
            )
            .await?,
        )?;
        seed(&h, None, &[(quest::KILL_COUNTER_BASE + 42, 6666)]).await?;
        let receipt = committed(
            apply(
                &h,
                &authority,
                &sequencer,
                &catalogue,
                command(3)?,
                Action::CompleteTask(42),
                &yes,
            )
            .await?,
        )?;
        assert_eq!(
            value(&h, &authority, quest::QUEST_STORAGE_BASE + 42).await?,
            2
        );
        assert_eq!(value(&h, &authority, 41300).await?, 1);
        assert!(receipt.experience.is_none());
        assert_eq!(
            apply(
                &h,
                &authority,
                &sequencer,
                &catalogue,
                command(3)?,
                Action::CompleteTask(42),
                &yes
            )
            .await?,
            QuestTransitionOutcome::AlreadyCommitted(receipt)
        );
        let pending = h
            .root
            .pending_grizzly_reward(&authority, fence(1)?.character_id)
            .await
            .map_err(debug)?
            .ok_or("earned holy icon")?;
        assert_eq!(pending.item_source_id, 9388);
        assert_eq!(
            pending.transition_key,
            Action::CompleteTask(42).transition_key()
        );
        assert_eq!(
            pending.claim_key,
            crate::durability::quest_state::grizzly::HOLY_ICON_CLAIM
        );
        // A new authority handle and sequencer retain the entitlement; no process-local
        // conversation or successful response is needed to collect it later.
        drop(authority);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let rebuilt = CharacterRevisionSequencer::new();
        assert_eq!(
            h.root
                .pending_grizzly_reward(&authority, fence(1)?.character_id)
                .await
                .map_err(debug)?,
            Some(pending)
        );
        assert_eq!(
            apply(
                &h,
                &authority,
                &rebuilt,
                &catalogue,
                command(4)?,
                Action::CompleteTask(42),
                &yes
            )
            .await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        );
        let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_grizzly_reward_entitlements),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-42')")
            .fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1));
        deliver(&h, &authority).await?;
        eprintln!("GRIZZLY_DEMON_REWARD POSTGRES_CASE_COMPLETED");
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn deliver(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let world = fence(1)?.runtime_scope.world_id();
    let (controller, _pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native content")?;
    let index = native.item_index().ok_or("native item index")?;
    let facts = |key: &str| {
        let revision = index.revision(key).ok_or("native item revision")?;
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            },
        )
        .map_err(debug)
    };
    let backpack = facts("oteryn:item.tibia.i2854")?;
    let filler = facts("oteryn:item.tibia.i3031")?;
    let reward = facts("oteryn:item.tibia.i9388")?;
    assert_eq!(backpack.container_capacity, Some(20));
    // Existing item fixtures establish a full source backpack. Delivery itself uses
    // the runtime MINT owner, its real native item facts, capacity and audit gates.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(
            170,
            &backpack.definition.production_key,
            &backpack.definition.revision_ref,
            1,
        ),
    ))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(170).as_slice()).bind(id(42).as_slice()).bind(id(171).as_slice())
        .execute(&mut *tx).await?;
    for n in 0..20_u8 {
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                180 + n * 2,
                &filler.definition.production_key,
                &filler.definition.revision_ref,
                1,
            ),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(170).as_slice()).bind(id(180 + n * 2).as_slice())
            .bind(id(42).as_slice()).bind((u64::from(n) + 1).to_string()).bind(id(240).as_slice())
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let pending = h
        .root
        .pending_grizzly_reward(authority, fence(1)?.character_id)
        .await
        .map_err(debug)?
        .ok_or("pending icon")?;
    let request = RewardClaimMintRequest {
        command: CommandRef::new(
            fence(1)?.game_session_id,
            CommandId::new(100).map_err(debug)?,
        ),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: pending.claim_key,
            revision_ref: "grizzly-adams-r1".into(),
        },
        source_placement: pending.transition_key,
        item: reward,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        backpack,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let full = h
        .root
        .claim_pending_grizzly_reward(
            authority,
            &h.node,
            fence(1)?.item_fence(),
            request.clone(),
            &NativeItemWeights(native),
        )
        .await;
    assert!(
        matches!(
            full,
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::MainBackpackFull
            ))
        ),
        "{full:?}"
    );
    assert!(
        h.root
            .pending_grizzly_reward(authority, fence(1)?.character_id)
            .await
            .map_err(debug)?
            .is_some()
    );
    let reservations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(reservations, 0, "full bag leaves no stranded reservation");
    assert!(
        h.root
            .reconcile_grizzly_reward(authority, fence(1)?.character_id, request.clone())
            .await
            .map_err(debug)?
            .is_none()
    );
    // Establish the later free-slot predecessor, as if the unrelated item move owner
    // removed a filler. No quest or reward rows are edited to make the retry succeed.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid AND placement_ordinal=20")
        .bind(id(170).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let receipt = match h
        .root
        .claim_pending_grizzly_reward(
            authority,
            &h.node,
            fence(1)?.item_fence(),
            request.clone(),
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?
    {
        RewardClaimMintOutcome::Committed(receipt) => receipt,
        other => return Err(format!("expected icon mint: {other:?}").into()),
    };
    assert_eq!(
        h.root
            .claim_pending_grizzly_reward(
                authority,
                &h.node,
                fence(1)?.item_fence(),
                request.clone(),
                &NativeItemWeights(native)
            )
            .await
            .map_err(debug)?,
        RewardClaimMintOutcome::AlreadyCommitted(receipt.clone())
    );
    assert_eq!(
        h.root
            .reconcile_grizzly_reward(authority, fence(1)?.character_id, request.clone())
            .await
            .map_err(debug)?,
        Some(receipt)
    );
    assert!(
        h.root
            .pending_grizzly_reward(authority, fence(1)?.character_id)
            .await
            .map_err(debug)?
            .is_none()
    );
    let mut another = request;
    another.command = CommandRef::new(
        fence(1)?.game_session_id,
        CommandId::new(101).map_err(debug)?,
    );
    assert!(matches!(
        h.root
            .claim_pending_grizzly_reward(
                authority,
                &h.node,
                fence(1)?.item_fence(),
                another,
                &NativeItemWeights(native)
            )
            .await,
        Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::AlreadyClaimed
        ))
    ));
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_reward_claims WHERE claim_production_key='oteryn:reward-claim.grizzly-holy-icon'),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9388')")
        .fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 1));
    Ok(())
}
