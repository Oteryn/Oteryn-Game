//! Depot transfers share the item-loop transaction, replay and conservation owner.
use super::*;
use crate::domain::premium::PremiumGate;
use oteryn_gameplay_items::depot::{DEPOT_BOXES, DEPOT_FREE_ENTRIES, DEPOT_HARD_ENTRIES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DepotMove {
    pub town: u16,
    pub box_number: u8,
    pub deposit: bool,
    /// Canonical wire request digest, so replay needs no ephemeral handle resolution.
    pub request_digest: [u8; 32],
    /// Current native entitlement decision. Never reconstructed from a receipt or client tier.
    #[serde(skip)]
    pub premium: PremiumGate,
}

pub(super) async fn plan_deposit(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    access: DepotMove,
    height: i32,
    quantity: u32,
    facts: &ItemDefinitionFacts,
) -> LoopResult<PlannedDestination> {
    if !(1..=DEPOT_BOXES).contains(&access.box_number) || height != 0 {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let (maximum, _) = stack_maximum(facts.stack)
        .map_err(|_| LoopError::Refused(LoopMoveRefusal::NotSupported))?;
    if quantity > maximum {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(committed),0)::bigint FROM game_character_depot_box_counters \
         WHERE character_id=encode($1,'hex')::uuid AND town_id=$2",
    )
    .bind(character.as_bytes().as_slice())
    .bind(i32::from(access.town))
    .fetch_one(&mut **tx)
    .await?;
    let limit = if access.premium.premium_current() {
        DEPOT_HARD_ENTRIES
    } else {
        DEPOT_FREE_ENTRIES
    };
    if count >= i64::from(limit) {
        return refuse(LoopMoveRefusal::NoRoom);
    }
    Ok(PlannedDestination {
        target: Target::Depot {
            town: access.town,
            box_number: access.box_number,
        },
        receiver: None,
        rest: quantity,
    })
}

pub(super) async fn place(
    tx: &mut Transaction<'_, Postgres>,
    item: [u8; 16],
    world: [u8; 16],
    character: CharacterId,
    town: u16,
    box_number: u8,
) -> LoopResult<()> {
    sqlx::query(
        "INSERT INTO game_item_character_depot_locations \
        (item_instance_id,world_id,character_id,town_id,box_number) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5)",
    )
    .bind(item.as_slice())
    .bind(world.as_slice())
    .bind(character.as_bytes().as_slice())
    .bind(i32::from(town))
    .bind(i16::from(box_number))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl DurabilityRoot {
    /// Depot-only replay with exact intent binding, before resolving a session handle.
    pub async fn read_depot_move_replay(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
        access: DepotMove,
    ) -> Result<LoopMoveReplay, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=super::super::db::begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            lock_loop_command(&mut tx,command).await?;
            let row=sqlx::query("SELECT character_id::text,intent FROM game_item_loop_move_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let replay=if let Some(row)=row {
                let intent:Vec<u8>=row.try_get("intent")?;
                let intent:serde_json::Value=serde_json::from_slice(&intent).map_err(|_|DurabilityError::InvalidStoredState)?;
                if uuid_text(row.try_get("character_id")?)? == *character.as_bytes()
                    && intent.get("depot")==Some(&serde_json::to_value(access).map_err(|_|DurabilityError::InvalidStoredState)?) {
                    LoopMoveReplay::Moved
                } else { LoopMoveReplay::Conflicting }
            } else { LoopMoveReplay::Absent };
            super::super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(replay)
        })).await
    }

    /// Whole empty-item/stack transfer, under the same current fence and receipt as item moves.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_depot_move<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        access: DepotMove,
        item: [u8; 16],
    ) -> std::result::Result<std::result::Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError>
    {
        if !(1..=DEPOT_BOXES).contains(&access.box_number) {
            return Ok(Err(LoopMoveRefusal::Rejected));
        }
        self.commit_loop_with_depot(
            permit,
            recovery,
            node,
            fence,
            command,
            content,
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::MainBackpack,
            },
            Some(access),
        )
        .await
    }
}
