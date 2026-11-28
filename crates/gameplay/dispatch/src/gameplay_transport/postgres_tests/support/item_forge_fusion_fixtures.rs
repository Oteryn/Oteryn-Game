use super::TestResult;

use super::fixture::{Harness, debug, id};

use super::item_forge_fixtures::pair_wire::{PairSelection, exchange_pair};

use super::item_forge_fixtures::support::{self, cooldown_elapsed, exchange, request, seed_items};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::AdmittedSession;

use oteryn_gameplay_items::forge::{EXALTED_CORE_KEY, FORGE_SLIVER_KEY};

use oteryn_protocol_oteryn::{
    CommandStatus,
    item_forge::{Operation, Request},
};

use serde_json::Value;

#[path = "item_forge_fusion/fee.rs"]
pub(super) mod fee;

use fixture::{assets, bank, item_state, nest_pair, occurrence_for, receipt, seed_bank};

const SWORD: &str = "oteryn:item.tibia.i3264";

const CLASS_FOUR: &str = "oteryn:item.tibia.i34082";

const OTHER_CLASS_FOUR: &str = "oteryn:item.tibia.i34084";

pub(super) async fn insufficient_cores(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_bank(h).await?;
    seed_items(h, &[(101, CLASS_FOUR, 1, 0), (102, OTHER_CLASS_FOUR, 1, 1)]).await?;
    assert_eq!(support::balances(h).await?, (1000, 1000, 0, 0));
    let (action, refusal) = refused(
        h,
        owner,
        admitted,
        pair(Operation::ConvergenceTransfer, 135, 101, 102),
        "INSUFFICIENT_CORES",
    )
    .await?;
    seed_items(h, &[(103, EXALTED_CORE_KEY, 1, 0)]).await?;
    let funded = assets(h).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, refusal)
    );
    assert_eq!(
        assets(h).await?,
        funded,
        "a core refill cannot reopen the recorded refusal"
    );
    let mut fresh = pair(Operation::ConvergenceTransfer, 136, 101, 102);
    // The new core was appended after the two equipment inputs. Select their
    // actual displayed handles again, with a new operation occurrence.
    fresh.selection.indices = Some((2, 1));
    committed(
        h,
        owner,
        admitted,
        fresh,
        Expected {
            succeeded: true,
            bonus: 0,
            tiers: (1, None),
            cost: (160, 1, 65_000_000),
        },
    )
    .await?;
    Ok(())
}

pub(super) async fn exercise(
    operation: Operation,
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(h, &[(103, EXALTED_CORE_KEY, 3, 0)]).await?;
    match operation {
        Operation::Fusion => fusion(h, owner, admitted).await?,
        Operation::ConvergenceFusion => convergence_fusion(h, owner, admitted).await?,
        Operation::Transfer | Operation::ConvergenceTransfer => {
            transfer(operation, h, owner, admitted).await?;
        }
        _ => return Err("item Forge scenario requires an item operation".into()),
    }
    let staged: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_forge_wallets)+(SELECT count(*) FROM game_character_forge_items)+(SELECT count(*) FROM game_character_forge_receipts)+(SELECT count(*) FROM game_character_forge_history)")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        staged, 0,
        "the staged wallet/inventory prototype never owns a live operation"
    );
    Ok(())
}

struct PairAction {
    request: Request,
    selection: PairSelection,
    // Native fixture identities are used only to assert database after-state.
    native: ([u8; 16], [u8; 16]),
}

impl PairAction {
    fn wrong_source(mut self) -> Self {
        // The oldest entry is the seeded core: main backpack shows newest first.
        self.selection.second_from_backpack = Some(match self.request.operation {
            Operation::Transfer => 1, // nested input bag, then core
            _ => 2,                   // donor, receiver, then core
        });
        self
    }
}

fn pair(operation: Operation, occurrence: u8, first: u8, second: u8) -> PairAction {
    let selection = match operation {
        Operation::ConvergenceFusion => PairSelection {
            indices: Some((18, 17)), // 17 newer fillers, donor, receiver, core
            ..PairSelection::default()
        },
        Operation::Transfer => PairSelection {
            container: Some(0),
            indices: Some((1, 0)),
            ..PairSelection::default()
        },
        _ => PairSelection::default(),
    };
    PairAction {
        request: request(operation, occurrence),
        selection,
        native: (id(first), id(second)),
    }
}

struct Expected {
    succeeded: bool,
    bonus: u8,
    tiers: (u8, Option<u8>),
    /// Dust, cores and gold, respectively; all are actual native expenditures.
    cost: (i64, i64, i64),
}

async fn committed(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: PairAction,
    expected: Expected,
) -> TestResult {
    cooldown_elapsed(h).await?;
    let before = support::balances(h).await?;
    let gold = bank(h).await?;
    let native = action.native;
    let (action, (status, reply)) =
        exchange_pair(owner, admitted, action.request, action.selection).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(reply["applied"], true, "{reply}");
    assert_eq!(reply["succeeded"], expected.succeeded, "{reply}");
    assert_eq!(reply["bonus"], expected.bonus, "{reply}");
    assert_eq!(reply["reason"], Value::Null);
    let result = &reply["result"];
    assert_eq!(result["first_tier"], expected.tiers.0);
    assert_eq!(result["second_tier"], serde_json::json!(expected.tiers.1));
    assert_eq!(result["gold_spent"], expected.cost.2);
    assert_eq!(result["cores_spent"], expected.cost.1);
    assert_eq!(result["dust"], before.0 - expected.cost.0);
    assert_eq!(
        result["first_item"],
        action.first_item.ok_or("receiver handle")?.get()
    );
    assert_eq!(
        result["second_item"],
        action.second_item.ok_or("donor handle")?.get()
    );
    assert_eq!(
        support::balances(h).await?,
        (
            before.0 - expected.cost.0,
            before.1,
            before.2,
            before.3 - expected.cost.1
        )
    );
    assert_eq!(bank(h).await?, gold - expected.cost.2);
    item_state(h, native.0, Some(expected.tiers.0)).await?;
    item_state(h, native.1, expected.tiers.1).await?;
    receipt(h, action, &reply, Some(expected.cost)).await?;

    // A fresh connection issues new view handles; original bytes still replay even
    // though the consumed source is absent and the old receiver handle is stale.
    let after = assets(h).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, reply.clone())
    );
    assert_eq!(
        assets(h).await?,
        after,
        "replay cannot spend, reroll, mutate a tier or append audit"
    );
    receipt(h, action, &reply, Some(expected.cost)).await?;
    Ok(())
}

async fn refused(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: PairAction,
    reason: &str,
) -> TestResult<(Request, Value)> {
    cooldown_elapsed(h).await?;
    let before = assets(h).await?;
    let (action, (status, reply)) =
        exchange_pair(owner, admitted, action.request, action.selection).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(reply["applied"], false, "{reply}");
    assert_eq!(reply["succeeded"], false);
    assert_eq!(reply["reason"], reason);
    assert_eq!(
        assets(h).await?,
        before,
        "a refusal cannot leave a partial debit or tier change"
    );
    receipt(h, action, &reply, None).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, reply.clone())
    );
    assert_eq!(assets(h).await?, before);
    Ok((action, reply))
}

async fn fusion(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_bank(h).await?;
    // This replica has a source-backed Forge profile and needs a native Item
    // admission plus the immutable SQL projection to complete a paid fusion.
    const REPLICA: &str = "oteryn:item.tibia.i25973";
    seed_items(h, &[(101, REPLICA, 1, 0), (102, REPLICA, 1, 0)]).await?;
    let mut action = pair(Operation::Fusion, 120, 101, 102);
    action.request.occurrence =
        occurrence_for(h, 120, |rolls| rolls[0] < 50 && rolls[2] < 7400).await?;
    committed(
        h,
        owner,
        admitted,
        action,
        Expected {
            succeeded: true,
            bonus: 0,
            tiers: (1, None),
            cost: (100, 0, 25_000),
        },
    )
    .await?;

    // A failure still charges; a protection draw below 50 loses one source tier.
    seed_items(
        h,
        &[(104, OTHER_CLASS_FOUR, 1, 1), (105, OTHER_CLASS_FOUR, 1, 1)],
    )
    .await?;
    let mut action = pair(Operation::Fusion, 121, 104, 105);
    action.request.protect = true;
    action.request.occurrence =
        occurrence_for(h, 121, |rolls| rolls[0] >= 50 && rolls[1] < 50).await?;
    committed(
        h,
        owner,
        admitted,
        action,
        Expected {
            succeeded: false,
            bonus: 0,
            tiers: (1, Some(0)),
            cost: (100, 1, 20_000_000),
        },
    )
    .await?;

    // The other protection outcome keeps both instance tiers and records bonus 8.
    seed_items(
        h,
        &[(106, OTHER_CLASS_FOUR, 1, 1), (107, OTHER_CLASS_FOUR, 1, 1)],
    )
    .await?;
    let mut action = pair(Operation::Fusion, 122, 106, 107);
    action.request.protect = true;
    action.request.occurrence =
        occurrence_for(h, 122, |rolls| rolls[0] >= 50 && rolls[1] >= 50).await?;
    committed(
        h,
        owner,
        admitted,
        action,
        Expected {
            succeeded: false,
            bonus: 8,
            tiers: (1, Some(1)),
            cost: (100, 1, 20_000_000),
        },
    )
    .await?;

    // A draw in 50..65 succeeds only with the requested core; bonus 1 waives dust.
    seed_items(h, &[(108, SWORD, 1, 0), (109, SWORD, 1, 0)]).await?;
    let mut action = pair(Operation::Fusion, 123, 108, 109);
    action.request.boost = true;
    action.request.occurrence = occurrence_for(h, 123, |rolls| {
        (50..65).contains(&rolls[0]) && (7400..9000).contains(&rolls[2])
    })
    .await?;
    committed(
        h,
        owner,
        admitted,
        action,
        Expected {
            succeeded: true,
            bonus: 1,
            tiers: (1, None),
            cost: (0, 1, 25_000),
        },
    )
    .await?;

    // Bonus 3 also exercises the native audit path without a gold fee event.
    seed_items(h, &[(110, SWORD, 1, 0), (111, SWORD, 1, 0)]).await?;
    let mut action = pair(Operation::Fusion, 124, 110, 111);
    action.request.occurrence = occurrence_for(h, 124, |rolls| {
        rolls[0] < 50 && (9500..9525).contains(&rolls[2])
    })
    .await?;
    committed(
        h,
        owner,
        admitted,
        action,
        Expected {
            succeeded: true,
            bonus: 3,
            tiers: (1, None),
            cost: (100, 0, 0),
        },
    )
    .await?;

    seed_items(h, &[(112, SWORD, 1, 1), (113, SWORD, 1, 1)]).await?;
    refused(
        h,
        owner,
        admitted,
        pair(Operation::Fusion, 125, 112, 113),
        "WRONG_ITEMS",
    )
    .await?;
    Ok(())
}

async fn convergence_fusion(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(h, &[(101, CLASS_FOUR, 1, 0), (102, OTHER_CLASS_FOUR, 1, 0)]).await?;
    let filler: Vec<_> = (150..167)
        .map(|seed| (seed, FORGE_SLIVER_KEY, 1, 0))
        .collect();
    seed_items(h, &filler).await?;
    let slots: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(id(70).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        slots, 20,
        "identity-preserving Forge needs no Exaltation Chest slot (Q3)"
    );
    let action = pair(Operation::ConvergenceFusion, 130, 101, 102);
    let (action, refusal) = refused(h, owner, admitted, action, "INSUFFICIENT_GOLD").await?;
    seed_bank(h).await?;
    let funded = assets(h).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, refusal)
    );
    assert_eq!(
        assets(h).await?,
        funded,
        "funding cannot reopen a retained refusal"
    );
    committed(
        h,
        owner,
        admitted,
        pair(Operation::ConvergenceFusion, 131, 101, 102),
        Expected {
            succeeded: true,
            bonus: 0,
            tiers: (1, None),
            cost: (130, 0, 55_000_000),
        },
    )
    .await?;
    Ok(())
}

async fn transfer(
    operation: Operation,
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_bank(h).await?;
    let convergence = operation == Operation::ConvergenceTransfer;
    seed_items(
        h,
        &[
            (101, CLASS_FOUR, 1, 0),
            (102, OTHER_CLASS_FOUR, 1, if convergence { 1 } else { 2 }),
        ],
    )
    .await?;
    if !convergence {
        nest_pair(h).await?;
    }
    refused(
        h,
        owner,
        admitted,
        pair(operation, 140, 101, 103).wrong_source(),
        "WRONG_ITEMS",
    )
    .await?;
    // Crystal indexes normal gold/core prices by receiver result tier, not donor tier.
    committed(
        h,
        owner,
        admitted,
        pair(operation, 141, 101, 102),
        Expected {
            succeeded: true,
            bonus: 0,
            tiers: (1, None),
            cost: if convergence {
                (160, 1, 65_000_000)
            } else {
                (100, 1, 8_000_000)
            },
        },
    )
    .await?;
    Ok(())
}

#[path = "item_forge_fusion/fixture.rs"]
pub(super) mod fixture;
