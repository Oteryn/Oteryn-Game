//! Current Tibia: each native Cake bite adds240s through the unchanged food kernel.
use crate::durability::character_condition_snapshot::{self, CharacterConditionSnapshot};
use crate::durability::item_craft::MapHarvestRequest;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::{DurabilityError, DurabilityRoot};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct CakeFoodContext {
    pub actor_identity: [u8; 16],
    pub movement_revision: u64,
    pub state_revision: u64,
    pub before: CharacterConditionSnapshot,
    pub after: CharacterConditionSnapshot,
}
impl CakeFoodContext {
    pub(crate) fn valid(&self, fence: &CurrentCharacterItemFence, digest: [u8; 32]) -> bool {
        use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey};
        use oteryn_gameplay_foundation::foundation::condition::ConditionStore;
        if !self.before.valid()
            || !self.after.valid()
            || self.before.lease_generation != fence.character_lease_generation
            || self.after.lease_generation != fence.character_lease_generation
            || self.before.content_digest != digest
            || self.after.content_digest != digest
        {
            return false;
        }
        let Ok(after) = ConditionStore::from_persistent_snapshot(&self.after.conditions, 0) else {
            return false;
        };
        let Some(food) = after.get(ConflictKey::FoodRegeneration) else {
            return false;
        };
        if food.definition().key() != "food.regeneration"
            || !matches!(
                food.definition().values(),
                ConditionValues::FoodRegeneration {
                    added_ms: 240_000,
                    interval_ms: 1000
                }
            )
        {
            return false;
        }
        let split = |value: &serde_json::Value| -> Option<(u64, Vec<serde_json::Value>)> {
            let mut remaining = 0;
            let mut others = Vec::new();
            for row in value.get("conditions")?.as_array()? {
                if row
                    .get("definition")?
                    .get("values")?
                    .get("FoodRegeneration")
                    .is_some()
                {
                    remaining = row.get("ends_at")?.as_u64()?;
                } else {
                    others.push(row.clone());
                }
            }
            Some((remaining, others))
        };
        let (Some((previous, a)), Some((next, b))) = (
            split(&self.before.conditions),
            split(&self.after.conditions),
        ) else {
            return false;
        };
        a == b && next < 1_200_000_000 && previous.checked_add(240_000_000) == Some(next)
    }
}

pub(crate) async fn persist(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    request: &MapHarvestRequest,
    food: &CakeFoodContext,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
) -> Result<(), DurabilityError> {
    let fact = node.fact();
    sqlx::query("SELECT game_cake_arm_food_observation(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8,$9::text::numeric,$10::text::numeric,$11,$12,$13)")
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(food.actor_identity.as_slice()).bind(food.movement_revision.to_string()).bind(food.state_revision.to_string()).bind(food.after.content_digest.as_slice()).bind(&food.before.conditions).bind(&food.after.conditions).execute(&mut ***tx).await?;
    let inserted=sqlx::query("INSERT INTO game_seasonal_cake_food_uses(game_session_id,command_id,character_id,world_id,event_start,content_digest,actor_identity,movement_revision,state_revision,character_lease_generation,conditions_before,conditions_after,conditions_sha256) SELECT b.game_session_id,b.command_id,b.character_id,b.world_id,b.event_start,$4,$5,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9,$10,sha256(convert_to($10::jsonb::text,'UTF8')) FROM game_seasonal_cake_bite_receipts b WHERE b.game_session_id=encode($1,'hex')::uuid AND b.command_id=$2::text::numeric AND b.character_id=encode($3,'hex')::uuid AND b.created_xact_id=pg_current_xact_id()")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice())
        .bind(food.after.content_digest.as_slice()).bind(food.actor_identity.as_slice()).bind(food.movement_revision.to_string()).bind(food.state_revision.to_string()).bind(fence.character_lease_generation.to_string()).bind(&food.before.conditions).bind(&food.after.conditions).execute(&mut ***tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    character_condition_snapshot::write_in_transaction(tx, fence, &food.after).await
}

impl DurabilityRoot {
    /// Same command advisory lock as the writer: false proves its transaction did not commit.
    pub async fn read_cake_food_receipt(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: MapHarvestRequest,
        food: CakeFoodContext,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:terrain-command:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut *tx).await?;
            let row=sqlx::query("SELECT f.character_id=encode($3,'hex')::uuid AS same_character,t.intent=$4 AS same_intent,f.conditions_sha256=sha256(convert_to($5::jsonb::text,'UTF8')) AS same_conditions FROM game_seasonal_cake_food_uses f JOIN game_terrain_tool_receipts t USING(game_session_id,command_id) WHERE f.game_session_id=encode($1,'hex')::uuid AND f.command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(&request.intent).bind(&food.after.conditions).fetch_optional(&mut *tx).await?;
            let committed=if let Some(row)=row{
                if !row.try_get::<bool,_>("same_character")? || !row.try_get::<bool,_>("same_intent")? || !row.try_get::<bool,_>("same_conditions")? {return Err(DurabilityError::InvalidStoredState);}
                true
            }else{
                let orphan:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_terrain_tool_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric)").bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_one(&mut *tx).await?;
                if orphan{return Err(DurabilityError::InvalidStoredState);}
                false
            };
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(committed)
        })).await
    }
}
