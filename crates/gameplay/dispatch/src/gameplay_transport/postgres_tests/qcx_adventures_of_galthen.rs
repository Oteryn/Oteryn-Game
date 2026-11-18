//! City of Gold reward through the carried-item USE route.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const IDOL: u16 = 40578;
const IDOL_KEY: &str = "oteryn:item.tibia.i40578";
const OUTFIT: &str = "oteryn:outfit.ancient_aucar";

#[test]
fn living_idol_unlocks_base_outfit_and_consumes_only_on_first_use() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx81-living-idol", || {
        run(async |admin| {
            let harness = Harness::create_appearance(admin, "qcx81_living_idol").await?;
            super::reset();
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
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
                .movement_cells(pin.qualified_room().movement_cells())
                .map_err(debug)?;
            let bundle_room = super::rust_lottery::bundle_action_room(
                pin.qualified_room(),
                &map,
                deps.admitted.world_id,
                &cells,
            )?;
            let content_pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                bundle_room.frame_binding().digest(),
                bundle_room.map_revision_digest(),
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
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            let coordinator = SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(harness.root.clone()),
            );
            let mut owner = deps.owner(&harness, &authority);
            owner.premium_coordinator = Some(&coordinator);
            owner.admit_premium(id(40), deps.admitted.game_session_id);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
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

            super::item_look::seed_main_backpack(&harness, &[(IDOL_KEY, 1), (IDOL_KEY, 1)]).await?;
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(fence);
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                &crate::gameplay_transport::appearance_change::offer::offered(
                    owner.offered_capabilities(),
                    owner.appearance_available(),
                ),
                &[3, 4, 6, 11, 15, 18, 25],
            )
            .ok_or("offers")?;
            assert!(admitted.continuity.selected_capabilities.contains(25));

            // TibiaWiki spoiler 94719/1199250, City of Gold / Ahau.
            use super::qengine_voucher_stories::{mask, quantity, use_native};
            assert_eq!(quantity(&harness, IDOL).await?, 2);
            use_native(&owner, &mut admitted, IDOL, UseDisposition::Committed).await?;
            mask(&owner, &admitted, OUTFIT, 0).await?;
            assert_eq!(quantity(&harness, IDOL).await?, 1);
            use_native(&owner, &mut admitted, IDOL, UseDisposition::Rejected).await?;
            assert_eq!(quantity(&harness, IDOL).await?, 1);

            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx81_living_idol duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
