//! Shared framed-use fishing and gathering scenario.
use super::fixture::{Harness, debug, id, run};
use super::item_craft_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) struct GatheringCase {
    pub tool: u16,
    pub target_id: u16,
    pub after: Option<u16>,
    pub yield_item: Option<u16>,
    pub committed: bool,
    pub in_sewers: bool,
    pub projected_desert: bool,
    pub select_roll: Option<fn(u32, u32) -> bool>,
    pub consumes_bait: bool,
}

pub(super) fn gathering_scenario(case: GatheringCase) -> TestResult {
    let GatheringCase { tool, target_id, after, yield_item, committed, in_sewers, projected_desert: desert, select_roll, consumes_bait } = case;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "m09_gathering", true, false).await?;
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
        let water_key = format!("oteryn:item.tibia.i{target_id}");
        let water = water_key.as_str();
        let bag = "oteryn:item.tibia.i2854";
        let tool_key = format!("oteryn:item.tibia.i{tool}");
        let rod = tool_key.as_str();
        for key in [
            rod,
            water,
            "oteryn:item.tibia.i3492",
            "oteryn:item.tibia.i3578",
        ] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native fishing profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let sewer = in_sewers
            && tool == 9306
            && oteryn_gameplay_items::fishing::MECHANICAL_FISHING_WATER.contains(&target_id);
        let bait_key = if sewer {
            oteryn_gameplay_items::fishing::NAIL
        } else {
            oteryn_gameplay_items::fishing::WORM
        };
        let epoch = if desert {
            river_reset(&harness, deps.admitted.world_id).await?
        } else {
            0
        };
        let start = if desert {
            crate::map::overlay::TilePos { x: 33431, y: 31523, floor: -11 }
        } else { crate::map::overlay::TilePos {
            x: if sewer { 32742 } else { 6 },
            y: if sewer { 31168 } else { 0 },
            floor: if sewer { -8 } else { -7 },
        }};
        let (mut bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (water, reference(water).ok_or("water")?.get() - 1),
        );
        if sewer || desert {
            // Re-anchor the verified fixture at the exact gathering source.
            let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
            let dx = start.x - 6;
            let dy = start.y;
            data.manifest.world.min_x += dx;
            data.manifest.world.max_x += dx;
            data.manifest.world.min_y += dy;
            data.manifest.world.max_y += dy;
            data.manifest.world.floors = vec![start.floor];
            for sector in &mut data.sectors {
                sector.floor = start.floor;
                sector.sx = (sector.sx * 32 + dx) / 32;
                sector.sy = (sector.sy * 32 + dy) / 32;
                for tile in &mut sector.tiles {
                    tile.x += dx;
                    tile.y += dy;
                }
            }
            if desert {
                use oteryn_world_bundle::bundle::{Family, PaletteEntry, Terrain, TerrainKind};
                let projected_water = data.manifest.palette.iter_mut().find(|entry| entry.key == water)
                    .ok_or("desert fixture water")?;
                projected_water.key = "oteryn:terrain.tibia.i13989".into();
                projected_water.family = Family::Terrain;
                let terrain = Terrain {
                    kind: TerrainKind::Ground, walkable: Some(true), ground_speed: Some(200),
                    blocks_projectile: None,
                };
                projected_water.terrain = Some(terrain);
                data.manifest.palette.push(PaletteEntry {
                    key: "oteryn:terrain.tibia.i13988".into(),
                    family: Family::Terrain,
                    id: reference("oteryn:item.tibia.i13988").ok_or("spent desert water")?.get()-1,
                    terrain: Some(terrain),
                });
            }
            bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
                &data.manifest,
                &data.sectors,
                &data.spawns,
                &data.npcs,
            )?;
            pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        }
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
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
        let map = checked.boot(deps.admitted.world_id, deps.admitted.channel_id, definitions)
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
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
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
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    floor: -i16::from(start.floor),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.reset_epoch = epoch;
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
        let mut carried = vec![(rod, 1)];
        let bait_count = if matches!(target_id, 12560 | 9582) { 0 } else { 3 };
        if bait_count != 0 {
            carried.push((bait_key, bait_count));
        }
        super::item_look_fixtures::seed_main_backpack(&harness, &carried).await?;
        super::achievement_granters::seed_gathering(&harness).await?;
        if target_id == 9582 {
            return fish_corpse(&owner, &harness, deps.actor, fence, yield_item.ok_or("corpse payout")?).await;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let mut placement = map.base().digest().to_vec();
        let target_pos = crate::map::overlay::TilePos {
            x: start.x + 1,
            y: start.y,
            floor: start.floor,
        };
        placement.extend_from_slice(
            &crate::map::view::placement_key(target_pos, 1)
                .ok_or("target placement")?
                .to_be_bytes(),
        );
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The wire helper resolves the current rod handle from the inventory snapshot.
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target.clone())),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;

        // A pickup may already be prepared when the tool changes/removes the map item.
        let mut stale_pickup = if target_id == 7441 {
            let definition = crate::gameplay_transport::item_look::typed_definition(
                index,
                reference(water).ok_or("ice")?,
            )
            .ok_or("ice definition")?;
            let request = crate::durability::map_item_mint::MapItemMintRequest {
                flat_crate: None,
                key_number: None,
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(999).map_err(debug)?,
                ),
                base_bundle_digest: map.base().digest(),
                placement_key: crate::map::view::placement_key(target_pos, 1).ok_or("placement")?,
                reset_epoch: 0,
                item: crate::gameplay_transport::item_ref_admission::generation_item_facts(
                    native,
                    &definition,
                )
                .map_err(debug)?,
                quantity: 1,
                content_revision: format!(
                    "sha256:{}",
                    native
                        .source_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                    .into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            };
            Some(
                owner
                    .root
                    .freeze_map_item_mint(owner.character, owner.holder, fence, request)
                    .await
                    .map_err(debug)?,
            )
        } else {
            None
        };
        eprintln!(
            "FIXM09_PG_START tool={tool} target={target_id} after={after:?} yield={yield_item:?}"
        );
        let started = std::time::Instant::now();
        let session = admitted.game_session_id;
        // Resume after a valid prior handle history. The next inventory snapshot issues the
        // carried tool at the third handle, as in the existing fishing fixture. Command 1
        // remains the first command; only the server-issued view handles vary.
        let counter = (0_u64..100_000)
            .find(|counter| {
                let mut candidate = intent.clone();
                candidate.target = UseTarget::Item(
                    std::num::NonZeroU64::new(counter + 3).unwrap_or(std::num::NonZeroU64::MAX),
                );
                let Ok(actual) = encode_item_use_intent(&candidate) else {
                    return false;
                };
                use sha2::{Digest, Sha256};
                let digest = Sha256::new()
                    .chain_update(b"oteryn.fishing.catch.v1")
                    .chain_update(session.as_bytes())
                    .chain_update(1_u64.to_be_bytes())
                    .chain_update(&actual)
                    .finalize();
                let mut occurrence = [0; 16];
                occurrence.copy_from_slice(&digest[..16]);
                let draw = oteryn_simulation_determinism::deterministic_decision_u64(
                    &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                        native.source_digest(),
                    ),
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),
                    "fishing.catch",
                    0,
                )
                .ok();
                let catch_roll = draw
                    .map(|draw| {
                        u32::try_from(crate::spell::uniform_draw(draw, 1, 100_000)).unwrap_or(0)
                    })
                    .unwrap_or(0);
                let roll = if target_id == 12560 {
                    let digest = Sha256::new()
                        .chain_update(b"oteryn.gathering.yield.v1")
                        .chain_update(session.as_bytes())
                        .chain_update(1_u64.to_be_bytes())
                        .chain_update(&actual)
                        .chain_update(native.source_digest())
                        .finalize();
                    occurrence.copy_from_slice(&digest[..16]);
                    oteryn_simulation_determinism::deterministic_decision_u64(
                        &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(digest.into()),
                        oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),
                        "gathering.yield",
                        0,
                    )
                    .map_or(0, |draw| u32::try_from(draw % 600 + 1).unwrap_or(0))
                } else {
                    crate::gameplay_transport::item_craft::gathering_roll(
                        native.source_digest(), session, 1, &actual,
                    )
                    .unwrap_or(0)
                };
                if let Some(select_roll) = select_roll {
                    return select_roll(catch_roll, roll);
                }
                let attempt = oteryn_gameplay_items::fishing::gathering_attempt(
                    tool, target_id, 10, catch_roll, roll,
                );
                if !committed {
                    attempt.is_none()
                } else {
                    attempt.is_some_and(|p| p.after == after && p.yield_item == yield_item)
                }
            })
            .ok_or("deterministic handle history exists")?;
        admitted.continuity.item_view.handle_counter = counter;
        let command_id = 1;
        let (next, result, actual) =
            exchange(&owner, admitted, command_id, &payload, reference(rod)).await?;
        let actual_intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            &[4, 6, 11, 15, 18],
            &actual,
        )
        .map_err(debug)?;
        assert_eq!(
            actual_intent.target,
            UseTarget::Item(std::num::NonZeroU64::new(counter + 3).ok_or("issued source handle")?)
        );
        assert_eq!(
            result,
            if committed {
                UseDisposition::Committed
            } else {
                UseDisposition::NothingToUse
            }
        );
        let state: Option<i64> =
            sqlx::query_scalar("SELECT state_item_id FROM game_terrain_tool_transforms")
                .fetch_optional(&harness.pool)
                .await?;
        assert_eq!(state, committed.then(|| after.map_or(0, i64::from)));
        let map = owner.bundle_map().ok_or("bundle map")?;
        let tile = crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos)
            .map_err(debug)?
            .ok_or("target tile")?;
        if let Some(appearance) = after.or((!committed).then_some(target_id)) {
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == appearance));
        } else {
            assert!(!tile.entries.iter().any(|e| e.source.key()
                == crate::map::view::MapEntryKey::Base(
                    crate::map::view::placement_key(target_pos, 1).unwrap_or(0)
                )));
        }
        let minted: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(jsonb_array_length(outputs)), 0)::bigint FROM game_item_craft_records",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(minted, i64::from(yield_item.is_some()));
        let inventory_yield: Option<i64> = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(format!("oteryn:item.tibia.i{}", yield_item.unwrap_or(5878))).fetch_one(&harness.pool).await?;
        assert_eq!(inventory_yield, yield_item.map(|_| 1));
        let tool_quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(rod).fetch_one(&harness.pool).await?;
        assert_eq!(tool_quantity, 1);
        let bait: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1",
        )
        .bind(bait_key)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(bait, bait_count - i64::from(consumes_bait));
        let earned = match (tool, target_id, yield_item) {
            (5908, 5969, Some(5878)) => Some("oteryn:achievement/skin_deep"),
            (5942, 5995, Some(5906)) => Some("oteryn:achievement/ashes_to_dust"),
            (3483 | 9306, 7236, Some(7158 | 3580 | 7159)) => {
                Some("oteryn:achievement/exquisite_taste")
            }
            (3483 | 9306, 7236 | 4597 | 13988, Some(3578)) => Some("oteryn:achievement/here_fishy_fishy"),
            _ => None,
        };
        let achievement = super::achievement_granters::assert_gathering(&harness, earned).await?;
        if committed {
            let (_, replay, _) = exchange(&owner, next, command_id, &actual, None).await?;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            owner.refresh_terrain_tools().await;
        }
        assert_eq!(
            achievement,
            super::achievement_granters::assert_gathering(&harness, earned).await?
        );
        if let Some(candidate) = &mut stale_pickup {
            let permit = owner.spell_lane_permit().await.ok_or("pickup lane")?;
            let outcome = owner
                .root
                .commit_map_item_mint(&permit, owner.character, owner.holder, fence, candidate)
                .await;
            assert!(
                matches!(
                    outcome,
                    Err(crate::durability::map_item_mint::MapItemMintError::Refused(
                        crate::durability::map_item_mint::MapItemMintRefusal::AlreadyTaken
                    ))
                ),
                "stale pickup: {outcome:?}"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_item_mint_receipts")
                    .fetch_one(&harness.pool)
                    .await?,
                0
            );
            if after == Some(7442) {
                let mut request = candidate.request().clone();
                request.command = crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(1000).map_err(debug)?,
                );
                let definition = crate::gameplay_transport::item_look::typed_definition(
                    index,
                    reference("oteryn:item.tibia.i7442").ok_or("sculpture ref")?,
                )
                .ok_or("sculpture definition")?;
                request.item =
                    crate::gameplay_transport::item_ref_admission::generation_item_facts(
                        native,
                        &definition,
                    )
                    .map_err(debug)?;
                let mut fresh = owner
                    .root
                    .freeze_map_item_mint(owner.character, owner.holder, fence, request)
                    .await
                    .map_err(debug)?;
                assert!(matches!(
                    owner
                        .root
                        .commit_map_item_mint(
                            &permit,
                            owner.character,
                            owner.holder,
                            fence,
                            &mut fresh
                        )
                        .await
                        .map_err(debug)?,
                    crate::durability::map_item_mint::MapItemMintOutcome::Committed(_)
                ));
                let key: String = sqlx::query_scalar(
                    "SELECT definition_production_key FROM game_map_item_mint_receipts",
                )
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(key, "oteryn:item.tibia.i7442");
            }
        }
        if sewer && committed {
            let schedule: (Vec<i64>, Vec<i64>) = sqlx::query_as(
                "SELECT decay_states,decay_delays FROM game_terrain_tool_transforms",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(schedule, (vec![4597], vec![1_200_000]));
            // Receipt replay preserves the rod and consumes exactly one nail.
            let bait: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1",
            )
            .bind(bait_key)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(bait, 2);
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                .execute(&harness.pool)
                .await?;
            owner.refresh_terrain_tools().await;
            let map = owner.bundle_map().ok_or("bundle map")?;
            let tile =
                crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos)
                    .map_err(debug)?
                    .ok_or("tile")?;
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 4597));
        }
        if target_id == 7236 && committed {
            let schedule: (Vec<i64>, Vec<i64>) = sqlx::query_as(
                "SELECT decay_states,decay_delays FROM game_terrain_tool_transforms",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(schedule, (vec![7200], vec![900_000])); // Crystal items.xml:18879-18880.
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                .execute(&harness.pool)
                .await?;
            owner.refresh_terrain_tools().await;
            let map = owner.bundle_map().ok_or("bundle map")?;
            let tile =
                crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos)
                    .map_err(debug)?
                    .ok_or("tile")?;
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 7200));
        }
        eprintln!(
            "FIXM09_PG_END tool={tool} target={target_id} duration={:?}",
            started.elapsed()
        );
        eprintln!(
            "POSTGRES_CASE_COMPLETED P11 gathering tool={tool} target={target_id} earned={earned:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn fish_corpse(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    expected_yield: u16,
) -> TestResult {
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
        ItemMintRequest, TypedDefinitionRef,
    };
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_view::ItemKey;
    use std::num::NonZeroU64;

    super::maintain_ready(owner.root).await?;
    let inventory = owner.root.read_item_use_inventory(owner.character, owner.holder, fence).await?;
    let rod = inventory.backpack.iter().flat_map(|bag| &bag.entries)
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i3483")
        .ok_or("carried rod")?.item.item_instance_id;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i9582".into(),
        revision_ref: "definition-r1".into(),
    };
    let native = owner.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("active loot table")?;
    let race = native.reward_table().row("oteryn:creature.water_elemental").map_err(debug)?
        .race.clone().ok_or("water elemental race")?;
    let cause = ItemMintCause::for_test(
        owner.world_id, owner.channel_id, fence.scope_ownership_generation, 819, 1,
        definition.clone(), CORPSE_MATERIALIZATION_PURPOSE_KEY.into(), 0,
    ).with_corpse_race(race).map_err(debug)?;
    let mut candidate = owner.root.freeze_item_mint(owner.character, owner.holder, ItemMintRequest {
        cause,
        item: definition,
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: crate::durability::map_item_mint_audit::map_item_spatial_position(7, 0, -7),
            corpse_ref: vec![5; 16],
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: vec![6; 16],
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }).await?;
    let permit = candidate.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(minted) = owner.root.commit_corpse_mint(
        &permit, owner.character, &harness.node, &mut candidate, id(41),
    ).await? else {
        return Err("water elemental corpse was not minted".into());
    };
    drop(permit);
    let corpse = minted.item_instance_id;
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(NonZeroU64::new(2).ok_or("target handle")?)),
        use_with_position: None,
    };
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};
    let command_id = (940_u64..4_000)
        .find(|candidate| {
            let digest = Sha256::new()
                .chain_update(fence.game_session_id.as_bytes())
                .chain_update(candidate.to_be_bytes())
                .chain_update(corpse)
                .finalize();
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&digest[..16]);
            // Last of the five ordinary trash outcomes: 90% / 5 of all draws.
            deterministic_decision_u64(
                &GameplayDecisionRoot::from_bytes(native.source_digest()),
                DecisionOccurrenceId::from_bytes(occurrence),
                "oteryn.corpse-fishing.v1",
                0,
            ).is_ok_and(|roll| roll % 6000 >= 4920)
        }).ok_or("deterministic trash-tier corpse draw")?;
    let command = UseCommand {
        game_session_id: fence.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let first = owner.use_cooking_conversion(
        actor, command, intent.clone(), Some(ItemKey::Instance(rod)), Some(ItemKey::Instance(corpse)),
    ).await.ok_or("corpse fishing disposition")?;
    assert_eq!(first.disposition, UseDisposition::Committed);
    let replay = owner.use_cooking_conversion(actor, command, intent, None, None)
        .await.ok_or("corpse fishing replay disposition")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    let (key, revision): (String, String) = sqlx::query_as(
        "SELECT definition_production_key,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(corpse.as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(key, "oteryn:item.tibia.i9583");
    assert_eq!(revision, "2");
    let craft_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_craft_records WHERE NOT consume_source",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(craft_count, 1);
    let outputs: serde_json::Value = sqlx::query_scalar(
        "SELECT outputs FROM game_item_craft_records WHERE recipe='oteryn:fishing.corpse'",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(outputs.as_array().ok_or("corpse craft outputs")?.len(), 1);
    assert_eq!(outputs[0]["key"], format!("oteryn:item.tibia.i{expected_yield}"));
    Ok(())
}

async fn river_reset(h: &Harness, world: crate::foundation::WorldId) -> TestResult<u64> {
    use oteryn_gameplay_world_change::variants::Selected;
    let epoch = (1..1_000).find(|epoch| Selected::roll(world.as_bytes(), *epoch)
        .is_ok_and(|selected| selected.active().any(|(definition, _)| definition.key == "river_runs_deep")))
        .ok_or("flowing Zao river epoch")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'1970-01-01T06:00:00Z','1970-01')")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7_u8;32].as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&mut *tx).await?;
    crate::durability::world_event::changes::finalize_reset_for_test(&mut tx, world, epoch).await?;
    tx.commit().await?;
    Ok(epoch)
}
