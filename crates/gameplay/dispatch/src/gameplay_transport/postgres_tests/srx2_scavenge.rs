//! P01: normal composed tool use commits a minted source corpse exactly once.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};

#[test]
fn srx2_scavenge_production_use_minted_corpse_atomic_replay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "scavenge PG panic")??;
    eprintln!(
        "SRX2_SCAVENGE_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "srx2_scavenge", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let station = "oteryn:item.tibia.i3600";
        let bag = "oteryn:item.tibia.i2854";
        let tool = "oteryn:item.tibia.i5908";
        for key in [
            tool,
            station,
            "oteryn:item.tibia.i5969",
            "oteryn:item.tibia.i4012",
            "oteryn:item.tibia.i5878",
        ] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native skinning profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (station, reference(station).ok_or("water")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
        };
        // The production boot policy: an unknown fact blocks and is not pickupable.
        let definitions = |key: &str| {
            let policy = native.item_policy(key, "definition-r1").map(|p| p.record());
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.and_then(|p| p.attributes.blocks_movement),
                blocks_projectile: policy.and_then(|p| p.attributes.blocks_projectile).unwrap_or(true),
                pickupable: policy.is_some_and(|p| matches!(&p.semantics.physical, crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true))),
                door: false,
            })
        };
        let map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(
            &harness,
            &[(tool, 1), ("oteryn:item.tibia.i5942", 1)],
        )
        .await?;

        let race = native
            .reward_table()
            .row("oteryn:creature.minotaur")
            .map_err(debug)?
            .race
            .clone()
            .ok_or("minotaur race")?;
        let corpse = mint_corpse(&owner, &harness, native, 5969, Some(race), 900).await?;
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_corpse_source_races WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).fetch_one(&harness.pool).await?,1);
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("target handle")?,
            )),
            use_with_position: None,
        };
        let command = UseCommand {
            game_session_id: deps.admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let result = owner
            .use_cooking_conversion(
                deps.actor,
                command,
                intent.clone(),
                Some(ItemKey::Instance(id(71))),
                Some(ItemKey::Instance(corpse)),
            )
            .await
            .ok_or("production USE outcome")?;
        assert_eq!(
            result.disposition,
            UseDisposition::Committed,
            "ordinary USE must consume the exact source-qualified corpse"
        );
        let state:(String,i64)=sqlx::query_as("SELECT definition_production_key,state_revision::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(state, ("oteryn:item.tibia.i4012".into(), 2));
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
            .fetch_one(&harness.pool)
            .await?;
        let replay = owner
            .use_cooking_conversion(deps.actor, command, intent, None, None)
            .await
            .ok_or("production replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                .fetch_one(&harness.pool)
                .await?,
            before
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_corpse_skinning_records")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        stages(&owner, &harness, native, fence, deps.actor).await?;
        entity_target(&owner, &harness, native, fence, deps.actor).await?;
        refusals(&owner, &harness, native, fence, deps.actor).await?;
        Ok(())
    })
}

// The inputs are source-native corpse/race pairs. The deliberately narrow RNG windows
// straddle fractional percentages (11.2,13.3,15.4), not rounded whole percentages.
async fn stages(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    let cases = [
        (
            1_i16,
            "minotaur",
            5969_u32,
            4012_u32,
            5878_u32,
            71_u8,
            111_000_000_u64,
            112_000_000_u64,
            true,
        ),
        (
            1,
            "minotaur",
            5969,
            4012,
            5878,
            71,
            112_000_000,
            113_000_000,
            false,
        ),
        (
            2,
            "minotaur",
            5969,
            4012,
            5878,
            71,
            132_000_000,
            133_000_000,
            true,
        ),
        (
            2,
            "minotaur",
            5969,
            4012,
            5878,
            71,
            133_000_000,
            134_000_000,
            false,
        ),
        (
            3,
            "minotaur",
            5969,
            4012,
            5878,
            71,
            153_000_000,
            154_000_000,
            true,
        ),
        (
            3,
            "minotaur",
            5969,
            4012,
            5878,
            71,
            154_000_000,
            155_000_000,
            false,
        ),
        (
            3, "demon", 5995, 4098, 5906, 72, 65_000_000, 66_000_000, true,
        ),
        // Another source race uses precisely the same corpse Item. Assignment remains minotaur.
        (
            3,
            "minotaur_cult_follower",
            5969,
            4012,
            5878,
            71,
            100_000_000,
            110_000_000,
            false,
        ),
    ];
    for (offset, (stage, species, source, spent, output, tool, low, high, success)) in
        cases.into_iter().enumerate()
    {
        super::maintain_ready(owner.root).await?;
        let race_key = format!("oteryn:creature.{species}");
        let race = native
            .reward_table()
            .row(&race_key)
            .map_err(debug)?
            .race
            .clone()
            .ok_or("fully admitted source race")?;
        seed_scavenge(
            harness,
            stage,
            if species == "minotaur_cult_follower" {
                "oteryn:creature.minotaur"
            } else {
                &race_key
            },
        )
        .await?;
        let corpse = mint_corpse(
            owner,
            harness,
            native,
            source,
            Some(race),
            u32::try_from(offset)? + 901,
        )
        .await?;
        let command_id = draw_command(
            native,
            fence.game_session_id,
            corpse,
            low,
            high,
            100 + u64::try_from(offset)? * 1_000_000,
        )?;
        let command = UseCommand {
            game_session_id: fence.game_session_id,
            command_id,
            item_fence: Some(fence),
        };
        let intent = skinning_intent()?;
        let result = owner
            .use_cooking_conversion(
                actor,
                command,
                intent.clone(),
                Some(ItemKey::Instance(id(tool))),
                Some(ItemKey::Instance(corpse)),
            )
            .await
            .ok_or("stage USE result")?;
        assert_eq!(
            result.disposition,
            UseDisposition::Committed,
            "stage{stage} {species}"
        );
        let (actual_spent,revision):(String,i64)=sqlx::query_as("SELECT definition_production_key,state_revision::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(
            (actual_spent, revision),
            (format!("oteryn:item.tibia.i{spent}"), 2)
        );
        let outputs:serde_json::Value=sqlx::query_scalar("SELECT outputs FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric").bind(fence.game_session_id.as_bytes().as_slice()).bind(command_id.to_string()).fetch_one(&harness.pool).await?;
        assert_eq!(
            outputs.as_array().ok_or("output array")?.len(),
            usize::from(success),
            "stage{stage} exact fractional probability"
        );
        if success {
            assert_eq!(outputs[0]["key"], format!("oteryn:item.tibia.i{output}"));
        }
        let before = footprint(harness).await?;
        let replay = owner
            .use_cooking_conversion(actor, command, intent, None, None)
            .await
            .ok_or("stage replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        assert_eq!(
            footprint(harness).await?,
            before,
            "replay cannot mint or audit again"
        );
    }
    // A canonical corpse without immutable Creature provenance must never infer a race
    // from its shared appearance, even on a draw that would certainly succeed.
    let corpse = mint_corpse(owner, harness, native, 5969, None, 950).await?;
    let command_id = draw_command(
        native,
        fence.game_session_id,
        corpse,
        0,
        10_000_000,
        20_000_100,
    )?;
    let before = footprint(harness).await?;
    let command = UseCommand {
        game_session_id: fence.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let result = owner
        .use_cooking_conversion(
            actor,
            command,
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await
        .ok_or("unregistered corpse result")?;
    assert_ne!(result.disposition, UseDisposition::Committed);
    assert_eq!(footprint(harness).await?, before);
    provenance_refuses_late_binding(harness, corpse).await?;
    Ok(())
}

fn skinning_intent() -> TestResult<UseIntent> {
    Ok(UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            std::num::NonZeroU64::new(2).ok_or("corpse handle")?,
        )),
        use_with_position: None,
    })
}

fn draw_command(
    native: &crate::content::native_gameplay::NativeGameplayState,
    session: crate::foundation::GameSessionId,
    corpse: [u8; 16],
    low: u64,
    high: u64,
    first: u64,
) -> TestResult<u64> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};
    for command in first..first + 1_000_000 {
        let digest = Sha256::new()
            .chain_update(session.as_bytes())
            .chain_update(command.to_be_bytes())
            .chain_update(corpse)
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let draw = deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(native.source_digest()),
            DecisionOccurrenceId::from_bytes(occurrence),
            "oteryn.skinning.v1",
            0,
        )
        .map_err(debug)?;
        let scaled = u128::from(draw) * 1_000_000_000;
        if scaled >= u128::from(low) << 64 && scaled < u128::from(high) << 64 {
            return Ok(command);
        }
    }
    Err("no bounded deterministic draw in test interval".into())
}

async fn footprint(harness: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances),(SELECT count(*) FROM game_item_craft_records),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&harness.pool).await?)
}

async fn seed_scavenge(harness: &Harness, stage: i16, race: &str) -> TestResult {
    // Standard fixture admission only; production consumption must read the persisted
    // current unlock and exact race assignment under the actual item transaction.
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_charm_unlocks(character_id,charm_key,unlocked_stage,committed_character_revision,last_charm_occurrence_id) VALUES(encode($1,'hex')::uuid,'oteryn:charm.scavenge',$2,2,encode($3,'hex')::uuid) ON CONFLICT(character_id,charm_key) DO UPDATE SET unlocked_stage=EXCLUDED.unlocked_stage").bind(id(41).as_slice()).bind(stage).bind(id(200).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_charm_assignments(character_id,charm_key,race_key,charm_category,committed_character_revision,charm_occurrence_id) VALUES(encode($1,'hex')::uuid,'oteryn:charm.scavenge',$2,2,2,encode($3,'hex')::uuid) ON CONFLICT(character_id,charm_key) DO UPDATE SET race_key=EXCLUDED.race_key").bind(id(41).as_slice()).bind(race).bind(id(201).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn mint_corpse(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    source: u32,
    race: Option<crate::domain::bestiary::BestiaryRace>,
    local: u32,
) -> TestResult<[u8; 16]> {
    let key = format!("oteryn:item.tibia.i{source}");
    native
        .item_index()
        .ok_or("item index")?
        .definition_ref("Item", &key, "definition-r1")
        .ok_or("source corpse admitted in current Item index")?;
    super::maintain_ready(owner.root).await?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: key,
        revision_ref: "definition-r1".into(),
    };
    let generation = owner
        .runtime
        .lock()
        .await
        .owner_fence()
        .map_err(debug)?
        .generation();
    if let Some(race) = &race {
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == race.key())
            .ok_or("canonical source creature")?;
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
        else {
            return Err("canonical Creature profile".into());
        };
        assert_eq!(
            profile
                .details
                .as_ref()
                .and_then(|d| d.corpse_item.as_ref())
                .map(|r| r.key.as_str()),
            Some(definition.production_key.as_str()),
            "fixture corpse must be the actual source Creature corpse"
        );
    }
    let mut cause = ItemMintCause::for_test(
        owner.world_id,
        owner.channel_id,
        generation,
        local,
        1,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    );
    if let Some(race) = race {
        cause = cause.with_corpse_race(race).map_err(debug)?;
    }
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause,
                item: definition,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(7, 0, -7),
                    corpse_ref: vec![5; 16],
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: vec![6; 16],
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    match owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            &harness.node,
            &mut candidate,
            id(41),
        )
        .await
        .map_err(debug)?
    {
        ItemMintOutcome::Committed(v) => Ok(v.item_instance_id),
        other => Err(format!("fresh corpse {other:?}").into()),
    }
}

async fn refusals(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    let race = native
        .reward_table()
        .row("oteryn:creature.minotaur")
        .map_err(debug)?
        .race
        .clone()
        .ok_or("minotaur race")?;
    seed_scavenge(h, 3, "oteryn:creature.minotaur").await?;
    let corpse = mint_corpse(owner, h, native, 5969, Some(race), 960).await?;
    let command_id = draw_command(
        native,
        fence.game_session_id,
        corpse,
        0,
        10_000_000,
        30_000_100,
    )?;
    let command = UseCommand {
        game_session_id: fence.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let before = full_footprint(h).await?;
    let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
        character_lease_generation: fence.character_lease_generation + 1,
        ..fence
    };
    let result = owner
        .use_cooking_conversion(
            actor,
            UseCommand {
                item_fence: Some(stale),
                ..command
            },
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await;
    assert!(!result.is_some_and(|v| v.disposition == UseDisposition::Committed));
    assert_eq!(
        full_footprint(h).await?,
        before,
        "stale lease cannot transform, mint, or audit"
    );

    let tool_entry:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(e) FROM game_item_container_entries e WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let before = full_footprint(h).await?;
    let result = owner
        .use_cooking_conversion(
            actor,
            command,
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await;
    assert!(!result.is_some_and(|v| v.disposition == UseDisposition::Committed));
    assert_eq!(
        full_footprint(h).await?,
        before,
        "selected tool requires actual current backpack custody"
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_container_entries SELECT (jsonb_populate_record(NULL::game_item_container_entries,$1)).*").bind(tool_entry).execute(&mut *tx).await?;
    tx.commit().await?;

    // A real corpse from a stale scope remains physically present but is not current authority.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=2 WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let before = full_footprint(h).await?;
    let result = owner
        .use_cooking_conversion(
            actor,
            command,
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await;
    assert!(!result.is_some_and(|v| v.disposition == UseDisposition::Committed));
    assert_eq!(
        full_footprint(h).await?,
        before,
        "old Channel generation cannot be skinned"
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=1 WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET channel_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).bind(id(44).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let before = full_footprint(h).await?;
    let result = owner
        .use_cooking_conversion(
            actor,
            command,
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await;
    assert!(!result.is_some_and(|v| v.disposition == UseDisposition::Committed));
    assert_eq!(
        full_footprint(h).await?,
        before,
        "another Channel's corpse cannot be selected"
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET channel_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).bind(id(43).as_slice()).execute(&mut *tx).await?;
    // Occupy every slot and make existing output stacks full so the selected successful
    // result must roll back. This is initial bag-state setup, never a bypass in the owner.
    sqlx::query("UPDATE game_item_instances SET quantity=100 WHERE definition_production_key='oteryn:item.tibia.i5878'").execute(&mut *tx).await?;
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).fetch_one(&mut *tx).await?;
    for offset in count..20 {
        sqlx::query("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3600','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,game_character_uuid_v7() FROM i").bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(100+offset).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let before = full_footprint(h).await?;
    let result = owner
        .use_cooking_conversion(
            actor,
            command,
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await
        .ok_or("full bag result")?;
    assert_eq!(result.disposition, UseDisposition::NoRoom);
    assert_eq!(
        full_footprint(h).await?,
        before,
        "full bag rolls back corpse, yield, provenance, receipt and audit"
    );
    Ok(())
}

async fn full_footprint(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'ground',(SELECT jsonb_agg(to_jsonb(g) ORDER BY item_instance_id) FROM game_item_ground_locations g),'bag',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),'craft',(SELECT count(*) FROM game_item_craft_records),'skinning',(SELECT count(*) FROM game_corpse_skinning_records),'source',(SELECT jsonb_agg(to_jsonb(p) ORDER BY item_instance_id) FROM game_corpse_source_races p),'audit',(SELECT count(*) FROM game_item_audit_outbox))").fetch_one(&h.pool).await?)
}

async fn entity_target(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    player: crate::foundation::ExactActorRef,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let position = MovementLocalPosition {
        x: 7,
        y: 0,
        floor: 7,
    };
    assert_eq!(
        owner.movement_cells()
            .index()
            .lookup(
                owner.movement_cells().scope(),
                crate::content::LogicalCell { x: 7, y: 0, z: 7 }
            )
            .map_err(debug)?,
        crate::content::CollisionClass::Walkable
    );
    let (actor, death, position) = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        let creature = runtime
            .realize_native_qualification_spawn(pin, "oteryn:creature.minotaur", position)
            .map_err(debug)?;
        let health = runtime
            .companion_snapshot(creature.actor)
            .map_err(debug)?
            .health;
        runtime
            .commit_monster_lab_damage(creature.actor, b"srx2-scavenge-native-corpse", health)
            .map_err(debug)?;
        let mut combat = runtime.borrow_combat_death();
        let projection =
            oteryn_gameplay_entities::project_fixed_one_creature_death(&mut combat, creature.actor)
                .map_err(debug)?;
        (
            creature.actor,
            projection.occurrence().death_key().map_err(debug)?,
            projection.position(),
        )
    };
    let row = native
        .reward_table()
        .row("oteryn:creature.minotaur")
        .map_err(debug)?;
    let race = row.race.clone().ok_or("canonical minotaur race")?;
    seed_scavenge(h, 3, race.key()).await?;
    let definition = TypedDefinitionRef {
        family: row.corpse_item.family.clone(),
        production_key: row.corpse_item.production_key.clone(),
        revision_ref: row.corpse_item.revision_ref.clone(),
    };
    let cause = ItemMintCause::from_creature_death(
        death,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(debug)?
    .with_corpse_race(race)
    .map_err(debug)?;
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause,
                item: definition.clone(),
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            position.x.try_into()?,
                            position.y.try_into()?,
                            (-position.floor).try_into()?,
                        ),
                    corpse_ref: actor.corpse_identity().to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: vec![6; 16],
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    let corpse = match owner
        .root
        .commit_corpse_mint(&permit, owner.character, &h.node, &mut candidate, id(41))
        .await
        .map_err(debug)?
    {
        ItemMintOutcome::Committed(v) => v.item_instance_id,
        other => return Err(format!("native corpse mint {other:?}").into()),
    };
    drop(permit);
    owner
        .runtime
        .lock()
        .await
        .bind_corpse_item(
            actor,
            corpse,
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )
        .map_err(debug)?;
    let command_id = draw_command(
        native,
        fence.game_session_id,
        corpse,
        0,
        10_000_000,
        40_000_100,
    )?;
    let command = UseCommand {
        game_session_id: fence.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let intent = skinning_intent()?;
    let result = owner
        .use_cooking_conversion(
            player,
            command,
            intent.clone(),
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Entity(actor.corpse_identity())),
        )
        .await
        .ok_or("native corpse Entity USE")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    let visible = owner
        .runtime
        .lock()
        .await
        .bound_corpse(actor.corpse_identity())
        .ok_or("transformed native corpse stays visible")?;
    assert_eq!(
        visible.item.production_key, "oteryn:item.tibia.i4012",
        "the actual native Entity projection must update to the spent appearance"
    );
    let before = full_footprint(h).await?;
    let replay = owner
        .use_cooking_conversion(player, command, intent, None, None)
        .await
        .ok_or("native Entity replay")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert_eq!(full_footprint(h).await?, before);
    Ok(())
}

async fn provenance_refuses_late_binding(h: &Harness, unregistered: [u8; 16]) -> TestResult {
    let before:serde_json::Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(p) ORDER BY item_instance_id),'[]'::jsonb) FROM game_corpse_source_races p").fetch_one(&h.pool).await?;
    let error=sqlx::query("INSERT INTO game_corpse_source_races(item_instance_id,mint_transaction_id,race_key,race_definition_revision) SELECT item_instance_id,transaction_id,'oteryn:creature.minotaur','definition-r1' FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(unregistered.as_slice()).execute(&h.pool).await.err().ok_or("late source binding must fail")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514"),
        "a previous real corpse MINT cannot acquire a race in a later transaction"
    );
    for statement in [
        "UPDATE game_corpse_source_races SET race_key='oteryn:creature.demon'",
        "DELETE FROM game_corpse_source_races",
    ] {
        let error = sqlx::query(statement)
            .execute(&h.pool)
            .await
            .err()
            .ok_or("source race must be immutable")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
    }
    let after:serde_json::Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(p) ORDER BY item_instance_id),'[]'::jsonb) FROM game_corpse_source_races p").fetch_one(&h.pool).await?;
    assert_eq!(after, before);
    Ok(())
}

#[path = "srx2_scavenge/m41.rs"]
mod m41;
