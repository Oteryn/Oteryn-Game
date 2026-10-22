//! Shared durable setup for equipment first-entry and committed-move dispatch tests.

use super::*;

pub(super) fn equipment_scenario(admission: Option<bool>) -> TestResult {
    crate::gameplay_transport::monk_save::postgres_tests::equipment_scenarios::run_equipment_scenario(
        async |supplied, pool, request| {
            equipment_dispatch_scenario(supplied, pool, request, admission).await
        },
    )
}

async fn equipment_dispatch_scenario(
    supplied: crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    request: crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
    admission: Option<bool>,
) -> TestResult {
    use crate::durability::item_transfer::ItemTransferOutcome;
    use crate::foundation::{ChannelId, ChannelRuntimeV1, GameSessionId, NodeId, WorldId};
    use crate::gameplay_transport::ComposedFreshAdmission;
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, StatusKind};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    use tokio::sync::Mutex;

    // An explicitly authored untimed, nonstackable Feet item in the otherwise production
    // generation. Production v1 has no SPEED rows, so a nonzero fixture proves the wiring.
    let mut input = fixture_input(serde_json::json!([]), Some(0))?;
    input
        .build_training
        .as_mut()
        .ok_or("build training")?
        .content_revision = "content-1".into();
    let bytes = input.item_profiles.as_mut().ok_or("profiles")?;
    let mut profiles: serde_json::Value = serde_json::from_slice(&bytes.bytes)?;
    let records = profiles["records"].as_array_mut().ok_or("records")?;
    let backpack = records
        .iter()
        .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i2854")
        .ok_or("backpack")?["semantics"]
        .clone();
    let boot = records
        .iter_mut()
        .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i3277")
        .ok_or("boot fixture")?;
    boot["semantics"] = backpack;
    boot["semantics"]["container"] = serde_json::json!({"state":"NOT_APPLICABLE"});
    boot["semantics"]["equipment"]["value"]["patterns"]["value"][0]["primary_slot"]["value"] =
        serde_json::json!("FEET");
    boot["semantics"]["skill_modifiers"] = serde_json::json!({"state":"KNOWN","value":{"modifiers":{"state":"KNOWN","value":[
        {"kind":"SPEED","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":30}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}},
        {"kind":"SUPPRESS_DRUNK","parameter":{"state":"KNOWN","value":{"kind":"BOOLEAN","value":true}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}
    ]}}});
    boot["attributes"]["speed_bonus"] = serde_json::json!(30);
    boot["admission"]["stack_class"] = serde_json::json!("NonStackable");
    bytes.bytes = serde_json::to_vec(&profiles)?;
    use sha2::{Digest, Sha256};
    bytes.sha256 = Sha256::digest(&bytes.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, &input)?;
    let (controller, pin) = activated(&input, world)?;
    let active = controller.active().ok_or("activated generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let pin_server_digest = pin.identity().server_artifact_digest();
    let pin_client_digest = pin.identity().client_artifact_digest();
    let pin_frame_digest = pin.frame_binding().digest();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin.into_channel_pin(),
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    if admission.is_none() {
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
    }

    let backpack_definition = snapshot(native, "oteryn:item.tibia.i2854", 9)
        .items
        .remove(0)
        .definition;
    let boot_definition = snapshot(native, "oteryn:item.tibia.i3277", 4)
        .items
        .remove(0)
        .definition;
    let backpack_facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
        native,
        &backpack_definition,
    )
    .map_err(debug)?;
    let boot_facts =
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(native, &boot_definition)
            .map_err(debug)?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(supplied.root.clone())
            .current_session_at(session)
            .await?
            .0;
    let f = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    let mut items = Vec::new();
    for (ordinal, definition) in [backpack_definition, boot_definition]
        .into_iter()
        .enumerate()
    {
        use crate::durability::item_mint::{
            GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
        };
        let request = ItemMintRequest {
            cause: ItemMintCause::for_test(
                world,
                channel,
                f.scope_ownership_generation,
                ordinal as u32 + 1,
                1,
                TypedDefinitionRef {
                    family: "LootTable".into(),
                    production_key: "fixture:loot.b3".into(),
                    revision_ref: "definition-r1".into(),
                },
                "fixture:purpose.drop".into(),
                0,
            ),
            item: definition,
            quantity: 1,
            ground: GroundPlacement {
                spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
                corpse_ref: id(3).to_vec(),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: id(6).to_vec(),
            },
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        };
        let mut candidate = supplied
            .root
            .freeze_item_mint(supplied.character, supplied.holder, request)
            .await
            .map_err(debug)?;
        let outcome = supplied
            .root
            .commit_item_mint(
                &candidate.fresh_death_lane_permit().await,
                supplied.character,
                supplied.holder,
                &mut candidate,
            )
            .await
            .map_err(debug)?;
        let ItemMintOutcome::Committed(minted) = outcome else {
            return Err(format!("mint: {outcome:?}").into());
        };
        items.push(minted.item_instance_id);
    }
    let [backpack_item, boot_item] = items[..] else {
        return Err("two minted items".into());
    };
    for (command_id, item, destination, facts, backpack) in [
        (
            1,
            backpack_item,
            crate::durability::item_transfer::ItemTransferDestination::ContainerSlot,
            backpack_facts.clone(),
            None,
        ),
        (
            2,
            boot_item,
            crate::durability::item_transfer::ItemTransferDestination::MainBackpack,
            boot_facts,
            Some(backpack_facts),
        ),
    ] {
        let request = crate::durability::item_transfer::ItemTransferRequest {
            weight_generation_digest: native.source_digest(),
            command: crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(command_id).map_err(debug)?,
            ),
            source_item_instance_id: item,
            destination,
            item: facts,
            backpack,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        };
        let mut candidate = supplied
            .root
            .freeze_item_transfer(supplied.character, supplied.holder, f, request, &weights)
            .await
            .map_err(debug)?;
        let permit =
            crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f.runtime_scope)
                .await
                .map_err(debug)?;
        let outcome = supplied
            .root
            .commit_item_transfer(
                &permit,
                supplied.character,
                supplied.holder,
                f,
                &mut candidate,
                &weights,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, ItemTransferOutcome::Committed(_)),
            "{outcome:?}"
        );
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin_server_digest.as_slice()).bind(pin_client_digest.as_slice()).bind(pin_frame_digest.as_slice()).execute(pool).await?;

    let door_content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
        .map_err(debug)?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(supplied.root.clone())
            .current_session_at(session)
            .await?
            .0;
    let f = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
        f.runtime_scope,
        f.scope_ownership_generation,
        door_content,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            f.runtime_scope,
            f.scope_ownership_generation,
        )
        .map_err(debug)?,
    );
    let runtime = Mutex::new(runtime);
    let owner = ComposedFreshAdmission {
        runtime: &runtime,
        movement_cells: room.movement_cells(),
        door: &door,
        chest: room.door(),
        active_generation: Some(active),
        ..supplied
    };
    let drunk = ConditionDefinition::new(
        "equip.fixture.drunk",
        1,
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 10_000,
        },
    )
    .ok_or("drunk")?;
    let decision_root = GameplayDecisionRoot::from_bytes([7; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &decision_root,
        occurrence: DecisionOccurrenceId::from_bytes([8; 16]),
    };
    if let Some(equipped) = admission {
        super::first_entry_tests::assert_first_entry(
            &owner, actor, boot_item, request, equipped, &drunk, &facts,
        )
        .await
    } else {
        super::committed_moves_tests::assert_committed_moves(
            &owner, actor, boot_item, f, &drunk, &facts,
        )
        .await
    }
}
