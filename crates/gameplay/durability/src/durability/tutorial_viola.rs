//! Lost Wisdom: source NPC state, exact item hand-in and native XP in one transaction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceRewardOccurrence, assert_gameplay_fence,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_craft::{ItemCraftRequest, audit};
use super::item_loop_move::{ItemLoopContent, carried_weight};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{read_derived_transfer_capacity_centi_oz, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::domain::progression::FiniteProgressionPolicy;
use crate::foundation::CommandRef;
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViolaAction {
    Mission,
    Accept,
}
impl ViolaAction {
    fn key(self) -> i16 {
        match self {
            Self::Mission => 1,
            Self::Accept => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViolaReply {
    Offer,
    Accepted,
    MissingBook,
    Completed,
    AlreadyCompleted,
    NoRoom,
    ReturnOffer,
}
impl ViolaReply {
    fn code(self) -> i16 {
        match self {
            Self::Offer => 0,
            Self::Accepted => 1,
            Self::MissingBook => 2,
            Self::Completed => 3,
            Self::AlreadyCompleted => 4,
            Self::NoRoom => 5,
            Self::ReturnOffer => 6,
        }
    }
    fn stored(code: i16) -> Result<Self, DurabilityError> {
        Ok(match code {
            0 => Self::Offer,
            1 => Self::Accepted,
            2 => Self::MissingBook,
            3 => Self::Completed,
            4 => Self::AlreadyCompleted,
            5 => Self::NoRoom,
            6 => Self::ReturnOffer,
            _ => return Err(DurabilityError::InvalidStoredState),
        })
    }
}

impl DurabilityRoot {
    /// A historical response never grants a new write or chooses a new book.
    pub async fn reconcile_viola(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
    ) -> Result<Option<ViolaReply>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let result = load(&mut tx, character, command, None).await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
            .map_err(Into::into)
    }

    /// Called under the Character revision slot after exact placed-NPC authorization.
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing recovery, node, revision, item and progression owners share one NPC transaction"
    )]
    pub async fn commit_viola<C: ItemLoopContent, const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        action: ViolaAction,
        policy: FiniteProgressionPolicy<String, N>,
        content: &C,
        output: TypedDefinitionRef,
        digest: [u8; 32],
    ) -> Result<ViolaReply, CharacterProgressionError> {
        if command.game_session_id() != fence.game_session_id
            || content.loop_content_digest() != digest
            || output.family != "Item"
            || output.production_key != "oteryn:item.tibia.i3572"
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,content|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:viola:'||encode($1,'hex'),0))").bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
            if let Some(reply)=load(&mut tx,fence.character_id,command,Some(action)).await? {tx.commit(deadline).await?;return Ok(Ok(reply));}
            if let Err(e)=assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(e));}
            let stage:i16=sqlx::query_scalar("SELECT coalesce((SELECT stage FROM game_character_viola WHERE character_id=encode($1,'hex')::uuid),0::smallint)").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let mut craft=None;
            let mut xp=None;
            let reply=match (stage,action) {
                (2,_)=>ViolaReply::AlreadyCompleted,
                (0,ViolaAction::Mission)=>ViolaReply::Offer,
                (0,ViolaAction::Accept)=>{sqlx::query("INSERT INTO game_character_viola(character_id,stage) VALUES(encode($1,'hex')::uuid,1)").bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;ViolaReply::Accepted},
                (1,ViolaAction::Mission)=>ViolaReply::ReturnOffer,
                (1,ViolaAction::Accept)=>{
                    match super::character_equipment::assert_current_equipment_in_transaction(&mut tx,&recovery,&node,&fence.item_fence(),Some(command),digest).await {
                        Ok(_)=>{},
                        Err(super::character_equipment::EquipmentError::Rejected(_))=>return Ok(Err(CharacterProgressionError::AuthorityRejected)),
                        Err(super::character_equipment::EquipmentError::Durability(e))=>return Err(e),
                        Err(super::character_equipment::EquipmentError::Database(e))=>return Err(e.into()),
                    }
                    match exchange(&mut tx,fence,command,*content,&output,digest).await? {
                        Exchange::Missing=>ViolaReply::MissingBook,
                        Exchange::NoRoom=>ViolaReply::NoRoom,
                        Exchange::Done(transaction)=>{
                            let bytes:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())").fetch_one(&mut **tx).await?;
                            let occurrence=ExperienceRewardOccurrence::from_bytes(bytes.try_into().map_err(|_|DurabilityError::InvalidStoredState)?).map_err(|_|DurabilityError::InvalidStoredState)?;
                            let award=ExperienceAwardRequest {occurrence,amount:oteryn_simulation_determinism::ExactI64::new(800),context:policy.context.clone(),policy_revision:policy.policy_revision.clone(),reward_revision:policy.reward_revision.clone(),policy:policy.clone()};
                            if let Err(e)=super::character_experience_transaction::commit_in_transaction(&mut tx,&node,fence,&award,false,None).await? {return Ok(Err(e));}
                            sqlx::query("UPDATE game_character_viola SET stage=2 WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
                            craft=Some(transaction);xp=Some(occurrence.as_bytes().to_vec());ViolaReply::Completed
                        }
                    }
                },
                _=>return Err(DurabilityError::InvalidStoredState),
            };
            sqlx::query("INSERT INTO game_viola_receipts(character_id,game_session_id,command_id,action,reply,craft_transaction_id,xp_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6::uuid,encode($7,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(action.key()).bind(reply.code()).bind(craft).bind(xp).execute(&mut **tx).await?;
            tx.commit(deadline).await?;Ok(Ok(reply))
        })).await?
    }
}

async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    command: CommandRef,
    action: Option<ViolaAction>,
) -> Result<Option<ViolaReply>, DurabilityError> {
    let row:Option<(i16,i16)>=sqlx::query_as("SELECT action,reply FROM game_viola_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric").bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    row.map(|(stored, reply)| {
        if action.is_some_and(|a| a.key() != stored) {
            Err(DurabilityError::InvalidStoredState)
        } else {
            ViolaReply::stored(reply)
        }
    })
    .transpose()
}

enum Exchange {
    Missing,
    NoRoom,
    Done(String),
}
async fn exchange<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    content: &C,
    output: &TypedDefinitionRef,
    digest: [u8; 32],
) -> Result<Exchange, DurabilityError> {
    let (world, _) =
        scope_of(&fence.item_fence()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let source=sqlx::query("SELECT to_jsonb(i) AS before,uuid_send(i.item_instance_id) AS source,uuid_send(b.item_instance_id) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal)::text AS ordinal,CASE WHEN e.item_instance_id IS NULL THEN to_jsonb(n) ELSE NULL END AS nested_source FROM game_item_container_slots b JOIN game_item_instances i ON i.world_id=b.world_id LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=b.character_id AND e.parent_item_instance_id=b.item_instance_id AND e.world_id=b.world_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id AND n.world_id=b.world_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL WHERE b.character_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i51977' AND (e.item_instance_id IS NOT NULL OR (owner.owner_character=b.character_id AND owner.depth BETWEEN 1 AND 7)) ORDER BY e.placement_ordinal NULLS LAST,n.placement_ordinal,i.item_instance_id LIMIT 1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(source) = source else {
        return Ok(Exchange::Missing);
    };
    let before: Value = source.try_get("before")?;
    let parent: Vec<u8> = source.try_get("parent")?;
    let nested: Option<Value> = source.try_get("nested_source")?;
    let source_id: [u8; 16] = source
        .try_get::<Vec<u8>, _>("source")?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i51977".into(),
        revision_ref: before["definition_revision_ref"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
    };
    let Some(book_weight) = content.loop_facts(&definition).and_then(|f| f.weight) else {
        return Ok(Exchange::NoRoom);
    };
    let Some(scarf_weight) = content.loop_facts(output).and_then(|f| f.weight) else {
        return Ok(Exchange::NoRoom);
    };
    let carried = carried_weight(tx, fence.character_id, *world.as_bytes(), content)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
    if carried
        .checked_sub(u64::from(book_weight))
        .and_then(|v| v.checked_add(u64::from(scarf_weight)))
        .is_none_or(|v| v > capacity)
    {
        return Ok(Exchange::NoRoom);
    }
    let quantity = before["quantity"]
        .as_i64()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (occupied,highest):(i64,String)=sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let bag:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE").bind(&parent).fetch_one(&mut ***tx).await?;
    let bag = TypedDefinitionRef {
        family: bag["definition_family"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
        production_key: bag["definition_production_key"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
        revision_ref: bag["definition_revision_ref"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
    };
    if occupied + 1 - i64::from(quantity == 1 && nested.is_none())
        > i64::from(
            content
                .loop_facts(&bag)
                .and_then(|f| f.container_capacity)
                .unwrap_or(0)
                .min(20),
        )
    {
        return Ok(Exchange::NoRoom);
    }
    let ordinal = if quantity == 1 && nested.is_none() {
        source.try_get::<String, _>("ordinal")?
    } else {
        highest
            .parse::<u64>()
            .ok()
            .and_then(|v| v.checked_add(1))
            .ok_or(DurabilityError::InvalidStoredState)?
            .to_string()
    };
    let (item,transaction,event,occurred):(String,String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let outputs = vec![
        json!({"item":item,"key":output.production_key,"revision":output.revision_ref,"count":1,"ordinal":ordinal}),
    ];
    let request = ItemCraftRequest {
        command,
        source: source_id,
        tool: None,
        ingredient: None,
        station: None,
        intent: b"oteryn:npc.viola/mission".to_vec(),
        gathering: None,
        enchanting: None,
    };
    let envelope = audit::encode(
        fence.item_fence(),
        &request,
        "oteryn:npc-handoff.viola",
        digest,
        &before,
        &outputs,
        &transaction,
        &event,
        occurred,
        tx.tuple().tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,event_source_location) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,'oteryn:npc-handoff.viola',encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric,$11,$12,$13,sha256($14),$15)")
        .bind(&transaction).bind(&event).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(source_id.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).bind(&nested).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid").bind(source_id.as_slice()).bind(&transaction).execute(&mut ***tx).await?;
    if quantity == 1 {
        let remove = if nested.is_some() {
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        } else {
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        };
        sqlx::query(remove)
            .bind(source_id.as_slice())
            .execute(&mut ***tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3572',$3,1,1,$4::uuid)").bind(&item).bind(world.as_bytes().as_slice()).bind(&output.revision_ref).bind(&transaction).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::uuid)").bind(&item).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(&ordinal).bind(&transaction).execute(&mut ***tx).await?;
    let tuple = tx.tuple().tuple();
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)").bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(source_id.as_slice()).bind(occurred).bind(super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(Exchange::Done(transaction))
}
