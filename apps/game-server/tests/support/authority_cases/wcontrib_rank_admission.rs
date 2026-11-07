//! Real stored-rank trade/travel consumers, using native catalogue routes and offers.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_last_position::LastPositionCell;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use crate::durability::npc_trade::{
    NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeOutcome, NpcTradeRequest, NpcTradeSide,
};
use crate::durability::npc_travel::{
    NpcTravelGate, PaidTravelRequest, PendingArrival, TravelOccurrence, TravelRefusal,
};
use crate::durability::quest_state::QuestTransitionOutcome;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use crate::gameplay_transport::NpcTalkBook;
use crate::wcontrib_rathleton as rath;
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute;
use oteryn_gameplay_content::content::{
    NpcDataCatalogue, ProjectV2ServiceOfferDirection, npc_catalogue_preproduction_limits,
};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use std::sync::Arc;

type TestResult<T = ()> = fixture::TestResult<T>;
struct Premium;
impl NpcTravelGate for Premium {
    async fn travel_permitted(
        &self,
        _tx: &mut Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}

#[test]
fn wcontrib_rank_admission_postgres_real_trades_boats_and_paid_arrivals() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wcontrib-rank-admission-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(exercise(admin))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "rank admission PG thread failed")??;
    Ok(())
}
async fn exercise(admin: String) -> TestResult {
    let started = std::time::Instant::now();
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    );
    let book = book()?;
    let h = fixture::Harness::create(admin.clone(), "wcontrib_rank_consumers", true).await?;
    let result = rank_consumers(&h, &catalogue, &book).await;
    h.cleanup().await?;
    result?;
    let h = fixture::Harness::create(admin, "wcontrib_winner_arrival", true).await?;
    let result = winner_arrival(&h, &catalogue, &book).await;
    h.cleanup().await?;
    result?;
    eprintln!(
        "WCONTRIB_RANK_ADMISSION_PG_EXECUTED: actual Alaistar/Flint native trades, promoted rank versus spent, citizen boat denials/acceptance, Squire and elected-dungeon paid-arrival commits/replay; duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
fn book() -> TestResult<NpcTalkBook> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(fixture::debug)?;
    let model = catalogue
        .service_model_for_project()
        .map_err(fixture::debug)?;
    let book = NpcTalkBook::from_model(&model);
    // This is the production native conversation registry, not synthetic TravelRoutes.
    for (npc, keywords) in [
        (
            "marvin",
            &[
                "dungeon golems",
                "dungeon minotaurs",
                "dungeon wrathofevil",
                "hero of rathleton",
            ][..],
        ),
        ("palomino", &["transport"][..]),
        ("appaloosa", &["transport"][..]),
    ] {
        let entry = book
            .get(&format!("oteryn:npc.{npc}"))
            .ok_or("native travel NPC conversation")?;
        for keyword in keywords {
            assert!(
                entry
                    .travel_routes
                    .iter()
                    .any(|route| route.keyword == *keyword)
            );
        }
    }
    Ok(book)
}
fn route(book: &NpcTalkBook, npc: &str, key: &str) -> TestResult<TravelRoute> {
    book.get(&format!("oteryn:npc.{npc}"))
        .and_then(|entry| {
            entry
                .travel_routes
                .iter()
                .find(|route| route.route.key == key)
        })
        .cloned()
        .ok_or_else(|| format!("native route {npc}/{key}").into())
}
fn typed(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}
fn backpack() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: typed("oteryn:item.tibia.i2854"),
        stack: ItemStackClass::NonStackable,
        container_capacity: Some(20),
        container_slot_equip_pattern: true,
        weight: None,
    }
}
fn command(ordinal: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
        CommandId::new(ordinal).map_err(fixture::debug)?,
    ))
}
fn trade(
    book: &NpcTalkBook,
    npc: &str,
    key: &str,
    side: NpcTradeSide,
    seed: u8,
) -> TestResult<NpcTradeRequest> {
    let entry = book.get(npc).ok_or("native trade NPC")?;
    let direction = match side {
        NpcTradeSide::Buy => ProjectV2ServiceOfferDirection::SellToPlayer,
        NpcTradeSide::Sell => ProjectV2ServiceOfferDirection::BuyFromPlayer,
    };
    let (index, offer) = entry
        .offers()
        .iter()
        .enumerate()
        .find(|(_, offer)| offer.item.key == key && offer.direction == direction)
        .ok_or("native admitted trade offer")?;
    Ok(NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: npc.into(),
            catalogue_revision: book.catalogue_revision(),
            offer_index: u32::try_from(index)?,
            side,
            occurrence: fixture::id(seed),
        },
        command: command(u64::from(seed))?,
        quantity: 1,
        unit_price: offer.unit_price,
        offer_count: offer.count.unwrap_or(1),
        item: ItemDefinitionFacts {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: offer.item.key.clone(),
                revision_ref: offer.item.revision.clone(),
            },
            stack: if side == NpcTradeSide::Sell {
                ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                }
            } else {
                ItemStackClass::NonStackable
            },
            container_capacity: None,
            container_slot_equip_pattern: false,
            weight: None,
        },
        transaction_id: fixture::id(seed + 1),
        event_id: fixture::id(seed + 2),
        occurred_at_unix_ms: 1_790_000_000_000,
        output_item_instance_ids: [
            fixture::id(seed + 3),
            fixture::id(seed + 4),
            fixture::id(seed + 5),
        ],
        coins: NpcTradeCoins {
            gold: typed("oteryn:item.tibia.i3031"),
            platinum: typed("oteryn:item.tibia.i3035"),
            crystal: typed("oteryn:item.tibia.i3043"),
        },
        backpack: backpack(),
    })
}
async fn commit_trade(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    request: NpcTradeRequest,
) -> TestResult<Result<NpcTradeOutcome, NpcTradeError>> {
    Ok(h.root
        .commit_npc_trade(
            authority,
            &h.node,
            fixture::fence(rath::revision(h).await?)?.item_fence(),
            request,
        )
        .await)
}
async fn historical_custody(h: &fixture::Harness) -> TestResult {
    // Historical starting custody only; production triggers are restored before all calls.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, count) in [
        (80_u8, "oteryn:item.tibia.i2854", 1_i64),
        (83, "oteryn:item.tibia.i3031", 100),
        (86, "oteryn:item.tibia.i21182", 2),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)")
            .bind(fixture::id(seed).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(key).bind(count).bind(fixture::id(seed+1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(80).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(82).as_slice()).execute(&mut *tx).await?;
    for (seed, ordinal) in [(83_u8, 1_i64), (86, 2)] {
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
            .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(80).as_slice()).bind(fixture::id(seed).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(ordinal).bind(fixture::id(seed+2).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
async fn footprint(h: &fixture::Harness) -> TestResult<(i64, i64, i64)> {
    Ok((
        h.count("game_npc_trades").await?,
        h.count("game_item_instances").await?,
        h.count("game_item_audit_outbox").await?,
    ))
}
async fn promotion(
    h: &fixture::Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    c: &Arc<QuestStateCatalogue>,
    name: &str,
    ordinal: u64,
) -> TestResult {
    assert!(matches!(
        rath::commit(
            h,
            a,
            c,
            &format!("oteryn:quest-transition/rathleton/promote/{name}"),
            ordinal
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    Ok(())
}
fn passage(route: TravelRoute, seed: u8, ordinal: u64) -> TestResult<PaidTravelRequest> {
    let world = fixture::scope()?.world_id();
    let target = &route.route.destination;
    let arrival = PendingArrival {
        occurrence: TravelOccurrence::from_bytes(fixture::id(seed)).map_err(fixture::debug)?,
        world_id: world,
        destination: LastPositionCell {
            x: target.x,
            y: target.y,
            floor: target.floor,
        },
    };
    Ok(PaidTravelRequest {
        command: command(ordinal)?,
        fee: FeeBurnRequest {
            cause: FeeBurnCause::NpcTravel {
                service: route.service.clone(),
                occurrence: fixture::id(seed),
            },
            fee_gold_units: route.route.price,
            transaction_id: fixture::id(seed + 1),
            event_id: fixture::id(seed + 2),
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server".into(),
            change: FeeChangeFacts {
                item_instance_ids: [fixture::id(seed + 3), fixture::id(seed + 4)],
                platinum: typed("oteryn:item.tibia.i3035"),
                gold: typed("oteryn:item.tibia.i3031"),
                crystal: typed("oteryn:item.tibia.i3043"),
                backpack: backpack(),
            },
        },
        arrival,
        route,
    })
}
async fn quote(
    h: &fixture::Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    route: TravelRoute,
) -> TestResult<Result<u64, TravelRefusal>> {
    Ok(h.root
        .quote_npc_travel(
            a,
            &h.node,
            fixture::fence(rath::revision(h).await?)?,
            route,
            &Premium,
        )
        .await
        .map_err(fixture::debug)?)
}
async fn commit_passage(
    h: &fixture::Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    request: PaidTravelRequest,
) -> TestResult<Result<Option<PendingArrival>, TravelRefusal>> {
    let fence = fixture::fence(rath::revision(h).await?)?;
    let sequencer = CharacterRevisionSequencer::new();
    Ok(sequencer
        .acquire(fence.character_id)
        .await
        .commit_npc_travel(&h.root, a, &h.node, fence, request, &Premium)
        .await
        .map_err(fixture::debug)?)
}
async fn assert_stored_arrival(
    h: &fixture::Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    expected: PendingArrival,
) -> TestResult {
    assert_eq!(
        h.root
            .read_npc_pending_arrival(a, &h.node, fixture::fence(rath::revision(h).await?)?)
            .await
            .map_err(fixture::debug)?,
        Some(expected)
    );
    assert_eq!(h.count("game_character_pending_arrivals").await?, 1);
    assert_eq!(h.count("game_newhaven_travel_receipts").await?, 1);
    Ok(())
}
async fn rank_consumers(
    h: &fixture::Harness,
    catalogue: &Arc<QuestStateCatalogue>,
    book: &NpcTalkBook,
) -> TestResult {
    rath::seed(h, catalogue).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    rath::anchor(h, &a, catalogue).await?;
    rath::historical_points(h, 3, 500, 0).await?;
    historical_custody(h).await?;
    let alaistar = trade(
        book,
        "oteryn:npc.alaistar",
        "oteryn:item.tibia.i21182",
        NpcTradeSide::Sell,
        10,
    )?;
    let before = footprint(h).await?;
    assert!(matches!(
        commit_trade(h, &a, alaistar.clone()).await?,
        Err(NpcTradeError::RequirementNotMet)
    ));
    assert_eq!(
        footprint(h).await?,
        before,
        "unpromoted lifetime spending does not authorize trade"
    );
    promotion(h, &a, catalogue, "commoner", 1000).await?;
    assert!(matches!(
        commit_trade(h, &a, alaistar.clone())
            .await?
            .map_err(fixture::debug)?,
        NpcTradeOutcome::Committed(_)
    ));
    let committed = footprint(h).await?;
    assert!(matches!(
        commit_trade(h, &a, alaistar)
            .await?
            .map_err(fixture::debug)?,
        NpcTradeOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(footprint(h).await?, committed);
    let flint = trade(
        book,
        "oteryn:npc.flint",
        "oteryn:item.tibia.i3268",
        NpcTradeSide::Buy,
        20,
    )?;
    assert!(matches!(
        commit_trade(h, &a, flint.clone()).await?,
        Err(NpcTradeError::RequirementNotMet)
    ));
    assert_eq!(footprint(h).await?, committed);
    promotion(h, &a, catalogue, "inhabitant", 1001).await?;
    assert!(matches!(
        commit_trade(h, &a, flint).await?.map_err(fixture::debug)?,
        NpcTradeOutcome::Committed(_)
    ));
    let boat_routes = [
        route(book, "captain_gulliver", "issavi")?,
        route(book, "captain_seahorse", "oramond")?,
    ];
    for route in &boat_routes {
        assert_eq!(
            quote(h, &a, route.clone()).await?,
            Err(TravelRefusal::UnavailableRoute)
        );
    }
    promotion(h, &a, catalogue, "citizen", 1002).await?;
    for route in &boat_routes {
        assert_eq!(quote(h, &a, route.clone()).await?, Ok(route.route.price));
    }
    let request = passage(route(book, "marvin", "hero_of_rathleton")?, 150, 2000)?;
    assert_eq!(
        commit_passage(h, &a, request.clone()).await?,
        Err(TravelRefusal::UnavailableRoute)
    );
    assert_eq!(h.count("game_character_pending_arrivals").await?, 0);
    promotion(h, &a, catalogue, "squire", 1003).await?;
    let fees_before_admission = h.count("game_item_fee_burns").await?;
    assert_eq!(
        commit_passage(h, &a, request.clone()).await?,
        Ok(Some(request.arrival))
    );
    assert_stored_arrival(h, &a, request.arrival).await?;
    assert_eq!(
        commit_passage(h, &a, request.clone()).await?,
        Ok(Some(request.arrival))
    );
    assert_stored_arrival(h, &a, request.arrival).await?;
    assert_eq!(
        h.count("game_item_fee_burns").await?,
        fees_before_admission,
        "authored free admission creates no fee or grant"
    );
    Ok(())
}
async fn winner_arrival(
    h: &fixture::Harness,
    catalogue: &Arc<QuestStateCatalogue>,
    book: &NpcTalkBook,
) -> TestResult {
    rath::seed(h, catalogue).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    rath::anchor(h, &a, catalogue).await?;
    rath::historical_points(h, 1, 50, 0).await?;
    promotion(h, &a, catalogue, "commoner", 3000).await?;
    assert!(matches!(
        rath::commit(
            h,
            &a,
            catalogue,
            "oteryn:quest-transition/world-contribution/votes/dungeon/golems",
            3001
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    rath::activate(h, 2).await?;
    for key in ["dungeon_minotaurs", "dungeon_wrathofevil"] {
        assert_eq!(
            quote(h, &a, route(book, "marvin", key)?).await?,
            Err(TravelRefusal::UnavailableRoute)
        );
    }
    let request = passage(route(book, "marvin", "dungeon_golems")?, 160, 3002)?;
    assert_eq!(quote(h, &a, request.route.clone()).await?, Ok(0));
    assert_eq!(
        commit_passage(h, &a, request.clone()).await?,
        Ok(Some(request.arrival))
    );
    assert_eq!(
        request.arrival.destination,
        LastPositionCell {
            x: 31061,
            y: 32605,
            floor: 9
        }
    );
    assert_stored_arrival(h, &a, request.arrival).await?;
    assert_eq!(
        commit_passage(h, &a, request.clone()).await?,
        Ok(Some(request.arrival))
    );
    assert_stored_arrival(h, &a, request.arrival).await?;
    assert_eq!(
        rath::tracks(h).await?,
        (0, 51, 1),
        "admission spends no second vote or rank"
    );
    Ok(())
}
