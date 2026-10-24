// NPC-TRADE-1 NPC BUY/SELL commit (migration 0082) on the B3 item-transfer harness: coins and
// items reach the equipped main backpack through the real MINT and TRANSFER paths, then
// `commit_npc_trade` runs under the complete Character item fence.

use crate::domain::currency::Coin;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::durability::npc_trade::{
    CommittedNpcTrade, NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeLineKind,
    NpcTradeOutcome, NpcTradeReceipt, NpcTradeRequest, NpcTradeSide,
};
use crate::durability::npc_trade_audit::decode_npc_trade_envelope;
use crate::foundation::ConnectionGeneration;
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHARACTER, Harness, TestResult, backpack_facts, command, configured_admin, debug,
    fence, id, runtime, to_backpack, to_slot,
};
use sqlx::Row;

const SWORD: &str = "fixture:npc.sword";
const REVISION: &str = "definition-r1";
const OCCURRED_AT: i64 = 1_790_000_000_000;

fn typed(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: REVISION.into(),
    }
}

fn stackable(key: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        weight: None,
        definition: typed(key),
        stack: ItemStackClass::Stackable {
            proven_maximum: None,
        },
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

/// A trade of occurrence `seed`: TransactionId `seed + 1`, EventId `seed + 2`, outputs
/// `seed + 3..=seed + 5`, under command `seed`.
fn trade(
    seed: u8,
    side: NpcTradeSide,
    quantity: u32,
    unit_price: u64,
) -> TestResult<NpcTradeRequest> {
    Ok(NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: "fixture:npc.trader".into(),
            catalogue_revision: 1,
            offer_index: 0,
            side,
            occurrence: id(seed),
        },
        command: command(u64::from(seed))?,
        quantity,
        unit_price,
        offer_count: 1,
        item: stackable(SWORD),
        transaction_id: id(seed + 1),
        event_id: id(seed + 2),
        occurred_at_unix_ms: OCCURRED_AT,
        output_item_instance_ids: [id(seed + 3), id(seed + 4), id(seed + 5)],
        coins: NpcTradeCoins {
            gold: typed(Coin::Gold.production_key()),
            platinum: typed(Coin::Platinum.production_key()),
            crystal: typed(Coin::Crystal.production_key()),
        },
        backpack: backpack_facts(),
    })
}

/// MINT `quantity` of `key` on Ground and move it into the backpack (a new entry).
async fn give(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    key: &str,
    quantity: u32,
    command_id: u64,
) -> TestResult<[u8; 16]> {
    let item = harness
        .mint_definition(authority, typed(key), quantity)
        .await?;
    // This positive bootstrap resolves the exact fixture catalogue; trade-only facts
    // remain weight-unknown because that writer does not enforce incoming capacity.
    let mut item_facts = stackable(key);
    item_facts.weight = match key {
        SWORD => Some(1),
        "oteryn:item.tibia.i3031" | "oteryn:item.tibia.i3035" | "oteryn:item.tibia.i3043" => {
            Some(10)
        }
        _ => None,
    };
    match harness
        .transfer(
            authority,
            fence()?,
            to_backpack(command(command_id)?, item, item_facts),
        )
        .await
        .map_err(debug)?
    {
        ItemTransferOutcome::Committed(_) => Ok(item),
        other => Err(format!("unexpected TRANSFER outcome {other:?}").into()),
    }
}

async fn equip(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<[u8; 16]> {
    let backpack = harness.mint(authority, BACKPACK, 1).await?;
    match harness
        .transfer(
            authority,
            fence()?,
            to_slot(command(200)?, backpack, backpack_facts()),
        )
        .await
        .map_err(debug)?
    {
        ItemTransferOutcome::Committed(_) => Ok(backpack),
        other => Err(format!("unexpected TRANSFER outcome {other:?}").into()),
    }
}

async fn commit(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    request: NpcTradeRequest,
) -> Result<NpcTradeOutcome, NpcTradeError> {
    harness
        .root
        .commit_npc_trade(
            authority,
            &harness.node,
            fence().map_err(|_| NpcTradeError::InvalidInput)?,
            request,
        )
        .await
}

async fn committed(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    request: NpcTradeRequest,
) -> TestResult<CommittedNpcTrade> {
    match commit(harness, authority, request).await.map_err(debug)? {
        NpcTradeOutcome::Committed(result) => Ok(result),
        other => Err(format!("expected a fresh commit, got {other:?}").into()),
    }
}

/// Row counts of every relation a trade writes.
async fn footprint(harness: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = Vec::new();
    for relation in [
        "game_npc_trades",
        "game_npc_trade_lines",
        "game_item_audit_outbox",
        "game_item_instances",
        "game_item_container_entries",
        "game_item_instances WHERE lifecycle <> 1",
    ] {
        counts.push(harness.count(relation).await?);
    }
    Ok(counts)
}

/// The backpack's direct entries, highest ordinal first: (item, ordinal, key, quantity).
async fn entries(
    harness: &Harness,
    backpack: [u8; 16],
) -> TestResult<Vec<([u8; 16], i64, String, i64)>> {
    let rows = sqlx::query(
        "SELECT decode(replace(e.item_instance_id::text, '-', ''), 'hex') AS item, \
                e.placement_ordinal::bigint AS ordinal, i.definition_production_key, i.quantity \
           FROM game_item_container_entries e \
           JOIN game_item_instances i USING (item_instance_id) \
          WHERE e.parent_item_instance_id = encode($1,'hex')::uuid \
          ORDER BY e.placement_ordinal DESC",
    )
    .bind(backpack.as_slice())
    .fetch_all(&harness.pool)
    .await?;
    rows.iter()
        .map(|row| {
            let item: Vec<u8> = row.try_get("item")?;
            Ok((
                <[u8; 16]>::try_from(item.as_slice())?,
                row.try_get("ordinal")?,
                row.try_get("definition_production_key")?,
                row.try_get("quantity")?,
            ))
        })
        .collect()
}

async fn character_revision(harness: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

type LineShape = (NpcTradeLineKind, [u8; 16], u64, Option<Coin>, u32, u32);
type RefusalCase<'a> = (&'a str, NpcTradeRequest, fn(&NpcTradeError) -> bool);

fn shape(trade: &CommittedNpcTrade) -> Vec<LineShape> {
    trade
        .lines
        .iter()
        .map(|line| {
            (
                line.kind,
                line.item_instance_id,
                line.placement_ordinal,
                line.coin,
                line.quantity_before,
                line.quantity_after,
            )
        })
        .collect()
}

#[test]
fn buy_with_change_then_partial_and_whole_sell_replay_and_conflicts() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "npctrade").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = equip(&harness, &authority).await?;
        let gold = give(&harness, &authority, Coin::Gold.production_key(), 10, 201).await?;
        let platinum = give(
            &harness,
            &authority,
            Coin::Platinum.production_key(),
            3,
            202,
        )
        .await?;
        let revision = character_revision(&harness).await?;

        // BUY 5 swords at 47 (235): 10 gold (ordinal 1) and 3 platinum (ordinal 2) burned whole,
        // 75 gold change at ordinal 3, the swords at ordinal 4.
        let request = trade(10, NpcTradeSide::Buy, 5, 47)?;
        let bought = committed(&harness, &authority, request.clone()).await?;
        assert_eq!(
            shape(&bought),
            vec![
                (NpcTradeLineKind::CoinBurn, gold, 1, Some(Coin::Gold), 10, 0),
                (
                    NpcTradeLineKind::CoinBurn,
                    platinum,
                    2,
                    Some(Coin::Platinum),
                    3,
                    0
                ),
                (
                    NpcTradeLineKind::CoinMint,
                    id(14),
                    3,
                    Some(Coin::Gold),
                    0,
                    75
                ),
                (NpcTradeLineKind::ItemMint, id(15), 4, None, 0, 5),
            ]
        );
        assert_eq!(
            (
                bought.total_gold_units,
                bought.burned_gold_units,
                bought.minted_gold_units
            ),
            (235, 310, 75)
        );
        // NPC-TRADE-2 replay first: the receipt names what command 8 asked; another occurrence
        // has none; the occurrence read under another CommandRef conflicts.
        let character_id = fence()?.character_id;
        assert_eq!(
            harness
                .root
                .read_npc_trade(&authority, character_id, command(10)?, id(10))
                .await
                .map_err(debug)?,
            Some(NpcTradeReceipt {
                npc_key: "fixture:npc.trader".into(),
                catalogue_revision: 1,
                offer_index: 0,
                side: NpcTradeSide::Buy,
                quantity: 5,
                unit_price: 47,
            })
        );
        assert_eq!(
            harness
                .root
                .read_npc_trade(&authority, character_id, command(90)?, id(90))
                .await
                .map_err(debug)?,
            None
        );
        assert!(matches!(
            harness
                .root
                .read_npc_trade(&authority, character_id, command(10)?, id(91))
                .await,
            Err(NpcTradeError::ConflictingOccurrence)
        ));
        assert_eq!(
            entries(&harness, backpack).await?,
            vec![
                (id(15), 4, SWORD.to_owned(), 5),
                (id(14), 3, Coin::Gold.production_key().to_owned(), 75),
            ]
        );
        for item in [gold, platinum] {
            assert_eq!(harness.item_state(item).await?, (0, 2));
        }
        // One pending event that both gates admit; no CharacterRevision advance.
        let envelope: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id = encode($1,'hex')::uuid",
        )
        .bind(id(12).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        let (_, payload) = decode_npc_trade_envelope(&envelope).map_err(debug)?;
        assert_eq!(payload.lines.len(), 4);
        assert_eq!(character_revision(&harness).await?, revision);

        // A retry of the occurrence returns the first outcome, even with new retry-local ids.
        let before = footprint(&harness).await?;
        let mut retry = trade(10, NpcTradeSide::Buy, 5, 47)?;
        retry.transaction_id = id(150);
        retry.event_id = id(151);
        retry.output_item_instance_ids = [id(152), id(153), id(154)];
        retry.occurred_at_unix_ms = OCCURRED_AT + 1;
        for request in [request.clone(), retry] {
            match commit(&harness, &authority, request).await.map_err(debug)? {
                NpcTradeOutcome::AlreadyCommitted(first) => assert_eq!(first, bought),
                other => return Err(format!("expected the first outcome, got {other:?}").into()),
            }
        }
        // The same occurrence with another binding, and another occurrence reusing the command
        // or an identity, conflict.
        let mut changed = request.clone();
        changed.quantity = 4;
        let mut same_command = trade(30, NpcTradeSide::Buy, 1, 1)?;
        same_command.command = request.command;
        let mut same_event = trade(30, NpcTradeSide::Buy, 1, 1)?;
        same_event.event_id = id(12);
        let mut same_output = trade(30, NpcTradeSide::Buy, 1, 1)?;
        same_output.output_item_instance_ids[2] = id(15);
        match commit(&harness, &authority, changed).await {
            Err(NpcTradeError::ConflictingOccurrence) => {}
            other => return Err(format!("expected ConflictingOccurrence, got {other:?}").into()),
        }
        for request in [same_command, same_event, same_output] {
            match commit(&harness, &authority, request).await {
                Err(NpcTradeError::ConflictingCandidate) => {}
                other => return Err(format!("expected ConflictingCandidate, got {other:?}").into()),
            }
        }
        assert_eq!(footprint(&harness).await?, before);

        // SELL 2 of the 5 swords at 12,345: 2 crystal, 46 platinum, 90 gold at 5..=7.
        let sold = committed(
            &harness,
            &authority,
            trade(40, NpcTradeSide::Sell, 2, 12_345)?,
        )
        .await?;
        assert_eq!(
            shape(&sold),
            vec![
                (NpcTradeLineKind::ItemBurn, id(15), 4, None, 5, 3),
                (
                    NpcTradeLineKind::CoinMint,
                    id(43),
                    5,
                    Some(Coin::Crystal),
                    0,
                    2
                ),
                (
                    NpcTradeLineKind::CoinMint,
                    id(44),
                    6,
                    Some(Coin::Platinum),
                    0,
                    46
                ),
                (
                    NpcTradeLineKind::CoinMint,
                    id(45),
                    7,
                    Some(Coin::Gold),
                    0,
                    90
                ),
            ]
        );
        assert_eq!(harness.item_state(id(15)).await?, (3, 1));
        // SELL the last 3 at 1: the stack retires and leaves its entry; 3 gold at ordinal 8.
        let sold = committed(&harness, &authority, trade(60, NpcTradeSide::Sell, 3, 1)?).await?;
        assert_eq!(
            shape(&sold),
            vec![
                (NpcTradeLineKind::ItemBurn, id(15), 4, None, 3, 0),
                (
                    NpcTradeLineKind::CoinMint,
                    id(65),
                    8,
                    Some(Coin::Gold),
                    0,
                    3
                ),
            ]
        );
        assert_eq!(harness.item_state(id(15)).await?, (0, 2));
        assert_eq!(
            entries(&harness, backpack)
                .await?
                .iter()
                .map(|(item, ordinal, _, quantity)| (*item, *ordinal, *quantity))
                .collect::<Vec<_>>(),
            vec![
                (id(65), 8, 3),
                (id(45), 7, 90),
                (id(44), 6, 46),
                (id(43), 5, 2),
                (id(14), 3, 75),
            ]
        );
        // Price 0 (NPC-0 §5.3, §5.4): a BUY burns no coin and mints only the item; a SELL burns
        // only the item. The coin stacks stay untouched.
        let free = committed(&harness, &authority, trade(70, NpcTradeSide::Buy, 1, 0)?).await?;
        assert_eq!(
            shape(&free),
            vec![(NpcTradeLineKind::ItemMint, id(75), 9, None, 0, 1)]
        );
        assert_eq!(
            (
                free.total_gold_units,
                free.burned_gold_units,
                free.minted_gold_units
            ),
            (0, 0, 0)
        );
        let given = committed(&harness, &authority, trade(90, NpcTradeSide::Sell, 1, 0)?).await?;
        assert_eq!(
            shape(&given),
            vec![(NpcTradeLineKind::ItemBurn, id(75), 9, None, 1, 0)]
        );
        assert_eq!(harness.item_state(id(75)).await?, (0, 2));
        assert_eq!(entries(&harness, backpack).await?.len(), 5);
        assert_eq!(harness.count("game_npc_trades").await?, 5);
        assert_eq!(
            harness.count("game_npc_trade_lines").await?,
            4 + 4 + 2 + 1 + 1
        );
        assert_eq!(character_revision(&harness).await?, revision);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn refusals_write_nothing() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "npcrefuse").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let before = footprint(&harness).await?;
        match commit(&harness, &authority, trade(10, NpcTradeSide::Buy, 1, 1)?).await {
            Err(NpcTradeError::NoBackpack) => {}
            other => return Err(format!("expected NoBackpack, got {other:?}").into()),
        }
        assert_eq!(footprint(&harness).await?, before);

        equip(&harness, &authority).await?;
        give(&harness, &authority, Coin::Gold.production_key(), 10, 201).await?;
        give(&harness, &authority, SWORD, 2, 202).await?;
        let before = footprint(&harness).await?;

        let mut small = trade(30, NpcTradeSide::Buy, 1, 1)?;
        small.backpack.container_capacity = Some(2);
        let mut other_backpack = trade(40, NpcTradeSide::Buy, 1, 1)?;
        other_backpack.backpack.definition.revision_ref = "definition-r2".into();
        let mut stale = trade(50, NpcTradeSide::Buy, 1, 1)?;
        stale.item.definition.production_key = "fixture:npc.shield".into();
        let mut coin_offer = trade(60, NpcTradeSide::Buy, 1, 1)?;
        coin_offer.item = stackable(Coin::Gold.production_key());
        let cases: Vec<RefusalCase> = vec![
            (
                "insufficient funds",
                trade(10, NpcTradeSide::Buy, 1, 11)?,
                |error| matches!(error, NpcTradeError::InsufficientFunds),
            ),
            (
                "nothing to sell",
                trade(20, NpcTradeSide::Sell, 3, 1)?,
                |error| matches!(error, NpcTradeError::ItemNotFound),
            ),
            ("does not fit", small, |error| {
                matches!(error, NpcTradeError::DoesNotFit)
            }),
            ("another backpack definition", other_backpack, |error| {
                matches!(error, NpcTradeError::InvalidInput)
            }),
            ("a coin offer", coin_offer, |error| {
                matches!(error, NpcTradeError::InvalidInput)
            }),
        ];
        for (name, request, expected) in cases {
            match commit(&harness, &authority, request).await {
                Err(error) if expected(&error) => {}
                other => return Err(format!("{name}: unexpected {other:?}").into()),
            }
            assert_eq!(footprint(&harness).await?, before, "{name}");
        }
        // A shield nobody holds cannot be sold either.
        let mut shield = trade(50, NpcTradeSide::Sell, 1, 1)?;
        shield.item = stale.item.clone();
        match commit(&harness, &authority, shield).await {
            Err(NpcTradeError::ItemNotFound) => {}
            other => return Err(format!("expected ItemNotFound, got {other:?}").into()),
        }
        // A stale Character item fence is rejected.
        let mut old = fence()?;
        old.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        match harness
            .root
            .commit_npc_trade(
                &authority,
                &harness.node,
                old,
                trade(70, NpcTradeSide::Buy, 1, 1)?,
            )
            .await
        {
            Err(NpcTradeError::AuthorityRejected) => {}
            other => return Err(format!("expected AuthorityRejected, got {other:?}").into()),
        }
        assert_eq!(footprint(&harness).await?, before);

        // Exactly the 10 gold: the stack retires and the swords are minted.
        let bought = committed(&harness, &authority, trade(80, NpcTradeSide::Buy, 2, 5)?).await?;
        assert_eq!(bought.lines.len(), 2);
        assert_eq!(bought.minted_gold_units, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn the_database_refuses_an_unproven_trade() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "npcguard").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip(&harness, &authority).await?;
        let gold = give(&harness, &authority, Coin::Gold.production_key(), 10, 201).await?;

        // An item burn or a trade record without its lines and event never commits.
        let mut tx = harness.pool.begin().await?;
        let error = sqlx::query(
            "UPDATE game_item_instances SET quantity = 5 \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(gold.as_slice())
        .execute(&mut *tx)
        .await;
        let error = match error {
            Ok(_) => tx.commit().await.err(),
            Err(error) => {
                tx.rollback().await?;
                Some(error)
            }
        };
        assert!(error.is_some(), "an unproven coin change commits");
        assert_eq!(harness.item_state(gold).await?, (10, 1));

        // A coin stack that is not in its default state (a timed row) is never paid from.
        let mut seed = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query(
            "INSERT INTO game_item_timed_states \
             VALUES (encode($1,'hex')::uuid, 1, NULL, NULL, 1)",
        )
        .bind(gold.as_slice())
        .execute(&mut *seed)
        .await?;
        seed.commit().await?;
        let before = footprint(&harness).await?;
        match commit(&harness, &authority, trade(10, NpcTradeSide::Buy, 1, 1)?).await {
            Err(NpcTradeError::InvalidInput) => {}
            other => return Err(format!("expected InvalidInput, got {other:?}").into()),
        }
        assert_eq!(footprint(&harness).await?, before);
        assert_eq!(harness.item_state(gold).await?, (10, 1));
        // A free BUY pays from no coin, so that stack does not block it.
        let free = committed(&harness, &authority, trade(20, NpcTradeSide::Buy, 1, 0)?).await?;
        assert_eq!(free.lines.len(), 1);
        assert_eq!(harness.item_state(gold).await?, (10, 1));

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
