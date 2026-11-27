use super::TestResult;

use super::fixture::{Harness, debug, id};

use super::item_forge_fixtures::pair_wire::{PairSelection, exchange_pair, extract_chest_item};

use super::item_forge_fixtures::support::{self, cooldown_elapsed, exchange, request, seed_items};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority};

use oteryn_gameplay_items::forge::{EXALTED_CORE_KEY, FORGE_SLIVER_KEY};

use oteryn_protocol_oteryn::{
    CommandStatus,
    item_forge::{Operation, Request},
};

use serde_json::Value;

#[path = "item_forge_fusion/fee.rs"]
pub(super) mod fee;

use fixture::{assets, bank, item_state, nest_pair, occurrence_for, receipt, seed_bank};

pub(super) const CHEST: &str = "oteryn:item.tibia.i37561";

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

pub(super) async fn chest_case(
    operation: Operation,
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    if operation == Operation::Transfer {
        seed_items(h, &[(103, EXALTED_CORE_KEY, 1, 0)]).await?;
        return transfer(operation, h, owner, admitted).await;
    }
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 28, 152],
        )
        .ok_or("Forge chest and Look capabilities")?;
    seed_bank(h).await?;
    seed_items(h, &[(101, SWORD, 1, 0), (102, SWORD, 1, 0)]).await?;
    let mut action = pair(Operation::Fusion, 120, 101, 102);
    action.selection.chest_items = Some(2);
    action.request.occurrence = occurrence_for(h, 120, |rolls| {
        rolls[0] < 50 && (9550..9950).contains(&rolls[2])
    })
    .await?;
    committed(
        h,
        owner,
        admitted,
        action,
        Expected {
            succeeded: true,
            bonus: 5,
            tiers: (1, Some(0)),
            cost: (100, 0, 25_000),
        },
    )
    .await?;
    let world = admitted
        .item_fence
        .ok_or("Forge item fence")?
        .runtime_scope
        .world_id();
    let metadata = owner
        .root
        .read_inspection_metadata(owner.character, world, id(101))
        .await?
        .ok_or("upgraded item's inspection metadata")?;
    assert_eq!(metadata.definition.production_key, SWORD);
    assert_eq!(metadata.tier, 1);
    let non_container_custody: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(id(101).as_slice()).fetch_one(&h.pool).await?;
    assert!(
        !non_container_custody,
        "a chest item has no direct bag, equipment or position address"
    );
    extract_chest_item(owner, admitted).await?;
    let moved: (Vec<u8>, i16) = sqlx::query_as("SELECT uuid_send(e.item_instance_id),t.tier::smallint FROM game_item_container_entries e JOIN game_item_tiers t USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(id(70).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(moved, (id(101).to_vec(), 1));
    Ok(())
}

pub(super) async fn scheduled_inputs(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    const STAFF: &str = "oteryn:item.tibia.i3289";
    seed_bank(h).await?;
    seed_items(h, &[(101, STAFF, 1, 0), (102, STAFF, 1, 0)]).await?;
    let schedules = [
        (
            "DELETE FROM game_item_timed_states s WHERE item_instance_id=encode($1,'hex')::uuid RETURNING to_jsonb(s)",
            "INSERT INTO game_item_timed_states AS s(item_instance_id,remaining_ms,deadline_at,state_revision) VALUES(encode($1,'hex')::uuid,1000,floor(extract(epoch FROM statement_timestamp())*1000)::bigint+86400000,1) RETURNING to_jsonb(s)",
        ),
        (
            "DELETE FROM game_spell_item_temporal_schedules s WHERE item_instance_id=encode($1,'hex')::uuid RETURNING to_jsonb(s)",
            "INSERT INTO game_spell_item_temporal_schedules AS s(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest) VALUES(encode($1,'hex')::uuid,encode($1,'hex')::uuid,1,1000,floor(extract(epoch FROM statement_timestamp())*1000)::bigint+86400000,'Item','oteryn:item.tibia.i3289','definition-r1',sha256('chest timer fixture'::bytea)) RETURNING to_jsonb(s)",
        ),
        (
            "DELETE FROM game_spell_field_temporal_schedules s WHERE item_instance_id=encode($1,'hex')::uuid RETURNING to_jsonb(s)",
            "INSERT INTO game_spell_field_temporal_schedules AS s(item_instance_id,source_stage,blocks_movement,blocks_projectile,immovable_block_solid,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest) VALUES(encode($1,'hex')::uuid,1,false,false,false,encode($1,'hex')::uuid,1,1000,floor(extract(epoch FROM statement_timestamp())*1000)::bigint+86400000,'Item','oteryn:item.tibia.i3289','definition-r1',sha256('chest timer fixture'::bytea)) RETURNING to_jsonb(s)",
        ),
    ];
    for (offset, (remove, insert)) in schedules.into_iter().enumerate() {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let before: Value = sqlx::query_scalar(insert)
            .bind(id(101).as_slice())
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        let occurrence = 120 + u8::try_from(offset)?;
        let mut action = pair(Operation::Fusion, occurrence, 101, 102);
        action.request.occurrence =
            occurrence_for(h, occurrence, |rolls| rolls[0] < 50 && rolls[2] < 7400).await?;
        refused(h, owner, admitted, action, "WRONG_ITEMS").await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let after: Value = sqlx::query_scalar(remove)
            .bind(id(101).as_slice())
            .fetch_one(&mut *tx)
            .await?;
        assert_eq!(after, before);
        tx.commit().await?;
    }
    // An enchanted staff decays into a forgeable staff. Keep the original
    // schedule and settled receipt as immutable history for the same ItemInstance.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,60000,1,'Item','oteryn:item.tibia.i3321','definition-r1','Item','oteryn:item.tibia.i3289','definition-r1',sha256('completed staff timer fixture'::bytea))")
        .bind(id(101).as_slice()).bind(id(160).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_spell_item_temporal_receipts(transaction_id,event_id,item_instance_id,world_id,channel_id,ownership_generation,state_revision_before,quantity_before,occurred_at_unix_ms,custody_kind,character_id,game_session_id,character_lease_generation,connection_generation,source_parent,source_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,1,1,1,2,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,1,encode($8,'hex')::uuid,1)")
        .bind(id(161).as_slice()).bind(id(162).as_slice()).bind(id(101).as_slice())
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(41).as_slice())
        .bind(admitted.game_session_id.as_bytes().as_slice()).bind(id(70).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut action = pair(Operation::Fusion, 123, 101, 102);
    action.request.occurrence =
        occurrence_for(h, 123, |rolls| rolls[0] < 50 && rolls[2] < 7400).await?;
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
    let history: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules s JOIN game_spell_item_temporal_receipts r USING(item_instance_id) WHERE s.item_instance_id=encode($1,'hex')::uuid AND s.definition_production_key='oteryn:item.tibia.i3321' AND r.transaction_id=encode($2,'hex')::uuid)")
        .bind(id(101).as_slice()).bind(id(161).as_slice()).fetch_one(&h.pool).await?;
    assert!(history);
    Ok(())
}

pub(super) async fn full_inventory(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_bank(h).await?;
    // Every parent that can hold both a chest and its children has a full
    // stack of slots. The empty quiver cannot accept a chest, and removing
    // the pair only frees slots at maximum depth.
    let mut items: Vec<_> = (120..138)
        .map(|seed| (seed, FORGE_SLIVER_KEY, 1, 0))
        .collect();
    items.push((138, "oteryn:item.tibia.i35562", 1, 0));
    items.push((140, CHEST, 1, 0));
    for depth in 0..6 {
        items.push((141 + depth, CHEST, 1, 0));
        items.push((150 + depth, FORGE_SLIVER_KEY, 1, 0));
    }
    items.extend([(101, SWORD, 1, 0), (102, SWORD, 1, 0)]);
    seed_items(h, &items).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for depth in 0..=6 {
        let pair = if depth == 6 {
            [101, 102]
        } else {
            [150 + depth, 141 + depth]
        };
        for (index, item) in pair.into_iter().enumerate() {
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(item).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)")
                .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(140 + depth).as_slice())
                .bind(i64::try_from(index + 1)?).bind(id(72).as_slice()).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    let mut action = pair(Operation::Fusion, 120, 101, 102);
    action.selection.container = Some(0);
    action.selection.nested_path = &[0; 6];
    action.request.occurrence =
        occurrence_for(h, 120, |rolls| rolls[0] < 50 && rolls[2] < 7400).await?;
    refused(h, owner, admitted, action, "NO_ROOM").await?;
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
    let prove_chest_readers = action.selection.chest_items.is_some();
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

    if expected.succeeded {
        let outputs: Value = sqlx::query_scalar("SELECT outputs FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
            .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(outputs.as_array().map(Vec::len), Some(1));
        let chest = outputs[0]["item"].as_str().ok_or("chest identity")?;
        assert_eq!(outputs[0]["key"], CHEST);
        let stored: (String, i64, i16, bool) = sqlx::query_as("SELECT definition_production_key,quantity,lifecycle,game_forge_carried(encode($2,'hex')::uuid,world_id,item_instance_id,$3) FROM game_item_instances WHERE item_instance_id=$1::uuid")
            .bind(chest).bind(id(41).as_slice()).bind(&outputs[0]["location"]).fetch_one(&h.pool).await?;
        assert_eq!(stored, (CHEST.into(), 1, 1, true));
        let children: Vec<(Vec<u8>, i16)> = sqlx::query_as("SELECT uuid_send(e.item_instance_id),coalesce(t.tier,0)::smallint FROM game_item_nested_entries e LEFT JOIN game_item_tiers t USING(item_instance_id) WHERE e.parent_item_instance_id=$1::uuid ORDER BY e.placement_ordinal")
            .bind(chest).fetch_all(&h.pool).await?;
        let mut wanted = vec![(native.0.to_vec(), i16::from(expected.tiers.0))];
        if let Some(tier) = expected.tiers.1 {
            wanted.push((native.1.to_vec(), i16::from(tier)));
        }
        assert_eq!(children, wanted);
    }

    // A fresh connection issues new view handles; original bytes still replay even
    // though the consumed source is absent and the old receiver handle is stale.
    let after = assets(h).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, reply.clone())
    );
    if prove_chest_readers {
        let mut stale = *admitted;
        stale.continuity.connection_generation += 1;
        assert_eq!(
            owner
                .item_forge(stale, admitted.continuity.next_command_id, action, None)
                .await,
            Some(Vec::new()),
            "an incorrect connection generation cannot replay a chest receipt"
        );
    }
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
    seed_items(h, &[(101, SWORD, 1, 0), (102, SWORD, 1, 0)]).await?;
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
    assert_eq!(slots, 20, "the backpack is full before the pair is removed");
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
                (100, 1, 20_000_000)
            },
        },
    )
    .await?;
    if !convergence {
        let proof: (String, i64) = sqlx::query_as(
            "SELECT r.ruleset_revision, p.gold_spent
             FROM game_forge_operations p JOIN game_forge_reservations r
               USING (character_id, occurrence_id)
             WHERE p.game_session_id=encode($1,'hex')::uuid AND p.operation=3 AND p.applied",
        )
        .bind(admitted.game_session_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(proof, ("global-forge-v2".to_owned(), 20_000_000));
    }
    Ok(())
}

#[path = "item_forge_fusion/fixture.rs"]
pub(super) mod fixture;
