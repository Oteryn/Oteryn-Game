//! Authority selection stays inside the native Inbox owner. The ordinary reward
//! path retains its original Channel fence; only explicit Hireling entrypoints
//! can prove a named House or a server-owned return of an original placement.
use super::*;
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::runtime_scope_assignment::prove_current_incarnation;
use crate::foundation::RuntimeScopeRefV1;

/// Return intent, not an authority capability. The native owner independently
/// proves current House assignment and the exact persisted placement provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirelingRecallRequest {
    pub hireling: [u8; 16],
    pub house: String,
    pub scope_ownership_generation: u64,
}

#[derive(Debug, Clone)]
pub(super) enum InboxAuthority {
    Character(CurrentCharacterItemFence),
    Hireling(CurrentCharacterItemFence),
    Recall(HirelingRecallRequest),
}

#[derive(Debug, Clone)]
pub(super) struct HouseAuditAuthority {
    pub house: String,
    pub generation: u64,
    /// 1 is a live House session; 2 is independent current House recall authority.
    pub kind: u8,
}

#[derive(Debug, Clone)]
pub(super) struct ReceiptGenerations {
    pub connection: u64,
    pub character_lease: u64,
    pub house: Option<HouseAuditAuthority>,
}

impl InboxAuthority {
    pub(super) async fn assert_current(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        node: &NodeIncarnationProof,
        plan: &RewardClaimInboxPlan,
    ) -> Result<Option<ReceiptGenerations>, DurabilityError> {
        let fence = match self {
            Self::Character(fence) | Self::Hireling(fence) => fence,
            Self::Recall(request) => return recall_current(tx, node, plan, request).await,
        };
        if !live_plan_matches(fence, plan) {
            return Ok(None);
        }
        let current = match (self, fence.runtime_scope) {
            (Self::Hireling(_), RuntimeScopeRefV1::Instance { .. }) => {
                crate::durability::house_hireling::fence::assert_house_inventory_fence(
                    tx, node, fence, *plan.channel_id.as_bytes(),
                ).await?
            }
            _ => character_item_fence_is_current(
                tx, node, fence, plan.command, plan.character_id,
                *plan.world_id.as_bytes(), *plan.channel_id.as_bytes(),
            ).await?,
        };
        if !current { return Ok(None); }
        let house = if matches!((self, fence.runtime_scope),
            (Self::Hireling(_), RuntimeScopeRefV1::Instance { .. })) {
            let house: String = sqlx::query_scalar(
                "SELECT runtime_scope_house_key FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_house_key IS NOT NULL",
            ).bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            Some(HouseAuditAuthority { house, generation: fence.scope_ownership_generation.get(), kind: 1 })
        } else { None };
        Ok(Some(ReceiptGenerations {
            connection: fence.connection_generation.get(),
            character_lease: fence.character_lease_generation,
            house,
        }))
    }
}

fn live_plan_matches(fence: &CurrentCharacterItemFence, plan: &RewardClaimInboxPlan) -> bool {
    fence.game_session_id == plan.command.game_session_id()
        && fence.character_id == plan.character_id
        && fence.runtime_scope.world_id() == plan.world_id
}

async fn recall_current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    plan: &RewardClaimInboxPlan,
    request: &HirelingRecallRequest,
) -> Result<Option<ReceiptGenerations>, DurabilityError> {
    let Some(house) = HouseId::new(plan.world_id, &request.house) else { return Ok(None); };
    if request.scope_ownership_generation == 0 || !prove_current_incarnation(tx, node).await? {
        return Ok(None);
    }
    // Houses have their own assignment namespace. An origin Channel or an
    // activity Instance never substitutes for this current server authority.
    let history: bool = sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
        .fetch_one(&mut **tx).await?;
    if !history { return Ok(None); }
    let fact = node.fact();
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key=$1 AND scope_kind=2 AND world_id=encode($2,'hex')::uuid \
           AND house_key=$3 AND state=1 AND ownership_generation=$4::text::numeric \
           AND holder_node_id=encode($5,'hex')::uuid \
           AND holder_registration_revision=$6::text::numeric FOR SHARE)",
    ).bind(house.scope_key()).bind(plan.world_id.as_bytes().as_slice()).bind(&request.house)
     .bind(request.scope_ownership_generation.to_string()).bind(fact.node_id().as_bytes().as_slice())
     .bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
    if !assigned { return Ok(None); }
    let root = sqlx::query(
        "SELECT profile_revision,ruleset_revision,content_revision,starter_template_revision \
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid \
           AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE",
    ).bind(plan.character_id.as_bytes().as_slice()).bind(plan.world_id.as_bytes().as_slice())
     .fetch_optional(&mut **tx).await?;
    let current = sqlx::query(
        "SELECT profile_revision,ruleset_revision,content_revision,starter_template_revision \
         FROM game_character_interpretations ORDER BY interpretation_revision DESC LIMIT 1",
    ).fetch_optional(&mut **tx).await?;
    let (Some(root), Some(current)) = (root, current) else { return Ok(None); };
    for column in ["profile_revision", "ruleset_revision", "content_revision", "starter_template_revision"] {
        if root.try_get::<String,_>(column)? != current.try_get::<String,_>(column)? { return Ok(None); }
    }
    // Migration0700 seals the genuine0405 receipt's identity association before
    // making it immutable. The old reconnect session may be gone or elsewhere;
    // its ongoing currentness is never required for this House-owned recovery.
    let row = sqlx::query(
        "SELECT h.last_lamp_connection_generation::text AS connection_generation, \
                h.last_lamp_lease_generation::text AS character_lease_generation \
         FROM game_house_hirelings h WHERE h.hireling_id=encode($1,'hex')::uuid \
           AND h.owner_character_id=encode($2,'hex')::uuid AND h.world_id=encode($3,'hex')::uuid \
           AND h.active AND h.house_key=$4 AND h.last_lamp_session=encode($5,'hex')::uuid \
           AND h.last_lamp_command=$6::text::numeric AND h.last_lamp_channel=encode($7,'hex')::uuid \
           AND h.last_lamp_connection_generation>0 AND h.last_lamp_lease_generation>0 \
           AND ((h.legacy_grant AND EXISTS(SELECT 1 FROM game_house_hireling_receipts r \
                 WHERE r.game_session_id=h.last_lamp_session AND r.command_id=h.last_lamp_command \
                   AND r.hireling_id=h.hireling_id AND r.character_id=h.owner_character_id \
                   AND r.active AND (r.x,r.y,r.floor)=(h.x,h.y,h.floor))) \
             OR EXISTS(SELECT 1 FROM game_hireling_lamp_transitions t \
                 WHERE t.game_session_id=h.last_lamp_session AND t.command_id=h.last_lamp_command \
                   AND t.hireling_id=h.hireling_id AND t.character_id=h.owner_character_id \
                   AND t.world_id=h.world_id AND t.house_key=h.house_key AND t.kind=1 \
                   AND t.transaction_id=h.last_lamp_transaction)) FOR UPDATE OF h",
    ).bind(request.hireling.as_slice()).bind(plan.character_id.as_bytes().as_slice())
     .bind(plan.world_id.as_bytes().as_slice()).bind(&request.house)
     .bind(plan.command.game_session_id().as_bytes().as_slice()).bind(plan.command.command_id().get().to_string())
     .bind(plan.channel_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| Ok(ReceiptGenerations {
        connection: numeric_u64(&row, "connection_generation")?,
        character_lease: numeric_u64(&row, "character_lease_generation")?,
        house: Some(HouseAuditAuthority { house: request.house.clone(),
            generation: request.scope_ownership_generation, kind: 2 }),
    })).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{CommandId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};

    #[test]
    fn hireling_inbox_plan_cannot_substitute_session_character_or_world() -> Result<(), Box<dyn std::error::Error>> {
        let id = |tag: u8| {
            let mut bytes = [tag; 16]; bytes[6] = 0x70 | (tag & 15); bytes[8] = 0x80 | (tag & 63); bytes
        };
        fn invalid<T>(_: T) -> &'static str { "invalid fixture identity" }
        let session = GameSessionId::decode(&id(1)).map_err(invalid)?;
        let world = WorldId::decode(&id(2)).map_err(invalid)?;
        let channel = ChannelId::decode(&id(3)).map_err(invalid)?;
        let character = CharacterId::from_bytes(id(4)).map_err(invalid)?;
        let fence = CurrentCharacterItemFence {
            character_id: character, game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).map_err(invalid)?,
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(world, channel),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(invalid)?,
        };
        let command = CommandId::new(1).map_err(invalid)?;
        let plan = RewardClaimInboxPlan {
            command: CommandRef::new(session, command), character_id: character,
            world_id: world, channel_id: channel, transaction_id: id(5),
            source_binding: [7;32], item_instance_ids: vec![id(6)],
        };
        assert!(live_plan_matches(&fence, &plan));
        let mut other = plan.clone();
        other.command = CommandRef::new(GameSessionId::decode(&id(8)).map_err(invalid)?, command);
        assert!(!live_plan_matches(&fence, &other));
        other = plan.clone();
        other.character_id = CharacterId::from_bytes(id(9)).map_err(invalid)?;
        assert!(!live_plan_matches(&fence, &other));
        other = plan;
        other.world_id = WorldId::decode(&id(10)).map_err(invalid)?;
        assert!(!live_plan_matches(&fence, &other));
        Ok(())
    }
}
