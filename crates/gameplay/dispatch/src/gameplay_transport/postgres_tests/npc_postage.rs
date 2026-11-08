//! NPCFIX: production talk windows and durable purchases use the same character postal rank.
use super::fixture::{Harness, debug, id, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[path = "npc_postage/duplicate_store.rs"]
mod duplicate_store;

#[path = "npc_postage/exact_shops.rs"]
mod exact_shops;

#[path = "npc_postage/bank_services.rs"]
mod bank_services;

#[path = "npc_postage/rashid_weekdays.rs"]
mod rashid_weekdays;

#[test]
fn npcfix_postgres_captured_bank_and_postal_services_and_replay() -> TestResult {
    postgres_services(&["muzir", "murim", "lokur", "wally"])
}

#[test]
fn npcfix_postgres_rashid_current_weekday_exact_trade_and_replay() -> TestResult {
    postgres_services(&["rashid"])
}

#[test]
fn npcfix_postgres_dispatch_services_and_replay() -> TestResult {
    postgres_services(&[
        "benjamin",
        "liane",
        "ray",
        "robson",
        "alesar",
        "nah_bob",
        "rashid",
        "kasmir",
        "albinius",
        "a_ghostly_guardian",
        "flint",
        "romella",
        "pugwah",
    ])
}

#[test]
fn npcfix_postgres_guardian_sorted_literal_topics_and_multipart_order() -> TestResult {
    postgres_services(&["a_ghostly_guardian"])
}

#[test]
fn npcfix_postgres_retained_exact_shop_prices_and_replay() -> TestResult {
    postgres_services(&["dixi", "lubo", "pompan", "shiantis", "toffee"])
}

#[test]
fn npcfix_postgres_additional_postal_services_and_replay() -> TestResult {
    postgres_services(&["olrik", "chemar", "chrystal", "fyodor", "nelly"])
}

#[test]
fn npcfix_postgres_ahmet_filled_fluid_mints_exact_matching_and_replay() -> TestResult {
    postgres_services(&["ahmet"])
}

fn postgres_services(keys: &'static [&'static str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(keys).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "postal PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario(keys: &[&str]) -> TestResult {
    run(async |admin| {
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = Arc::new(NpcTalkBook::from_model(&model));
        let mut failures = Vec::new();
        for &key in keys {
            reset();
            let h = Harness::create_with_admission_seed(
                admin.clone(),
                &format!("npc_postage_{key}"),
                true,
                false,
            )
            .await?;
            let result: TestResult = async {
                eprintln!("POSTGRES_STARTED NPCFIX postage {key}");
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
                let (controller, pin) = activated_with_loot_tables(world, None);
                let active = controller.active().ok_or("active content")?;
                let channel_pin = ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    pin.qualified_room().map_revision_digest(),
                    (2, 0, 7),
                );
                let deps = Dependencies::with_content_and_capacity(
                    channel_pin,
                    crate::content::qualify_native_entry_room(world).map_err(debug)?,
                    crate::spell::cast::v1_spell_book().map_err(debug)?,
                    2,
                )?;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                deps.initialize(&owner).await;
                sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice())
                    .bind(pin.identity().server_artifact_digest().as_slice())
                    .bind(pin.identity().client_artifact_digest().as_slice())
                    .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
                let npc_key = format!("oteryn:npc.{key}");
                assert!(book.get(&npc_key).is_some(), "postal talk entry {key}");
                owner.states.chat.install_npc_talk(Arc::clone(&book));
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
                    let mut spot = runtime
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position();
                    if key == "rashid" {
                        let at = oteryn_gameplay_entities::rashid_schedule::destination_at(
                            crate::gameplay_transport::npc_trade_command::unix_ms_now(),
                        )
                        .placement;
                        spot = crate::foundation::MovementLocalPosition {
                            x: i32::from(at.x) - 1,
                            y: i32::from(at.y),
                            floor: -i16::from(at.floor),
                        };
                        runtime
                            .place_admitted_respawn(deps.actor, session, Some(spot), |_| true)
                            .map_err(debug)?;
                    }
                    runtime
                        .place_npcs(
                            &NpcPlacementTable {
                                npcs: vec![NpcPlacements {
                                    key: npc_key.clone(),
                                    placements: vec![NpcPlacement {
                                        floor: -i8::try_from(spot.floor)?,
                                        x: u16::try_from(spot.x + 1)?,
                                        y: u16::try_from(spot.y)?,
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
                let exact_shop = exact_shops::find(key)?;
                let merchandise = match key {
                    "alesar" => Some("oteryn:item.tibia.i3297"),
                    "nah_bob" => Some("oteryn:item.tibia.i3284"),
                    "rashid" => Some("oteryn:item.tibia.i814"),
                    "albinius" => Some("oteryn:item.tibia.i23986"),
                    _ => None,
                };
                let mut inventory = vec![("oteryn:item.tibia.i3031", 100)];
                if key == "toffee" { inventory.push(("oteryn:item.tibia.i2854", 1)); }
                if let Some(item) = merchandise {
                    inventory.extend([(item, 1), (item, 1)]);
                }
                if let Some(contract) = &exact_shop {
                    for offer in &contract.offers {
                        if matches!(offer.direction, crate::content::ProjectV2ServiceOfferDirection::BuyFromPlayer) {
                            inventory.extend([(offer.item.key.as_str(), 1), (offer.item.key.as_str(), 1)]);
                        }
                    }
                }
                seed_main_backpack(&h, &inventory).await?;
                let current =
                    crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                        .current_session_at(session)
                        .await?
                        .0;
                let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
                let command = |command_id| UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence),
                };
                if matches!(key, "muzir" | "murim" | "lokur") {
                    bank_services::run(&h, &owner, deps.actor, npc, command,
                        "Yes? What may I do for you, Fixture Hero? Bank business, perhaps?").await?;
                    if key == "muzir" { return Ok(()); }
                }
                let command = |ordinal| command(if matches!(key, "murim" | "lokur") {
                    ordinal + 100
                } else { ordinal });
                let mut window = None;
                for (command_id, text) in [
                    (1, "hi"),
                    (2, if key == "robson" { "parcels" } else { "trade" }),
                ] {
                    let outcome = owner
                        .npc_talk(
                            deps.actor,
                            command(command_id),
                            NpcTalkIntent {
                                npc_actor: npc,
                                text: text.into(),
                            },
                        )
                        .await;
                    assert_eq!(
                        outcome.disposition,
                        NpcIntentDisposition::Ok,
                        "{key}: {text}"
                    );
                    window = outcome.window;
                }
                if let Some(contract) = &exact_shop {
                    if contract.offers.iter().any(|o| matches!(o.direction,
                        crate::content::ProjectV2ServiceOfferDirection::SellToPlayer)) {
                        seed_purchase_bank(&h).await?;
                    }
                    let window = window.ok_or("retained exact shop window")?;
                    let (_, revision) = window.open.ok_or("retained shop open")?;
                    let native = active.native_gameplay().ok_or("native shop catalogue")?;
                    let item_index = native.item_index().ok_or("native shop Item index")?;
                    for (offset, offer) in contract.offers.iter().enumerate() {
                        let item = &offer.item.key;
                        let reference = item_index.definition_ref("Item", item,
                            &offer.item.revision).ok_or_else(|| format!("{key}: missing native contract Item {item}"))?.get();
                        let side = match offer.direction {
                            crate::content::ProjectV2ServiceOfferDirection::SellToPlayer => NpcTradeSide::Buy,
                            crate::content::ProjectV2ServiceOfferDirection::BuyFromPlayer => NpcTradeSide::Sell,
                        };
                        let count = offer.count.unwrap_or(1);
                        let unit_price = u32::try_from(offer.unit_price)?;
                        let offer_index = window.offers.iter().position(|o|
                            o.item_definition_ref == reference && o.side == side
                            && o.unit_price == unit_price && o.count == count)
                            .ok_or_else(|| format!("{key}: exact retained offer missing {item}"))?;
                        let intent = NpcTradeIntent { npc_actor: npc, catalogue_revision: revision,
                            offer_index: u32::try_from(offer_index)?, side, quantity: 1,
                            expected_unit_price: unit_price };
                        let ordinal = 10 + u64::try_from(offset)? * if key == "toffee" { 3 } else { 2 };
                        let before_gold = exact_shops::gold(&h).await?;
                        let before_items = exact_shops::quantity(&h, item).await?;
                        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
                            .fetch_one(&h.pool).await?;
                        let wrong = NpcTradeIntent { expected_unit_price: unit_price + 1, ..intent };
                        assert!(matches!(owner.npc_trade(deps.actor, command(ordinal), wrong).await,
                            NpcTradeStep::Refused(_)), "{key}: mismatched price {item}");
                        assert_eq!(exact_shops::gold(&h).await?, before_gold, "wrong price never charges/pays");
                        assert_eq!(exact_shops::quantity(&h, item).await?, before_items);
                        let unchanged: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
                            .fetch_one(&h.pool).await?;
                        assert_eq!(unchanged, receipts);
                        for _ in 0..2 {
                            require_trade(owner.npc_trade(deps.actor, command(ordinal + 1), intent).await, item)?;
                        }
                        let price = i64::try_from(offer.unit_price)?;
                        let units = i64::from(count);
                        let (gold, quantity) = if side == NpcTradeSide::Buy {
                            (before_gold - price, before_items + units)
                        } else { (before_gold + price, before_items - units) };
                        assert_eq!(exact_shops::gold(&h).await?, gold, "{key}: exact gold once across replay {item}");
                        assert_eq!(exact_shops::quantity(&h, item).await?, quantity,
                            "{key}: exact physical units once across replay {item}");
                        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
                            .fetch_one(&h.pool).await?;
                        assert_eq!(after, receipts + 1, "one durable receipt per exact contract offer");
                        if key == "toffee" {
                            duplicate_store::apply(&h, &owner, deps.actor,
                                command(ordinal + 2), item).await?;
                            assert_eq!(exact_shops::gold(&h).await?, gold, "moving the retained duplicate preserves money");
                        }
                    }
                    return Ok(());
                }
                if key == "ahmet" {
                    let window = window.ok_or("Ahmet trade window")?;
                    let (_, revision) = window.open.ok_or("Ahmet open window")?;
                    let index = active.native_gameplay().and_then(|n| n.item_index())
                        .ok_or("native Item index")?;
                    let mut paid = 0_u32;
                    for (offset, (item, subtype, price)) in [
                        (2874, 7, 20), (2874, 1, 40), (2901, 1, 40),
                    ].into_iter().enumerate() {
                        let definition = format!("oteryn:item.tibia.i{item}");
                        let reference = index.definition_ref("Item", &definition, "definition-r1")
                            .ok_or("fluid container reference")?.get();
                        let source_subtype = u16::try_from(subtype)?;
                        assert!(book.get(&npc_key).is_some_and(|entry| entry.offers().iter().any(|offer|
                            offer.item.key == definition && offer.sub_type == Some(source_subtype)
                                && offer.count == Some(1) && offer.unit_price == u64::from(price))),
                            "exact sourced fluid subtype in one physical container");
                        let offer = window.offers.iter().position(|o| o.item_definition_ref == reference
                            && o.side == NpcTradeSide::Buy && o.count == 1 && o.unit_price == price)
                            .ok_or("Ahmet must offer the exact sourced filled container")?;
                        let intent = NpcTradeIntent { npc_actor: npc, catalogue_revision: revision,
                            offer_index: u32::try_from(offer)?, side: NpcTradeSide::Buy,
                            quantity: 1, expected_unit_price: price };
                        let ordinal = 30 + u64::try_from(offset)? * 3;
                        let wrong = NpcTradeIntent { expected_unit_price: price - 1, ..intent };
                        assert!(matches!(owner.npc_trade(deps.actor, command(ordinal), wrong).await,
                            NpcTradeStep::Refused(_)), "misquoted fluid purchase must not charge");
                        let bulk = NpcTradeIntent { quantity: 2, ..intent };
                        assert!(matches!(owner.npc_trade(deps.actor, command(ordinal + 1), bulk).await,
                            NpcTradeStep::Refused(_)), "stateful nonstackable purchase remains one physical container");
                        for _ in 0..2 {
                            require_trade(owner.npc_trade(deps.actor, command(ordinal + 2), intent).await,
                                "Ahmet filled container")?;
                        }
                        paid += price;
                        let gold: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3031'")
                            .fetch_one(&h.pool).await?;
                        assert_eq!(gold, i64::from(100 - paid), "exact price once across replay");
                        let containers: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_fluid_states f USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$1 AND i.quantity=1 AND f.fluid_subtype=$2")
                            .bind(&definition).bind(i16::try_from(subtype)?).fetch_one(&h.pool).await?;
                        assert_eq!(containers, 1, "one physical container with exact fluid subtype");
                        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
                            .fetch_one(&h.pool).await?;
                        assert_eq!(receipts, i64::try_from(offset + 1)?);
                    }
                    let mut tx = h.pool.begin().await?;
                    assert!(sqlx::query("UPDATE game_item_fluid_states SET fluid_subtype=8 WHERE fluid_subtype=7")
                        .execute(&mut *tx).await.is_err(), "mutable fluid state needs its own current proof");
                    tx.rollback().await?;
                    let mut tx = h.pool.begin().await?;
                    assert!(sqlx::query("DELETE FROM game_item_fluid_states WHERE fluid_subtype=7")
                        .execute(&mut *tx).await.is_err(), "old mint receipt cannot authorize emptying the container");
                    tx.rollback().await?;
                    return Ok(());
                }
                if matches!(key, "flint" | "romella" | "pugwah") {
                    seed_purchase_bank(&h).await?;
                    let window = window.ok_or("default-state purchase window")?;
                    let (_, revision) = window.open.ok_or("open purchase window")?;
                    let purchases = if key == "pugwah" { vec![(2880, 4, 0)] }
                        else { vec![(50293, 347_222, 500), (50294, 1_250_000, 1_800), (50295, 10_000_000, 14_400)] };
                    for (offset, (item_id, price, charges)) in purchases.iter().enumerate() {
                        let key = format!("oteryn:item.tibia.i{item_id}");
                        let reference = active.native_gameplay().and_then(|native| native.item_index())
                            .and_then(|index| index.definition_ref(crate::content::item_ref::ITEM_FAMILY, &key, "definition-r1"))
                            .ok_or("purchase Item reference")?.get();
                        let index = window.offers.iter().position(|o| o.item_definition_ref == reference && o.side == NpcTradeSide::Buy && o.unit_price == *price).ok_or("audited default-state offer")?;
                        let intent = NpcTradeIntent { npc_actor: npc, catalogue_revision: revision,
                            offer_index: u32::try_from(index)?, side: NpcTradeSide::Buy, quantity: 1, expected_unit_price: *price };
                        let command_id = 3 + u64::try_from(offset)?;
                        for _ in 0..2 {
                            require_trade(owner.npc_trade(deps.actor, command(command_id), intent).await, &key)?;
                        }
                        let item: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_npc_trades WHERE item_definition_production_key=$1")
                            .bind(&key).fetch_one(&h.pool).await?;
                        let item: [u8; 16] = item.try_into().map_err(|_| "purchase UUID")?;
                        if *charges > 0 {
                            let definition = crate::durability::item_mint::TypedDefinitionRef { family: "Item".into(), production_key: key, revision_ref: "definition-r1".into() };
                            let (definition, _) = crate::gameplay_transport::timed_item_content::facts(active.native_gameplay().ok_or("active native policy")?, &definition).ok_or("active full-charge policy")?;
                            let fence = owner.current_quest_fence(session).await.map_err(|_| "training fence")?.ok_or("session")?;
                            let weapon = h.root.read_training_weapon(&authority, &h.node, fence, item, definition).await.map_err(debug)?;
                            assert_eq!(weapon.charges, *charges);
                            assert_eq!(weapon.revision, 0, "fresh full state survives the trade replay");
                        } else {
                            let states: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fluid_states WHERE item_instance_id=encode($1,'hex')::uuid")
                                .bind(item.as_slice()).fetch_one(&h.pool).await?;
                            assert_eq!(states, 0, "empty mug uses the existing default empty-fluid state");
                        }
                    }
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades").fetch_one(&h.pool).await?;
                    assert_eq!(count, i64::try_from(purchases.len())?);
                    return Ok(());
                }
                if key == "a_ghostly_guardian" {
                    for (offset, text, count, expected) in [
                        (0, "bone master", 2, "By using stolen secrets"),
                        (1, "brotherhood", 3, "The Brotherhood intruded"),
                        (2, "evil", 3, "The Brotherhood intruded"),
                        (3, "dream master", 1, "The Dream Master is perhaps"),
                        (4, "job", 1, "Once I was a guardian"),
                        (5, "guardian", 1, "Once I was a guardian"),
                        (6, "nightmare knights", 2, "Once the Dream Challenge"),
                        (7, "dream challenge", 2, "Once the Dream Challenge"),
                    ] {
                        owner.clock_origin -= std::time::Duration::from_secs(1);
                        let outcome = owner.npc_talk(deps.actor, command(3 + offset), NpcTalkIntent {
                            npc_actor: npc, text: text.into(),
                        }).await;
                        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
                        let lines = outcome.conversation.ok_or("guardian topic")?.lines;
                        assert_eq!(lines.len(), count, "{text}");
                        assert!(lines[0].starts_with(expected), "{text}");
                    }
                    return Ok(());
                }
                if key == "albinius" {
                    let window = window.ok_or("Albinius trade window")?;
                    let (_, revision) = window.open.ok_or("Albinius open window")?;
                    let index = window.offers.iter().position(|offer| offer.side == NpcTradeSide::Sell && offer.unit_price == 30).ok_or("Heavy Old Tome offer")?;
                    let intent = NpcTradeIntent { npc_actor: npc, catalogue_revision: revision,
                        offer_index: u32::try_from(index)?, side: NpcTradeSide::Sell, quantity: 1, expected_unit_price: 30 };
                    for _ in 0..2 {
                        require_trade(owner.npc_trade(deps.actor, command(3), intent).await, key)?;
                    }
                    let prices: Vec<i64> = sqlx::query_scalar("SELECT unit_price::bigint FROM game_npc_trades").fetch_all(&h.pool).await?;
                    assert_eq!(prices, [30]);
                    return Ok(());
                }
                if key == "kasmir" {
                    use oteryn_gameplay_domain::domain::condition::DotElement;
                    use oteryn_gameplay_foundation::foundation::condition::{
                        ApplicationFacts, ConditionDefinition, ConditionSourceKind, DamageSchedule,
                        DamageSegment,
                    };
                    let runtime = deps.runtime.lock().await;
                    let mut spells = owner.states.spell_states.try_lock().map_err(debug)?;
                    let state = spells
                        .get_mut(&runtime, deps.actor, session)
                        .ok_or("healer player")?;
                    state.set_health_for_test(10);
                    let before = state.owned_conditions().clone();
                    let mut conditions = before.clone();
                    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                        runtime.content_pin().server_artifact_digest(),
                    );
                    let facts = ApplicationFacts {
                        now: owner.owner_now().get(),
                        base_speed: 1,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: true,
                        decision_root: &root,
                        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                            [0x6a; 16],
                        ),
                    };
                    for (name, element) in [
                        ("fire", DotElement::Fire),
                        ("poison", DotElement::Poison),
                        ("energy", DotElement::Energy),
                    ] {
                        let definition = ConditionDefinition::new_damage_schedule(
                            name,
                            1,
                            element,
                            DamageSchedule::Fixed {
                                delayed: true,
                                segments: vec![DamageSegment {
                                    count: 3,
                                    interval_ms: 60_000,
                                    amount: 1,
                                }],
                            },
                        )
                        .ok_or("healer condition")?;
                        conditions
                            .apply(
                                &definition,
                                Some("npcfix-test".to_owned()),
                                ConditionSourceKind::Creature,
                                &[],
                                &facts,
                            )
                            .map_err(debug)?;
                    }
                    state
                        .apply_batch_conditions(&before, &conditions)
                        .map_err(debug)?;
                    drop(spells);
                    drop(runtime);
                    for (command_id, remaining) in [(3, 2), (4, 1), (5, 0)] {
                        owner.clock_origin -= std::time::Duration::from_secs(1);
                        let healed = owner
                            .npc_talk(
                                deps.actor,
                                command(command_id),
                                NpcTalkIntent {
                                    npc_actor: npc,
                                    text: "heal".into(),
                                },
                            )
                            .await;
                        assert_eq!(healed.disposition, NpcIntentDisposition::Ok);
                        let runtime = deps.runtime.lock().await;
                        let spells = owner.states.spell_states.try_lock().map_err(debug)?;
                        let state = spells
                            .get(&runtime, deps.actor, session)
                            .ok_or("cured player")?;
                        assert_eq!(state.owned_conditions().instances().len(), remaining);
                        assert_eq!(
                            state.vitals().health,
                            10,
                            "a condition cure precedes healing"
                        );
                    }
                    owner.clock_origin -= std::time::Duration::from_secs(1);
                    let healed = owner
                        .npc_talk(
                            deps.actor,
                            command(6),
                            NpcTalkIntent {
                                npc_actor: npc,
                                text: "heal".into(),
                            },
                        )
                        .await;
                    assert_eq!(healed.disposition, NpcIntentDisposition::Ok);
                    let runtime = deps.runtime.lock().await;
                    let spells = owner.states.spell_states.try_lock().map_err(debug)?;
                    let state = spells
                        .get(&runtime, deps.actor, session)
                        .ok_or("healed player")?;
                    assert_eq!(state.vitals().health, 40);
                    let revision = state.revision();
                    drop(spells);
                    drop(runtime);
                    owner.clock_origin -= std::time::Duration::from_secs(1);
                    owner
                        .npc_talk(
                            deps.actor,
                            command(7),
                            NpcTalkIntent {
                                npc_actor: npc,
                                text: "heal".into(),
                            },
                        )
                        .await;
                    let runtime = deps.runtime.lock().await;
                    let spells = owner.states.spell_states.try_lock().map_err(debug)?;
                    let state = spells
                        .get(&runtime, deps.actor, session)
                        .ok_or("unchanged player")?;
                    assert_eq!((state.vitals().health, state.revision()), (40, revision));
                    return Ok(());
                }
                if let Some((quest, track, required)) =
                    crate::content::npc_catalogue::service::merchant_access(&npc_key)
                {
                    assert!(window.is_none(), "locked merchant must not open: {key}");
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica")
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) \
                        VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)")
                        .bind(id(41).as_slice()).bind(track).bind(quest).bind(required).execute(&mut *tx).await?;
                    tx.commit().await?;
                    owner.refresh_quest_session(session, true).await;
                    let window = owner
                        .npc_talk(
                            deps.actor,
                            command(3),
                            NpcTalkIntent {
                                npc_actor: npc,
                                text: "trade".into(),
                            },
                        )
                        .await
                        .window
                        .ok_or("eligible faction shop")?;
                    let (_, revision) = window.open.ok_or("open faction shop")?;
                    let item = merchandise.ok_or("faction merchandise")?;
                    let runtime = deps.runtime.lock().await;
                    let index = crate::gameplay_transport::item_ref_admission::pinned_index(
                        &runtime,
                        Some(active),
                    )
                    .ok_or("item index")?;
                    let definition = index
                        .definition_ref("Item", item, "definition-r1")
                        .ok_or("faction item reference")?
                        .get();
                    drop(runtime);
                    let (index, shown) = window
                        .offers
                        .iter()
                        .enumerate()
                        .find(|(_, o)| {
                            o.item_definition_ref == definition && o.side == NpcTradeSide::Sell
                        })
                        .ok_or("retail faction offer")?;
                    let intent = NpcTradeIntent {
                        npc_actor: npc,
                        catalogue_revision: revision,
                        offer_index: u32::try_from(index)?,
                        side: NpcTradeSide::Sell,
                        quantity: 1,
                        expected_unit_price: shown.unit_price,
                    };
                    assert!(
                        matches!(
                            owner.npc_trade(deps.actor, command(4), intent).await,
                            NpcTradeStep::Traded
                        ),
                        "unlocked {key}"
                    );
                    if key == "alesar" {
                        seed_purchase_bank(&h).await?;
                        let reference = active.native_gameplay().and_then(|native| native.item_index())
                            .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i3284", "definition-r1"))
                            .ok_or("ice rapier reference")?.get();
                        let index = window.offers.iter().position(|offer| offer.item_definition_ref == reference && offer.side == NpcTradeSide::Buy && offer.unit_price == 5_000)
                            .ok_or("ice rapier purchase")?;
                        let purchase = NpcTradeIntent { npc_actor: npc, catalogue_revision: revision,
                            offer_index: u32::try_from(index)?, side: NpcTradeSide::Buy, quantity: 1, expected_unit_price: 5_000 };
                        for _ in 0..2 {
                            assert!(matches!(owner.npc_trade(deps.actor, command(6), purchase).await, NpcTradeStep::Traded));
                        }
                    }
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica")
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("UPDATE game_character_quest_tracks SET value=0 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                        .bind(id(41).as_slice()).bind(track).execute(&mut *tx).await?;
                    tx.commit().await?;
                    assert!(
                        matches!(
                            owner.npc_trade(deps.actor, command(5), intent).await,
                            NpcTradeStep::Refused(_)
                        ),
                        "durable gate ignores stale window rank: {key}"
                    );
                    assert!(
                        matches!(
                            owner.npc_trade(deps.actor, command(4), intent).await,
                            NpcTradeStep::Traded
                        ),
                        "merchant replay {key}"
                    );
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
                        .fetch_one(&h.pool)
                        .await?;
                    assert_eq!(count, if key == "alesar" { 2 } else { 1 }, "no second sale or replay mutation {key}");
                    return Ok(());
                }
                let item_index = active
                    .native_gameplay()
                    .and_then(|native| native.item_index())
                    .ok_or("postal Item index")?;
                let parcel = item_index
                    .definition_ref("Item", "oteryn:item.tibia.i3503", "definition-r1")
                    .ok_or("postal parcel reference")?
                    .get();
                let letter = item_index
                    .definition_ref("Item", "oteryn:item.tibia.i3505", "definition-r1")
                    .ok_or("postal letter reference")?
                    .get();
                let ordinary = window.ok_or("ordinary postal window")?;
                let (_, revision) = ordinary.open.ok_or("open postal window")?;
                let ordinary_index = ordinary
                    .offers
                    .iter()
                    .position(|o| {
                        o.item_definition_ref == parcel
                            && o.side == NpcTradeSide::Buy
                            && o.unit_price == 15
                    })
                    .ok_or("parcel at 15")?;
                let mut intent = NpcTradeIntent {
                    npc_actor: npc,
                    catalogue_revision: revision,
                    offer_index: u32::try_from(ordinary_index)?,
                    side: NpcTradeSide::Buy,
                    quantity: 1,
                    expected_unit_price: 15,
                };
                assert!(
                    matches!(
                        owner.npc_trade(deps.actor, command(3), intent).await,
                        NpcTradeStep::Traded
                    ),
                    "ordinary {key}"
                );
                // The existing Character root serializes this rank with each item trade.
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) \
                    VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.the_postman_missions_quest',3,2)")
                    .bind(id(41).as_slice()).bind(crate::content::npc_catalogue::service::POSTMAN_RANK_TRACK)
                    .execute(&mut *tx).await?;
                tx.commit().await?;
                if key != "robson" {
                    assert!(
                        matches!(
                            owner.npc_trade(deps.actor, command(4), intent).await,
                            NpcTradeStep::Refused(_)
                        ),
                        "stale ordinary price after completion"
                    );
                }
                // Replaying the old occurrence still returns its original committed price.
                assert!(matches!(
                    owner.npc_trade(deps.actor, command(3), intent).await,
                    NpcTradeStep::Traded
                ));
                owner.refresh_quest_session(session, true).await;
                let fresh = owner
                    .npc_talk(
                        deps.actor,
                        command(5),
                        NpcTalkIntent {
                            npc_actor: npc,
                            text: "trade".into(),
                        },
                    )
                    .await
                    .window
                    .ok_or("completed postal window")?;
                let expected = if key == "robson" { 15 } else { 10 };
                let index = fresh
                    .offers
                    .iter()
                    .position(|o| {
                        o.item_definition_ref == parcel
                            && o.side == NpcTradeSide::Buy
                            && o.unit_price == expected
                    })
                    .ok_or("completed parcel price")?;
                if key != "robson" {
                    assert!(
                        fresh.offers.iter().any(|o| {
                            o.item_definition_ref == letter
                                && o.side == NpcTradeSide::Buy
                                && o.unit_price == 5
                        }),
                        "completed letter price"
                    );
                }
                intent.offer_index = u32::try_from(index)?;
                intent.expected_unit_price = expected;
                assert!(
                    matches!(
                        owner.npc_trade(deps.actor, command(6), intent).await,
                        NpcTradeStep::Traded
                    ),
                    "completed {key}"
                );
                assert!(
                    matches!(
                        owner.npc_trade(deps.actor, command(6), intent).await,
                        NpcTradeStep::Traded
                    ),
                    "completed replay {key}"
                );
                if key != "robson" {
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                    sqlx::query("UPDATE game_character_quest_tracks SET value=2 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                        .bind(id(41).as_slice()).bind(crate::content::npc_catalogue::service::POSTMAN_RANK_TRACK)
                        .execute(&mut *tx).await?;
                    tx.commit().await?;
                    assert!(matches!(owner.npc_trade(deps.actor, command(7), intent).await, NpcTradeStep::Refused(_)), "rank two cannot use a stale Grand Postman price");
                    assert!(matches!(owner.npc_trade(deps.actor, command(6), intent).await, NpcTradeStep::Traded), "committed replay survives rank revocation");
                }
                let trades: Vec<i64> = sqlx::query_scalar(
                    "SELECT unit_price::bigint FROM game_npc_trades ORDER BY unit_price",
                )
                .fetch_all(&h.pool)
                .await?;
                let mut expected_prices = vec![15, i64::from(expected)];
                expected_prices.sort();
                assert_eq!(trades, expected_prices, "one debit per occurrence, {key}");
                Ok(())
            }.await;
            h.cleanup().await?;
            if let Err(error) = result {
                failures.push(format!("{key}: {error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n").into())
        }
    })
}

fn require_trade(step: NpcTradeStep, context: &str) -> TestResult {
    match step {
        NpcTradeStep::Traded => Ok(()),
        NpcTradeStep::Refused(refusal) => {
            Err(format!("{context}: refused {:?}", refusal.disposition).into())
        }
        NpcTradeStep::Unknown => Err(format!("{context}: unknown storage outcome").into()),
    }
}

async fn seed_purchase_bank(h: &Harness) -> TestResult {
    // Bank fallback belongs to the existing V2 audit owner; V1 intentionally stays coins-only.
    // Activate through the same migration-owner boundary as the native hireling bank fixture.
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES (1, now())")
        .execute(&h.pool)
        .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let hex = |n: u8| id(n).iter().map(|b| format!("{b:02x}")).collect::<String>();
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES('{occ}'::uuid,'{op}'::uuid,'{event}'::uuid,1,sha256('seed'),'{character}'::uuid,'{account}'::uuid,'{world}'::uuid,'{channel}'::uuid,1,20000000,0,1,sha256('seed'),0,'{p}'::uuid,'{g}'::uuid,'{backpack}'::uuid); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES('{entry}'::uuid,'{op}'::uuid,'{account}'::uuid,'{world}'::uuid,1,20000000,0,20000000,'{character}'::uuid); INSERT INTO game_account_bank_balances VALUES('{account}'::uuid,'{world}'::uuid,20000000,'{entry}'::uuid);",occ=hex(80),op=hex(81),event=hex(82),character=hex(41),account=hex(40),world=hex(42),channel=hex(43),p=hex(83),g=hex(84),backpack=hex(70),entry=hex(85)))).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
