//! WVAR: actual native craft owner, item debit, terrain and World credit in one PG commit.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_craft::{GatheringCraft, ItemCraftRequest};
use crate::durability::terrain_tool::{TerrainMutation, TerrainScope};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_items::fishing::GatheringAttempt;
use oteryn_gameplay_world_change::variants;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn wvar_station_native_atomic_debit_replay_and_current_epoch() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "WVAR native station PG panic")??;
    eprintln!(
        "WVAR_STATION_POSTGRES_CASE_EXECUTED native-pump,debit,receipt,replay,rollback,current-epoch; duration={:?}",
        start.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "wvar_station", true, false).await?;
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let native = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("native catalogue")?;
        let weights = NativeItemWeights(native);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice())
            .execute(&h.pool).await?;
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i12600", 3),
                ("oteryn:item.tibia.i3492", 3),
                ("oteryn:item.tibia.i3483", 1),
            ],
        )
        .await?;
        let world = deps.admitted.world_id;
        let epoch = 1_u64;
        seed_epoch(&h, world, epoch).await?;
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("persisted day")?;
        assert!(day.variant("awash").is_some());
        let definition = variants::definitions()?
            .iter()
            .find(|d| d.key == "awash")
            .ok_or("Awash")?;
        let station = definition.stations.first().ok_or("exact source pump")?;
        assert_eq!(station.position, (32558, 32007, 9));
        assert_eq!(station.appearance, 12666);
        let source = native
            .item_policy_for_source_id(12600)
            .ok_or("native coal")?
            .record();
        let source_key = source.production_definition.production_key.clone();
        let source_revision = source.production_definition.revision_ref.clone();
        // The exact source coordinates/ordinal are reused. This owner-level fixture
        // uses an independently fixed map certificate; bundle geometry is tested by
        // the map owner, not represented as an invented map loaded in this test.
        let certificate = [0xA7; 32];
        let mut placement = certificate.to_vec();
        let (x, y, z) = station.position;
        let key = (u64::from(x) << 32) | (u64::from(y) << 16) | (u64::try_from(z)? << 8) | 1;
        placement.extend_from_slice(&key.to_be_bytes());
        assert_eq!(
            variants::station_position(&placement),
            Some(station.position)
        );
        let request = ItemCraftRequest {
            command: CommandRef::new(
                deps.admitted.game_session_id,
                CommandId::new(501).map_err(debug)?,
            ),
            source: id(71),
            tool: None,
            ingredient: None,
            station: None,
            intent: vec![71, 1],
            enchanting: None,
            gathering: Some(GatheringCraft {
                scope: TerrainScope {
                    map_revision: format!("sha256:{}", hex_digest(certificate)),
                    content_generation: native.source_digest(),
                    reset_epoch: i64::try_from(epoch)?,
                },
                mutation: TerrainMutation {
                    tool: id(71),
                    tool_key: source_key,
                    tool_revision: source_revision,
                    placement,
                    expected_revision: 0,
                    before: u32::from(station.appearance),
                    after: u32::from(station.appearance),
                    decay: vec![],
                },
                attempt: GatheringAttempt {
                    after: Some(station.appearance),
                    yield_item: None,
                    consumes_bait: true,
                },
            }),
        };
        let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
            fence.runtime_scope,
        )
        .await
        .map_err(debug)?;
        let before = footprint(&h).await?;
        // Prefix mismatch fails station resolution and cannot debit carried coal.
        let mut invalid = request.clone();
        invalid
            .gathering
            .as_mut()
            .ok_or("gathering")?
            .scope
            .map_revision = format!("sha256:{}", hex_digest([0xA8; 32]));
        assert_ne!(
            h.root
                .craft_item(
                    &permit, &authority, &h.node, fence, invalid, native, &weights, false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(footprint(&h).await?, before);
        assert_eq!(
            h.root
                .craft_item(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    request.clone(),
                    native,
                    &weights,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(quantity(&h, 71).await?, 2);
        assert_eq!(
            scalar(&h, "SELECT count(*) FROM game_item_craft_records").await?,
            1
        );
        assert_eq!(
            scalar(&h, "SELECT count(*) FROM game_terrain_tool_receipts").await?,
            1
        );
        assert_eq!(
            scalar(
                &h,
                "SELECT count(*) FROM game_world_change_contributions WHERE resource='awash_coal'"
            )
            .await?,
            1
        );
        assert_eq!(
            scalar(
                &h,
                "SELECT tally::bigint FROM game_world_change_tallies WHERE resource='awash_coal'"
            )
            .await?,
            1
        );
        // SQL and Rust operation IDs agree because the deferred reverse proof committed.
        assert_eq!(scalar(&h,"SELECT count(*) FROM game_item_craft_records c JOIN game_terrain_tool_receipts r USING(game_session_id,command_id) JOIN game_world_change_contributions w ON w.world_id=c.world_id AND w.operation_id=game_world_change_craft_operation_id(c.game_session_id,c.command_id) WHERE game_world_change_craft_pair(c,r,w)").await?,0,"same-xact predicate deliberately expires after commit");
        let after = footprint(&h).await?;
        assert_eq!(
            h.root
                .craft_item(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    request.clone(),
                    native,
                    &weights,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(footprint(&h).await?, after);
        // The same placement's stale revision must roll back all three owners.
        let mut stale = request.clone();
        stale.command = CommandRef::new(
            deps.admitted.game_session_id,
            CommandId::new(502).map_err(debug)?,
        );
        stale.intent = vec![71, 2];
        assert_ne!(
            h.root
                .craft_item(
                    &permit, &authority, &h.node, fence, stale, native, &weights, false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(footprint(&h).await?, after);
        seed_epoch(&h, world, epoch + 1).await?;
        let mut old = request.clone();
        old.command = CommandRef::new(
            deps.admitted.game_session_id,
            CommandId::new(503).map_err(debug)?,
        );
        old.intent = vec![71, 3];
        old.gathering
            .as_mut()
            .ok_or("gathering")?
            .mutation
            .expected_revision = 1;
        assert_ne!(
            h.root
                .craft_item(
                    &permit, &authority, &h.node, fence, old, native, &weights, false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(footprint(&h).await?, after);
        // Historical exact craft replay remains a receipt lookup after reset.
        assert_eq!(
            h.root
                .craft_item(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    request.clone(),
                    native,
                    &weights,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(footprint(&h).await?, after);
        // River fishing is admitted only at an exact selected fishing-object member.
        let river = variants::definitions()?
            .iter()
            .find(|d| d.key == "river_runs_deep")
            .ok_or("River")?;
        let (river_index, river_variant) = river
            .variants
            .iter()
            .enumerate()
            .find(|(_, v)| {
                v.tiles
                    .iter()
                    .any(|t| t.entries.iter().any(|e| e.0 == 13989))
            })
            .ok_or("active fishing population")?;
        let river_epoch = (100_u64..100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.variant("river_runs_deep") == Some(river_index))
            })
            .ok_or("River save")?;
        seed_epoch(&h, world, river_epoch).await?;
        let objects: Vec<_> = river_variant
            .tiles
            .iter()
            .filter_map(|t| {
                t.entries
                    .iter()
                    .position(|e| e.0 == 13989)
                    .map(|ordinal| (t.position, ordinal))
            })
            .take(2)
            .collect();
        assert_eq!(objects.len(), 2);
        super::achievement_granters::seed_gathering(&h).await?;
        for (index, (position, ordinal)) in objects.into_iter().enumerate() {
            let mut fish =
                request_for_fishing(&request, native, position, ordinal, river_epoch, index)?;
            let before = footprint(&h).await?;
            let mut outside = fish.clone();
            // Same appearance and current epoch cannot substitute an unselected source cell.
            outside
                .gathering
                .as_mut()
                .ok_or("fishing")?
                .mutation
                .placement[32..]
                .copy_from_slice(&((1_u64 << 32) | (1_u64 << 16) | (9_u64 << 8) | 1).to_be_bytes());
            assert_ne!(
                h.root
                    .craft_item(
                        &permit, &authority, &h.node, fence, outside, native, &weights, false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            assert_eq!(footprint(&h).await?, before);
            let bait_before = quantity(&h, 72).await?;
            for wrong_prefix in [false, true] {
                let mut invalid = fish.clone();
                let placement = &mut invalid
                    .gathering
                    .as_mut()
                    .ok_or("fishing")?
                    .mutation
                    .placement;
                if wrong_prefix {
                    placement[0] ^= 1;
                } else {
                    placement[39] = 63;
                }
                assert_ne!(
                    h.root
                        .craft_item(
                            &permit, &authority, &h.node, fence, invalid, native, &weights, false
                        )
                        .await?,
                    Some(UseDisposition::Committed),
                    "invalid source prefix={wrong_prefix}"
                );
                assert_eq!(footprint(&h).await?, before);
                assert_eq!(
                    quantity(&h, 72).await?,
                    bait_before,
                    "invalid operand cannot debit bait"
                );
            }
            fish.intent = vec![91, index as u8];
            let bait = quantity(&h, 72).await?;
            if index == 1 {
                assert!(
                    native.item_policy_for_source_id(13992).is_some(),
                    "13992 desert-fish source policy must be admitted"
                );
            }
            assert_eq!(
                h.root
                    .craft_item(
                        &permit,
                        &authority,
                        &h.node,
                        fence,
                        fish.clone(),
                        native,
                        &weights,
                        false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            assert_eq!(quantity(&h, 72).await?, bait - 1);
            let after = footprint(&h).await?;
            if index == 0 {
                eprintln!(
                    "WVAR_RIVER_EMPTY_CATCH_POSTGRES_EXECUTED source-member,consumed-worm,terrain-receipt"
                );
            }
            assert_eq!(
                h.root
                    .craft_item(
                        &permit, &authority, &h.node, fence, fish, native, &weights, false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            assert_eq!(footprint(&h).await?, after);
        }
        let sandfish:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i13992' AND i.quantity=1 AND i.lifecycle=1 AND e.character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            sandfish, 1,
            "native sandfish is physically carried exactly once"
        );
        super::achievement_granters::assert_gathering(&h, None).await?;
        super::achievement_granters::assert_progress(
            &h,
            "oteryn:achievement/desert_fisher",
            1,
            true,
            "oteryn:item-craft",
        )
        .await?;
        drop(permit);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
fn hex_digest(digest: [u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
async fn scalar(h: &Harness, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(item).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut result = vec![quantity(h, 71).await?];
    for query in [
        "SELECT count(*) FROM game_item_craft_records",
        "SELECT count(*) FROM game_terrain_tool_receipts",
        "SELECT count(*) FROM game_terrain_tool_transforms",
        "SELECT count(*) FROM game_world_change_contributions",
    ] {
        result.push(scalar(h, query).await?);
    }
    Ok(result)
}
async fn seed_epoch(h: &Harness, world: crate::foundation::WorldId, epoch: u64) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).bind((epoch*86400) as f64).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    crate::durability::world_event::changes::record_reset_fixture(&mut tx, world, epoch).await?;
    tx.commit().await?;
    Ok(())
}

fn request_for_fishing(
    base: &ItemCraftRequest,
    native: &crate::content::native_gameplay::NativeGameplayState,
    position: variants::Position,
    ordinal: usize,
    epoch: u64,
    index: usize,
) -> TestResult<ItemCraftRequest> {
    let mut request = base.clone();
    request.command = CommandRef::new(
        base.command.game_session_id(),
        CommandId::new(600 + u64::try_from(index)?).map_err(debug)?,
    );
    request.source = id(72);
    request.tool = Some(id(73));
    let gathering = request.gathering.as_mut().ok_or("gathering fixture")?;
    gathering.scope.reset_epoch = i64::try_from(epoch)?;
    let (x, y, z) = position;
    let key = (u64::from(x) << 32)
        | (u64::from(y) << 16)
        | (u64::try_from(z)? << 8)
        | u64::try_from(ordinal)?;
    gathering.mutation.placement[32..].copy_from_slice(&key.to_be_bytes());
    gathering.mutation.tool = id(73);
    let rod = native
        .item_policy_for_source_id(3483)
        .ok_or("native rod")?
        .record();
    gathering.mutation.tool_key = rod.production_definition.production_key.clone();
    gathering.mutation.tool_revision = rod.production_definition.revision_ref.clone();
    gathering.mutation.expected_revision = 0;
    gathering.mutation.before = 13989;
    gathering.mutation.after = 13988;
    gathering.mutation.decay = vec![(13989, 1_200_000)];
    gathering.attempt = GatheringAttempt {
        after: Some(13988),
        yield_item: if index == 0 { None } else { Some(13992) },
        consumes_bait: true,
    };
    Ok(request)
}
