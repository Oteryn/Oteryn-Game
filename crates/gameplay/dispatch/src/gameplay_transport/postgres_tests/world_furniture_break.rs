use super::fixture::{Harness, debug, id, run};
use super::item_look_fixtures::seed_main_backpack;
use super::rust_lottery_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, GameSessionId, MovementLocalPosition,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::boot::{BootPins, native_item_definition};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, RoofFacts, compose_tile, placement_key};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

const WEAPON: &str = "oteryn:item.tibia.i3280";
const BAR_POS: TilePos = TilePos {
    x: 7,
    y: 0,
    floor: -7,
};

#[test]
fn world_wooden_bars_break_replay_and_restore_postgres() -> TestResult {
    run_case("break_replay_and_restore", || {
        scenario(2295, Case::Replay)?;
        scenario(2296, Case::Replay)
    })
}

#[test]
fn world_furniture_break_refuses_bound_or_distant_target_postgres() -> TestResult {
    run_case("refuses_bound_or_distant_target", || {
        scenario(2295, Case::Refusals)
    })
}

#[test]
fn world_wooden_bars_break_crystal_variants_postgres() -> TestResult {
    run_case("crystal_variants", || {
        for bar in [2291, 2292, 2293, 2294] {
            scenario(bar, Case::Replay)?;
        }
        Ok(())
    })
}

#[test]
fn world_wooden_bars_break_with_equipped_weapon_postgres() -> TestResult {
    run_case("equipped_weapon", || scenario(2295, Case::Equipped))
}

#[test]
fn world_wooden_bars_break_projects_spell_tile_postgres() -> TestResult {
    run_case("spell_tile", || scenario(2295, Case::SpellTile))
}

#[test]
fn world_wooden_bars_restore_transparent_but_solid_postgres() -> TestResult {
    run_case("restored_sight", || scenario(2295, Case::Sight))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Replay,
    Refusals,
    Equipped,
    SpellTile,
    Sight,
}

fn run_case(label: &str, case: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || case().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "world furniture break scenario panicked")??;
    eprintln!(
        "WORLD_FURNITURE_BREAK_PG_EXECUTED {label} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn scenario(bar_id: u16, case: Case) -> TestResult {
    run(async move |admin| exercise(admin, bar_id, case).await)
}

async fn exercise(admin: String, bar_id: u16, case: Case) -> TestResult {
    let refusals = case == Case::Refusals;
    let harness =
        Harness::create_with_admission_seed(admin, "world_furniture_break", true, false).await?;
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
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let index = native.item_index().ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let bar = format!("oteryn:item.tibia.i{bar_id}");
    let bar_reference = reference(&bar).ok_or("bar definition")?;
    let bag = "oteryn:item.tibia.i2854";
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (bag, reference(bag).ok_or("bag definition")?.get() - 1),
        (&bar, bar_reference.get() - 1),
    );
    let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
    // Use a served ground source identity so the real bundle spell reader qualifies the tile.
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i103".into();
    let collision = crate::map::facts::wooden_bar_collision(bar_id).ok_or("pinned bar facts")?;
    data.manifest.palette[4].terrain = Some(oteryn_world_bundle::bundle::Terrain {
        kind: collision.terrain_kind.ok_or("pinned bar terrain")?,
        walkable: Some(!collision.block_solid),
        ground_speed: None,
        blocks_projectile: Some(collision.blocks_projectile),
    });
    if refusals {
        let tiles = &mut data.sectors[0].tiles;
        let mut distant = tiles.last().ok_or("bars tile")?.clone();
        distant.x = 9;
        let mut unique = tiles.last().ok_or("bars tile")?.clone();
        unique.y = 1;
        unique.items.last_mut().ok_or("unique bars")?.attrs.unique = Some(401);
        tiles
            .last_mut()
            .ok_or("bars tile")?
            .items
            .last_mut()
            .ok_or("action bars")?
            .attrs
            .action = Some(101);
        tiles.extend([distant, unique]);
        tiles.sort_by_key(|tile| (tile.y, tile.x));
    }
    let (bytes, digest) = fixture_bundle(
        data,
        native.source_digest(),
        deps.admitted.game_session_id,
        refusals,
        matches!(bar_id, 2291 | 2292),
    )?;
    pins.digest = digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let boot_pins = BootPins {
        map_revision: crate::map::overlay::map_revision(&base),
        bundle: pins,
        start: TilePos { x: 6, ..BAR_POS },
    };
    let boot = || {
        crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| native_item_definition(Some(native), key),
        )
        .map_err(debug)
    };
    let map = boot()?;
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

    let mut admitted = deps.admitted;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0;
    admitted.item_fence =
        Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 15, 18])
            .ok_or("terrain offers")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
    seed_main_backpack(&harness, &[(WEAPON, 1)]).await?;
    if case == Case::Equipped {
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("WITH weapon AS (DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid RETURNING character_id,item_instance_id,world_id,placed_transaction_id) INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) SELECT character_id,5,item_instance_id,world_id,placed_transaction_id FROM weapon")
            .bind(id(41).as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }

    let weapon = reference(WEAPON).ok_or("weapon definition")?;

    if refusals {
        for (pos, expected) in [
            (BAR_POS, UseDisposition::NothingToUse),
            (TilePos { y: 1, ..BAR_POS }, UseDisposition::NothingToUse),
            (TilePos { x: 9, ..BAR_POS }, UseDisposition::TooFar),
        ] {
            let (key, target) = target(&base, pos)?;
            let command_id = admitted.continuity.next_command_id;
            let (next, result, _) = exchange(
                &owner,
                admitted,
                command_id,
                &payload(target)?,
                Some(weapon),
                None,
            )
            .await?;
            assert_eq!(result, expected, "refusal at {pos:?}");
            assert_eq!(map.facts().pinned().object_revision(key), 0);
            assert!(!map.enterable(pos));
            admitted = next;
        }
        let transforms: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_terrain_tool_transforms")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(transforms, 0);
        assert_eq!(receipts(&harness).await?, 0);
        return Ok(());
    }

    let (key, initial_target) = target(&base, BAR_POS)?;
    assert!(!map.enterable(BAR_POS));
    let cell = crate::content::LogicalCell { x: 7, y: 0, z: 7 };
    if case == Case::SpellTile {
        assert!(
            cells
                .spell_tiles()
                .lookup(cells.scope(), cell)
                .map_err(debug)?
                .flags()
                .block_solid
        );
    }
    let (_, result, wire_payload) = exchange(
        &owner,
        admitted,
        1,
        &payload(initial_target)?,
        Some(weapon),
        None,
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "eligible bars must break on a successful draw"
    );
    if case == Case::SpellTile {
        let tile = cells
            .spell_tiles()
            .lookup(cells.scope(), cell)
            .map_err(debug)?;
        assert!(
            !tile.flags().block_solid,
            "the field creation reader must see the open passage"
        );
        assert_eq!(tile.top_source_ids(), &[3145]);
    }
    let (debris, intact) = if matches!(bar_id, 2291 | 2292) {
        (3146, 2296)
    } else {
        (3145, 2295)
    };
    let state: (i64, i64, Vec<i64>, Vec<i64>, bool) = sqlx::query_as(
        "SELECT state_item_id, object_revision, decay_states, decay_delays, decay_at_unix_ms > floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_terrain_tool_transforms"
    ).fetch_one(&harness.pool).await?;
    assert_eq!(
        state,
        (
            i64::from(debris),
            1,
            vec![i64::from(intact)],
            vec![120_000],
            true
        )
    );
    assert_eq!(receipts(&harness).await?, 1);
    assert_eq!(map.facts().object_revision(key), 1);
    assert!(map.enterable(BAR_POS));
    assert_eq!(appearance(&map, bar_reference)?, debris);

    let restored = boot()?;
    let restored_cells = restored
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut restarted = deps.owner(&harness, &authority);
    restarted.active_generation = Some(active);
    restarted.movement_cells = (&restored_cells).into();
    deps.initialize(&restarted).await;
    assert_eq!(restored.facts().object_revision(key), 0);
    let (next, replay, _) = exchange(&restarted, admitted, 1, &wire_payload, None, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(receipts(&harness).await?, 1);
    restarted.refresh_terrain_tools().await;
    assert_eq!(restored.facts().object_revision(key), 1);
    assert_eq!(appearance(&restored, bar_reference)?, debris);
    assert!(restored.enterable(BAR_POS));

    sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
        .execute(&harness.pool)
        .await?;
    restarted.refresh_terrain_tools().await;
    let restored_state: (i64, i64, bool) = sqlx::query_as(
        "SELECT state_item_id, object_revision, decay_at_unix_ms IS NULL FROM game_terrain_tool_transforms"
    ).fetch_one(&harness.pool).await?;
    assert_eq!(restored_state, (i64::from(intact), 2, true));
    assert_eq!(restored.facts().object_revision(key), 2);
    assert_eq!(appearance(&restored, bar_reference)?, intact);
    assert!(!restored.enterable(BAR_POS));
    if case == Case::SpellTile {
        let tile = restored_cells
            .spell_tiles()
            .lookup(restored_cells.scope(), cell)
            .map_err(debug)?;
        assert!(tile.flags().block_solid);
        assert_eq!(tile.top_source_ids(), &[2295]);
    }
    if case == Case::Sight {
        let facts = restored.facts().pinned();
        let tile = compose_tile(restored.overlay(), &facts, BAR_POS).map_err(debug)?;
        assert!(
            RoofFacts::of(tile.as_ref()).look_through,
            "restored solid bars must be transparent to the map visibility reader"
        );
    }

    let (_, mut restored_target) = target(&base, BAR_POS)?;
    restored_target.expected_revision = 2;
    let (next, miss, _) = exchange(
        &restarted,
        next,
        2,
        &payload(restored_target)?,
        Some(weapon),
        None,
    )
    .await?;
    assert_eq!(miss, UseDisposition::Committed);
    assert_eq!(restored.facts().object_revision(key), 2);
    assert_eq!(appearance(&restored, bar_reference)?, intact);
    assert_eq!(receipts(&harness).await?, 1, "a miss creates no receipt");

    let late = boot()?;
    let late_cells = late
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut late_owner = deps.owner(&harness, &authority);
    late_owner.active_generation = Some(active);
    late_owner.movement_cells = (&late_cells).into();
    late_owner.refresh_terrain_tools().await;
    assert_eq!(late.facts().object_revision(key), 2);
    assert_eq!(appearance(&late, bar_reference)?, intact);
    assert!(!late.enterable(BAR_POS));
    assert_eq!(receipts(&harness).await?, 1);

    if matches!(bar_id, 2291 | 2292) {
        // The next crystal draw uses the restored 2296, while the immutable map still has
        // 2291/2292 at this placement. That second break must project revision 3 as 3145.
        let (_, mut next_target) = target(&base, BAR_POS)?;
        next_target.expected_revision = 2;
        let (_, result, _) = exchange(
            &restarted,
            next,
            3,
            &payload(next_target)?,
            Some(weapon),
            None,
        )
        .await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(restored.facts().object_revision(key), 3);
        assert_eq!(appearance(&restored, bar_reference)?, 3145);
        assert!(restored.enterable(BAR_POS));
        assert_eq!(receipts(&harness).await?, 2);
    }
    Ok(())
}

fn target(base: &crate::map::WorldBase, pos: TilePos) -> TestResult<(u64, WorldObjectTarget)> {
    let key = placement_key(pos, 1).ok_or("bar placement")?;
    let mut placement = base.digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok((
        key,
        WorldObjectTarget {
            placement,
            expected_revision: 0,
        },
    ))
}

fn payload(target: WorldObjectTarget) -> TestResult<Vec<u8>> {
    encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)
}

fn fixture_bundle(
    mut data: oteryn_world_bundle::bundle::Bundle,
    root: [u8; 32],
    session: GameSessionId,
    refusals: bool,
    rebreak: bool,
) -> TestResult<(Vec<u8>, [u8; 32])> {
    let write = |data: &oteryn_world_bundle::bundle::Bundle| -> TestResult<_> {
        let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
            &data.manifest,
            &data.sectors,
            &data.spawns,
            &data.npcs,
        )?;
        let digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        Ok((bytes, digest))
    };
    if refusals {
        return write(&data);
    }
    // Only a new stream's first command can be replayed here after a fresh map boot.
    // Vary a remote grass tile until command 1 hits and command 2 misses.
    // The sequential decay-chain cases also need command 3 to hit.
    let grass = data.sectors[0].tiles.first().ok_or("grass tile")?.clone();
    data.sectors[0].tiles.push(grass);
    let key = placement_key(BAR_POS, 1).ok_or("bar placement")?;
    for y in 1..32 {
        for x in 1..=4 {
            let extra = data.sectors[0].tiles.last_mut().ok_or("extra grass tile")?;
            (extra.x, extra.y) = (x, y);
            let (bytes, digest) = write(&data)?;
            let mut placement = digest.to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            if hits(root, session, 1, &placement)?
                && !hits(root, session, 2, &placement)?
                && (!rebreak || hits(root, session, 3, &placement)?)
            {
                return Ok((bytes, digest));
            }
        }
    }
    Err("no matching furniture draws".into())
}

fn hits(
    root: [u8; 32],
    session: GameSessionId,
    command_id: u64,
    placement: &[u8],
) -> TestResult<bool> {
    let hash = Sha256::new()
        .chain_update(b"oteryn.world.furniture.destroy.v1")
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .chain_update(placement)
        .finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(hash[..16].try_into()?);
    let draw = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(root),
        occurrence,
        "world.furniture.destroy.roll",
        0,
    )
    .map_err(debug)?;
    // The fire sword's pinned attack is 24. Exclude any first draw that would be resampled.
    assert!(draw < u64::MAX - u64::MAX % 80);
    Ok(draw % 80 < 24)
}

fn appearance(map: &crate::map::boot::BundleWorld, base: std::num::NonZeroU32) -> TestResult<u16> {
    Ok(map
        .facts()
        .pinned()
        .base_entry(BAR_POS, 1, base.get() - 1)
        .ok_or("bar entry")?
        .appearance_id)
}

async fn receipts(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_terrain_tool_receipts")
            .fetch_one(&harness.pool)
            .await?,
    )
}
