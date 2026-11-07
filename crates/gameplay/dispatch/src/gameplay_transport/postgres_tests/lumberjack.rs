//! Real Native NPC actors and current Quest/trade writers; geometry has separate map proofs.
use super::*;
use crate::durability::npc_trade::{
    NpcTradeCause, NpcTradeCoins, NpcTradeOutcome, NpcTradeRequest, NpcTradeSide,
};
use crate::durability::quest_state::{
    NativeWorldChangeNpc, QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, MovementFacing, MovementLocalPosition, NpcPlacement,
    NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::item_ref_admission::generation_item_facts;
use oteryn_gameplay_quests::quest::world_change_npcs as rules;
use std::sync::Arc;

#[test]
fn wvar_lumberjack_native_npc_home_report_trade_and_current_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("WVAR_LUMBERJACK_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Lumberjack PG thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let start = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "wvar_lumberjack", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = deps.admitted.world_id;
        let definition = oteryn_gameplay_world_change::variants::definitions()?
            .iter()
            .find(|d| d.key == rules::CHANGE)
            .ok_or("Lumberjack recipe")?;
        let home = definition
            .variants
            .iter()
            .flat_map(|v| &v.npcs)
            .find(|n| n.key == rules::CHIP)
            .ok_or("Chip source home")?
            .position;
        let epoch = (1..100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.variant(rules::CHANGE).is_some())
            })
            .ok_or("active epoch")?;
        let inactive = (epoch + 1..epoch + 100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.variant(rules::CHANGE).is_none())
            })
            .ok_or("inactive epoch")?;
        let model = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model);
        let chip = book
            .get(rules::CHIP)
            .ok_or("production Chip dialogue admission")?;
        assert_eq!(chip.offers().len(), 1, "real native Chip shop admitted");
        let offer = &chip.offers()[0];
        assert_eq!(offer.item.key, "oteryn:item.tibia.i5901");
        assert_eq!(offer.unit_price, 50);
        assert_eq!(
            offer.direction,
            crate::content::ProjectV2ServiceOfferDirection::BuyFromPlayer
        );
        assert!(
            book.get(rules::QUEEN).is_some(),
            "Queen promotion/dialogue retained"
        );
        let origin = MovementLocalPosition {
            x: i32::from(home.0) - 1,
            y: i32::from(home.1),
            floor: i16::from(home.2),
        };
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            [8; 32],
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        // The real actor owner admits only NPCs accepted by the actual production book.
        let placements = NpcPlacementTable {
            npcs: vec![
                NpcPlacements {
                    key: rules::CHIP.into(),
                    placements: vec![NpcPlacement {
                        x: home.0,
                        y: home.1,
                        floor: -home.2,
                        direction: MovementFacing::South,
                    }],
                },
                NpcPlacements {
                    key: rules::QUEEN.into(),
                    placements: vec![NpcPlacement {
                        x: home.0 - 1,
                        y: home.1 + 1,
                        floor: -home.2,
                        direction: MovementFacing::South,
                    }],
                },
            ],
        };
        let placed = runtime
            .place_npcs(&placements, |key| book.get(key).is_some(), true)
            .map_err(debug)?;
        let chip_actor = placed.actors[0];
        let queen_actor = placed.actors[1];
        assert_eq!(runtime.npc_key(chip_actor), Some(rules::CHIP));
        assert_eq!(
            runtime
                .npc_home_position(chip_actor)
                .map(|p| (p.x, p.y, p.floor)),
            Some((i32::from(home.0), i32::from(home.1), i16::from(home.2)))
        );
        let before = runtime
            .borrow_movement_position()
            .read(chip_actor)
            .map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_cardinal(
                before,
                MovementLocalPosition {
                    x: i32::from(home.0),
                    y: i32::from(home.1) - 1,
                    floor: i16::from(home.2),
                },
            )
            .map_err(debug)?;
        deps.runtime = Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        let catalogue = Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        owner.quest_catalogue = Some(Arc::clone(&catalogue));
        owner.reset_epoch = epoch;
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i5901", 2)]).await?;
        seed_epoch(
            &h,
            world,
            epoch,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let command = |n| {
            CommandRef::new(
                deps.admitted.game_session_id,
                CommandId::new(n).expect("nonzero fixture command"),
            )
        };
        // Resolve visible generation rather than relying on an externally claimed actor number.
        let generation_of = |runtime: &ChannelRuntimeV1,
                             actor: crate::foundation::ExactActorRef| {
            runtime
                .visible_entities()
                .npcs
                .iter()
                .find(|n| n.actor == actor)
                .map(|n| n.generation)
                .ok_or("NPC generation")
        };
        let capture = |runtime: &ChannelRuntimeV1,
                       n: u64,
                       actor: crate::foundation::ExactActorRef,
                       day: u64|
         -> TestResult<NativeWorldChangeNpc> {
            NativeWorldChangeNpc::from_current_runtime(
                runtime,
                deps.actor,
                command(n),
                actor.placement_identity(),
                generation_of(runtime, actor)?,
                day,
                fence.item_fence(),
            )
            .map_err(debug)
        };
        let request = |key: &str, n| QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(command(n)),
        };
        let sequencer =
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence.character_id).await;
        let queen_source = capture(&*owner.runtime.lock().await, 100, queen_actor, epoch)?;
        assert!(
            slot.commit_world_change_npc(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::REPORT, 100),
                Arc::clone(&catalogue),
                queen_source
            )
            .await
            .is_err()
                || progress(&h).await? == 0
        );
        assert_eq!(progress(&h).await?, 0);
        // Generic and aliased transitions cannot claim an unvisited Chip.
        assert!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fence,
                    request(rules::TALK, 101),
                    Arc::clone(&catalogue)
                )
                .await
                .is_err()
        );
        {
            let runtime = owner.runtime.lock().await;
            let command = command(105);
            let npc_generation = generation_of(&runtime, chip_actor)?;
            assert!(
                NativeWorldChangeNpc::from_current_runtime(
                    &runtime,
                    deps.actor,
                    command,
                    [0; 16],
                    npc_generation,
                    epoch,
                    fence.item_fence()
                )
                .is_err()
            );
            assert!(
                NativeWorldChangeNpc::from_current_runtime(
                    &runtime,
                    deps.actor,
                    command,
                    chip_actor.placement_identity(),
                    npc_generation + 1,
                    epoch,
                    fence.item_fence()
                )
                .is_err()
            );
            let mut stale = fence.item_fence();
            stale.character_lease_generation += 1;
            assert!(
                NativeWorldChangeNpc::from_current_runtime(
                    &runtime,
                    deps.actor,
                    command,
                    chip_actor.placement_identity(),
                    npc_generation,
                    epoch,
                    stale
                )
                .is_err()
            );
        }
        let alias_key = "oteryn:quest-transition/world-change/lumberjack/renamed";
        let malicious = Arc::new(
            oteryn_gameplay_quests::quest::QuestStateCatalogue::new(
                "content-1",
                vec![catalogue.track(rules::TRACK).ok_or("track")?.clone()],
                vec![oteryn_gameplay_quests::quest::QuestTransition {
                    key: alias_key.into(),
                    quest: rules::QUEST.into(),
                    effects: vec![oteryn_gameplay_quests::quest::QuestEffect {
                        track: rules::TRACK.into(),
                        from: oteryn_gameplay_quests::quest::QuestComparison::Eq(0),
                        effect: oteryn_gameplay_quests::quest::QuestEffectKind::Set(2),
                    }],
                    completes: true,
                    experience: None,
                }],
            )
            .map_err(debug)?,
        );
        let alias_source = capture(&*owner.runtime.lock().await, 105, chip_actor, epoch)?;
        assert!(
            h.root
                .commit_character_world_change_npc(
                    &authority,
                    &h.node,
                    fence,
                    request(alias_key, 105),
                    malicious,
                    alias_source
                )
                .await
                .is_err()
        );
        assert_eq!(
            progress(&h).await?,
            0,
            "aliased transition cannot bypass visit"
        );
        let chip_source = capture(&*owner.runtime.lock().await, 102, chip_actor, epoch)?;
        assert!(matches!(
            slot.commit_world_change_npc(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::TALK, 102),
                Arc::clone(&catalogue),
                chip_source
            )
            .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        assert_eq!(progress(&h).await?, 1);
        let trade_source = capture(&*owner.runtime.lock().await, 200, chip_actor, epoch)?;
        let wood = native
            .item_policy_for_source_id(5901)
            .ok_or("wood native policy")?
            .record();
        let item_ref = crate::durability::item_mint::TypedDefinitionRef {
            family: wood.production_definition.family.clone(),
            production_key: wood.production_definition.production_key.clone(),
            revision_ref: wood.production_definition.revision_ref.clone(),
        };
        let item = generation_item_facts(native, &item_ref).map_err(debug)?;
        let reference = |id| -> TestResult<crate::durability::item_mint::TypedDefinitionRef> {
            let r = native.item_policy_for_source_id(id).ok_or("coin")?.record();
            Ok(crate::durability::item_mint::TypedDefinitionRef {
                family: r.production_definition.family.clone(),
                production_key: r.production_definition.production_key.clone(),
                revision_ref: r.production_definition.revision_ref.clone(),
            })
        };
        let bag = reference(2854)?;
        let trade = NpcTradeRequest {
            cause: NpcTradeCause {
                npc_key: rules::CHIP.into(),
                catalogue_revision: book.catalogue_revision(),
                offer_index: 0,
                side: NpcTradeSide::Sell,
                occurrence: id(111),
            },
            command: command(200),
            quantity: 1,
            unit_price: 50,
            offer_count: 1,
            item,
            transaction_id: id(112),
            event_id: id(113),
            occurred_at_unix_ms: 1,
            output_item_instance_ids: [id(114), id(115), id(116)],
            coins: NpcTradeCoins {
                gold: reference(3031)?,
                platinum: reference(3035)?,
                crystal: reference(3043)?,
            },
            backpack: generation_item_facts(native, &bag).map_err(debug)?,
        };
        assert!(
            h.root
                .commit_npc_trade(&authority, &h.node, fence.item_fence(), trade.clone())
                .await
                .is_err()
        );
        assert!(matches!(
            h.root
                .commit_world_change_npc_trade(
                    &authority,
                    &h.node,
                    fence.item_fence(),
                    trade.clone(),
                    trade_source.clone(),
                    false
                )
                .await?,
            NpcTradeOutcome::Committed(_)
        ));
        assert_eq!(wood_quantity(&h).await?, 1);
        assert_eq!(gold(&h).await?, 50);
        assert!(matches!(
            h.root
                .commit_world_change_npc_trade(
                    &authority,
                    &h.node,
                    fence.item_fence(),
                    trade.clone(),
                    trade_source,
                    false
                )
                .await?,
            NpcTradeOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(wood_quantity(&h).await?, 1);
        assert_eq!(gold(&h).await?, 50);
        let stale_source = capture(&*owner.runtime.lock().await, 250, chip_actor, epoch)?;
        seed_epoch(
            &h,
            world,
            inactive,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let mut stale = trade.clone();
        stale.command = command(250);
        stale.cause.occurrence = id(120);
        stale.transaction_id = id(121);
        stale.event_id = id(122);
        assert!(
            h.root
                .commit_world_change_npc_trade(
                    &authority,
                    &h.node,
                    fence.item_fence(),
                    stale,
                    stale_source,
                    false
                )
                .await
                .is_err()
        );
        assert_eq!(wood_quantity(&h).await?, 1);
        assert_eq!(gold(&h).await?, 50);
        assert!(
            h.root
                .reconcile_character_quest_transition(
                    &authority,
                    fence.character_id,
                    request(rules::TALK, 102)
                )
                .await?
                .is_some()
        );
        let queen_source = capture(&*owner.runtime.lock().await, 300, queen_actor, inactive)?;
        let report = request(rules::REPORT, 300);
        assert!(matches!(
            slot.commit_world_change_npc(
                &h.root,
                &authority,
                &h.node,
                fence,
                report.clone(),
                Arc::clone(&catalogue),
                queen_source.clone()
            )
            .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        assert_eq!(progress(&h).await?, 2);
        assert!(matches!(
            slot.commit_world_change_npc(
                &h.root,
                &authority,
                &h.node,
                fence,
                report,
                Arc::clone(&catalogue),
                queen_source
            )
            .await?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        let grants:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/whistle_blower'").fetch_one(&h.pool).await?;
        assert_eq!(grants, 1);
        assert_eq!(gold(&h).await?, 50, "report cannot invent a quest payout");
        drop(slot);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "WVAR_LUMBERJACK POSTGRES_CASE_COMPLETED native-chip,home-after-wander,visit,report-after-end,WhistleBlower1,wood1-gold50,replay,stale-epoch duration={:?}",
            start.elapsed()
        );
        Ok(())
    })
}
async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/world-change/lumberjack/report'),0)").fetch_one(&h.pool).await?)
}
async fn wood_quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i5901'").fetch_one(&h.pool).await?)
}
async fn gold(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1").fetch_one(&h.pool).await?)
}
pub(super) async fn seed_epoch(
    h: &Harness,
    world: crate::foundation::WorldId,
    epoch: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4),'1970-01') ON CONFLICT DO NOTHING").bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(server.as_slice()).bind((epoch*86400)as f64).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch").bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    crate::durability::world_event::changes::record_reset_fixture(&mut tx, world, epoch).await?;
    tx.commit().await?;
    Ok(())
}
