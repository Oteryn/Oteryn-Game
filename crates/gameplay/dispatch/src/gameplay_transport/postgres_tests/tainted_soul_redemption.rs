//! Sacred Antler Talisman through the composed USE owner and PostgreSQL.
use super::full_world_support::{ENTRY, TestResult, debug, world_scenario};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_entities::NativeSummonAdmissionSpec;
use oteryn_gameplay_quests::quest::{loader, world_callbacks};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const TALISMAN: &str = "oteryn:item.tibia.i22009";
const TAINTED: &str = "oteryn:creature.tainted_soul";
const REDEEMED: &str = "oteryn:creature.redeemed_soul";

#[test]
fn tainted_soul_redemption_postgres_use_counts_wild_and_summoned_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e::scenario_thread::run("tainted-soul-redemption", move || {
            scenario(&admin)
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED tainted-soul-redemption duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn cell(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

fn entity(runtime: &ChannelRuntimeV1, target: ExactActorRef) -> TestResult<EntityRef> {
    let visible = runtime.visible_entities();
    let target = visible
        .creatures
        .iter()
        .find(|c| c.actor == target)
        .ok_or("visible target")?;
    Ok(EntityRef {
        identity: target.actor.placement_identity(),
        generation: target.generation,
    })
}

fn scenario(admin: &str) -> TestResult {
    assert!(!admin.is_empty(), "PostgreSQL admin URL must be configured");
    world_scenario!(
        "tainted_soul_redemption",
        |h,
         _request,
         _seal,
         authority,
         world_id,
         native,
         world,
         _cells,
         _room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            owner.quest_catalogue = Some(Arc::new(
                loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            owner.refresh_quest_session(session, true).await;
            super::quest_world_callbacks::input_item(
                &owner,
                fence,
                &world_callbacks::Offering {
                    position: world_callbacks::Position {
                        x: u16::try_from(ENTRY.0)?,
                        y: u16::try_from(ENTRY.1)?,
                        floor: -7,
                    },
                    item: TALISMAN.into(),
                    count: 1,
                },
                811,
                true,
            )
            .await?;
            let definition = native
                .item_index()
                .ok_or("item index")?
                .definition_ref("Item", TALISMAN, "definition-r1")
                .ok_or("talisman reference")?;
            let command = |command_id| UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            };
            let intent = |target| UseIntent {
                target: UseTarget::ItemByDefinition(definition),
                use_with: Some(target),
                use_with_map_item: None,
                use_with_position: None,
            };
            let (wild, wild_entity) = {
                let mut runtime = deps.runtime.lock().await;
                let wild = runtime
                    .realize_native_qualification_spawn(pin.clone(), TAINTED, cell(9, 0))
                    .map_err(debug)?
                    .actor;
                (wild, entity(&runtime, wild)?)
            };
            let clock = owner.freeze_owner_clock_for_test();
            let now = owner.owner_now().get();
            let first = owner
                .use_quest_tool(actor, command(20), intent(wild_entity), None, None)
                .await
                .map_err(|()| "redemption USE failed")?
                .ok_or("redemption not registered")?;
            assert_eq!(first.disposition, UseDisposition::Committed);

            let redeemed = {
                let runtime = deps.runtime.lock().await;
                assert!(runtime.matches_live_creature_identity(wild, TAINTED.as_bytes()));
                runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .find(|c| runtime.matches_live_creature_identity(c.actor, REDEEMED.as_bytes()))
                    .ok_or("redeemed soul appeared beside player")?
                    .actor
            };
            owner.drain_tainted_soul_expiry(now + 49_999).await;
            assert!(
                deps.runtime
                    .lock()
                    .await
                    .matches_live_creature_identity(wild, TAINTED.as_bytes())
            );
            owner.drain_tainted_soul_expiry(now + 50_000).await;
            assert!(
                !deps
                    .runtime
                    .lock()
                    .await
                    .matches_live_creature_identity(wild, TAINTED.as_bytes())
            );
            let replay = owner
                .use_quest_tool(actor, command(20), intent(wild_entity), None, None)
                .await
                .map_err(|()| "redemption replay failed")?
                .ok_or("redemption receipt missing")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            owner.drain_tainted_soul_expiry(now + 29_999_999).await;
            assert!(
                deps.runtime
                    .lock()
                    .await
                    .matches_live_creature_identity(redeemed, REDEEMED.as_bytes())
            );
            owner.drain_tainted_soul_expiry(now + 30_000_000).await;
            assert!(
                !deps
                    .runtime
                    .lock()
                    .await
                    .matches_live_creature_identity(redeemed, REDEEMED.as_bytes())
            );
            drop(clock);

            let (summon, summon_entity, master) = {
                let mut runtime = deps.runtime.lock().await;
                let master = runtime
                    .realize_native_qualification_spawn(pin.clone(), TAINTED, cell(11, 0))
                    .map_err(debug)?
                    .actor;
                let spec = NativeSummonAdmissionSpec::qualified(
                    TAINTED,
                    TAINTED,
                    250,
                    0,
                    2,
                    2,
                    native.source_digest(),
                )
                .map_err(debug)?;
                let stamp = runtime.issue_owner_work().map_err(debug)?;
                let summon = runtime
                    .admit_current_native_summon(stamp, master, &spec, cell(9, 0))
                    .map_err(debug)?;
                // Fill the final actor slot so no Redeemed Soul can be admitted.
                runtime
                    .realize_native_qualification_spawn(pin.clone(), TAINTED, cell(11, 1))
                    .map_err(debug)?;
                assert_eq!(runtime.native_summon_master(summon), Some(master));
                (summon, entity(&runtime, summon)?, master)
            };
            let _clock = owner.freeze_owner_clock_for_test();
            let next_turn = owner.owner_now().get() + 50_000;
            let second = owner
                .use_quest_tool(actor, command(21), intent(summon_entity), None, None)
                .await
                .map_err(|()| "summoned redemption USE failed")?
                .ok_or("summoned redemption missing")?;
            assert_eq!(second.disposition, UseDisposition::Committed);
            {
                let runtime = deps.runtime.lock().await;
                assert!(runtime.matches_live_creature_identity(summon, TAINTED.as_bytes()));
                assert!(runtime.visible_entities().creatures.iter().all(|c| {
                    !runtime.matches_live_creature_identity(c.actor, REDEEMED.as_bytes())
                }));
            }
            owner.drain_tainted_soul_expiry(next_turn).await;
            {
                let runtime = deps.runtime.lock().await;
                assert!(!runtime.matches_live_creature_identity(summon, TAINTED.as_bytes()));
                assert!(runtime.matches_live_creature_identity(master, TAINTED.as_bytes()));
            }
            let progress: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/tainted-souls/freed'")
                .fetch_one(&h.pool).await?;
            assert_eq!(progress, 2);
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                .bind(TALISMAN).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1);
            Ok(())
        }
    )
}
