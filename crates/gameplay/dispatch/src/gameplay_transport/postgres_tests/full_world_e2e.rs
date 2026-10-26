//! E2E-WORLD-1: one Character's playable chain on one bundle World through the composed owner
//! and PostgreSQL: admission, paced steps, an NPC purchase, an auto-attack kill with its loot, a
//! depot deposit over the real connection, logout and a successor admission. After the relogin
//! the durable gold, items, depot and position are exactly what the chain committed, and every
//! replayed step answers from its stored outcome without moving value a second time.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{ItemTransferDestination, ItemTransferRequest};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementFacing,
    MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, UseCommand,
};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use crate::gameplay_transport::last_position::AdmissionPosition;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::depot::*;
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
use std::time::Duration;

const GOLD: &str = "oteryn:item.tibia.i3031";
const BACKPACK: &str = "oteryn:item.tibia.i2854";
/// A generated-reply merchant whose Service sells cheap goods for plain coin.
const NPC: &str = "oteryn:npc.asphota";
/// The pinned entry start; two paced West steps reach the trade cell.
const ENTRY: (i32, i32) = (10, 0);
const TRADE_CELL: MovementLocalPosition = MovementLocalPosition {
    x: 8,
    y: 0,
    floor: 7,
};
/// Beside the depot locker the bundle fixture places at x 6.
const LOCKER_CELL: MovementLocalPosition = MovementLocalPosition {
    x: 7,
    y: 0,
    floor: 7,
};
const NPC_CELL: (u16, u16) = (9, 1);
const RAT_CELL: MovementLocalPosition = MovementLocalPosition {
    x: 8,
    y: 1,
    floor: 7,
};
const DEPOT_BOX: u8 = 17;

#[test]
fn full_world_chain_survives_relogin_and_replays_move_no_value_on_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("full-world-e2e", scenario)
}

/// Every durable value the chain can move, as one comparable row set.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ledger {
    /// `(production key, quantity, instance)` of the main backpack's entries, ordered.
    backpack: Vec<(String, i64, [u8; 16])>,
    npc_trades: i64,
    npc_trade_lines: i64,
    xp_receipts: i64,
    experience: i64,
    corpse_mints: i64,
    corpse_entries: i64,
    transfer_receipts: i64,
    depot_receipts: i64,
    depot_counter: Option<(String, i32)>,
    live_items: i64,
    live_quantity: i64,
}

async fn ledger(h: &Harness) -> TestResult<Ledger> {
    let backpack: Vec<(String, i64, Vec<u8>)> = sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity::bigint, decode(replace(i.item_instance_id::text,'-',''),'hex') \
         FROM game_item_container_entries e JOIN game_item_instances i USING (item_instance_id) \
         WHERE e.character_id = encode($1,'hex')::uuid ORDER BY e.placement_ordinal",
    )
    .bind(id(41).as_slice())
    .fetch_all(&h.pool)
    .await?;
    let count = |sql: &'static str| async move {
        Ok::<i64, Box<dyn std::error::Error>>(
            sqlx::query_scalar(sql).fetch_one(&h.pool).await?,
        )
    };
    Ok(Ledger {
        backpack: backpack
            .into_iter()
            .map(|(key, quantity, id)| Ok((key, quantity, id.as_slice().try_into()?)))
            .collect::<TestResult<_>>()?,
        npc_trades: count("SELECT count(*) FROM game_npc_trades").await?,
        npc_trade_lines: count("SELECT count(*) FROM game_npc_trade_lines").await?,
        xp_receipts: count("SELECT count(*) FROM game_character_xp_receipts").await?,
        experience: count("SELECT total_experience FROM game_character_progression_state")
            .await?,
        corpse_mints: sqlx::query_scalar(
            "SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key = $1 \
             AND materialized_at IS NOT NULL",
        )
        .bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY)
        .fetch_one(&h.pool)
        .await?,
        corpse_entries: count("SELECT count(*) FROM game_item_corpse_container_entries").await?,
        transfer_receipts: count("SELECT count(*) FROM game_item_transfer_receipts").await?,
        depot_receipts: count(
            "SELECT count(*) FROM game_item_loop_move_receipts WHERE depot_town=0 AND depot_box=17",
        )
        .await?,
        depot_counter: sqlx::query_as(
            "SELECT next_ordinal::text, committed FROM game_character_depot_box_counters \
             WHERE town_id=0 AND box_number=17",
        )
        .fetch_optional(&h.pool)
        .await?,
        live_items: count("SELECT count(*) FROM game_item_instances WHERE lifecycle = 1").await?,
        live_quantity: count(
            "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle = 1",
        )
        .await?,
    })
}

fn gold(ledger: &Ledger) -> i64 {
    ledger
        .backpack
        .iter()
        .filter(|(key, _, _)| key == GOLD)
        .map(|(_, quantity, _)| quantity)
        .sum()
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

async fn step_west(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<MovementLocalPosition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    let (outcome, _) = owner
        .paced_step(actor, session, StepDirection::West)
        .await;
    assert_eq!(outcome.disposition, StepDisposition::Moved, "{outcome:?}");
    position(owner, actor).await
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "full_world_e2e", false, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;

        // One production generation (spells manifest with loot tables) for items, NPC offers,
        // the rat and its loot.
        let (controller, activated) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world_id, None,
            );
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("item index")?;
        let typed = |key: &str| {
            crate::gameplay_transport::item_look::typed_definition(
                index,
                index.definition_ref("Item", key, "definition-r1")?,
            )
        };
        let backpack_definition = typed(BACKPACK).ok_or("backpack definition")?;
        let gold_definition = typed(GOLD).ok_or("gold definition")?;
        let backpack_facts = generation_item_facts(native, &backpack_definition).map_err(debug)?;
        let gold_facts = generation_item_facts(native, &gold_definition).map_err(debug)?;
        let wire_id = |key: &str| -> TestResult<u32> {
            Ok(index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("wire id")?
                .get()
                - 1)
        };

        // The depot bundle fixture (locker at x 6), widened with open grass for the walk, the
        // NPC and the rat, through the production writer, loader and boot.
        let (bytes, mut bundle_pins) = bundle_fixture::bundle_with_item_ids(
            (BACKPACK, wire_id(BACKPACK)?),
            (GOLD, wire_id(GOLD)?),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        // The fixture's `terrain:grass` carries no Tibia source id, which leaves its tiles'
        // spell facts unknown and refuses every attack on them (BUNDLE-TILES-1): route the
        // ground through a Tibia ground key, as the PvP bundle fixture does.
        bundle.manifest.palette.first_mut().ok_or("ground palette")?.key =
            "oteryn:item.tibia.i100".into();
        let sector = bundle.sectors.first_mut().ok_or("bundle sector")?;
        sector
            .tiles
            .iter_mut()
            .find(|tile| tile.x == 6)
            .ok_or("locker tile")?
            .items
            .get_mut(1)
            .ok_or("locker item")?
            .attrs
            .depot = Some(0);
        let grass = |x, y| oteryn_world_bundle::sector::Tile {
            x,
            y,
            flags: 0,
            house: 0,
            zones: vec![],
            items: vec![oteryn_world_bundle::sector::Item {
                palette: 0,
                depth: 0,
                attrs: oteryn_world_bundle::sector::Attrs::default(),
            }],
        };
        sector.tiles.extend((8..=12).map(|x| grass(x, 0)));
        sector.tiles.extend((6..=12).map(|x| grass(x, 1)));
        sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        bundle_pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &bundle_pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: bundle_pins,
                start: crate::map::overlay::TilePos {
                    x: u16::try_from(ENTRY.0)?,
                    y: u16::try_from(ENTRY.1)?,
                    floor: -7,
                },
            },
            world_id,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            |key| {
                Some(ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key == BACKPACK),
                    blocks_projectile: false,
                    pickupable: key == GOLD,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(world_id),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: ENTRY.0,
                    y: ENTRY.1,
                    floor: 7,
                },
            )
            .ok_or("bundle combat room")?;
        let pin = ChannelContentPin::from_activation(
            world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (ENTRY.0, ENTRY.1, 7),
        );
        let mut deps =
            Dependencies::with_content(pin.clone(), room.clone(), native.spell_book().clone())?;
        deps.combat_room = Some(room.clone());
        // The Channel holds the player, the merchant, the rat and the successor's actor, as the
        // autowalk case widens its runtime: the fixture default seats two.
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin.clone(),
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;

        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice())
            .execute(&h.pool).await?;

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(600))
            .ok_or("a monotonic clock older than ten minutes")?;

        // 1. Login and admission: the committed fresh session enters the Channel on the pinned
        // entry start with its Character lease bound, and its session is tracked for logout.
        let fence = crate::gameplay_transport::item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0,
        )
        .ok_or("item fence")?;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            // Minimum fist damage stays above the rat's defence and armor.
                            level: 50,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }
        track(&owner, &deps)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());
        assert_eq!(
            position(&owner, actor).await?,
            MovementLocalPosition {
                x: ENTRY.0,
                y: ENTRY.1,
                floor: 7
            }
        );

        // The purse: a backpack and 100 gold through the existing MINT and TRANSFER owners.
        for (ordinal, definition, quantity) in [
            (0_u64, &backpack_definition, 1_u32),
            (1, &gold_definition, 100_u32),
        ] {
            let mut seed = h
                .root
                .freeze_item_mint(
                    &authority,
                    &h.node,
                    ItemMintRequest {
                        cause: ItemMintCause::for_test(
                            world_id,
                            deps.admitted.channel_id,
                            fence.scope_ownership_generation,
                            u32::try_from(ordinal + 1)?,
                            1,
                            TypedDefinitionRef {
                                family: "LootTable".into(),
                                production_key: "fixture:loot.full-world-purse".into(),
                                revision_ref: "definition-r1".into(),
                            },
                            "fixture:purpose.full-world-purse".into(),
                            0,
                        ),
                        item: definition.clone(),
                        quantity,
                        ground: GroundPlacement {
                            spatial_position:
                                crate::durability::map_item_mint_audit::map_item_spatial_position(
                                    u16::try_from(ENTRY.0)?, u16::try_from(ENTRY.1)?, -7,
                                ),
                            corpse_ref: id(3).to_vec(),
                            map_revision: world.map_revision().into(),
                            content_revision: "content-1".into(),
                            native_room_placement_context: world.base().digest().to_vec(),
                        },
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            let permit = owner.spell_lane_permit().await.ok_or("lane")?;
            let minted = match h
                .root
                .commit_item_mint(&permit, &authority, &h.node, &mut seed)
                .await
                .map_err(debug)?
            {
                ItemMintOutcome::Committed(item) => item.item_instance_id,
                other => return Err(debug(other)),
            };
            let mut transfer = h
                .root
                .freeze_item_transfer(
                    &authority,
                    &h.node,
                    fence,
                    ItemTransferRequest {
                        weight_generation_digest: native.source_digest(),
                        command: CommandRef::new(
                            session,
                            CommandId::new(900 + ordinal).map_err(debug)?,
                        ),
                        source_item_instance_id: minted,
                        destination: if ordinal == 0 {
                            ItemTransferDestination::ContainerSlot
                        } else {
                            ItemTransferDestination::MainBackpack
                        },
                        item: if ordinal == 0 {
                            backpack_facts.clone()
                        } else {
                            gold_facts.clone()
                        },
                        backpack: (ordinal != 0).then(|| backpack_facts.clone()),
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                    &NativeItemWeights(native),
                )
                .await
                .map_err(debug)?;
            h.root
                .commit_item_transfer(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    &mut transfer,
                    &NativeItemWeights(native),
                )
                .await
                .map_err(debug)?;
        }
        let purse = ledger(&h).await?;
        assert_eq!(gold(&purse), 100, "{purse:?}");

        // 2. Move: two paced West steps through the real step owner on the bundle cells.
        let mut at = step_west(&mut owner, actor, session).await?;
        at = step_west(&mut owner, actor, session).await.map_err(|e| format!("{at:?}: {e}"))?;
        assert_eq!(at, TRADE_CELL);

        // 3. Buy from an NPC: the production NPC book, a placed NPC beside the trade cell,
        // NPC_TALK "hi" then "trade" opens the window, NPC_TRADE buys its first Buy offer.
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
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                floor: -7,
                                x: NPC_CELL.0,
                                y: NPC_CELL.1,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let use_command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let mut window = None;
        for (command_id, text) in [(10, "hi"), (11, "trade")] {
            let outcome = owner
                .npc_talk(
                    actor,
                    use_command(command_id),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert!(!outcome.unknown, "{text}");
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
            window = outcome.window;
        }
        let window = window.ok_or("the trade window")?;
        let (_, revision) = window.open.ok_or("an open trade window")?;
        let mut bought = None;
        for (offer_index, offer) in window.offers.iter().enumerate() {
            if offer.side != NpcTradeSide::Buy || u64::from(offer.unit_price) > 100 {
                continue;
            }
            let intent = NpcTradeIntent {
                npc_actor: npc,
                catalogue_revision: revision,
                offer_index: u32::try_from(offer_index)?,
                side: NpcTradeSide::Buy,
                quantity: 1,
                expected_unit_price: offer.unit_price,
            };
            let command_id = 12 + u64::try_from(offer_index)?;
            if let NpcTradeStep::Traded = owner
                .npc_trade(actor, use_command(command_id), intent)
                .await
            {
                bought = Some((command_id, intent, *offer));
                break;
            }
        }
        let (trade_command, trade_intent, offer) = bought.ok_or("a Buy offer trades")?;
        let traded = ledger(&h).await?;
        assert_eq!(traded.npc_trades, 1, "{traded:?}");
        assert_eq!(
            gold(&traded),
            100 - i64::from(offer.unit_price),
            "the purchase charged its unit price once"
        );
        let item = traded
            .backpack
            .iter()
            .find(|(key, _, id)| {
                key != GOLD && !purse.backpack.iter().any(|(_, _, seen)| seen == id)
            })
            .map(|(_, _, id)| *id)
            .ok_or("the bought item is in the backpack")?;
        // Replay: the same command and intent answer from the stored trade.
        assert!(matches!(
            owner
                .npc_trade(actor, use_command(trade_command), trade_intent)
                .await,
            NpcTradeStep::Traded
        ));
        assert_eq!(ledger(&h).await?, traded, "an NPC trade replay moves nothing");

        // 4. Kill a creature and loot it: a production rat beside the trade cell, held as the
        // auto-attack target and driven by the real swing and reward drains.
        let rat = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .realize_native_qualification_spawn(pin.clone(), "oteryn:creature.rat", RAT_CELL)
                .map_err(|e| format!("rat spawn: {e:?}"))?;
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .ok_or("the rat is visible")?;
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };
        {
            let runtime = owner.runtime.lock().await;
            let now = owner.owner_now();
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    now,
                    Some(rat),
                    1
                ),
                AttackIntentDisposition::Ok
            );
        }
        let mut swings = 0;
        loop {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
            swings += 1;
            assert!(swings < 64, "the held rat dies to auto-attack swings");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .ok_or("owner clock")?;
        }
        let mut drains = 0;
        while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
            drains += 1;
            assert!(drains < 8, "the queued kill settles");
        }
        let killed = ledger(&h).await?;
        assert_eq!((killed.xp_receipts, killed.experience), (1, 5));
        assert_eq!(killed.corpse_mints, 1);
        // Replay of the settlement: the drained queue settles nothing again.
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        assert_eq!(ledger(&h).await?, killed, "a drained kill settles once");
        let corpse = owner
            .runtime
            .lock()
            .await
            .visible_corpses()
            .into_iter()
            .next()
            .ok_or("the kill's visible corpse")?;
        let contents = h
            .root
            .read_corpse_contents(&authority, corpse.item.item_instance_id)
            .await
            .map_err(debug)?
            .ok_or("corpse contents")?;
        let loot = contents.first().ok_or("the rat's corpse carries loot")?;
        let (loot_id, loot_quantity) = (loot.item.item_instance_id, loot.item.quantity);
        owner
            .take_corpse_entry(
                actor,
                use_command(20),
                ItemKey::Entity(corpse.identity),
                ItemKey::Instance(loot_id),
            )
            .await
            .map_err(|e| format!("loot take: {e:?}"))?;
        let looted = ledger(&h).await?;
        assert_eq!(looted.corpse_entries, killed.corpse_entries - 1);
        assert_eq!(
            looted.backpack.iter().map(|(_, q, _)| q).sum::<i64>(),
            killed.backpack.iter().map(|(_, q, _)| q).sum::<i64>() + i64::from(loot_quantity),
            "the loot entry moved into the backpack once"
        );
        let _ = owner
            .take_corpse_entry(
                actor,
                use_command(20),
                ItemKey::Entity(corpse.identity),
                ItemKey::Instance(loot_id),
            )
            .await;
        assert_eq!(ledger(&h).await?, looted, "a loot replay moves nothing");

        // 5. Deposit at the depot over the real connection: step beside the locker, then
        // DEPOT_INTENT browse and deposit of the bought item through `serve_admitted`.
        assert_eq!(step_west(&mut owner, actor, session).await?, LOCKER_CELL);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37])
                .ok_or("selected capabilities")?;
        let mut view = SessionItemView::resume(ItemViewContinuity::default());
        view.snapshot(
            owner
                .observe_character_inventory(actor, session)
                .await
                .ok_or("inventory")?,
            None,
        )
        .map_err(debug)?;
        let handle = view
            .table()
            .handle(&ItemKey::Instance(item))
            .ok_or("bought item handle")?;
        let browse = DepotIntent {
            town: None,
            box_number: DEPOT_BOX,
            page: 0,
            action: DepotAction::Browse,
        };
        let deposit = DepotIntent {
            town: Some(0),
            action: DepotAction::Deposit(handle),
            ..browse
        };
        let mut input = Vec::new();
        for (command_id, intent) in [(1, browse), (2, deposit)] {
            let payload = encode_depot_intent(intent).map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_DEPOT_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            input.extend(u32::try_from(frame.len())?.to_be_bytes());
            input.extend(frame);
        }
        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        let mut replies = Vec::new();
        let mut cursor = output.as_slice();
        while !cursor.is_empty() {
            let length =
                u32::from_be_bytes(cursor.get(..4).ok_or("frame length")?.try_into()?) as usize;
            let frame = cursor.get(4..4 + length).ok_or("frame")?;
            cursor = &cursor[4 + length..];
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
            if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                replies.push((
                    result.status,
                    decode_depot_reply(result.payload).map_err(debug)?.outcome,
                ));
            }
        }
        assert_eq!(
            replies,
            [
                (
                    oteryn_protocol_oteryn::CommandStatus::Accepted,
                    DepotOutcome::Opened
                ),
                (
                    oteryn_protocol_oteryn::CommandStatus::Accepted,
                    DepotOutcome::Moved
                )
            ],
            "{end}"
        );
        let deposited = ledger(&h).await?;
        assert_eq!(deposited.depot_receipts, 1);
        assert_eq!(deposited.depot_counter, Some(("2".into(), 1)));
        assert!(
            !deposited.backpack.iter().any(|(_, _, id)| *id == item),
            "the deposited item left the backpack"
        );
        assert_eq!(deposited.live_items, looted.live_items);
        assert_eq!(deposited.live_quantity, looted.live_quantity);
        // Replay of the deposit with the connection's outcome cache gone: durable, no second move.
        let replay = owner
            .depot(actor, use_command(2), deposit, None)
            .await
            .ok_or("durable depot replay")?;
        assert_eq!(replay.outcome, DepotOutcome::Moved);
        assert_eq!(ledger(&h).await?, deposited, "a depot replay moves nothing");

        // 6. Logout: the terminal release writes the final cell; a replay is TERMINAL and a
        // second logout finds no actor. Out of fight first: the owner clock moves past it.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(120))
            .ok_or("owner clock")?;
        assert_eq!(
            owner.logout_session(admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let stored: (i32, i32, i16) = sqlx::query_as(
            "SELECT tile_x, tile_y, tile_floor FROM game_character_last_positions \
             WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stored, (7, 0, 7));
        let replay = FreshAdmissionStore::from_root(h.root.clone())
            .release_logout_session(
                session,
                &crate::gameplay_transport::canonical_uuid(&id(40)),
                admitted.controller.ok_or("controller")?.transport,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(replay, ExpiredLossReleaseV1::Terminal));
        assert_eq!(
            owner.logout_session(admitted).await,
            LogoutReply::Result(LogoutResult::Busy)
        );
        let logged_out = ledger(&h).await?;
        assert_eq!(logged_out, deposited, "logout moves no value");

        // 7. Relogin: the successor fresh admission of the same Character, then its admission
        // position from the durable last position through the production owner steps.
        let next_session = relogin(&h).await?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let current = store.current_session(next_session).await?;
        assert_eq!(current.current_character_lease().generation(), 3);
        let next_fence = crate::gameplay_transport::item_fence_of(current).ok_or("next fence")?;
        let next_actor = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(next_session).map_err(debug)?;
            runtime.commit_fresh_session(reservation).map_err(debug)?
        };
        let gameplay_fence = owner
            .current_quest_fence(next_session)
            .await
            .map_err(|()| "successor fence")?
            .ok_or("successor session")?;
        let decided = owner
            .read_admission_position(gameplay_fence)
            .await
            .map_err(|()| "admission position")?;
        assert!(
            matches!(decided, AdmissionPosition::Stored(_)),
            "the relogin reads the stored last position"
        );
        {
            let mut runtime = deps.runtime.lock().await;
            owner
                .initialize_admitted_position(&mut runtime, next_actor, next_session, &decided)
                .await
                .map_err(debug)?;
        }
        assert!(owner.track_restored(next_session, next_actor, &decided, gameplay_fence));
        assert_eq!(position(&owner, next_actor).await?, LOCKER_CELL);
        let relogged = ledger(&h).await?;
        assert_eq!(relogged, deposited, "relogin moves no value");
        let depot_view = owner
            .depot(
                next_actor,
                UseCommand {
                    game_session_id: next_session,
                    command_id: 1,
                    item_fence: Some(next_fence),
                },
                DepotIntent {
                    town: Some(0),
                    ..browse
                },
                None,
            )
            .await
            .ok_or("successor depot browse")?;
        assert_eq!(depot_view.outcome, DepotOutcome::Opened);
        assert_eq!(depot_view.entries.len(), 1);
        assert_eq!(depot_view.entries[0].count, 1);
        assert_eq!(ledger(&h).await?, deposited, "browsing moves no value");
        drop(owner);
        drop(authority);
        drop(seal);

        // A reopened Character authority reads the same backpack the chain left.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let reloaded = h
            .root
            .read_character_backpack(&reopened, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack reload")?;
        let mut durable = reloaded
            .entries
            .iter()
            .map(|entry| (entry.item.quantity, entry.item.item_instance_id))
            .collect::<Vec<_>>();
        durable.sort();
        let mut expected = deposited
            .backpack
            .iter()
            .map(|(_, quantity, id)| Ok((u32::try_from(*quantity)?, *id)))
            .collect::<TestResult<Vec<_>>>()?;
        expected.sort();
        assert_eq!(durable, expected);
        drop(reopened);
        drop(seal);
        drop(request);
        h.cleanup().await
    })
}

/// The successor admission of the fixture Character, as the loop-training reload does: refresh
/// the fixture Platform sources through their owner API, then commit the next fresh admission.
async fn relogin(h: &Harness) -> TestResult<GameSessionId> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityGuardStateV1 as State,
        AdmissionAuthorityPublicationV1, AdmissionPublicationPreconditionV1,
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let mut source = super::fresh_fixture::fresh::Source::new(now)?;
    let keys = source
        .rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current source publications")?;
    let mut refresh = super::fresh_fixture::fresh::Source::new(now)?;
    refresh.rows = source
        .rows
        .iter()
        .filter(|row| matches!(row.key, Key::Account { .. } | Key::SigningTrust { .. }))
        .cloned()
        .collect();
    for row in &mut refresh.rows {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: row.publication_revision,
        };
        row.publication_revision += 1;
        row.source.source_revision += 1;
        row.source.decision_identity = format!("full-world-source-{}", row.source.source_revision);
        row.source.source_observed_at = now;
        if let State::Account { security, .. } = &mut row.state {
            security.provenance.publication_revision = row.publication_revision;
            security.provenance.source_revision = row.source.source_revision;
            security.provenance.accepted_source_revision = row.source.source_revision;
            security.provenance.decision_identity = row.source.decision_identity.clone();
            security.provenance.accepted_decision_identity = row.source.decision_identity.clone();
            security.provenance.source_observed_at = now;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&refresh, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("refreshed current publications")?;
    source.current.character_lease_generation = 2;
    let next = source
        .appearance_observer_request(&crate::gameplay_transport::canonical_uuid(&id(41)), now)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&next).await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(
            _
        )
    ));
    // A replayed commit of the same request admits no third lease.
    let _ = store.commit(&next).await;
    let next_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    assert_eq!(
        store
            .current_session(next_session)
            .await?
            .current_character_lease()
            .generation(),
        3
    );
    Ok(next_session)
}
