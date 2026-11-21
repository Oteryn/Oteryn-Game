use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) fn run(name: &'static str, trigger: Position, expected: Position) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::stateless_scenario(name, trigger, expected)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native Tomb scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

pub(super) fn present_3950() -> TestResult {
    run(
        "l6-tomb-native-present-3950",
        Position {
            x: 33174,
            y: 32694,
            floor: -14,
        },
        Position {
            x: 33182,
            y: 32714,
            floor: -14,
        },
    )
}

pub(super) fn present_3951() -> TestResult {
    run(
        "l6-tomb-native-present-3951",
        Position {
            x: 33396,
            y: 32852,
            floor: -14,
        },
        Position {
            x: 33174,
            y: 32934,
            floor: -15,
        },
    )
}

pub(super) fn present_3952() -> TestResult {
    run(
        "l6-tomb-native-present-3952",
        Position {
            x: 33191,
            y: 32959,
            floor: -15,
        },
        Position {
            x: 33126,
            y: 32591,
            floor: -15,
        },
    )
}

pub(super) fn present_3954() -> TestResult {
    run(
        "l6-tomb-native-present-3954",
        Position {
            x: 33073,
            y: 32781,
            floor: -14,
        },
        Position {
            x: 33041,
            y: 32774,
            floor: -14,
        },
    )
}

pub(super) fn present_3955() -> TestResult {
    run(
        "l6-tomb-native-present-3955",
        Position {
            x: 33103,
            y: 32590,
            floor: -15,
        },
        Position {
            x: 33349,
            y: 32827,
            floor: -14,
        },
    )
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    unique: u16,
    trigger: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    let recipe = oteryn_gameplay_quests::quest::carried_step::RECIPES
        .iter()
        .find(|r| r.unique == unique)
        .ok_or("source carried recipe")?;
    let policy = native
        .item_policy(recipe.item, "definition-r1")
        .ok_or("source key native policy")?;
    assert_eq!(
        policy.record().semantics.charges,
        crate::content::ReferenceItemField::NotApplicable
    );
    let physical =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert!(
        physical.materializable
            && physical.movable
            && physical.inventory_destination
            && physical.ground_destination
    );
    assert_eq!(physical.stack_maximum, 1);
    assert!(physical.container_capacity.is_none());
    let decay = physical.decay.ok_or("source24h key lifetime")?;
    assert_eq!(decay.duration_millis, 86_400_000);
    assert!(
        decay.target.is_none(),
        "source key expires to0, no replacement item"
    );
    let trigger_cell = (trigger.x, trigger.y, u8::try_from(-trigger.floor)?);
    if boss_portal(unique) == Some(trigger_cell)
        && let Some(crops) = boss_portal_crops(unique)
    {
        let world = super::l6_world_pickups_fixtures::native_scene(native, crops, &[])?;
        let portal = boss_portal(unique).ok_or("boss portal")?;
        let key =
            oteryn_world_bundle::bundle::placement_key(-(portal.2 as i8), portal.0, portal.1, 1)
                .ok_or("native portal key")?;
        assert!(
            !world
                .facts()
                .quest_callback_matches(key, Some(unique), None, None)
        );
        assert!(
            world
                .floors()
                .teleport(crate::map::overlay::TilePos {
                    x: portal.0,
                    y: portal.1,
                    floor: -(portal.2 as i8),
                })
                .is_none(),
            "native boss-room forcefield relies on the pass callback"
        );
        return Ok(world);
    }
    use super::l6_world_pickups_fixtures::{
        ExpectedTeleport, ExpectedTile, NativeCrop, native_scene,
    };
    let (bytes, region, portal, success, missing, raw): (&'static [u8], _, _, _, _, _) =
        match unique {
            3953 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z15-x129-y127.b3"
                )),
                (15, 129, 127),
                (33146, 32668),
                (33145, 32665),
                (33025, 32868),
                (33206, 32592),
            ),
            3956 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x129-y128.b3"
                )),
                (14, 129, 128),
                (33187, 33015),
                (33186, 33012),
                (33206, 32592),
                (33250, 32832),
            ),
            _ => return Err("unknown source UID".into()),
        };
    let tile = |position, ids| ExpectedTile {
        position,
        ids,
        counted_entry: None,
    };
    let mut specs = vec![NativeCrop {
        bytes,
        region,
        bounds: (success.0, success.1, portal.0 + 1, portal.1 + 1),
        start: (portal.0, portal.1 - 1),
        expected: vec![
            tile(portal, vec![423, 1949]),
            tile((portal.0, portal.1 - 1), vec![423]),
            tile(success, vec![417]),
        ],
    }];
    for target in std::collections::BTreeSet::from([missing, raw]) {
        let (bytes, region): (&'static [u8], _) = match (target.0 / 256, target.1 / 256) {
            (129, 127) => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x129-y127.b3"
                )),
                (8, 129, 127),
            ),
            (130, 127) => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x130-y127.b3"
                )),
                (8, 130, 127),
            ),
            (129, 128) => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x129-y128.b3"
                )),
                (8, 129, 128),
            ),
            _ => return Err("unknown source destination region".into()),
        };
        specs.push(NativeCrop {
            bytes,
            region,
            bounds: (target.0, target.1, target.0 + 1, target.1 + 1),
            start: target,
            expected: vec![tile(
                target,
                vec![if target == (33133, 32642) { 423 } else { 415 }],
            )],
        });
    }
    let world = native_scene(
        native,
        specs,
        &[ExpectedTeleport {
            position: (portal.0, portal.1, region.0),
            ordinal: 1,
            destination: (raw.0, raw.1, 8),
        }],
    )?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-i8::try_from(region.0)?, portal.0, portal.1, 1)
            .ok_or("native portal key")?;
    assert!(
        !world
            .facts()
            .quest_callback_matches(key, Some(unique), None, None),
        "actual native portal must not acquire a synthetic source UID"
    );
    assert_eq!(
        world.floors().teleport(crate::map::overlay::TilePos {
            x: portal.0,
            y: portal.1,
            floor: -i8::try_from(region.0)?
        }),
        Some(crate::map::overlay::TilePos {
            x: raw.0,
            y: raw.1,
            floor: -8
        }),
        "raw native teleport is retained, including its reachable destination"
    );
    Ok(world)
}

// The Global spoiler requires the forcefield in the boss room. Crystal startup
// item.lua:4437-4463 supplies the original UID-to-tile join for the retained map.
fn boss_portal(unique: u16) -> Option<(u16, u16, u8)> {
    match unique {
        3950 => Some((33174, 32694, 14)),
        3951 => Some((33396, 32852, 14)),
        3952 => Some((33191, 32959, 15)),
        3953 => Some((33195, 33002, 14)),
        3954 => Some((33073, 32781, 14)),
        3955 => Some((33103, 32590, 15)),
        3956 => Some((33116, 32656, 15)),
        _ => None,
    }
}

macro_rules! region {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z",
            $name,
            ".b3"
        ))
    };
}

fn boss_portal_crops(unique: u16) -> Option<Vec<super::l6_world_pickups_fixtures::NativeCrop>> {
    use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop};
    let (x, y, z) = boss_portal(unique)?;
    type PortalCrops = (
        &'static [u8],
        u32,
        &'static [u32],
        (u16, u16, u8),
        &'static [u8],
        (u16, u16, u8),
        &'static [u8],
        u32,
    );
    let (
        portal,
        portal_ground,
        north,
        reward,
        reward_region,
        destination,
        exit_region,
        exit_ground,
    ): PortalCrops = match unique {
        3950 => (
            region!("14-x129-y127"),
            417,
            &[417, 2190],
            (33182, 32714, 14),
            region!("14-x129-y127"),
            (33231, 32705, 8),
            region!("08-x129-y127"),
            415,
        ),
        3951 => (
            region!("14-x130-y128"),
            4397,
            &[4394],
            (33174, 32934, 15),
            region!("15-x129-y128"),
            (33282, 32744, 8),
            region!("08-x130-y127"),
            415,
        ),
        3952 => (
            region!("15-x129-y128"),
            415,
            &[422],
            (33126, 32591, 15),
            region!("15-x129-y127"),
            (33250, 32832, 8),
            region!("08-x129-y128"),
            415,
        ),
        3953 => (
            region!("14-x129-y128"),
            1236,
            &[415],
            (33145, 32665, 15),
            region!("15-x129-y127"),
            (33025, 32868, 8),
            region!("08-x129-y128"),
            415,
        ),
        3954 => (
            region!("14-x129-y128"),
            423,
            &[423],
            (33041, 32774, 14),
            region!("14-x129-y128"),
            (33133, 32642, 8),
            region!("08-x129-y127"),
            423,
        ),
        3955 => (
            region!("15-x129-y127"),
            418,
            &[418, 3521],
            (33349, 32827, 14),
            region!("14-x130-y128"),
            (33131, 32566, 8),
            region!("08-x129-y127"),
            415,
        ),
        3956 => (
            region!("15-x129-y127"),
            417,
            &[417],
            (33186, 33012, 14),
            region!("14-x129-y128"),
            (33206, 32592, 8),
            region!("08-x129-y127"),
            415,
        ),
        _ => return None,
    };
    let tile = |position, ids| ExpectedTile {
        position,
        ids,
        counted_entry: None,
    };
    let landing = |(x, y, z), bytes, ground| NativeCrop {
        bytes,
        region: (z, x / 256, y / 256),
        bounds: (x, y, x + 1, y + 1),
        start: (x, y),
        expected: vec![tile((x, y), vec![ground])],
    };
    Some(vec![
        // The bundle validator needs a walkable entry. The scenario separately
        // positions the actor one tile north to exercise the actual forcefield.
        landing(reward, reward_region, 417),
        NativeCrop {
            bytes: portal,
            region: (z, x / 256, y / 256),
            bounds: (x, y - 1, x + 1, y + 1),
            start: (x, y - 1),
            expected: vec![
                tile((x, y), vec![portal_ground, 1949]),
                tile((x, y - 1), north.to_vec()),
            ],
        },
        landing(destination, exit_region, exit_ground),
    ])
}

pub(super) async fn hunt_pass(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    h: &super::fixture::Harness,
    recipe: &oteryn_gameplay_quests::quest::carried_step::Recipe,
) -> TestResult {
    let boss = match recipe.unique {
        3950 => "morguthis",
        3951 => "thalas",
        3952 => "mahrdis",
        3953 => "omruc",
        3954 => "rahemos",
        3955 => "dipthrah",
        3956 => "vashresamun",
        _ => return Err("unknown Tomb pass".into()),
    };
    hunt_loot(owner, admitted, h, boss, recipe.item, 1, 20_000).await
}

pub(super) async fn hunt_loot(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    h: &super::fixture::Harness,
    creature: &str,
    item: &str,
    count: u16,
    command: u64,
) -> TestResult {
    use super::fixture::debug;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};

    let creature = format!("oteryn:creature.{creature}");
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay for quest loot")?;
    let reward = native.reward_table().row(&creature).map_err(debug)?;
    let loot = reward
        .loot_table_ref
        .as_ref()
        .ok_or("source loot reference")?;
    assert!(
        reward
            .loot_table
            .entries
            .iter()
            .any(|entry| entry.item.production_key == item),
        "{creature} must admit its source drop {item}"
    );
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    let (health, position) = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let position = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let health = runtime
            .companion_policy(&creature)
            .map_err(debug)?
            .maximum_health;
        (health, position)
    };
    let mut remaining = i64::from(count);
    for ordinal in 0..count {
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let mut selected = None;
            // Pick an exact actor generation whose authored roll drops the item.
            // This keeps the drop chance, quantity and native reward table intact.
            for _ in 0..20_000 {
                let candidate = runtime
                    .admit_encounter_actor(&creature, health, position)
                    .map_err(debug)?;
                let seed = LootPlanDeathKey::new(
                    *candidate.world_id().as_bytes(),
                    *candidate
                        .channel_id()
                        .ok_or("quest hunt Channel")?
                        .as_bytes(),
                    candidate.scope_generation().get(),
                    candidate.actor_local_id(),
                    candidate.actor_local_generation(),
                );
                let plan = plan_creature_loot(seed, loot, &reward.loot_table).map_err(debug)?;
                if plan
                    .entries
                    .iter()
                    .any(|entry| entry.item.production_key == item)
                {
                    runtime
                        .install_creature_policy(candidate, &creature)
                        .map_err(debug)?;
                    selected = Some(candidate);
                    break;
                }
                runtime.remove_encounter_actor(candidate).map_err(debug)?;
            }
            selected.ok_or("bounded source loot occurrence")?
        };
        super::grizzly_npc_fixtures::questpc8_campaign::lethal(
            owner,
            admitted,
            target,
            &creature,
            command,
            ordinal + 1,
        )
        .await?;
        let corpse = {
            let runtime = owner.runtime.lock().await;
            runtime
                .visible_corpses()
                .into_iter()
                .find(|corpse| corpse.identity == target.corpse_identity())
                .ok_or("settled creature corpse")?
                .item
                .item_instance_id
        };
        remaining -= collect_loot(
            owner,
            admitted,
            h,
            corpse,
            item,
            command + u64::from(ordinal) + 1,
        )
        .await?;
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(target)
            .map_err(debug)?;
        if remaining <= 0 {
            return Ok(());
        }
        // Long material hunts run against the real corpse budget. Advance
        // only the fixture clock anchor and let the decay owner free the body.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-61000 WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(corpse.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        owner.drain_corpse_deadlines().await;
    }
    Err(format!("source hunt did not deliver {count} units of {item}").into())
}

async fn collect_loot(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    h: &super::fixture::Harness,
    corpse: [u8; 16],
    item: &str,
    command: u64,
) -> TestResult<i64> {
    use super::fixture::debug;
    use crate::durability::{
        item_mint::TypedDefinitionRef,
        item_transfer::{ItemTransferDestination, ItemTransferRequest},
    };
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};

    let (raw, quantity): (Vec<u8>, i64) = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id),i.quantity::bigint          FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id)          WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key=$2          AND i.lifecycle=1 ORDER BY e.placement_ordinal LIMIT 1",
    )
    .bind(corpse.as_slice())
    .bind(item)
    .fetch_one(&h.pool)
    .await?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay for quest loot")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "quest loot fence")?
        .ok_or("quest loot session")?
        .item_fence();
    let definition = |key: &str| TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    let weights = NativeItemWeights(native);
    super::map_seed_planting_fixtures::maintain_ready(owner).await?;
    let mut candidate = owner
        .root
        .freeze_item_transfer(
            owner.character,
            owner.holder,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(command).map_err(debug)?,
                ),
                source_item_instance_id: raw.try_into().map_err(|_| "corpse item UUID")?,
                destination: ItemTransferDestination::MainBackpack,
                item: generation_item_facts(native, &definition(item)).map_err(debug)?,
                backpack: Some(
                    generation_item_facts(native, &definition("oteryn:item.tibia.i2854"))
                        .map_err(debug)?,
                ),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &weights,
        )
        .await
        .map_err(debug)?;
    super::map_seed_planting_fixtures::maintain_ready(owner).await?;
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("quest loot item lane")?;
    owner
        .root
        .commit_item_transfer(
            &permit,
            owner.character,
            owner.holder,
            fence,
            &mut candidate,
            &weights,
        )
        .await
        .map_err(debug)?;
    Ok(quantity)
}
