//! SUMMER: activated creature profiles and the Mayor through production dispatch.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::creature_damage_spell::SpellSource;
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn summer_mimar_haffar_targeted_mana_drain_commits_through_native_ai() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("summer-mimar-haffar", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(Box::pin(mimar_haffar_mana_drain(admin)))
    })
}

async fn mimar_haffar_mana_drain(admin: String) -> TestResult {
    const CREATURE: &str = "oteryn:creature.mimar_haffar";
    const SOURCE_INDEX: usize = 6;
    const STARTING_MANA: u32 = 2_000;
    const THINK_MS: u64 = crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS;
    const MAX_THINKS: u64 = 192;

    let harness =
        Harness::create_with_admission_seed(admin, "summer_mimar_haffar", false, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == CREATURE)
        .ok_or("Mimar Haffar profile")?;
    let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) = &record
        .behavior
        .as_ref()
        .ok_or("Mimar Haffar behavior")?
        .data
    else {
        return Err("Mimar Haffar behavior kind".into());
    };
    // Crystal 201189f8, bosses/mimar_haffar.lua:101.
    let entry = behavior
        .attacks
        .get(SOURCE_INDEX)
        .ok_or("mana drain entry")?;
    assert_eq!((entry.interval_ms, entry.chance_ppm), (2_500, 120_000));
    let definitions = record.attack_definitions.as_ref().ok_or("attack closure")?;
    let source = SpellSource::from_native(
        &record.profile.target,
        SOURCE_INDEX,
        &definitions.records,
        &definitions.profiles,
        native.source_digest(),
    )
    .map_err(debug)?;
    assert_eq!(source.element(), Some("mana_drain"));
    assert!(source.needs_target && source.area.is_none());
    assert_eq!(
        (source.range_tiles(), source.minimum, source.maximum),
        (7, 400, 800)
    );

    let room = pin.qualified_room().clone();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut deps = Dependencies::with_content(
        channel_pin,
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.combat_room = Some(deps.room.clone());
    let session = deps.admitted.game_session_id;
    let boss = {
        let mut runtime = deps.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        runtime
            .realize_native_qualification_spawn(
                pin,
                CREATURE,
                MovementLocalPosition {
                    x: 2,
                    y: 12,
                    floor: -7,
                },
            )
            .map_err(debug)?
            .actor
    };
    let mut owner = deps.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    owner.clock_origin -= Duration::from_secs(12);
    let first_think = owner.owner_now();
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
        let player = states
            .initialize(
                &runtime,
                deps.actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    // Leave room for the other source attacks during the chance rolls.
                    max_health: 1_000_000,
                    max_mana: STARTING_MANA,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                first_think,
            )
            .ok_or("current player state")?;
        assert_eq!(player.vitals().mana, STARTING_MANA);
    }

    // Keep the activated schedule and chance rolls. Each pass advances one
    // native think interval and reads back the same session's committed vitals.
    for tick in 0..MAX_THINKS {
        let now = SemanticTimeMicros::from_micros(first_think.get() + tick * THINK_MS * 1_000);
        owner.drain_monster_melee_at(now).await;
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.try_lock().map_err(debug)?;
        let player = states
            .get(&runtime, deps.actor, session)
            .ok_or("current player after think")?;
        let vitals = player.vitals();
        assert!(vitals.health > 0, "the source target must survive");
        if vitals.mana != STARTING_MANA {
            let ai = states
                .monster_melee
                .creature_ai
                .state(boss)
                .ok_or("native boss AI state")?;
            assert_eq!(ai.target(), Some(deps.actor));
            assert!((tick + 1) * THINK_MS >= entry.interval_ms);
            let drained = STARTING_MANA
                .checked_sub(vitals.mana)
                .ok_or("the attack must drain mana")?;
            assert!(
                (400..=800).contains(&drained),
                "committed mana loss was {drained}"
            );
            eprintln!(
                "Mimar Haffar drained {drained} mana through native AI ({} -> {})",
                STARTING_MANA, vitals.mana,
            );
            return Ok(());
        }
    }
    Err("Mimar Haffar did not commit a targeted mana drain within 192 native thinks".into())
}

#[test]
fn summer_population_all_six_radiant_rewards_admit_native_corpses() -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("activated generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    // Exercise the manifest-derived table used by the production reward drain.
    // Corpse and loot admission comes from the committed Item definitions; the
    // positive PostgreSQL router cases qualify their actual durable lifecycle.
    for name in [
        "acolyte",
        "inquisitor",
        "paragon",
        "templar",
        "warden",
        "zealot",
    ] {
        let key = format!("oteryn:creature.radiant_{name}");
        let profile = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == key)
            .ok_or("Radiant profile")?;
        assert!(
            profile
                .source_qualification
                .as_deref()
                .is_some_and(|note| { note.contains("not verified against Global") }),
            "{key} must retain its provisional source qualification in the active generation"
        );
        let melee = profile
            .monster_melee
            .as_ref()
            .ok_or("Radiant melee owner binding")?;
        assert_eq!(melee.interval_ms, 2_000);
        assert_eq!(melee.chance_ppm, 1_000_000);
        assert_eq!(melee.minimum, 0);
        assert_eq!(
            melee.source_revision,
            "201189f871d23597b82caf88fd611454a2ddefb8"
        );
        let reward = native.reward_table().row(&key).map_err(debug)?;
        assert!(reward.xp_amount.get() > 0);
        assert!(
            reward
                .corpse_item
                .as_ref()
                .ok_or("actual Summer source corpse")?
                .production_key
                .starts_with("oteryn:item.tibia.i54")
        );
    }
    Ok(())
}

#[test]
fn summer_population_boss_values_use_the_selected_pinned_fallback() -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    for (name, hp, xp) in [
        ("maior_domus", 50_000, 3_500_000),
        ("maior_domus_final", 65_000, 0),
        ("mimar_haffar", 50_000, 3_500_000),
        ("mimar_haffar_final", 65_000, 0),
        ("moonspawn_juggernaut", 6_000, 10_000),
        ("phosphorus", 70_000, 16_000_000),
        ("phosphorus_final", 90_000, 0),
        ("rakesh_moonfang", 25_500, 25_500),
        ("the_moonsnow_magnolia", 52_000, 240_000),
    ] {
        let key = format!("oteryn:creature.{name}");
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == key)
            .ok_or("boss profile")?;
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
        else {
            return Err("boss has no Creature profile".into());
        };
        assert_eq!(profile.health, Some(hp), "{key}");
        assert_eq!(profile.experience, Some(xp), "{key}");
        let note = record
            .source_qualification
            .as_deref()
            .ok_or("boss source qualification")?;
        assert!(note.contains("not verified against Global"));
        assert!(note.contains("201189f871d23597b82caf88fd611454a2ddefb8"));
        assert!(note.contains("BOSSENC-1"));
    }
    Ok(())
}

/// PAR-W16: the activated Summer boss profiles against Crystal `201189f8`
/// `data-global/monster/summer_update_2026/bosses/*.lua`: (hp, xp, speed, armor, defense).
#[test]
fn w16_crystal_summer_boss_parity() -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    for (name, crystal) in [
        ("maior_domus", (50_000, 3_500_000, 180, 85, 90)),
        ("maior_domus_final", (65_000, 0, 190, 95, 100)),
        ("mimar_haffar", (50_000, 3_500_000, 180, 85, 90)),
        ("mimar_haffar_final", (65_000, 0, 190, 95, 100)),
        ("phosphorus", (70_000, 16_000_000, 190, 90, 95)),
        ("phosphorus_final", (90_000, 0, 200, 100, 105)),
        ("rakesh_moonfang", (25_500, 25_500, 180, 85, 90)),
        ("the_moonsnow_magnolia", (52_000, 240_000, 190, 105, 110)),
    ] {
        let key = format!("oteryn:creature.{name}");
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == key)
            .ok_or("boss profile")?;
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
        else {
            return Err("boss has no Creature profile".into());
        };
        let oteryn = (
            profile.health.ok_or("health")?,
            profile.experience.ok_or("experience")?,
            profile.speed.ok_or("speed")?,
            profile.armor.ok_or("armor")?,
            profile.details.as_ref().ok_or("details")?.defense,
        );
        assert_eq!(oteryn, crystal, "{key}");
    }
    Ok(())
}

#[test]
fn summer_population_wayland_has_both_global_placements() -> TestResult {
    let content = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        content.join("world/npc-placements/index.json"),
    )?)?;
    let mut cells = Vec::new();
    for shard in index["shards"].as_array().ok_or("NPC placement shards")? {
        let path = shard
            .as_str()
            .ok_or("placement path")?
            .strip_prefix("content/")
            .ok_or("content path")?;
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(content.join(path))?)?;
        for row in document["records"].as_array().ok_or("placement records")? {
            if row["declaration"]["npc"] == "oteryn:npc.wayland_smythers" {
                let cell = &row["declaration"]["cell"];
                cells.push((
                    cell["x"].as_u64(),
                    cell["y"].as_u64(),
                    cell["floor"].as_i64(),
                    row["declaration"]["direction"].as_str().map(str::to_owned),
                ));
            }
        }
    }
    cells.sort_unstable();
    assert_eq!(
        cells,
        vec![
            (Some(32961), Some(32076), Some(6), Some("north".into())),
            (Some(33955), Some(32642), Some(6), Some("north".into())),
        ]
    );
    Ok(())
}

#[test]
fn summer_population_all_six_execute_exact_pinned_loot_plans_and_replay() -> TestResult {
    use crate::content::creature_reward::{LootTablesSection, SectionRef};
    use oteryn_gameplay_domain::domain::loot_data::{
        LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry,
    };
    use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};
    use std::collections::HashSet;

    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    let section = LootTablesSection::decode(
        include_bytes!("../../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
        native.creature_profiles(),
    )
    .map_err(debug)?;
    let lower = |r: &SectionRef| LootDefinitionRef::new(&r.family, &r.key, &r.revision);
    for name in [
        "acolyte",
        "inquisitor",
        "paragon",
        "templar",
        "warden",
        "zealot",
    ] {
        let key = format!("oteryn:creature.radiant_{name}");
        let binding = section
            .creature_loot
            .iter()
            .find(|r| r.creature.key == key)
            .and_then(|r| r.loot.as_ref())
            .ok_or("Radiant loot binding")?;
        let pinned = section
            .tables
            .iter()
            .find(|t| &t.identity == binding)
            .ok_or("Radiant loot table")?;
        assert_eq!(pinned.algorithm, "IndependentBernoulliPpm");
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: pinned
                .entries
                .iter()
                .map(|e| LootTableEntry {
                    item: lower(&e.item),
                    min_count: e.min_count,
                    max_count: e.max_count,
                    probability_ppm: e.probability_ppm,
                    details: Default::default(),
                })
                .collect(),
        };
        let reference = lower(binding);
        let mut seen = HashSet::new();
        // Exercise the actual planner without changing rare probabilities to guaranteed
        // drops. This proves selection/range/membership and replay, not durable settlement.
        for occurrence in 1..=20_000 {
            let death = LootPlanDeathKey::new(id(42), id(43), 1, occurrence, 1);
            let plan = plan_creature_loot(death, &reference, &table)?;
            assert_eq!(plan, plan_creature_loot(death, &reference, &table)?);
            for entry in &plan.entries {
                let source = table
                    .entries
                    .iter()
                    .find(|e| e.item == entry.item)
                    .ok_or("planner returned an item outside the pinned loot table")?;
                assert!((source.min_count..=source.max_count).contains(&entry.quantity));
                assert_eq!(
                    table
                        .entries
                        .get(usize::try_from(entry.draw_ordinal)?)
                        .map(|e| &e.item),
                    Some(&entry.item),
                );
                seen.insert(entry.item.production_key.clone());
            }
            if seen.len() == table.entries.len() {
                break;
            }
        }
        assert_eq!(
            seen.len(),
            table.entries.len(),
            "{key}: every real loot row executes"
        );
    }
    Ok(())
}

#[test]
fn summer_population_npc_placements_greeting_lore_and_stale_actor_on_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("summer-population-pg", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "summer_population", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let mut owner = deps.owner(&h, &authority);
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let revision_before: i64 = sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let content =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
        let catalogue = NpcDataCatalogue::load(
            content,
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
            content.join("world/npc-placements/index.json"),
        )?)?;
        let mut mayor = None;
        let mut wayland = Vec::new();
        for shard in index["shards"].as_array().ok_or("NPC placement shards")? {
            let path = shard
                .as_str()
                .ok_or("placement path")?
                .strip_prefix("content/")
                .ok_or("content path")?;
            let document: serde_json::Value =
                serde_json::from_slice(&std::fs::read(content.join(path))?)?;
            for row in document["records"].as_array().ok_or("placement records")? {
                if row["declaration"]["npc"] == "oteryn:npc.mayor_pocaro" {
                    assert!(
                        mayor.replace(row["declaration"].clone()).is_none(),
                        "one Mayor placement"
                    );
                } else if row["declaration"]["npc"] == "oteryn:npc.wayland_smythers" {
                    wayland.push(row["declaration"].clone());
                }
            }
        }
        let mayor = mayor.ok_or("Mayor is placed in the committed world family")?;
        let cell = &mayor["cell"];
        let (x, y, floor) = (
            u16::try_from(cell["x"].as_u64().ok_or("x")?)?,
            u16::try_from(cell["y"].as_u64().ok_or("y")?)?,
            -i8::try_from(cell["floor"].as_i64().ok_or("floor")?)?,
        );
        assert_eq!((x, y, floor), (34073, 32504, -6));
        assert_eq!(mayor["direction"], "north");
        wayland.sort_by_key(|row| row["cell"]["x"].as_u64());
        let wayland_cells = wayland
            .iter()
            .map(|row| {
                let cell = &row["cell"];
                (
                    cell["x"].as_u64(),
                    cell["y"].as_u64(),
                    cell["floor"].as_i64(),
                    row["direction"].as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            wayland_cells,
            vec![
                (Some(32961), Some(32076), Some(6), Some("north")),
                (Some(33955), Some(32642), Some(6), Some("north")),
            ]
        );
        let (x, y, floor) = (33955, 32642, -6);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            // Map cell admission is separately proven by the compiler test over the real
            // B3 region. Here the session fixture stands next to that exact placement.
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: i32::from(x) - 1,
                        y: i32::from(y),
                        floor: -i16::from(floor),
                    },
                )
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.wayland_smythers".into(),
                            placements: vec![NpcPlacement {
                                x,
                                y,
                                floor,
                                direction: MovementFacing::North,
                            }],
                        }],
                    },
                    |key| key == "oteryn:npc.wayland_smythers",
                    true,
                )
                .map_err(debug)?;
            let row = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: row.actor.placement_identity(),
                generation: row.generation,
            }
        };
        for (command_id, text, expected) in [
            (1, "hi", Some("Wayland Smythers, in service of science")),
            (2, "danger", Some("humble messenger")),
            (3, "name", Some("Wayland Smythers")),
            (4, "kingdom", None),
            (5, "bye", Some("do be careful")),
        ] {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(2))
                .ok_or("talk clock")?;
            let outcome = owner
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id,
                        item_fence: None,
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert!(!outcome.unknown, "{text}");
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
            if let Some(expected) = expected {
                let conversation = outcome.conversation.ok_or("catalogue reply")?;
                assert!(
                    conversation
                        .lines
                        .iter()
                        .any(|line| line.contains(expected)),
                    "{text}: {:?}",
                    conversation.lines
                );
            } else {
                assert!(
                    outcome.conversation.is_none(),
                    "unsupported multipart lore stays held"
                );
            }
            assert!(outcome.window.is_none(), "lore opens no trade window");
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("talk clock")?;
        let stale = EntityRef {
            generation: npc.generation + 1,
            ..npc
        };
        let outcome = owner
            .npc_talk(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 6,
                    item_fence: None,
                },
                NpcTalkIntent {
                    npc_actor: stale,
                    text: "hi".into(),
                },
            )
            .await;
        assert_eq!(outcome.disposition, NpcIntentDisposition::Stale);
        assert!(outcome.conversation.is_none());
        // The static population addition has no durable quest or economy consequence.
        let revisions: i64 = sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revisions, revision_before);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
