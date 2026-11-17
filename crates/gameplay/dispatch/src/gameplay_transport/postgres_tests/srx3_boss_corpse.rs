//! SRX-3 actual source raid rewardBoss -> corpse -> private material rewards.
//! Source raid coordinates, actors, chances and Items remain unchanged.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, NodeId};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn srx3_actual_grand_mother_raid_private_material_reward_chain_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-source-boss-chain", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "srx3_source_boss_chain", false, false)
                    .await?;
            scenario(&h).await?;
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ACTUAL_BOSS_CHAIN_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn scenario(harness: &Harness) -> TestResult {
    let key = "raid:farmine.draptor";
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let digest = active.identity().server_artifact_digest();
    let key = ProductionKey::new(key).map_err(debug)?;
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let source_raid = catalogue.get(&key).ok_or("catalogue raid")?;
    // The source has both a Dragon/Grand Mother variant and a Draptor-only
    // variant. Geometry is the union of actual source variants, independent of
    // whichever deterministic firing is selected below.
    let mut source_areas = source_raid
        .variants
        .iter()
        .flat_map(|v| &v.waves)
        .flat_map(|wave| &wave.entries)
        .map(|entry| &entry.area);
    let mut area = source_areas.next().ok_or("source raid area")?.clone();
    for source_area in source_areas {
        assert_eq!(source_area.from.floor, area.from.floor);
        assert_eq!(source_area.to.floor, area.to.floor);
        area.from.x = area.from.x.min(source_area.from.x);
        area.from.y = area.from.y.min(source_area.from.y);
        area.to.x = area.to.x.max(source_area.to.x);
        area.to.y = area.to.y.max(source_area.to.y);
    }
    assert_eq!(area.from.floor, area.to.floor);
    let from = (u16::try_from(area.from.x)?, u16::try_from(area.from.y)?);
    let to = (u16::try_from(area.to.x)?, u16::try_from(area.to.y)?);
    let native_floor = i8::try_from(area.from.floor.checked_neg().ok_or("native floor")?)?;
    let world = raid_world_with_reward_chest(
        native,
        deps.admitted.world_id,
        deps.admitted.channel_id,
        from,
        to,
        native_floor,
    )?;
    deps.room = activated.qualified_room().clone();
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err("bundle collision index".into());
    };
    let bundle_digest = index.map().overlay.base().digest();
    // Choose a real deterministic occurrence selecting the boss-bearing source
    // variant. Neither variant composition nor chances/counts/coordinates change.
    let (firing, selected) = (91..=250_u8)
        .find_map(|seed| {
            let firing = id(seed);
            let selected = source_raid.select(bundle_digest, firing).ok()?;
            let has_boss = selected
                .definition
                .waves
                .iter()
                .flat_map(|wave| &wave.entries)
                .any(|entry| entry.creature.as_str() == "oteryn:creature.grand_mother_foulscale");
            has_boss.then_some((firing, selected))
        })
        .ok_or("actual boss-bearing source variant")?;
    let room = deps
        .room
        .bundle_view(
            cells.clone(),
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: area.from.x,
                y: area.from.y,
                floor: area.from.floor,
            },
        )
        .ok_or("bundle combat view")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        digest,
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (area.from.x, area.from.y, area.from.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        256,
        pin.clone(),
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.reset_epoch = 1;
    seed_source_raid(
        harness,
        &owner,
        &pin,
        &key,
        &selected,
        firing,
        bundle_digest,
    )
    .await?;
    super::item_look::seed_main_backpack(harness, &[]).await?;
    let source_boss = "oteryn:creature.grand_mother_foulscale";
    let row = native.reward_table().row(source_boss).map_err(debug)?;
    assert_eq!(row.loot_table.entries.len(), 19);
    let mut progress = crate::gameplay_transport::raid_live::RaidEffectProgress::default();
    owner
        .drive_raid_firing(
            firing,
            &key,
            &catalogue,
            bundle_digest,
            21_000,
            &mut progress,
        )
        .await
        .map_err(|_| "actual source raid driver")?;
    let actor = {
        let runtime = deps.runtime.lock().await;
        runtime
            .raid_creatures(firing)
            .map_err(debug)?
            .iter()
            .find_map(|status| {
                let oteryn_gameplay_entities::carrier::RaidSpawnOutcome::Placed(actor) =
                    status.outcome
                else {
                    return None;
                };
                runtime
                    .companion_snapshot(actor)
                    .ok()
                    .filter(|snapshot| {
                        snapshot.state.policy.definition_key == source_boss
                            && snapshot.state.policy.reward_boss == Some(true)
                    })
                    .map(|_| actor)
            })
            .ok_or("actual source raid Grand Mother Foulscale")?
    };
    settle_source_boss(harness, &owner, deps.admitted, actor).await
}

async fn settle_source_boss(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    let actor = admitted.runtime_actor.ok_or("source participant")?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0;
    let fence = crate::gameplay_transport::item_fence_of(current)
        .ok_or("actual current Character fence")?;
    let mut admitted = admitted;
    admitted.item_fence = Some(fence);
    // Use the actual Character/equipment/cast-facts bootstrap, as a native
    // admission does. A raw empty equipment row omits the source and build
    // facts required by the production kill lane.
    super::srx3_activity_source_fixture::prepare_player(h, owner, &mut admitted, 1999).await?;
    let now = owner.owner_now();
    let batch = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let boss = runtime.companion_snapshot(target).map_err(debug)?;
        assert_eq!(
            boss.state.policy.definition_key,
            "oteryn:creature.grand_mother_foulscale"
        );
        assert_eq!(boss.state.policy.reward_boss, Some(true));
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "srx3:source-raid-boss-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"srx3-source-raid-boss-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: boss.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| effect.target == target
            && effect.health.is_some_and(|health| health.health_after == 0)));
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1_000,
        );
        batch
    };
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let death: Vec<u8> =
        sqlx::query_scalar("SELECT uuid_send(death_id) FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?;
    let death: [u8; 16] = death
        .as_slice()
        .try_into()
        .map_err(|_| "source death UUID")?;
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.death_scope_kind=1 AND g.scope_kind=1").fetch_one(&h.pool).await?, 1);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT (SELECT count(*) FROM game_item_corpse_container_entries)+(SELECT count(*) FROM game_item_nested_entries)").fetch_one(&h.pool).await?, 0, "personal boss loot must not leak into the physical corpse");
    for source_key in ["oteryn:item.tibia.i5920", "oteryn:item.tibia.i5877"] {
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_items WHERE definition_production_key=$1 AND collected_transaction_id IS NULL")
            .bind(source_key).fetch_one(&h.pool).await?, 1, "source guaranteed private material {source_key}");
    }
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                117,
                oteryn_protocol_oteryn::boss_rewards::CAPABILITY_BOSS_REWARDS_V1,
            ],
        )
        .ok_or("native boss reward capability closure")?;
    let all = oteryn_protocol_oteryn::boss_rewards::encode_collect_all_request(
        &oteryn_protocol_oteryn::boss_rewards::CollectAllRequest { death: Some(death) },
    )
    .map_err(debug)?;
    let (status, bytes) = super::boss_rewards::exchange(
        owner,
        &mut admitted,
        1,
        oteryn_protocol_oteryn::boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &all,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    let collected =
        oteryn_protocol_oteryn::boss_rewards::decode_collect_reply(&bytes).map_err(debug)?;
    assert_eq!(
        collected.status,
        oteryn_protocol_oteryn::boss_rewards::BossRewardStatus::Ok
    );
    assert!(collected.moved >= 2);
    assert_eq!(collected.remaining, 0);
    let (status, replay) = super::boss_rewards::exchange(
        owner,
        &mut admitted,
        1,
        oteryn_protocol_oteryn::boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &all,
    )
    .await?;
    assert_eq!(
        status,
        oteryn_protocol_oteryn::CommandStatus::DuplicateReplay
    );
    assert_eq!(replay, bytes);
    let audits: Vec<Vec<u8>> = sqlx::query_scalar("SELECT a.envelope FROM game_boss_reward_item_collections c JOIN game_item_audit_outbox a USING(event_id)").fetch_all(&h.pool).await?;
    assert_eq!(audits.len(), usize::from(collected.moved));
    for audit in audits {
        let (envelope, mint) =
            crate::durability::reward_claim_mint_audit::decode_reward_claim_mint_envelope(&audit)
                .map_err(debug)?;
        assert_eq!(
            envelope.channel_id.as_deref(),
            Some(admitted.channel_id.as_bytes().as_slice())
        );
        assert_eq!(
            mint.source
                .ok_or("source")?
                .boss_reward
                .ok_or("boss private provenance")?
                .death,
            death
        );
    }
    // The same actual committed native death cannot roll or mint another group.
    {
        let mut runtime = owner.runtime.lock().await;
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(!receipt.applied);
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    Ok(())
}

/// Test terrain at the unchanged donor raid coordinates, with a reachable static
/// i19250 chest. As in boss_rewards.rs, the fixture authors its collision facts;
/// it does not admit a mintable chest or replace production map geometry.
fn raid_world_with_reward_chest(
    native: &crate::content::native_gameplay::NativeGameplayState,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    from: (u16, u16),
    to: (u16, u16),
    floor: i8,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{Extent, Sector};
    let index = native.item_index().ok_or("source Item index")?;
    let reference = |key: &str| {
        index
            .definition_ref("Item", key, "definition-r1")
            .ok_or_else(|| format!("source Item {key}"))
    };
    let chest = "oteryn:item.tibia.i19250";
    let gold = "oteryn:item.tibia.i3031";
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (chest, reference(chest)?.get() - 1),
        (gold, reference(gold)?.get() - 1),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    // Creature source placement requires a real source ground ID. The generic
    // boot fixture's terrain:grass is intentionally unknown to spell/monster
    // tiles; use the registered actual source100 Terrain without an Item alias.
    let grass = bundle
        .manifest
        .palette
        .iter_mut()
        .find(|entry| entry.key == "terrain:grass")
        .ok_or("fixture grass Terrain")?;
    grass.key = "oteryn:terrain.tibia.i100".into();

    let chest_palette = u32::try_from(
        bundle
            .manifest
            .palette
            .iter()
            .position(|entry| entry.key == chest)
            .ok_or("chest palette")?,
    )?;
    let ground = bundle
        .sectors
        .iter()
        .flat_map(|sector| &sector.tiles)
        .find(|tile| tile.x == 2 && tile.y == 0)
        .and_then(|tile| tile.items.first())
        .cloned()
        .ok_or("fixture ground")?;
    let mut sectors = std::collections::BTreeMap::new();
    for y in from.1..=to.1 {
        for x in from.0..=to.0 {
            let mut items = vec![ground.clone()];
            if (x, y) == (from.0.saturating_add(1), from.1.saturating_add(1)) {
                items.push(Item {
                    palette: chest_palette,
                    depth: 0,
                    attrs: Attrs::default(),
                });
            }
            sectors
                .entry((y / 32, x / 32))
                .or_insert_with(Vec::new)
                .push(Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items,
                });
        }
    }
    bundle.sectors = sectors
        .into_iter()
        .map(|((sy, sx), tiles)| Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    bundle.manifest.world = Extent {
        min_x: from.0,
        min_y: from.1,
        max_x: to.0.saturating_add(1),
        max_y: to.1.saturating_add(1),
        floors: vec![floor],
    };
    let bytes = oteryn_world_bundle::writer::write_with_npcs(
        &bundle.manifest,
        &bundle.sectors,
        &bundle.spawns,
        &bundle.npcs,
    )
    .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: from.0,
                y: from.1,
                floor,
            },
        },
        world,
        channel,
        |key| {
            if key == chest {
                return Some(crate::map::facts::ItemDefinition {
                    reference: reference(key).ok()?,
                    solid: Some(true),
                    blocks_projectile: true,
                    pickupable: false,
                    door: false,
                });
            }
            let policy = native.item_policy(key, "definition-r1")?.record();
            let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical
            else {
                return None;
            };
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key).ok()?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                door: false,
            })
        },
    )
    .map_err(debug)
}

async fn seed_source_raid(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    pin: &ChannelContentPin,
    key: &ProductionKey,
    selected: &oteryn_gameplay_encounter_rules::raid::SelectedRaid,
    firing: [u8; 16],
    bundle: [u8; 32],
) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let map_revision = format!(
        "sha256:{}",
        pin.map_revision_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(owner.world_id.as_bytes().as_slice()).bind(bundle.as_slice()).bind(map_revision)
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).bind(firing.as_slice())
        .bind(i64::from(selected.definition.waves.iter().map(|wave| wave.delay_ms).max().ok_or("source wave")?)
            +i64::from(selected.definition.despawn_after_ms.unwrap_or(0))).bind(bundle.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    Ok(())
}
