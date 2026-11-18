use super::super::{Dependencies, TestResult};
use super::{ComposedFreshAdmission, Harness, NativeScenario, debug};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

pub(super) async fn exercise(
    mut owner: ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    harness: &Harness,
    route: NativeScenario,
) -> TestResult {
    let buddel = matches!(route, NativeScenario::Buddel | NativeScenario::BuddelEquipped | NativeScenario::BuddelFare | NativeScenario::BuddelNested);
    let key = if buddel {
        "oteryn:npc.buddel"
    } else {
        "oteryn:npc.iskan"
    };
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
    let session = deps.admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc_actor = {
        let mut runtime = deps.runtime.lock().await;
        let actor = runtime
            .place_dynamic_npc(
                key,
                NpcPlacement {
                    x: 32252,
                    y: 31098,
                    floor: -6,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let placement = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| n.actor == actor)
            .ok_or("placed NPC")?;
        EntityRef {
            identity: actor.placement_identity(),
            generation: placement.generation,
        }
    };
    let tracks = [
        (
            "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
            8_i64,
        ),
        (
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
            2,
        ),
        (
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission01",
            2,
        ),
    ];
    super::fixture::seed_ice_islands_tracks(harness, &tracks).await?;
    owner.refresh_quest_session(session, true).await;
    let mut command_id = 44000;
    let greeting = talk(&mut owner, deps, npc_actor, &mut command_id, "hi").await?;
    assert!(!greeting.is_empty());
    if matches!(route, NativeScenario::BuddelFare) {
        let denied = talk(&mut owner, deps, npc_actor, &mut command_id, "helheim").await?;
        assert_eq!(denied, "Sorry, not possible.");
        let quoted = talk(&mut owner, deps, npc_actor, &mut command_id, "okolnir").await?;
        assert!(quoted.contains("50 gold"));
        for _ in 0..32 {
            let line = talk(&mut owner, deps, npc_actor, &mut command_id, "yes").await?;
            if line == "Set the sails!" {
                break;
            }
            assert_eq!(line, "Sorry, not possible.");
        }
    } else if buddel {
        let ring = talk(&mut owner, deps, npc_actor, &mut command_id, "okolnir").await?;
        assert!(ring.contains("ring"), "{ring}");
        talk(&mut owner, deps, npc_actor, &mut command_id, "yes").await?;
    } else {
        talk(&mut owner, deps, npc_actor, &mut command_id, "passage").await?;
        let denied = talk(&mut owner, deps, npc_actor, &mut command_id, "yes").await?;
        assert!(denied.contains("mission"));
        talk(&mut owner, deps, npc_actor, &mut command_id, "mission").await?;
        talk(&mut owner, deps, npc_actor, &mut command_id, "passage").await?;
        talk(&mut owner, deps, npc_actor, &mut command_id, "yes").await?;
    }
    let actor = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    if matches!(route, NativeScenario::BuddelFare) {
        assert_eq!(actor.floor, 7);
        assert!(
            [
                (32225, 31381),
                (32462, 31174),
                (32333, 31227),
                (32021, 31294)
            ]
            .contains(&(actor.x, actor.y))
        );
        let fares: Vec<i64> =
            sqlx::query_scalar("SELECT fare_gold_units FROM game_newhaven_travel_receipts")
                .fetch_all(&harness.pool)
                .await?;
        assert_eq!(fares, [50]);
    } else if buddel {
        assert_eq!((actor.x, actor.y, actor.floor), (32225, 31381, 7));
        let ring: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3097' AND lifecycle=1")
            .fetch_one(&harness.pool).await?;
        assert_eq!(ring, 0);
        let fare: i64 = sqlx::query_scalar("SELECT fare_gold_units FROM game_newhaven_travel_receipts")
            .fetch_one(&harness.pool).await?;
        assert_eq!(fare, 0);
        if let NativeScenario::BuddelEquipped | NativeScenario::BuddelNested = route {
            let (lifecycle, quantity, located): (i16, i64, bool) = sqlx::query_as(
                "SELECT lifecycle,quantity,EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id UNION ALL SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(super::id(201).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!((lifecycle, quantity, located), (2, 0, false));
            let revision: String = sqlx::query_scalar("SELECT revision::text FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(super::id(41).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(revision, if matches!(route, NativeScenario::BuddelEquipped) { "2" } else { "1" });
        }
    } else {
        assert_eq!((actor.x, actor.y, actor.floor), (32325, 31049, 7));
        for (track, _) in tracks.into_iter().skip(1) {
            let stage: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind(track)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(stage, 3, "{track}");
        }
    }
    Ok(())
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    npc_actor: EntityRef,
    command_id: &mut u64,
    text: &str,
) -> TestResult<String> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            deps.actor,
            UseCommand {
                game_session_id: session,
                command_id: *command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor,
                text: text.into(),
            },
        )
        .await;
    *command_id += 1;
    assert_eq!(
        result.disposition,
        NpcIntentDisposition::Ok,
        "{text}: {:?}",
        result.conversation
    );
    Ok(result
        .conversation
        .map(|reply| reply.lines.join(" "))
        .unwrap_or_default())
}

pub(super) async fn seed_ring(harness: &Harness, route: NativeScenario) -> TestResult {
    use super::id;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let nested = matches!(route, NativeScenario::BuddelNested);
    for (seed, key) in [(201, "oteryn:item.tibia.i3097"), (202, "oteryn:item.tibia.i2854"), (203, "oteryn:item.tibia.i2854")].into_iter().take(if nested { 3 } else { 1 }) {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(id(204).as_slice()).execute(&mut *tx).await?;
    }
    if nested {
        sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(203).as_slice()).bind(id(204).as_slice()).execute(&mut *tx).await?;
        for (parent, child) in [(203, 202), (202, 201)] {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                .bind(id(child).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice()).bind(id(204).as_slice()).execute(&mut *tx).await?;
        }
    } else {
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,8,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(201).as_slice()).bind(id(42).as_slice()).bind(id(204).as_slice()).execute(&mut *tx).await?;
    }
    if !nested {
        sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,remaining_ms,state_revision) VALUES(encode($1,'hex')::uuid,3500000,1)")
            .bind(id(201).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
