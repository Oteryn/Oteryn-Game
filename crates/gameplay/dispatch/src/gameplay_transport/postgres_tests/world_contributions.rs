//! Shared contributions through the normal NPC travel, exchange and map USE owners.
use super::TestResult;
use super::world_contribution_fixtures::{junkar_route, quote_junkar_route, save};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn swamp_fever_medicine_exchange_advances_phase_once_and_resets_at_save() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-afflicted-swamp-fever")
}

#[test]
fn steamship_junkar_paid_quote_and_confirmation_require_an_active_working_week() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("steamship-paid-quote", steamship_quote)
}

fn steamship_quote() -> TestResult {
    use super::fixture::{Harness, debug, run};
    use super::npcfix_m38_travel_guards_fixtures::NoPremium;
    use crate::durability::{
        npc_travel::TravelRefusal,
        world_event::changes::{self, ContributionWrite},
    };
    use oteryn_gameplay_world_change::owner::ContributionKind;

    run(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "steamship_quote", true, false).await?;
        super::prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = super::Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|()| "current travel fence")?
            .ok_or("current travel fence")?;
        let world = fence.runtime_scope.world_id();
        let route = junkar_route("kazordoon")?;
        let quote = || {
            h.root
                .quote_npc_travel(&authority, &h.node, fence, route.clone(), &NoPremium)
        };
        save(&h, world, 1).await?;
        assert_eq!(
            quote().await.map_err(debug)?,
            Err(TravelRefusal::UnavailableRoute)
        );

        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert_eq!(
            changes::contribute(
                &mut tx,
                &recovery,
                world,
                1,
                [4; 16],
                ContributionKind::Coal,
                200
            )
            .await?,
            ContributionWrite::Applied(200)
        );
        tx.commit().await?;
        assert_eq!(
            quote().await.map_err(debug)?,
            Err(TravelRefusal::UnavailableRoute)
        );

        save(&h, world, 2).await?;
        assert_eq!(quote().await.map_err(debug)?, Ok(route.route.price));
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut confirmation =
            super::npcfix_m38_travel_return_fixtures::request(&owner, &deps, 3, "junkar_thais")
                .await?;
        confirmation.route = route.clone();
        let destination = &route.route.destination;
        confirmation.arrival.destination =
            crate::durability::character_last_position::LastPositionCell {
                x: destination.x,
                y: destination.y,
                floor: destination.floor,
            };
        confirmation.fee.fee_gold_units = route.route.price;
        for epoch in 3..=9 {
            save(&h, world, epoch).await?;
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert_eq!(
            slot.commit_npc_travel(
                &h.root,
                &authority,
                &h.node,
                fence,
                confirmation,
                &NoPremium
            )
            .await
            .map_err(debug)?,
            Err(TravelRefusal::UnavailableRoute),
        );
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_newhaven_travel_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(receipts, 0, "expired confirmation cannot charge a fare");
        drop(slot);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn awash_production_pump_opens_the_mines_route_after_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("awash-pump-use", awash_pump)
}

fn awash_pump() -> TestResult {
    use super::fixture::{Harness, debug, id, run};
    use crate::foundation::{ChannelContentPin, CharacterId, CharacterLease};
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use oteryn_gameplay_quests::quest::{
        loader::load_embedded_quest_state, world_callbacks::Offering,
    };
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
    };
    use std::sync::Arc;

    const COAL: &str = "oteryn:item.tibia.i12600";
    let loaded = load_embedded_quest_state("content-1").map_err(debug)?;
    let catalogue = Arc::new(loaded.catalogue().clone());
    let callbacks: Vec<_> = catalogue
        .authored_bindings()
        .ok_or("production quest bindings")?
        .world_callbacks()
        .filter(|callback| callback.quest == "oteryn:quest.awash_world_change")
        .collect();
    assert_eq!(callbacks.len(), 3);
    let callback = callbacks[0];
    let native_pump = callback
        .selector
        .native
        .as_ref()
        .ok_or("native pump selector")?;
    let pump = native_pump.position;
    let player = oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: pump.x + 1,
        ..pump
    };
    let bundle = super::quest_world_callbacks_fixtures::world_ordinal(
        callback,
        pump,
        native_pump.appearance,
        native_pump.ordinal,
    )?;
    run(async move |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "awash_pump", false, false).await?;
        super::prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, activation) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let base = activation.qualified_room().clone();
        let cells = bundle
            .movement_cells(base.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            (i32::from(player.x), i32::from(player.y), 9),
        );
        let mut deps = super::Dependencies::with_content_and_capacity(
            pin,
            base,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            4,
        )?;
        deps.combat_room = Some(
            activation
                .qualified_room()
                .bundle_view(
                    cells,
                    bundle.frame_binding_digest(world),
                    bundle.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: player.x.into(),
                        y: player.y.into(),
                        floor: 9,
                    },
                )
                .ok_or("pump room")?,
        );
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = deps
            .combat_room
            .as_ref()
            .ok_or("pump room")?
            .movement_cells()
            .into();
        owner.quest_catalogue = Some(catalogue);
        owner.reset_epoch = 1;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "pump fence")?
            .ok_or("pump fence")?;
        admitted.item_fence = Some(fence.item_fence());
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    super::quest_world_callbacks_fixtures::local(player),
                )
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        prepare_pump_inventory(&h, &owner, &admitted, bundle.frame_binding_digest(world)).await?;
        deps.initialize(&owner).await;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 11, 15, 18],
            )
            .ok_or("item use capabilities")?;

        save(&h, world, 1).await?;
        let route = junkar_route("mines")?;
        use crate::durability::npc_travel::TravelRefusal;
        assert_eq!(
            quote_junkar_route(&owner, &h, admitted.game_session_id, &route).await?,
            Err(TravelRefusal::UnavailableRoute)
        );
        let selected = native
            .item_index()
            .and_then(|index| index.definition_ref("Item", COAL, "definition-r1"))
            .ok_or("native coal")?;
        let pump_tile = crate::map::overlay::TilePos {
            x: pump.x,
            y: pump.y,
            floor: pump.floor,
        };
        let batches = [20; 10].into_iter().chain([2]);
        for (batch, count) in batches.enumerate() {
            // Only seed the fuel. Every tally increment comes from a player USE.
            super::quest_world_callbacks_fixtures::input_item(
                &owner,
                fence.item_fence(),
                &Offering {
                    position: player,
                    item: COAL.into(),
                    count,
                },
                8701 + u64::try_from(batch)?,
                true,
            )
            .await?;
            for unit in 1..=count {
                // Each committed use advances the pump's placement revision.
                let target = super::questpc8_authored_programmes_fixtures::object_target(
                    &owner, pump_tile, 1,
                )?;
                let payload = encode_item_use_intent(&UseIntent {
                    target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("tool handle")?),
                    use_with_map_item: Some(MapUseTarget::Object(target)),
                    use_with: None,
                    use_with_position: None,
                })
                .map_err(debug)?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("advance pump use clock")?;
                let command = admitted.continuity.next_command_id;
                let (end, disposition, wire) = super::item_craft_fixtures::exchange(
                    &owner,
                    admitted,
                    command,
                    &payload,
                    Some(selected),
                )
                .await?;
                admitted = end;
                assert_eq!(disposition, UseDisposition::Committed);
                if batch == 10 && unit == count {
                    let (end, replay, _) = super::item_craft_fixtures::exchange(
                        &owner, admitted, command, &wire, None,
                    )
                    .await?;
                    admitted = end;
                    assert_eq!(replay, UseDisposition::Committed);
                }
            }
        }
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(&h, 12600).await?,
            0
        );

        assert!(h.root.world_change_active(world, "awash_pumping").await?);
        assert!(!h.root.world_change_active(world, "awash").await?);
        assert!(!h.root.world_change_active(world, "steamship").await?);
        let tally: String = sqlx::query_scalar("SELECT tally::text FROM game_world_change_tallies WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1 AND resource='awash_coal'")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(tally, "202");
        save(&h, world, 2).await?;
        assert!(h.root.world_change_active(world, "awash").await?);
        assert_eq!(
            quote_junkar_route(&owner, &h, admitted.game_session_id, &route).await?,
            Ok(route.route.price)
        );
        assert!(!h.root.world_change_active(world, "awash_pumping").await?);
        save(&h, world, 3).await?;
        assert!(!h.root.world_change_active(world, "awash").await?);

        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn prepare_pump_inventory(
    h: &super::fixture::Harness,
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    frame: [u8; 32],
) -> TestResult {
    use super::fixture::{debug, id};
    let generation = owner.active_generation.ok_or("generation")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice())
        .execute(&h.pool).await?;
    let fence = admitted.item_fence.ok_or("pump item fence")?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            owner.character,
            &h.node,
            &fence,
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let native = generation.native_gameplay().ok_or("native")?;
    h.root
        .grant_starter_items(
            owner.character,
            &h.node,
            &fence,
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate::main(),
        )
        .await
        .map_err(debug)?;
    Ok(())
}

#[test]
fn steamship_production_donations_open_kazordoon_route_after_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-steamship-donation")
}
