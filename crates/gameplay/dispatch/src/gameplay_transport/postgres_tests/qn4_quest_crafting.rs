//! QN4: quest carried recipes through the production craft owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[test]
fn qn4_cursed_crystal_two_ingredients_atomic_replay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "quest craft panic")??;
    Ok(())
}
#[test]
fn qn4_cursed_crystal_final_recipe_progress_and_selected_input_replay_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "final quest recipe panic")??;
    Ok(())
}
fn scenario(final_recipe: bool) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(
            admin,
            if final_recipe {
                "qn4_cursed_final"
            } else {
                "qn4_cursed_recipe"
            },
            true,
            false,
        )
        .await?;
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
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
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
        if final_recipe {
            use oteryn_gameplay_quests::quest::{
                QuestStateCatalogue, QuestTrack, reward_callbacks,
            };
            let recipes: Vec<_> = reward_callbacks::load()?
                .into_iter()
                .filter(|c| c.key.starts_with("oteryn:quest-reward/m20/"))
                .collect();
            let transitions = recipes
                .iter()
                .map(reward_callbacks::primary_transition)
                .collect::<Result<Vec<_>, _>>()?;
            owner.quest_catalogue = Some(std::sync::Arc::new(
                QuestStateCatalogue::new(
                    "m20-cursed",
                    vec![QuestTrack {
                        key: "oteryn:quest-progress/quest/u10_70/the_cursed_crystal/questline"
                            .into(),
                        quest: "oteryn:quest.tibia_tales".into(),
                        initial: -1,
                        min: -1,
                        max: 4,
                    }],
                    transitions,
                )
                .map_err(debug)?,
            ));
        }
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
        eprintln!("QN4_POSTGRES_READY cursed_crystal final={final_recipe}");
        if final_recipe {
            owner
                .refresh_quest_session(fence.game_session_id, true)
                .await;
        }
        let entries = if final_recipe {
            vec![
                ("oteryn:item.tibia.i21505", 1),
                ("oteryn:item.tibia.i21507", 3),
                ("oteryn:item.tibia.i21505", 1),
            ]
        } else {
            vec![
                ("oteryn:item.tibia.i11466", 1),
                ("oteryn:item.tibia.i21504", 1),
            ]
        };
        super::item_look::seed_main_backpack(&harness, &entries).await?;
        let intent = UseIntent {
            target: UseTarget::Item(
                std::num::NonZeroU64::new(if final_recipe { 2 } else { 3 }).ok_or("source")?,
            ),
            use_with: None,
            use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(
                oteryn_protocol_oteryn::item_view::ItemHandle::new(if final_recipe {
                    3
                } else {
                    2
                })
                .ok_or("target")?,
            )),
            use_with_position: None,
        };
        let command = crate::gameplay_transport::connection::UseCommand {
            game_session_id: fence.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let result = owner
            .use_craft(
                deps.actor,
                command,
                intent.clone(),
                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                    if final_recipe { 73 } else { 71 },
                ))),
                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                    72,
                ))),
            )
            .await
            .ok_or("craft route")?;
        assert_eq!(result.disposition, UseDisposition::Committed);
        let replay = owner
            .use_craft(deps.actor, command, intent, None, None)
            .await
            .ok_or("replay route")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        if final_recipe {
            let rows:Vec<(String,i64)>=sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i21505','oteryn:item.tibia.i21507','oteryn:item.tibia.i21506') ORDER BY definition_production_key").fetch_all(&harness.pool).await?;
            assert_eq!(
                rows,
                vec![
                    ("oteryn:item.tibia.i21505".into(), 1),
                    ("oteryn:item.tibia.i21506".into(), 1),
                    ("oteryn:item.tibia.i21507".into(), 2)
                ]
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?,1,"the unselected, lower-ID mixture must survive");
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&harness.pool).await?,0);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/quest/u10_70/the_cursed_crystal/questline'").bind(id(41).as_slice()).fetch_one(&harness.pool).await?,2);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_quest_reward_sources WHERE plan ? 'recipe_source'"
                )
                .fetch_one(&harness.pool)
                .await?,
                1
            );
        } else {
            let rows:Vec<(String,i64)>=sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i11466','oteryn:item.tibia.i21504','oteryn:item.tibia.i21505') ORDER BY definition_production_key").fetch_all(&harness.pool).await?;
            assert_eq!(rows, vec![("oteryn:item.tibia.i21505".into(), 1)]);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
        }
        Ok(())
    })
}
