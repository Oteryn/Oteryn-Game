// ITEM-LOOP-1 (CP D1438) on PostgreSQL 18.6: the live item loop writer behind command 9.
// One fenced receipt per CommandRef moves, merges, splits, nests, drops or picks up one item of
// the carried tree under the 0128 proofs; every refusal writes nothing and a resent CommandRef
// replays its receipt. A pickup weighs the carried graph against the capacity derived from the
// seeded level 50 (156000 centi-oz) and an unknown weight fails closed. STARTER-1: the first
// entry's starter grant mints one main backpack once per Character and template.
use crate::corpse_transfer_postgres_cases::{equip_backpack, typed};
use crate::domain::CharacterId;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_equipment::{
    assert_equipment_authority_in_transaction, initialize_equipment_in_transaction,
};
use crate::durability::item_loop_move::{
    ITEM_LOOP_CHANNEL_DROPS_MAX, ITEM_LOOP_DROP_REF_PREFIX, ItemLoopContent, LoopDestination,
    LoopGround, LoopMoveReceipt, LoopMoveRefusal, LoopMoveReplay, LoopMoveRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::starter_grant::{StarterGrantOutcome, StarterTemplate};
use crate::foundation::{ChannelId, WorldId};
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, command, configured_admin,
    debug, fence, id, runtime, to_backpack,
};

const DIGEST: [u8; 32] = [1; 32];
const BAG: &str = "fixture:b3.bag";
const COIN: &str = "fixture:b3.coin";
const STONE: &str = "fixture:b3.stone";
const STARTER: &str = "oteryn:item.tibia.i2854";
/// A free Ground cell of Channel 43 (x=5, y=6, floor=7); the MINT fixture uses (1, 2, 3).
const CELL: [u8; 10] = [0, 0, 0, 5, 0, 0, 0, 6, 0, 7];

/// How the test Content weighs the stone: as the transfer catalogue does, over the derived
/// capacity, or not at all.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stone {
    Light,
    Heavy,
    Unknown,
}

/// The test Content generation: a main backpack of capacity 20, bags of capacity 2, coins that
/// stack to 100, plain stones and the starter backpack. Every weight is 1 centi-oz except a
/// heavy stone (156000, the whole seeded capacity).
struct Content(Stone);

impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [0xD1; 32]
    }

    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        if definition.family != "Item" || definition.revision_ref != "definition-r1" {
            return Err(ItemWeightRefused::Unknown);
        }
        match (definition.production_key.as_str(), self.0) {
            (STONE, Stone::Heavy) => Ok(156_000),
            (STONE, Stone::Unknown) => Err(ItemWeightRefused::Unknown),
            (BACKPACK | BAG | COIN | STONE | STARTER, _) => Ok(1),
            _ => Err(ItemWeightRefused::Unknown),
        }
    }
}

impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        DIGEST
    }

    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if definition.family != "Item" || definition.revision_ref != "definition-r1" {
            return None;
        }
        let (stack, container_capacity) = match definition.production_key.as_str() {
            BACKPACK => (ItemStackClass::NonStackable, Some(20)),
            BAG => (ItemStackClass::NonStackable, Some(2)),
            STARTER => (ItemStackClass::NonStackable, Some(20)),
            COIN => (
                ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                },
                None,
            ),
            STONE => (ItemStackClass::NonStackable, None),
            _ => return None,
        };
        Some(ItemDefinitionFacts {
            weight: Some(1),
            definition: definition.clone(),
            stack,
            container_capacity,
            container_slot_equip_pattern: container_capacity.is_some(),
        })
    }
}

/// The transfer facts of a plain fixture item carried into the main backpack.
fn carried(key: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        weight: Some(1),
        definition: typed("Item", key, "definition-r1"),
        stack: if key == COIN {
            ItemStackClass::Stackable {
                proven_maximum: Some(100),
            }
        } else {
            ItemStackClass::NonStackable
        },
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

fn ground() -> LoopGround {
    LoopGround {
        spatial_position: CELL.to_vec(),
        map_revision: "map-1".into(),
        content_revision: "content-1".into(),
        placement_context: id(6).to_vec(),
    }
}

fn request(item: [u8; 16], destination: LoopDestination) -> LoopMoveRequest {
    LoopMoveRequest {
        item,
        count: None,
        pickup_cell: None,
        destination,
    }
}

/// The active Content of Channel 43 and the initialized empty equipment owner, as the fresh
/// admission leaves them.
async fn activate_and_initialize(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_control_scope_grants \
         VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)",
    )
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
    .execute(&harness.pool)
    .await?;
    sqlx::query(
        "SELECT game_content_record_activation(encode($1,'hex')::uuid, \
         encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)",
    )
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
    .bind(DIGEST.as_slice())
    .bind([2_u8; 32].as_slice())
    .bind([3_u8; 32].as_slice())
    .execute(&harness.pool)
    .await?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let equipment = assert_equipment_authority_in_transaction(
        &mut tx,
        &harness.root,
        authority,
        &harness.node,
        &fence()?,
        command(90)?,
        DIGEST,
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &equipment)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(())
}

/// Mint `key` on the Ground and carry it into the main backpack through the B3-1 TRANSFER.
async fn carry(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    n: u64,
    key: &str,
    quantity: u32,
) -> TestResult<[u8; 16]> {
    let item = harness.mint(authority, key, quantity).await?;
    match harness
        .transfer(
            authority,
            fence()?,
            to_backpack(command(n)?, item, carried(key)),
        )
        .await
        .map_err(debug)?
    {
        ItemTransferOutcome::Committed(_) => Ok(item),
        other => Err(format!("expected a commit, got {other:?}").into()),
    }
}

async fn loop_move(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    n: u64,
    content: &Content,
    request: LoopMoveRequest,
) -> TestResult<Result<LoopMoveReceipt, LoopMoveRefusal>> {
    let fence = fence()?;
    Ok(harness
        .root
        .commit_item_loop_move(
            &SpellLanePermit::of_fresh_scope(fence.runtime_scope)
                .await
                .map_err(debug)?,
            authority,
            &harness.node,
            &fence,
            command(n)?,
            content,
            request,
        )
        .await
        .map_err(debug)?)
}

async fn moved(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    n: u64,
    content: &Content,
    request: LoopMoveRequest,
) -> TestResult<LoopMoveReceipt> {
    loop_move(harness, authority, n, content, request)
        .await?
        .map_err(|refusal| format!("{n}: expected a commit, got {refusal:?}").into())
}

/// Row counts of every relation a loop move could write.
async fn footprint(harness: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = harness.footprint().await?;
    for relation in [
        "game_item_loop_move_receipts",
        "game_item_loop_move_lines",
        "game_item_nested_entries",
        "game_item_instances",
        "game_item_instances WHERE lifecycle = 1",
        "game_character_starter_grants",
    ] {
        counts.push(harness.count(relation).await?);
    }
    Ok(counts)
}

/// The direct contents of `container` as (item, quantity), newest first.
async fn contents(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    container: [u8; 16],
) -> TestResult<Vec<([u8; 16], u32)>> {
    let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
    let read = harness
        .root
        .read_item_loop_container(authority, character, container)
        .await
        .map_err(debug)?
        .ok_or("container not in the carried tree")?;
    Ok(read
        .entries
        .iter()
        .map(|entry| (entry.item.item_instance_id, entry.item.quantity))
        .collect())
}

#[test]
fn item_loop_moves_nests_splits_merges_and_replays_once_per_command_ref() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemloop_move").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let bag = carry(&harness, &authority, 2, BAG, 1).await?;
        let coin = carry(&harness, &authority, 3, COIN, 50).await?;
        let stone = carry(&harness, &authority, 4, STONE, 1).await?;
        activate_and_initialize(&harness, &authority).await?;
        let content = Content(Stone::Light);

        // ITEM-MOVE-2a: the stone moves from the main backpack into the bag.
        let receipt = moved(
            &harness,
            &authority,
            10,
            &content,
            request(stone, LoopDestination::Container(bag)),
        )
        .await?;
        assert_eq!((receipt.operation, receipt.replayed), (1, false));
        assert_eq!(contents(&harness, &authority, bag).await?, [(stone, 1)]);
        assert!(
            !contents(&harness, &authority, backpack)
                .await?
                .iter()
                .any(|(item, _)| *item == stone)
        );

        // Replay first: the same CommandRef and request replays without a write; another
        // request under it is refused; the receipt read names this Character only.
        let committed = footprint(&harness).await?;
        let replay = moved(
            &harness,
            &authority,
            10,
            &content,
            request(stone, LoopDestination::Container(bag)),
        )
        .await?;
        assert!(replay.replayed);
        assert_eq!(replay.transaction_id, receipt.transaction_id);
        assert_eq!(
            loop_move(
                &harness,
                &authority,
                10,
                &content,
                request(coin, LoopDestination::Container(bag)),
            )
            .await?
            .err(),
            Some(LoopMoveRefusal::Rejected)
        );
        assert_eq!(footprint(&harness).await?, committed);
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let other = CharacterId::from_bytes(id(99)).map_err(debug)?;
        for (who, n, expected) in [
            (character, 10, LoopMoveReplay::Moved),
            (character, 11, LoopMoveReplay::Absent),
            (other, 10, LoopMoveReplay::Conflicting),
        ] {
            assert_eq!(
                harness
                    .root
                    .read_item_loop_move_receipt(&authority, who, command(n)?)
                    .await
                    .map_err(debug)?,
                expected
            );
        }

        // STACK-1 split: 20 of the 50 coins go into the bag as a fresh stack.
        let split = moved(
            &harness,
            &authority,
            11,
            &content,
            LoopMoveRequest {
                count: Some(20),
                ..request(coin, LoopDestination::Container(bag))
            },
        )
        .await?;
        assert_eq!(split.operation, 3);
        assert_eq!(harness.item_state(coin).await?, (30, 1));
        let in_bag = contents(&harness, &authority, bag).await?;
        let [(fresh, 20), (first, 1)] = in_bag[..] else {
            return Err(format!("expected the split stack and the stone, got {in_bag:?}").into());
        };
        assert_eq!(first, stone);
        assert_ne!(fresh, coin);

        // STACK-1 merge: the remaining 30 top the bag's stack up to 50 and the source retires.
        let merge = moved(
            &harness,
            &authority,
            12,
            &content,
            request(coin, LoopDestination::Container(bag)),
        )
        .await?;
        assert_eq!(merge.operation, 2);
        assert_eq!(harness.item_state(coin).await?.1, 2);
        assert_eq!(
            contents(&harness, &authority, bag).await?,
            [(fresh, 50), (stone, 1)]
        );

        // Every refusal rolls back: nothing is written.
        let second = carry(&harness, &authority, 5, STONE, 1).await?;
        let committed = footprint(&harness).await?;
        for (n, req, refusal) in [
            // The bag (capacity 2) is full.
            (
                13,
                request(second, LoopDestination::Container(bag)),
                LoopMoveRefusal::NoRoom,
            ),
            // A bag never enters itself, and a stone is no container.
            (
                14,
                request(bag, LoopDestination::Container(bag)),
                LoopMoveRefusal::NotSupported,
            ),
            (
                15,
                request(second, LoopDestination::Container(stone)),
                LoopMoveRefusal::NotSupported,
            ),
            // A retired source and a source that is not carried are stale.
            (
                16,
                request(coin, LoopDestination::MainBackpack),
                LoopMoveRefusal::Stale,
            ),
            (
                17,
                request(id(77), LoopDestination::MainBackpack),
                LoopMoveRefusal::Stale,
            ),
            // More than the stack holds.
            (
                18,
                LoopMoveRequest {
                    count: Some(51),
                    ..request(fresh, LoopDestination::MainBackpack)
                },
                LoopMoveRefusal::Rejected,
            ),
            // A carried source names no pickup cell.
            (
                19,
                LoopMoveRequest {
                    pickup_cell: Some(CELL.to_vec()),
                    ..request(second, LoopDestination::MainBackpack)
                },
                LoopMoveRefusal::Stale,
            ),
        ] {
            assert_eq!(
                loop_move(&harness, &authority, n, &content, req)
                    .await?
                    .err(),
                Some(refusal),
                "{n}"
            );
            assert_eq!(footprint(&harness).await?, committed, "{n}");
        }

        // A nested entry moves back to the main backpack.
        let back = moved(
            &harness,
            &authority,
            20,
            &content,
            request(stone, LoopDestination::MainBackpack),
        )
        .await?;
        assert_eq!(back.operation, 1);
        assert_eq!(contents(&harness, &authority, bag).await?, [(fresh, 50)]);
        assert!(
            contents(&harness, &authority, backpack)
                .await?
                .iter()
                .any(|(item, _)| *item == stone)
        );
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn item_loop_bounds_the_nesting_depth_and_refuses_cycles() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemloop_depth").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let mut bags = Vec::new();
        for n in 0..10 {
            bags.push(carry(&harness, &authority, 2 + n, BAG, 1).await?);
        }
        activate_and_initialize(&harness, &authority).await?;
        let content = Content(Stone::Light);
        // bags[k] goes into bags[k - 1] at depth k + 1, up to the admitted depth 8; bags[9]
        // waits at depth 2 inside bags[8].
        for k in 1..8 {
            moved(
                &harness,
                &authority,
                20 + k as u64,
                &content,
                request(bags[k], LoopDestination::Container(bags[k - 1])),
            )
            .await?;
        }
        moved(
            &harness,
            &authority,
            30,
            &content,
            request(bags[9], LoopDestination::Container(bags[8])),
        )
        .await?;
        assert_eq!(harness.count("game_item_nested_entries").await?, 8);
        let committed = footprint(&harness).await?;
        for (n, req, refusal) in [
            // Depth 9 is below the BAGS-1 ceiling.
            (
                40,
                request(bags[8], LoopDestination::Container(bags[7])),
                LoopMoveRefusal::NotSupported,
            ),
            // A container never enters its own subtree.
            (
                41,
                request(bags[0], LoopDestination::Container(bags[3])),
                LoopMoveRefusal::NotSupported,
            ),
            // A subtree of height 6 below depth 2 would reach depth 9.
            (
                42,
                request(bags[1], LoopDestination::Container(bags[9])),
                LoopMoveRefusal::NotSupported,
            ),
        ] {
            assert_eq!(
                loop_move(&harness, &authority, n, &content, req)
                    .await?
                    .err(),
                Some(refusal),
                "{n}"
            );
            assert_eq!(footprint(&harness).await?, committed, "{n}");
        }
        // The deepest bag reads with its parent; a container outside the tree reads as none.
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let deepest = harness
            .root
            .read_item_loop_container(&authority, character, bags[7])
            .await
            .map_err(debug)?
            .ok_or("deepest bag")?;
        assert_eq!(deepest.parent, Some(bags[6]));
        assert!(
            harness
                .root
                .read_item_loop_container(&authority, character, id(77))
                .await
                .map_err(debug)?
                .is_none()
        );
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn item_loop_drops_and_picks_up_within_the_derived_capacity() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemloop_ground").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let bag = carry(&harness, &authority, 2, BAG, 1).await?;
        let coin = carry(&harness, &authority, 3, COIN, 50).await?;
        let stone = carry(&harness, &authority, 4, STONE, 1).await?;
        // A Ground item the loop did not drop (a stage C MINT) is not a loop pickup.
        let minted = harness.mint(&authority, STONE, 1).await?;
        activate_and_initialize(&harness, &authority).await?;
        let light = Content(Stone::Light);
        moved(
            &harness,
            &authority,
            10,
            &light,
            request(coin, LoopDestination::Container(bag)),
        )
        .await?;

        // ITEM-MOVE-2b drop: the stone lands on the cell with a loop drop reference.
        let dropped_receipt = moved(
            &harness,
            &authority,
            11,
            &light,
            request(stone, LoopDestination::Ground(ground())),
        )
        .await?;
        assert_eq!(dropped_receipt.operation, 4);
        assert!(harness.on_ground(stone).await?);
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        let on_ground = harness
            .root
            .read_item_loop_ground(world, channel)
            .await
            .map_err(debug)?;
        let [dropped] = &on_ground[..] else {
            return Err(format!("expected one loop Ground item, got {on_ground:?}").into());
        };
        assert_eq!(dropped.item.item_instance_id, stone);
        assert_eq!(dropped.item.ground.spatial_position, CELL);
        assert_eq!(
            dropped.item.ground.corpse_ref,
            [
                ITEM_LOOP_DROP_REF_PREFIX,
                dropped_receipt.transaction_id.as_slice()
            ]
            .concat()
        );

        // Refusals write nothing: a non-empty container stays carried, a pickup from another
        // cell is stale, a MINT Ground item is no loop pickup, and an over-capacity or unknown
        // weight never enters the Character.
        let pickup = |item: [u8; 16], cell: &[u8]| LoopMoveRequest {
            pickup_cell: Some(cell.to_vec()),
            ..request(item, LoopDestination::MainBackpack)
        };
        let committed = footprint(&harness).await?;
        for (n, content, req, refusal) in [
            (
                12,
                Stone::Light,
                request(bag, LoopDestination::Ground(ground())),
                LoopMoveRefusal::NotSupported,
            ),
            (
                13,
                Stone::Light,
                pickup(stone, &[0, 0, 0, 1, 0, 0, 0, 2, 0, 3]),
                LoopMoveRefusal::Stale,
            ),
            (
                14,
                Stone::Light,
                request(stone, LoopDestination::MainBackpack),
                LoopMoveRefusal::Stale,
            ),
            (
                15,
                Stone::Light,
                pickup(minted, &[0, 0, 0, 1, 0, 0, 0, 2, 0, 3]),
                LoopMoveRefusal::NotSupported,
            ),
            (
                16,
                Stone::Heavy,
                pickup(stone, &CELL),
                LoopMoveRefusal::Overweight,
            ),
            (
                17,
                Stone::Unknown,
                pickup(stone, &CELL),
                LoopMoveRefusal::WeightUnknown,
            ),
        ] {
            assert_eq!(
                loop_move(&harness, &authority, n, &Content(content), req)
                    .await?
                    .err(),
                Some(refusal),
                "{n}"
            );
            assert_eq!(footprint(&harness).await?, committed, "{n}");
            assert!(harness.on_ground(stone).await?, "{n}");
        }

        // ITEM-MOVE-2b pickup: the whole stack comes back into the main backpack.
        let picked = moved(&harness, &authority, 18, &light, pickup(stone, &CELL)).await?;
        assert_eq!(picked.operation, 5);
        assert!(!harness.on_ground(stone).await?);
        assert!(
            contents(&harness, &authority, backpack)
                .await?
                .iter()
                .any(|(item, _)| *item == stone)
        );
        assert!(
            harness
                .root
                .read_item_loop_ground(world, channel)
                .await
                .map_err(debug)?
                .is_empty()
        );

        // A partial drop splits a fresh stack onto the cell; the rest stays nested.
        let split = moved(
            &harness,
            &authority,
            19,
            &light,
            LoopMoveRequest {
                count: Some(10),
                ..request(coin, LoopDestination::Ground(ground()))
            },
        )
        .await?;
        assert_eq!(split.operation, 4);
        assert_eq!(harness.item_state(coin).await?, (40, 1));
        let on_ground = harness
            .root
            .read_item_loop_ground(world, channel)
            .await
            .map_err(debug)?;
        let [part] = &on_ground[..] else {
            return Err(format!("expected the dropped part, got {on_ground:?}").into());
        };
        assert_eq!(part.item.quantity, 10);
        assert_ne!(part.item.item_instance_id, coin);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Copy the live item `template` `n` times outside the runtime guards; each copy lands where
/// `place` (one INSERT .. SELECT over `ids`, `$1` the template, `$3` a bytea parameter) puts it.
async fn seed_copies(
    harness: &Harness,
    template: [u8; 16],
    n: i64,
    param: &[u8],
    place: &str,
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "WITH ids AS MATERIALIZED ( \
           SELECT game_character_uuid_v7() AS item, n FROM generate_series(1, $2::bigint) n), \
         copied AS ( \
           INSERT INTO game_item_instances \
           SELECT (jsonb_populate_record(NULL::game_item_instances, \
                   to_jsonb(i) || jsonb_build_object('item_instance_id', ids.item))).* \
             FROM game_item_instances i, ids \
            WHERE i.item_instance_id = encode($1,'hex')::uuid) \
         {place}"
    )))
    .bind(template.as_slice())
    .bind(n)
    .bind(param)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// `n` live loop-dropped copies of the Ground item `template` in Channel 43, one per cell away
/// from [`CELL`], each with its own loop drop reference and Ground ordinal.
async fn seed_loop_drops(harness: &Harness, template: [u8; 16], n: i64) -> TestResult {
    seed_copies(
        harness,
        template,
        n,
        ITEM_LOOP_DROP_REF_PREFIX,
        "INSERT INTO game_item_ground_locations \
         SELECT (jsonb_populate_record(NULL::game_item_ground_locations, \
                 to_jsonb(g) || jsonb_build_object( \
                   'item_instance_id', ids.item, \
                   'spatial_position', \
                     '\\x' || encode(int4send(ids.n::int) || '\\x0000ffff0007'::bytea, 'hex'), \
                   'corpse_ref', \
                     '\\x' || encode($3::bytea || uuid_send(game_character_uuid_v7()), 'hex'), \
                   'stack_ordinal', nextval('game_item_ground_stack_ordinal_seq')))).* \
           FROM game_item_ground_locations g, ids \
          WHERE g.item_instance_id = encode($1,'hex')::uuid",
    )
    .await
}

/// `n` copies of the live item `template` nested directly under `parent`.
async fn seed_nested(
    harness: &Harness,
    template: [u8; 16],
    parent: [u8; 16],
    n: i64,
) -> TestResult {
    seed_copies(
        harness,
        template,
        n,
        &parent,
        "INSERT INTO game_item_nested_entries (item_instance_id, world_id, \
           parent_item_instance_id, placement_ordinal, placed_transaction_id) \
         SELECT ids.item, i.world_id, encode($3::bytea,'hex')::uuid, ids.n, \
                game_character_uuid_v7() \
           FROM game_item_instances i, ids \
          WHERE i.item_instance_id = encode($1,'hex')::uuid",
    )
    .await
}

async fn tree_size(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
            .bind(id(CHARACTER).as_slice())
            .fetch_one(&harness.pool)
            .await?,
    )
}

fn pickup(item: [u8; 16], cell: &[u8]) -> LoopMoveRequest {
    LoopMoveRequest {
        pickup_cell: Some(cell.to_vec()),
        ..request(item, LoopDestination::MainBackpack)
    }
}

#[test]
fn item_loop_drop_is_blocked_at_the_channel_cap_until_a_pickup_frees_room() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemloop_cap").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let stone = carry(&harness, &authority, 2, STONE, 1).await?;
        let other = carry(&harness, &authority, 3, STONE, 1).await?;
        let minted = harness.mint(&authority, STONE, 1).await?;
        activate_and_initialize(&harness, &authority).await?;
        let light = Content(Stone::Light);
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;

        // ITEMMOVE1-RL-02: one below the cap, a drop fills the channel to exactly 20,000.
        seed_loop_drops(&harness, minted, ITEM_LOOP_CHANNEL_DROPS_MAX - 1).await?;
        moved(
            &harness,
            &authority,
            10,
            &light,
            request(stone, LoopDestination::Ground(ground())),
        )
        .await?;
        let on_ground = harness
            .root
            .read_item_loop_ground(world, channel)
            .await
            .map_err(debug)?;
        assert_eq!(i64::try_from(on_ground.len())?, ITEM_LOOP_CHANNEL_DROPS_MAX);

        // At the cap the next drop is BLOCKED and writes nothing.
        let committed = footprint(&harness).await?;
        assert_eq!(
            loop_move(
                &harness,
                &authority,
                11,
                &light,
                request(other, LoopDestination::Ground(ground())),
            )
            .await?
            .err(),
            Some(LoopMoveRefusal::Blocked)
        );
        assert_eq!(footprint(&harness).await?, committed);
        assert!(!harness.on_ground(other).await?);

        // A pickup of a dropped item frees one place; the next drop then commits.
        moved(&harness, &authority, 12, &light, pickup(stone, &CELL)).await?;
        moved(
            &harness,
            &authority,
            13,
            &light,
            request(other, LoopDestination::Ground(ground())),
        )
        .await?;
        assert!(harness.on_ground(other).await?);
        assert_eq!(
            i64::try_from(
                harness
                    .root
                    .read_item_loop_ground(world, channel)
                    .await
                    .map_err(debug)?
                    .len()
            )?,
            ITEM_LOOP_CHANNEL_DROPS_MAX
        );
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn item_loop_full_merge_passes_at_the_tree_ceiling_where_a_placement_has_no_room() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemloop_ceiling").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let bag = carry(&harness, &authority, 2, BAG, 1).await?;
        let coin = carry(&harness, &authority, 3, COIN, 50).await?;
        let stone = carry(&harness, &authority, 4, STONE, 1).await?;
        activate_and_initialize(&harness, &authority).await?;
        let light = Content(Stone::Light);

        // Ten coins split onto the cell and the stone follows; the bag and 40 coins stay.
        moved(
            &harness,
            &authority,
            10,
            &light,
            LoopMoveRequest {
                count: Some(10),
                ..request(coin, LoopDestination::Ground(ground()))
            },
        )
        .await?;
        moved(
            &harness,
            &authority,
            11,
            &light,
            request(stone, LoopDestination::Ground(ground())),
        )
        .await?;
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        let part = harness
            .root
            .read_item_loop_ground(world, channel)
            .await
            .map_err(debug)?
            .iter()
            .map(|dropped| dropped.item.item_instance_id)
            .find(|item| *item != stone)
            .ok_or("the dropped coins are not on the Ground")?;

        // BAGS-1: 498 stones under the bag fill the carried tree to its 500 ceiling.
        seed_nested(&harness, stone, bag, 498).await?;
        assert_eq!(tree_size(&harness).await?, 500);

        // Placing the stone would grow the tree past the ceiling: NO_ROOM, nothing written.
        let committed = footprint(&harness).await?;
        assert_eq!(
            loop_move(&harness, &authority, 12, &light, pickup(stone, &CELL))
                .await?
                .err(),
            Some(LoopMoveRefusal::NoRoom)
        );
        assert_eq!(footprint(&harness).await?, committed);
        assert!(harness.on_ground(stone).await?);

        // STACK-1: a full merge adds nothing, so the coins top up the carried stack at 500.
        moved(&harness, &authority, 13, &light, pickup(part, &CELL)).await?;
        assert_eq!(harness.item_state(coin).await?, (50, 1));
        assert!(!harness.on_ground(part).await?);
        assert_eq!(tree_size(&harness).await?, 500);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn starter_grant_mints_one_main_backpack_once_and_skips_an_occupied_slot() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemloop_starter").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        activate_and_initialize(&harness, &authority).await?;
        let content = Content(Stone::Light);
        let grant = |template: StarterTemplate| {
            let harness = &harness;
            let authority = &authority;
            let content = &content;
            async move {
                harness
                    .root
                    .grant_starter_items(authority, &harness.node, &fence()?, content, &template)
                    .await
                    .map_err(|error| -> Box<dyn std::error::Error> { debug(error).into() })
            }
        };
        let main = StarterTemplate::main();
        assert_eq!(main.definition, typed("Item", STARTER, "definition-r1"));

        // A template whose item the active Content does not admit writes nothing.
        let before = footprint(&harness).await?;
        let unadmitted = StarterTemplate {
            key: "fixture:starter-template.unadmitted".into(),
            definition: typed("Item", "fixture:unknown", "definition-r1"),
            ..main.clone()
        };
        assert_eq!(grant(unadmitted).await?, StarterGrantOutcome::NotAdmitted);
        assert_eq!(footprint(&harness).await?, before);

        // The first entry mints the main backpack into the empty slot, once.
        let StarterGrantOutcome::Minted(item) = grant(main.clone()).await? else {
            return Err("expected the starter backpack".into());
        };
        let slot: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_item_container_slots \
              WHERE character_id = encode($1,'hex')::uuid \
                AND item_instance_id = encode($2,'hex')::uuid)",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(item.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert!(slot);
        assert_eq!(harness.item_state(item).await?, (1, 1));
        let granted = footprint(&harness).await?;
        assert_eq!(
            grant(main.clone()).await?,
            StarterGrantOutcome::AlreadyGranted
        );
        assert_eq!(footprint(&harness).await?, granted);

        // Another template finds the slot occupied and only records that.
        let other = StarterTemplate {
            key: "fixture:starter-template.other".into(),
            ..main
        };
        assert_eq!(grant(other).await?, StarterGrantOutcome::Skipped);
        assert_eq!(harness.count("game_character_starter_grants").await?, 2);
        assert_eq!(
            harness.count("game_item_instances").await?,
            granted[granted.len() - 3]
        );

        // The starter backpack is the loop's main backpack.
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let read = harness
            .root
            .read_item_loop_container(&authority, character, item)
            .await
            .map_err(debug)?
            .ok_or("starter backpack")?;
        assert_eq!(read.parent, None);
        assert!(read.entries.is_empty());
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
