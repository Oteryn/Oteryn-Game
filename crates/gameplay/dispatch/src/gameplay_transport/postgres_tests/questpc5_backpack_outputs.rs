//! Reuse independent native backpack MINT outputs for the selected Nomads route.
use super::TestResult;

#[test]
fn questpc5_independent_backpack_rewards_are_atomic_and_not_container_contents() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_backpack_outputs",
            true,
            false,
        )
        .await?;
        let initialized: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_progression_state WHERE level>0",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            initialized, 1,
            "weighted MINT uses an admitted player, not a bootstrap root"
        );
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED questpc5_backpack_outputs server_version={version}");
        let parsed = serde_json::from_value::<
            oteryn_gameplay_quests::quest::reward_callbacks::ItemReward,
        >(serde_json::json!({
            "item":"oteryn:item.tibia.i7343","count":1,"contents":[],
            "backpack_outputs":[{"item":"oteryn:item.tibia.i3032","count":3},{"item":"oteryn:item.tibia.i3054","count":1}]
        }));
        if let Err(error) = parsed {
            h.cleanup().await?;
            eprintln!(
                "POSTGRES_CASE_COMPLETED questpc5_backpack_outputs duration={:?} success=false",
                started.elapsed()
            );
            return Err(error.into());
        }
        let deps = super::Dependencies::new()?;
        let (controller, _activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        eprintln!("BACKPACK_PHASE native_ready");
        {
            super::prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(super::fixture::debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(super::fixture::debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?
                .item_fence();
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(super::fixture::id(42).as_slice()).bind(super::fixture::id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(super::fixture::id(42).as_slice()).bind(super::fixture::id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(_activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root
                .initialize_admission_equipment(
                    &authority,
                    &h.node,
                    &fence,
                    generation.identity().server_artifact_digest(),
                    fresh.operation(),
                )
                .await
                .map_err(super::fixture::debug)?;
            eprintln!("BACKPACK_PHASE equipment_ready");
            let definition = |id| crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            };
            let facts = |id| {
                crate::gameplay_transport::item_ref_admission::generation_item_facts(
                    native,
                    &definition(id),
                )
            };
            let template = crate::durability::starter_grant::StarterTemplate {
                definition: definition(2854),
                ..crate::durability::starter_grant::StarterTemplate::main()
            };
            h.root
                .grant_starter_items(
                    &authority,
                    &h.node,
                    &fence,
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    &template,
                )
                .await
                .map_err(super::fixture::debug)?;
            eprintln!("BACKPACK_PHASE starter_ready");
            let carried:Vec<String>=sqlx::query_scalar("SELECT DISTINCT definition_production_key FROM game_item_instances WHERE lifecycle=1 ORDER BY definition_production_key").fetch_all(&h.pool).await?;
            eprintln!("BACKPACK_CARRIED_DEFINITIONS {carried:?}");
            use crate::durability::reward_claim_mint::{
                RewardClaimMintContent, RewardClaimMintError, RewardClaimMintOutcome,
                RewardClaimMintRequest, RewardClaimRefusal,
            };
            let content_revision = format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
            let request = RewardClaimMintRequest {
                backpack_outputs: true,
                command: crate::foundation::CommandRef::new(
                    fence.game_session_id,
                    crate::foundation::CommandId::new(100).map_err(super::fixture::debug)?,
                ),
                claim: crate::durability::item_mint::TypedDefinitionRef {
                    family: "RewardClaim".into(),
                    production_key: "fixture:nomads-independent-outputs".into(),
                    revision_ref: "definition-r1".into(),
                },
                source_placement: "fixture:nomads-independent-outputs".into(),
                item: facts(7343).map_err(super::fixture::debug)?,
                quantity: 1,
                contents: [(3032, 3), (3054, 1)]
                    .into_iter()
                    .map(|(id, quantity)| {
                        Ok(RewardClaimMintContent {
                            item: facts(id).map_err(super::fixture::debug)?,
                            quantity,
                            key_number: None,
                            text: None,
                            fluid_subtype: None,
                            charges_remaining: None,
                        })
                    })
                    .collect::<TestResult<Vec<_>>>()?,
                root_text: None,
                root_fluid_subtype: None,
                root_charges_remaining: None,
                backpack: facts(2854).map_err(super::fixture::debug)?,
                content_revision,
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
                achievement: None,
                quest_transition: None,
                key_number: None,
            };
            eprintln!("BACKPACK_PHASE request_ready");
            let mut candidate = owner
                .root
                .freeze_reward_claim_mint_weighted(
                    owner.character,
                    owner.holder,
                    fence,
                    request.clone(),
                    &weights,
                )
                .await
                .map_err(super::fixture::debug)?;
            eprintln!("BACKPACK_PHASE frozen");
            let (outcome, _) = owner
                .root
                .commit_reward_claim_mint_weighted_noticed(
                    owner.character,
                    owner.holder,
                    fence,
                    &mut candidate,
                    None,
                    &weights,
                )
                .await
                .map_err(super::fixture::debug)?;
            eprintln!("BACKPACK_PHASE committed");
            assert!(matches!(outcome, RewardClaimMintOutcome::Committed(_)));
            let direct:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=ANY($1)").bind(vec!["oteryn:item.tibia.i7343","oteryn:item.tibia.i3032","oteryn:item.tibia.i3054"]).fetch_one(&h.pool).await?;
            assert_eq!(direct, 3, "each reward is a direct backpack entry");
            let nested:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances p ON p.item_instance_id=e.parent_item_instance_id WHERE p.definition_production_key='oteryn:item.tibia.i7343'").fetch_one(&h.pool).await?;
            assert_eq!(
                nested, 0,
                "the selected programme does not pack the Fur Bag"
            );
            let charged:Vec<(String,Option<i32>)>=sqlx::query_as("SELECT i.definition_production_key,s.charges FROM game_item_instances i LEFT JOIN game_item_timed_states s USING(item_instance_id) WHERE i.definition_production_key=ANY($1) ORDER BY i.definition_production_key").bind(vec!["oteryn:item.tibia.i7343","oteryn:item.tibia.i3032","oteryn:item.tibia.i3054"]).fetch_all(&h.pool).await?;
            assert_eq!(
                charged,
                vec![
                    ("oteryn:item.tibia.i3032".into(), None),
                    ("oteryn:item.tibia.i3054".into(), None),
                    ("oteryn:item.tibia.i7343".into(), None)
                ],
                "full native rewards need no partial-charge override"
            );
            use crate::content::ReferenceItemField::Known;
            let silver = native
                .item_policy("oteryn:item.tibia.i3054", "definition-r1")
                .ok_or("Silver Amulet")?;
            assert!(
                matches!(&silver.record().semantics.charges, Known(charges) if charges.count == Known(200)),
                "exact Silver Amulet retains its full native 200 charges"
            );
            for key in ["oteryn:item.tibia.i3032", "oteryn:item.tibia.i7343"] {
                let policy = native
                    .item_policy(key, "definition-r1")
                    .ok_or("reward policy")?;
                assert!(
                    !matches!(&policy.record().semantics.charges, Known(charges) if matches!(charges.count, Known(_))),
                    "charges cannot transfer to another reward item"
                );
            }
            let before = snapshot(&h).await?;
            let (replay, _) = owner
                .root
                .commit_reward_claim_mint_weighted_noticed(
                    owner.character,
                    owner.holder,
                    fence,
                    &mut candidate,
                    None,
                    &weights,
                )
                .await
                .map_err(super::fixture::debug)?;
            assert!(matches!(
                replay,
                RewardClaimMintOutcome::AlreadyCommitted(_)
            ));
            assert_eq!(
                snapshot(&h).await?,
                before,
                "replay creates no output or audit rows"
            );
            let mut changed = request.clone();
            changed.backpack_outputs = false;
            assert!(
                owner
                    .root
                    .freeze_reward_claim_mint_weighted(
                        owner.character,
                        owner.holder,
                        fence,
                        changed,
                        &weights
                    )
                    .await
                    .is_err(),
                "the same command cannot rebind independent outputs as container contents"
            );
            let mut full = request.clone();
            full.command = crate::foundation::CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(101).map_err(super::fixture::debug)?,
            );
            full.claim.production_key = "fixture:nomads-refusal".into();
            full.contents = vec![request.contents[0].clone(); 19];
            assert!(matches!(
                owner
                    .root
                    .freeze_reward_claim_mint_weighted(
                        owner.character,
                        owner.holder,
                        fence,
                        full,
                        &weights
                    )
                    .await,
                Err(RewardClaimMintError::Refused(
                    RewardClaimRefusal::MainBackpackFull
                ))
            ));
            assert_eq!(
                snapshot(&h).await?,
                before,
                "whole-set refusal writes no items, reservation, claim, charges or audit"
            );
            let mut retry = request.clone();
            retry.command = crate::foundation::CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(102).map_err(super::fixture::debug)?,
            );
            retry.claim.production_key = "fixture:nomads-refusal".into();
            let mut retry = owner
                .root
                .freeze_reward_claim_mint_weighted(
                    owner.character,
                    owner.holder,
                    fence,
                    retry,
                    &weights,
                )
                .await
                .map_err(super::fixture::debug)?;
            let (retry, _) = owner
                .root
                .commit_reward_claim_mint_weighted_noticed(
                    owner.character,
                    owner.holder,
                    fence,
                    &mut retry,
                    None,
                    &weights,
                )
                .await
                .map_err(super::fixture::debug)?;
            assert!(
                matches!(retry, RewardClaimMintOutcome::Committed(_)),
                "refusal did not consume once eligibility"
            );
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED questpc5_backpack_outputs duration={:?} success=true",
            started.elapsed()
        );
        Ok(())
    })
}

async fn snapshot(h: &super::fixture::Harness) -> TestResult<(i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances),(SELECT count(*) FROM game_reward_claim_mint_reservations),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_item_timed_states),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?)
}
