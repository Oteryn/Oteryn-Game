//! Sinatuki's fixed fish trades through live NPC talk and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.sinatuki";
const TROUT: &str = "oteryn:item.tibia.i7158";
const PERCH: &str = "oteryn:item.tibia.i7159";
const SHARD: &str = "oteryn:item.tibia.i7290";
const TRACKS: [(&str, i64); 2] = [
    (
        "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
        3,
    ),
    (
        "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
        5,
    ),
];

#[test]
fn sinatuki_exchange_postgres_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("sinatuki-exchange".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Sinatuki PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED sinatuki_exchange_postgres_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "sinatuki_exchange", true, false).await?;
        eprintln!("POSTGRES_STARTED sinatuki_exchange_postgres_dispatch");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
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
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .next()
                .ok_or("Sinatuki placement")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "item fence")?
            .ok_or("item fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        seed(&h).await?;
        let greeted = talk(&mut owner, deps.actor, command(1), npc, "hi").await?;
        assert!(
            greeted.conversation.is_some(),
            "Sinatuki must open focus from the production talk book"
        );
        for (index, (track, minimum)) in TRACKS.into_iter().enumerate() {
            set_progress(&h, track, minimum - 1).await?;
            let outcome = talk(
                &mut owner,
                deps.actor,
                command(2 + u64::try_from(index)?),
                npc,
                "nupi",
            )
            .await?;
            assert!(outcome.conversation.is_none());
            assert!(!outcome.inventory_changed);
            assert_inventory(&h, [100, 99, 0]).await?;
            set_progress(&h, track, minimum).await?;
        }
        for (id, reply, changed, expected) in [
            (
                4,
                "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>",
                true,
                [0, 99, 5],
            ),
            (
                5,
                "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>",
                true,
                [0, 0, 10],
            ),
            (6, "Kisavuta! <giggles>", false, [0, 98, 10]),
        ] {
            if id == 6 {
                let mut fixture = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *fixture)
                    .await?;
                seed_entry(&mut fixture, 207, PERCH, 98, 6).await?;
                fixture.commit().await?;
            }
            let outcome = talk(&mut owner, deps.actor, command(id), npc, "nupi").await?;
            assert_eq!(
                outcome
                    .conversation
                    .ok_or_else(|| format!(
                        "missing exchange reply for command {id}, inventory_changed={}",
                        outcome.inventory_changed
                    ))?
                    .lines,
                [reply]
            );
            assert_eq!(outcome.inventory_changed, changed);
            assert_inventory(&h, expected).await?;
        }
        drop(seal);
        h.cleanup().await
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    npc: EntityRef,
    text: &str,
) -> TestResult<oteryn_gameplay_npc::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let outcome = owner
        .npc_talk(
            actor,
            command,
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
    Ok(outcome)
}

async fn seed(h: &Harness) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=250 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *fixture).await?;
    for (track, value) in TRACKS {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.the_ice_islands_quest',$3,2)")
            .bind(id(41).as_slice()).bind(track).bind(value).execute(&mut *fixture).await?;
    }
    mint_fixture(&mut fixture, 200, "oteryn:item.tibia.i2854", 1).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(200).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice())
        .execute(&mut *fixture).await?;
    for (item, key, count, ordinal) in [
        (201, TROUT, 60, 1),
        (202, TROUT, 40, 2),
        (203, PERCH, 99, 3),
    ] {
        seed_entry(&mut fixture, item, key, count, ordinal).await?;
    }
    fixture.commit().await?;
    Ok(())
}

async fn mint_fixture(
    fixture: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seed: u8,
    item: &str,
    count: u32,
) -> TestResult {
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(seed, item, "definition-r1", count),
    ))
    .execute(&mut **fixture)
    .await?;
    Ok(())
}

async fn seed_entry(
    fixture: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seed: u8,
    item: &str,
    count: u32,
    ordinal: i64,
) -> TestResult {
    mint_fixture(fixture, seed, item, count).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(200).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice())
        .bind(ordinal).bind(id(246).as_slice()).execute(&mut **fixture).await?;
    Ok(())
}
async fn set_progress(h: &Harness, track: &str, value: i64) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=$3 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(track).bind(value).execute(&mut *fixture).await?;
    fixture.commit().await?;
    Ok(())
}

async fn assert_inventory(h: &Harness, expected: [i64; 3]) -> TestResult {
    for (item, quantity) in [TROUT, PERCH, SHARD].into_iter().zip(expected) {
        let actual: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i
             JOIN game_item_container_entries e USING(item_instance_id)
             WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2",
        ).bind(id(41).as_slice()).bind(item).fetch_one(&h.pool).await?;
        assert_eq!(actual, quantity, "{item}");
    }
    Ok(())
}
