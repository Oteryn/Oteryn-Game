use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::db::{begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction, lock_admission_relations};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::item_use::{self, ItemUseError, ItemUseInventory, ItemUseKind, ItemUseRequest};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::{fence, model, receipts};
use oteryn_protocol_oteryn::hireling::management::{Refusal, Reply};
use sqlx::Row;

/// The item identity and complete native use plan are fixed before the durable transition.
#[derive(Debug, Clone)]
pub struct HirelingLampPlacement {
    pub hireling: [u8; 16],
    pub input: super::HirelingLampInput,
    pub house: String,
    pub origin_channel: [u8; 16],
    pub position: (u16, u16, i8),
    pub item_use: ItemUseRequest,
}

impl HirelingLampPlacement {
    fn binding(&self) -> [u8; 32] {
        self.input.binding()
    }
}

impl DurabilityRoot {
    /// The same carried-tree projection as ordinary USE, after independent named House proof.
    pub async fn read_hireling_use_inventory(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        current: CurrentCharacterItemFence, origin_channel: [u8; 16],
    ) -> Result<ItemUseInventory, DurabilityError> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !fence::assert_house_inventory_fence(&mut tx, &node, &current, origin_channel).await? {
                return Err(DurabilityError::InvalidStoredState);
            }
            let row = sqlx::query("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(current.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let inventory = item_use::read_use_inventory(&mut tx, &current, row.try_get("content_revision")?, row.try_get("ruleset_revision")?)
                .await.map_err(item_error)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(inventory)
        })).await
    }

    /// Consume the owned physical lamp and activate its identity atomically. A receipt replay
    /// returns the original outcome and does not reapply its obsolete runtime placement.
    pub async fn commit_hireling_lamp(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        current: CurrentCharacterItemFence, plan: HirelingLampPlacement,
    ) -> Result<Reply, DurabilityError> {
        plan.item_use.validate().map_err(item_error)?;
        plan.input.validate(plan.item_use.source_item_instance_id,plan.position)?;
        if plan.house.is_empty() || plan.house.len() > 512 || !(-15..=0).contains(&plan.position.2)
            || plan.item_use.kind != (ItemUseKind::HirelingLamp { origin_channel: plan.origin_channel })
            || plan.item_use.source_quantity_before != 1
            || plan.item_use.source_definition.family != "Item"
            || plan.item_use.source_definition.production_key != model::HIRELING_LAMP_KEY {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let digest = plan.binding();
            if let Some(reply) = receipts::replay(&mut tx, &current, plan.item_use.command_id, &digest).await? {
                tx.commit(deadline).await?;
                return Ok(reply);
            }
            if !fence::assert_house_inventory_fence(&mut tx, &node, &current, plan.origin_channel).await? {
                return Ok(Reply::Refused(Refusal::Stale));
            }
            // A live refusal is a durable answer too; retrying its command after a
            // later state change cannot turn it into a successful item burn.
            macro_rules! refuse {
                ($reason:expr) => {{
                    let reply=Reply::Refused($reason);
                    receipts::write(&mut tx,&current,plan.item_use.command_id,None,digest,&reply,None).await?;
                    tx.commit(deadline).await?;
                    return Ok(reply);
                }};
            }
            let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_ownership o \
                JOIN game_durability_reconnect_sessions s ON s.world_id=o.world_id AND s.runtime_scope_house_key=o.house_key \
                JOIN game_character_roots r ON r.character_id=s.character_id \
                WHERE s.game_session_id=encode($1,'hex')::uuid AND o.house_key=$2 \
                  AND game_house_effective_owner(o.world_id,o.house_key)=s.character_id \
                  AND r.content_revision=$3 AND r.ruleset_revision=$4 FOR SHARE OF o)")
                .bind(current.game_session_id.as_bytes().as_slice()).bind(&plan.house)
                .bind(&plan.item_use.content_revision).bind(&plan.item_use.ruleset_revision)
                .fetch_one(&mut **tx).await?;
            if !valid { refuse!(Refusal::NotOwner); }
            let row = sqlx::query("SELECT active,uuid_send(lamp_item_instance_id) AS lamp FROM game_house_hirelings \
                WHERE hireling_id=encode($1,'hex')::uuid AND owner_character_id=encode($2,'hex')::uuid FOR UPDATE")
                .bind(plan.hireling.as_slice()).bind(current.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let Some(row) = row else { refuse!(Refusal::NotOwner); };
            if row.try_get::<bool,_>("active")? || row.try_get::<Option<Vec<u8>>,_>("lamp")?.as_deref()!=Some(plan.item_use.source_item_instance_id.as_slice()) {
                refuse!(Refusal::Conflict);
            }
            let reserved: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_store_reservations WHERE hireling_id=encode($1,'hex')::uuid AND state=1)")
                .bind(plan.hireling.as_slice()).fetch_one(&mut **tx).await?;
            if reserved { refuse!(Refusal::Conflict); }
            // Serialize all placements in this House, including different character roots.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:hireling-house:'||encode($1,'hex')||':'||$2,0))")
                .bind(current.runtime_scope.world_id().as_bytes().as_slice()).bind(&plan.house).execute(&mut **tx).await?;
            let occupied: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_hirelings WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND active AND x=$3 AND y=$4 AND floor=$5)")
                .bind(current.runtime_scope.world_id().as_bytes().as_slice()).bind(&plan.house)
                .bind(i32::from(plan.position.0)).bind(i32::from(plan.position.1)).bind(i16::from(plan.position.2)).fetch_one(&mut **tx).await?;
            if occupied { refuse!(Refusal::Conflict); }
            let tuple = tx.tuple();
            item_use::write(&mut tx, &current, &plan.item_use, tuple).await.map_err(item_error)?;
            sqlx::query("INSERT INTO game_hireling_lamp_transitions \
                (game_session_id,command_id,hireling_id,character_id,world_id,house_key,kind,lamp_item_instance_id,transaction_id,x,y,floor) \
                VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,1,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,$10,$11)")
                .bind(current.game_session_id.as_bytes().as_slice()).bind(plan.item_use.command_id.to_string())
                .bind(plan.hireling.as_slice()).bind(current.character_id.as_bytes().as_slice())
                .bind(current.runtime_scope.world_id().as_bytes().as_slice()).bind(&plan.house)
                .bind(plan.item_use.source_item_instance_id.as_slice()).bind(plan.item_use.transaction_id.as_slice())
                .bind(i32::from(plan.position.0)).bind(i32::from(plan.position.1)).bind(i16::from(plan.position.2))
                .execute(&mut **tx).await?;
            sqlx::query("UPDATE game_house_hirelings SET active=TRUE,house_key=$2,x=$3,y=$4,floor=$5, \
                last_lamp_session=encode($6,'hex')::uuid,last_lamp_command=$7::text::numeric,last_lamp_transaction=encode($8,'hex')::uuid, \
                last_lamp_connection_generation=$9::text::numeric,last_lamp_lease_generation=$10::text::numeric,last_lamp_channel=encode($11,'hex')::uuid \
                WHERE hireling_id=encode($1,'hex')::uuid")
                .bind(plan.hireling.as_slice()).bind(&plan.house).bind(i32::from(plan.position.0)).bind(i32::from(plan.position.1))
                .bind(i16::from(plan.position.2)).bind(current.game_session_id.as_bytes().as_slice())
                .bind(plan.item_use.command_id.to_string()).bind(plan.item_use.transaction_id.as_slice())
                .bind(current.connection_generation.get().to_string()).bind(current.character_lease_generation.to_string())
                .bind(plan.origin_channel.as_slice()).execute(&mut **tx).await?;
            let reply=Reply::Committed;
            receipts::write(&mut tx,&current,plan.item_use.command_id,Some(plan.hireling),digest,&reply,Some(plan.position)).await?;
            tx.commit(deadline).await?;
            Ok(reply)
        })).await
    }
}

fn item_error(error: ItemUseError) -> DurabilityError {
    match error { ItemUseError::Unavailable(error) => error, _ => DurabilityError::InvalidStoredState }
}
