//! KOSHEI: framed production use-with commits the selected amulet and RewardDoor.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{koshei_amulet as source, world_callbacks};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn koshei_validator_accepts_definition_route_and_refuses_appearance_only() -> TestResult {
    let callback = source::callback()?;
    assert_eq!(callback.effects.target_magic, Some(14));
    world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
    let mut appearance = callback.clone();
    appearance.selector.target_definition = None;
    appearance.selector.appearance = Some(7535);
    assert!(world_callbacks::validate_callbacks(&[appearance]).is_err());
    for invalid in 0..4 {
        let mut bad = callback.clone();
        match invalid {
            0 => bad.selector.target_definition = Some("oteryn:terrain.tibia.i7535".into()),
            1 => bad.trigger = world_callbacks::Trigger::StepIn,
            2 => bad.item_on_target.as_mut().ok_or("tool")?.selected_instance = false,
            _ => bad.selector.appearance = Some(7535),
        }
        assert!(world_callbacks::validate_callbacks(&[bad]).is_err());
    }
    Ok(())
}

#[test]
fn koshei_real_use_with_grants_storage_refuses_other_operands_and_replays() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("koshei-amulet", || {
        super::quest_world_callbacks::scenario(
            "koshei-amulet",
            world_callbacks::Position {
                x: 32700,
                y: 31200,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "KOSHEI_PG_EXECUTED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    target: world_callbacks::Position,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    for (ordinal, item) in [
        (901, "oteryn:item.tibia.i7532"),
        (902, "oteryn:item.tibia.i3031"),
    ] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &world_callbacks::Offering {
                position: target,
                item: item.into(),
                count: 1,
            },
            ordinal,
            true,
        )
        .await?;
    }
    let map = owner.bundle_map().ok_or("map")?;
    let payload = |ordinal| -> TestResult<Vec<u8>> {
        let key =
            oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, ordinal)
                .ok_or("key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
            use_with: None,
            use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
        })
        .map_err(debug)
    };
    let wrong_target = payload(1)?;
    let corpse = payload(2)?;
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("index")?;
    let amulet = index
        .definition_ref("Item", "oteryn:item.tibia.i7532", "definition-r1")
        .ok_or("amulet definition")?;
    let other = index
        .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
        .ok_or("other definition")?;
    for (bytes, reference) in [(&wrong_target, amulet), (&corpse, other)] {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let (next, result, _) = super::item_craft::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            bytes,
            Some(reference),
        )
        .await?;
        *admitted = next;
        assert_ne!(result, UseDisposition::Committed);
        assert_eq!(snapshot(h).await?, (0, 1, 0, 1));
        assert!(!door_access(owner, admitted)?);
    }
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) =
        super::item_craft::exchange(owner, *admitted, command, &corpse, Some(amulet)).await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    assert!(door_access(owner, admitted)?);
    let (_, replay, _) =
        super::item_craft::exchange(owner, *admitted, command, &actual, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    assert!(door_access(owner, admitted)?);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(source::TRACK), Some(&1));
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1 AND value=1), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7532' AND lifecycle=1), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1)")
        .bind(source::TRACK).bind(source::KEY).fetch_one(&h.pool).await?)
}

fn door_access(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<bool> {
    let sessions = owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest sessions")?;
    let copy = sessions
        .get(&admitted.game_session_id)
        .and_then(|state| state.copy.as_ref())
        .ok_or("quest copy")?;
    Ok(copy.meets_door_storage(41351))
}

#[test]
fn koshei_spawned_ground_target_real_use_with_refusals_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("koshei-amulet-ground", || {
        super::quest_world_callbacks::scenario(
            "koshei-amulet-ground",
            world_callbacks::Position {
                x: 32700,
                y: 31200,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "KOSHEI_GROUND_PG_EXECUTED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

pub(super) async fn exercise_ground(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    target: world_callbacks::Position,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    for (n, item) in [
        (901, "oteryn:item.tibia.i7532"),
        (902, "oteryn:item.tibia.i3031"),
    ] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &world_callbacks::Offering {
                position: target,
                item: item.into(),
                count: 1,
            },
            n,
            true,
        )
        .await?;
    }
    // Spawn at a different cell from the base-map lookalike: the callback has no fixed position.
    let ground_at = world_callbacks::Position {
        y: target.y + 1,
        ..target
    };
    ground_item(
        owner,
        fence.item_fence(),
        ground_at,
        "oteryn:item.tibia.i7535",
        903,
    )
    .await?;
    ground_item(
        owner,
        fence.item_fence(),
        ground_at,
        "oteryn:item.tibia.i7537",
        904,
    )
    .await?;
    let before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) ORDER BY i.item_instance_id) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i7535','oteryn:item.tibia.i7537')").fetch_one(&h.pool).await?;
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("index")?;
    let reference = |id| {
        index
            .definition_ref("Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1")
            .ok_or("definition")
    };
    let (amulet, gold, corpse, other) = (
        reference(7532)?,
        reference(3031)?,
        reference(7535)?,
        reference(7537)?,
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
    })
    .map_err(debug)?;
    for (tool, target) in [(amulet, other), (gold, corpse)] {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let (next, result, _) = super::rust_lottery::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(tool),
            Some(target),
        )
        .await?;
        *admitted = next;
        assert_ne!(result, UseDisposition::Committed);
        assert_eq!(snapshot(h).await?, (0, 1, 0, 1));
        assert!(!door_access(owner, admitted)?);
    }
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) = super::rust_lottery::exchange(
        owner,
        *admitted,
        command,
        &payload,
        Some(amulet),
        Some(corpse),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    assert!(door_access(owner, admitted)?);
    let (_, replay, _) =
        super::rust_lottery::exchange(owner, *admitted, command, &actual, None, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    let after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) ORDER BY i.item_instance_id) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i7535','oteryn:item.tibia.i7537')").fetch_one(&h.pool).await?;
    assert_eq!(
        before, after,
        "target and contents retain their native lifecycle"
    );
    let effects = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0);
    assert_eq!(
        effects
            .iter()
            .filter(|(_, at, effect)| *effect == 14
                && at.x == ground_at.x
                && at.y == ground_at.y
                && at.floor == ground_at.floor)
            .count(),
        1
    );

    Ok(())
}

async fn ground_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    at: world_callbacks::Position,
    item: &str,
    n: u64,
) -> TestResult {
    use crate::durability::item_mint::{
        GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    };
    let map = owner.bundle_map().ok_or("map")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let mut mint = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    fence.scope_ownership_generation,
                    1,
                    n,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:quest-callbacks".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                    0,
                ),
                item: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: item.into(),
                    revision_ref: "definition-r1".into(),
                },
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            at.x, at.y, -at.floor,
                        ),
                    corpse_ref: super::fixture::id(3).to_vec(),
                    map_revision: {
                        use sha2::{Digest, Sha256};
                        let revision = crate::map::overlay::map_revision(map.overlay.base());
                        format!(
                            "sha256:{}",
                            Sha256::digest(revision.as_bytes())
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        )
                    },
                    content_revision: revision,
                    native_room_placement_context: frame.to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let result = owner
        .root
        .commit_item_mint(&permit, owner.character, owner.holder, &mut mint)
        .await
        .map_err(debug)?;
    assert!(matches!(result, ItemMintOutcome::Committed(_)));
    Ok(())
}
