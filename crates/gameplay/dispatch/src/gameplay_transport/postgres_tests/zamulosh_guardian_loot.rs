use super::fixture::{Harness, debug, fence, run};
use super::quest_world_callbacks_fixtures::{callback, world_with_native_items};
use super::srx3_activity_source_fixture::prepare_player;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::NativeEntryStart;
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{SeamRooms, capabilities::SelectedCapabilities};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::boss_rewards::{
    BossRewardStatus, CAPABILITY_BOSS_REWARDS_V1, COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
    CollectAllRequest, decode_collect_reply, encode_collect_all_request,
};

const CREATURE: &str = "oteryn:creature.zamulosh";
const OBJECTIVE: &str = "oteryn:quest-death/questpc7-ferumbras-zamulosh";

#[test]
fn zamulosh_guardian_death_mints_corpse_and_collects_silver_token() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("zamulosh-guardian-loot", || {
        run(async |admin| {
            let world_id = fence(1)?.runtime_scope.world_id();
            let (controller, baseline) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    world_id,
                    "content-1",
                )?;
            let active = controller.active().ok_or("native activation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let entry = NativeEntryStart {
                x: 33643,
                y: 32757,
                floor: 11,
            };
            // Keep the real guardian coordinate and provide a nearby depot chest
            // in the fixture so collection travels through the physical route.
            let world = world_with_native_items(
                &callback("questpc7-ferumbras-entry")?,
                Position {
                    x: 33644,
                    y: 32757,
                    floor: -11,
                },
                &[],
                native,
                &[(
                    Position {
                        x: 33644,
                        y: 32758,
                        floor: -11,
                    },
                    19250,
                )],
            )?;
            let base_room = baseline.qualified_room();
            let cells = world
                .movement_cells(base_room.movement_cells())
                .map_err(debug)?;
            let room = base_room
                .bundle_view(
                    cells.clone(),
                    world.frame_binding_digest(world_id),
                    world.map_revision_digest(),
                    entry,
                )
                .ok_or("bundle combat room")?;
            let pin = ChannelContentPin::from_activation(
                world_id,
                active.activation_sequence(),
                active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(),
                room.frame_binding().digest(),
                room.map_revision_digest(),
                (entry.x, entry.y, entry.floor),
            );
            let mut deps = Dependencies::with_content(
                pin.clone(),
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            deps.combat_room = Some(room.clone());
            let h =
                Harness::create_with_admission_seed(admin, "zamulosh_guardian_loot", false, false)
                    .await?;
            super::reset();
            prepare_current_session(&h).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(world_id.as_bytes().as_slice()).bind(deps.admitted.channel_id.as_bytes().as_slice())
                .execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(world_id.as_bytes().as_slice()).bind(deps.admitted.channel_id.as_bytes().as_slice())
                .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
                .execute(&h.pool).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let account = super::fixture::id(40);
            let (consumer, clock, activation, _) = purchase_fixture::current(account);
            let premium = SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            owner.rooms = SeamRooms::new(Some(&room), Some(&room));
            owner.reset_epoch = 1;
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(account, deps.admitted.game_session_id);
            let quests =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(std::sync::Arc::new(quests.catalogue().clone()));
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 150).await?;
            super::questpc7_native_deaths_fixtures::prepare_native_hunter(&owner, &admitted)
                .await?;
            super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
            // The earlier Ferumbras seal is already earned. Admission must spawn
            // the authored Zamulosh guardian for this active player in his room.
            super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
                &h,
                &owner,
                &admitted,
                &[
                    (
                        "oteryn:quest-progress/questpc7/ferumbras/programme".into(),
                        2,
                    ),
                    (
                        "oteryn:quest-progress/questpc7/ferumbras/passages".into(),
                        1,
                    ),
                    ("oteryn:quest-progress/questpc7/ferumbras/seal_0".into(), 1),
                    ("oteryn:quest-progress/questpc7/ferumbras/seal_1".into(), 0),
                ],
                161,
            )
            .await?;
            owner.admit_quest_session(&admitted).await;
            assert!(!deps.runtime.lock().await.uses_classic_creature_deaths());
            let target = owner
                .quest_guardian_for_test(OBJECTIVE)
                .await
                .ok_or("live Zamulosh guardian")?;
            super::questpc7_native_deaths_fixtures::lethal_creature(
                &owner, &admitted, target, 91, CREATURE,
            )
            .await?;
            assert_eq!(
                super::questpc7_ferumbras_fixtures::value(
                    &owner,
                    &admitted,
                    "oteryn:quest-progress/questpc7/ferumbras/seal_1",
                )
                .await?,
                1,
                "the quest guardian's seal credit must settle with its death",
            );

            let outcomes: (i64, i64, i64, i64) = sqlx::query_as(
                "SELECT
                    (SELECT count(*) FROM game_item_mint_receipts r
                     JOIN game_item_ground_locations g USING (item_instance_id)
                     JOIN game_item_instances i USING (item_instance_id)
                     WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.materialized_at IS NOT NULL
                       AND i.definition_production_key='oteryn:item.tibia.i22495' AND i.lifecycle=1),
                    (SELECT count(*) FROM game_character_boss_reward_chests),
                    (SELECT count(*) FROM game_boss_reward_items
                     WHERE definition_production_key='oteryn:item.tibia.i22516'
                       AND quantity=1 AND collected_transaction_id IS NULL),
                    (SELECT count(*) FROM game_boss_reward_items
                     WHERE definition_production_key='oteryn:item.tibia.i22555')",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                outcomes,
                (1, 1, 1, 0),
                "Zamulosh death must mint one Ground corpse and a private Silver Token without the stone wall"
            );

            let death = sqlx::query_scalar::<_, Vec<u8>>(
                "SELECT uuid_send(death_id) FROM game_boss_reward_deaths",
            )
            .fetch_one(&h.pool)
            .await?
            .try_into()
            .map_err(|_| "boss death UUID")?;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 117, CAPABILITY_BOSS_REWARDS_V1],
            )
            .ok_or("boss reward capabilities")?;
            let request = encode_collect_all_request(&CollectAllRequest { death: Some(death) })
                .map_err(debug)?;
            let (status, reply) = super::boss_rewards_fixtures::exchange(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
                &request,
            )
            .await?;
            assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            let receipt = decode_collect_reply(&reply).map_err(debug)?;
            assert_eq!(receipt.status, BossRewardStatus::Ok);
            assert_eq!(receipt.remaining, 0);
            assert_eq!(
                super::questpc7_outfit_items_fixtures::quantity(&h, 22516).await?,
                1
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
