//! Source encounter creatures retain native HP and settle through the actual combat/XP owners.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, CharacterLease};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;

#[test]
fn questpc5_demon_source_four_native_kills_and_exactly_once_settlement() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "questpc5_demon_combat", false, false)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED demon_source_combat server_version={version}");
        // Admission must retain the accepted canonical facts of these exact
        // disabled loot Items, rather than replacing them with source Unknown.
        for (item, slots) in [(3366, 2), (3414, 1), (7382, 3)] {
            let path = if item == 7382 {
                "items-31000-31499.json"
            } else {
                "items-20500-20999.json"
            };
            let document: serde_json::Value = serde_json::from_slice(&std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/items/definitions")
                    .join(path),
            )?)?;
            let key = format!("oteryn:item.tibia.i{item}");
            let definition = &document["records"]
                .as_array()
                .ok_or("Item records")?
                .iter()
                .find(|r| r["definition"]["identity"]["key"] == key)
                .ok_or("exact Item")?["definition"];
            assert_eq!(
                definition["semantics"]["imbuement"]["value"]["slot_count"],
                serde_json::json!({"state":"KNOWN","value":slots}),
                "accepted canonical slots: {key}"
            );
        }
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        for (item, slots) in [(3366, 2), (3414, 1), (7382, 3)] {
            let key = format!("oteryn:item.tibia.i{item}");
            let policy = native
                .item_policy(&key, "definition-r1")
                .ok_or("native loot Item")?;
            let crate::content::ReferenceItemField::Known(imbuement) =
                &policy.record().semantics.imbuement
            else {
                return Err(format!("accepted native imbuement missing: {key}").into());
            };
            assert_eq!(
                imbuement.slot_count,
                crate::content::ReferenceItemField::Known(slots)
            );
        }
        assert_eq!(
            native
                .reward_table()
                .row("oteryn:creature.demon")
                .map_err(debug)?
                .xp_amount
                .get(),
            6000
        );
        let declaration: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/spawns/spawns-52000-53999.json"
        )))?;
        use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
        let mut sources = Vec::new();
        for record in declaration["records"].as_array().ok_or("spawn records")? {
            let source = &record["declaration"];
            let mut points = Vec::new();
            for point in source["points"].as_array().ok_or("spawn points")? {
                let cell = &point["cell"];
                if point["creature"] != "oteryn:creature.demon"
                    || cell["floor"] != 15
                    || ![
                        (33319, 31573),
                        (33323, 31573),
                        (33320, 31577),
                        (33324, 31576),
                    ]
                    .iter()
                    .any(|(x, y)| cell["x"] == *x && cell["y"] == *y)
                {
                    continue;
                }
                assert_eq!(point["direction"], "north");
                points.push(Point {
                    creature: 0,
                    x: u16::try_from(cell["x"].as_u64().ok_or("spawn x")?)?,
                    y: u16::try_from(cell["y"].as_u64().ok_or("spawn y")?)?,
                    direction: Direction::North,
                    respawn_ms: u32::try_from(point["respawn_ms"].as_u64().ok_or("respawn")?)?,
                    boss_policy: None,
                });
            }
            if !points.is_empty() {
                sources.push(Source {
                    key: source["identity"]["key"]
                        .as_str()
                        .ok_or("source key")?
                        .into(),
                    floor: -15,
                    x: u16::try_from(source["centre"]["x"].as_u64().ok_or("centre x")?)?,
                    y: u16::try_from(source["centre"]["y"].as_u64().ok_or("centre y")?)?,
                    points,
                });
            }
        }
        assert_eq!(sources.iter().map(|s| s.points.len()).sum::<usize>(), 4);
        let world = crate::map::spell_tiles::tests::movement_arena::rectangle(
            (33317, 31571),
            (33326, 31579),
            -15,
        )
        .with_spawns_for_test(Table {
            creatures: vec![Creature {
                key: "oteryn:creature.demon".into(),
                period: Period::All,
            }],
            sources,
        });
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let start = crate::content::NativeEntryStart {
            x: 33318,
            y: 31573,
            floor: 15,
        };
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(deps.admitted.world_id),
                world.map_revision_digest(),
                start,
            )
            .ok_or("bundle combat view")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            activated.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            // Four source creatures, the admitted fixture player and the
            // unchanged activation player's headroom (capacity / 2).
            10,
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
        let spawn_sources = world
            .activation_spawns(|key| {
                let policy = runtime.companion_policy(key).ok()?;
                Some((u64::try_from(policy.maximum_health).ok()?, false))
            })
            .ok_or("source spawn policies")?;
        runtime
            .begin_activation_spawn_sources(spawn_sources)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .realize_activation_spawn_window(0)
                .map_err(debug)?
                .realized,
            4
        );
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.combat_room = Some(room.clone());
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        owner.admit_progression_session(&mut admitted).await;
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("native actor")?;
            state.facts.level = 1_000_000;
        }
        for (ordinal, (x, y)) in [
            (33319, 31573),
            (33323, 31573),
            (33320, 31577),
            (33324, 31576),
        ]
        .into_iter()
        .enumerate()
        {
            // The committed source spawn is the authority for this exact creature and cell.
            assert!(
                declaration["records"]
                    .as_array()
                    .ok_or("spawn records")?
                    .iter()
                    .flat_map(|r| r["declaration"]["points"].as_array().into_iter().flatten())
                    .any(|p| p["cell"]["x"] == x
                        && p["cell"]["y"] == y
                        && p["cell"]["floor"] == 15
                        && p["creature"] == "oteryn:creature.demon"),
                "source Demon cell {x},{y}"
            );
            super::quest_catalogue_chests::relocate_source_actor(
                &owner,
                &admitted,
                deps.actor,
                oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: x - 1,
                    y,
                    floor: -15,
                },
            )
            .await?;
            let target = {
                let runtime = owner.runtime.lock().await;
                let visible = runtime.visible_entities();
                let creature = visible
                    .creatures
                    .iter()
                    .find(|c| {
                        runtime.read_actor_position(c.actor).is_ok_and(|p| {
                            p.position().x == i32::from(x) && p.position().y == i32::from(y)
                        })
                    })
                    .ok_or("source Demon")?;
                assert_eq!(
                    runtime
                        .companion_snapshot(creature.actor)
                        .map_err(debug)?
                        .maximum_health,
                    8200
                );
                EntityRef {
                    identity: creature.actor.placement_identity(),
                    generation: creature.generation,
                }
            };
            {
                let runtime = owner.runtime.lock().await;
                assert_eq!(
                    owner.states.attack.lock().await.set_target(
                        &runtime,
                        Some(&room),
                        deps.actor,
                        admitted.game_session_id,
                        owner.owner_now(),
                        Some(target),
                        ordinal as u64 + 1
                    ),
                    AttackIntentDisposition::Ok
                );
            }
            for swing in 0..64 {
                owner
                    .refresh_attack_weapon(deps.actor, admitted.game_session_id)
                    .await;
                owner.drain_auto_attacks().await;
                if owner
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .has_session(admitted.game_session_id)
                {
                    break;
                }
                assert!(swing < 63, "native Demon must die within bounded swings");
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(Duration::from_secs(3))
                    .ok_or("combat clock")?;
            }
            let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(before, ordinal as i64);
            let mut drained = false;
            for _ in 0..8 {
                if owner.drain_kill_rewards(admitted.game_session_id).await == DrainOutcome::Drained
                {
                    drained = true;
                    break;
                }
                owner.root.maintain_ready_once().await.map_err(debug)?;
            }
            assert!(drained, "actual kill must settle");
            let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(after, ordinal as i64 + 1);
            let corpses: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key=$1 AND r.materialized_at IS NOT NULL AND i.definition_production_key='oteryn:item.tibia.i5995'",
            ).bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY)
                .fetch_one(&h.pool).await?;
            assert_eq!(
                corpses,
                ordinal as i64 + 1,
                "every native kill has its own linked Demon corpse"
            );
            eprintln!(
                "POSTGRES_CASE_OBSERVED source_demon={ordinal} position=({x},{y},15) xp_receipts={after} corpse_receipts={corpses} duration={:?}",
                started.elapsed()
            );
            owner.drain_kill_rewards(admitted.game_session_id).await;
            let replay: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(replay, after);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED demon_source_four_native_kills duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
