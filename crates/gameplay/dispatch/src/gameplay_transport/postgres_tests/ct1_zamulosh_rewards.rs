use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::quest_world_callbacks_fixtures::{callback, local, world_with_native_items};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NativeEntryStart, channel_content_pin::ChannelContentPin};
use crate::foundation::WorldId;
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities, connection::FreshAdmissionAuthority,
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::{CommandStatus, boss_rewards};

const CREATURE: &str = "oteryn:creature.zamulosh";
const CORPSE: &str = "oteryn:item.tibia.i22495";
const SILVER_TOKEN: &str = "oteryn:item.tibia.i22516";
const STONE_WALL: &str = "oteryn:item.tibia.i22555";

#[test]
fn ct1_zamulosh_guardian_mints_corpse_and_awards_claimable_silver_token() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "ct1_zamulosh_rewards", false, false)
            .await?;
        let result = exercise(&h).await;
        let cleanup = h.cleanup().await;
        result.and(cleanup)
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let active = controller.active().ok_or("active generation")?;
    let guardian_cell = Position {
        x: 33643,
        y: 32756,
        floor: -11,
    };
    let player_cell = Position {
        y: 32757,
        ..guardian_cell
    };
    let start = local(player_cell);
    let chest_cell = Position {
        x: player_cell.x + 1,
        ..player_cell
    };
    let world = world_with_native_items(
        &callback("questpc7-ferumbras-entry")?,
        player_cell,
        &[],
        active.native_gameplay().ok_or("native gameplay")?,
        &[(chest_cell, 19250)],
    )?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let room = activated
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            world.map_revision_digest(),
            NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("Ferumbras arena bundle view")?;
    let server_digest = active.identity().server_artifact_digest();
    let client_digest = active.identity().client_artifact_digest();
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        server_digest,
        client_digest,
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut deps = Dependencies::with_content(
        pin,
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.combat_room = Some(room.clone());
    deps.runtime
        .lock()
        .await
        .use_classic_creature_deaths()
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.reset_epoch = 1;
    let quests = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(quests.catalogue().clone()));
    sqlx::query(
        "INSERT INTO game_control_scope_grants VALUES
         (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(deps.admitted.channel_id.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query(
        "SELECT game_content_record_activation(
            encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(deps.admitted.channel_id.as_bytes().as_slice())
    .bind(server_digest.as_slice())
    .bind(client_digest.as_slice())
    .bind(frame.as_slice())
    .execute(&h.pool)
    .await?;
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    let mut admitted = deps.admitted;
    super::srx3_activity_source_fixture::prepare_player(h, &owner, &mut admitted, 150).await?;
    super::questpc7_native_deaths_fixtures::prepare_native_hunter(&owner, &admitted).await?;
    // Start after the Plagirath seal. The production guardian owner spawns Zamulosh.
    let mut prior = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *prior)
        .await?;
    let imported = sqlx::query(
        "INSERT INTO game_character_quest_tracks
            (character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,seal.track_key,'oteryn:quest.ferumbras_ascension_quest',
            seal.value,character_revision
         FROM game_character_roots CROSS JOIN (VALUES
            ('oteryn:quest-progress/questpc7/ferumbras/seal_0',1),
            ('oteryn:quest-progress/questpc7/ferumbras/seal_1',0)
         ) seal(track_key,value) WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(&mut *prior)
    .await?;
    assert_eq!(imported.rows_affected(), 2);
    prior.commit().await?;
    owner.admit_quest_session(&admitted).await;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let target = owner
        .quest_guardian_for_test("oteryn:quest-death/questpc7-ferumbras-zamulosh")
        .await
        .ok_or("Ferumbras objective must spawn Zamulosh")?;
    super::questpc7_native_deaths_fixtures::lethal_creature(
        &owner, &admitted, target, 16_001, CREATURE,
    )
    .await?;

    let settlement: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM game_item_instances i
            JOIN game_item_ground_locations g USING(item_instance_id)
            JOIN game_item_mint_receipts r USING(item_instance_id)
            WHERE i.definition_production_key=$1 AND r.loot_purpose_key='CORPSE_MATERIALIZATION'),
         (SELECT count(*) FROM game_boss_reward_contributors WHERE eligible),
         (SELECT count(*) FROM game_boss_reward_items WHERE definition_production_key=$2),
         (SELECT count(*) FROM game_boss_reward_items WHERE definition_production_key=$3)",
    )
    .bind(CORPSE)
    .bind(SILVER_TOKEN)
    .bind(STONE_WALL)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        settlement,
        (1, 1, 1, 0),
        "Zamulosh ground corpse, eligible player, private Silver Token, and private stone wall counts"
    );

    let death: sqlx::types::Uuid =
        sqlx::query_scalar("SELECT death_id FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 117, boss_rewards::CAPABILITY_BOSS_REWARDS_V1],
    )
    .ok_or("native boss reward capabilities")?;
    let request = boss_rewards::encode_collect_all_request(&boss_rewards::CollectAllRequest {
        death: Some(*death.as_bytes()),
    })
    .map_err(debug)?;
    let (status, reply) = super::boss_rewards_fixtures::exchange(
        &owner,
        &mut admitted,
        1,
        boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &request,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let reply = boss_rewards::decode_collect_reply(&reply).map_err(debug)?;
    assert_eq!(reply.status, boss_rewards::BossRewardStatus::Ok);
    assert_eq!(reply.remaining, 0);
    let collected: bool = sqlx::query_scalar(
        "SELECT collected_transaction_id IS NOT NULL FROM game_boss_reward_items
         WHERE death_id=$1 AND definition_production_key=$2",
    )
    .bind(death)
    .bind(SILVER_TOKEN)
    .fetch_one(&h.pool)
    .await?;
    assert!(collected, "the player claimed the Silver Token");
    eprintln!("CT1_ZAMULOSH_REWARDS_POSTGRES_CASE_EXECUTED");
    Ok(())
}
