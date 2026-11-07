//! Required Newhaven rewards use admitted canonical and native Item owners.
use super::TestResult;
#[test]
fn questpc5_newhaven_required_item_admission() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_newhaven_items",
            false,
            false,
        )
        .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED newhaven_items server_version={version}");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/items/definitions");
        let mut selected = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(root)? {
            let path = entry?.path();
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("items-") && n.ends_with(".json"))
            {
                continue;
            }
            let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
            for row in doc["records"].as_array().ok_or("items")? {
                let d = &row["definition"];
                let key = d["identity"]["key"].as_str().ok_or("identity")?;
                if [
                    "oteryn:item.tibia.i52732",
                    "oteryn:item.tibia.i3572",
                    "oteryn:item.tibia.i51977",
                ]
                .contains(&key)
                {
                    selected.insert(key.to_owned(), d.clone());
                }
            }
        }
        for item in [52732, 3572, 51977] {
            let key = format!("oteryn:item.tibia.i{item}");
            assert_eq!(
                selected.get(&key).ok_or("required canonical Item")?["materializable"],
                true,
                "required Newhaven Item {item}"
            );
        }
        // Resolve the same accepted generation used by native Item producers.
        let deps = super::Dependencies::new()?;
        let (controller, _activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        for item in [52732, 3572, 51977] {
            let definition = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{item}"),
                revision_ref: "definition-r1".into(),
            };
            crate::gameplay_transport::item_ref_admission::generation_item_facts(
                native,
                &definition,
            )
            .map_err(super::fixture::debug)?;
        }
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
            for (index, item) in [52732, 3572, 51977].into_iter().enumerate() {
                let request = crate::durability::reward_claim_mint::RewardClaimMintRequest {
                    command: crate::foundation::CommandRef::new(
                        fence.game_session_id,
                        crate::foundation::CommandId::new(100 + index as u64)
                            .map_err(super::fixture::debug)?,
                    ),
                    claim: crate::durability::item_mint::TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        production_key: format!("fixture:newhaven-item.{item}"),
                        revision_ref: "definition-r1".into(),
                    },
                    source_placement: "fixture:newhaven-item-admission".into(),
                    item: facts(item).map_err(super::fixture::debug)?,
                    quantity: 1,
                    contents: Vec::new(),
                    root_text: None,
                    root_fluid_subtype: None,
                    root_charges_remaining: None,
                    backpack: facts(2854).map_err(super::fixture::debug)?,
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                    achievement: None,
                    quest_transition: None,
                    key_number: None,
                };
                let mut candidate = owner
                    .root
                    .freeze_reward_claim_mint(owner.character, owner.holder, fence, request)
                    .await
                    .map_err(super::fixture::debug)?;
                owner
                    .root
                    .commit_reward_claim_mint(owner.character, owner.holder, fence, &mut candidate)
                    .await
                    .map_err(super::fixture::debug)?;
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
                .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
                assert_eq!(count, 1, "actual native Item mint {item}");
            }
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED newhaven_items duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
