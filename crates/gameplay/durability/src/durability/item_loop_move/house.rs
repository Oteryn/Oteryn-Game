//! W18: current named-House containers use the existing nested item graph.
//! Crystal data/items/items.xml:65374-65384 gives cupboard capacity8; ordinary
//! Container::queryAdd/queryRemove retains container contents and item lineage.
use super::*;
use crate::durability::consume_transform::HouseActionContext;

/// Native supplementary capability446 is authenticated by the command owner.
#[derive(Clone)]
pub(super) struct HouseLoopContext {
    context: HouseActionContext,
    house_capacity_v2: bool,
}
impl HouseLoopContext {
    fn new(context: HouseActionContext, house_capacity_v2: bool) -> Self {
        Self {
            context,
            house_capacity_v2,
        }
    }
    pub(super) fn capacity_limit(&self) -> u32 {
        if self.house_capacity_v2 { 25 } else { 20 }
    }
}
impl std::ops::Deref for HouseLoopContext {
    type Target = HouseActionContext;
    fn deref(&self) -> &HouseActionContext {
        &self.context
    }
}
#[derive(Clone)]
pub(super) struct HousePath {
    pub root: [u8; 16],
    pub depth: i32,
}

pub(super) async fn current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    context: &HouseActionContext,
) -> LoopResult<bool> {
    if !super::super::runtime_scope_assignment::prove_current_incarnation(tx, node).await? {
        return Ok(false);
    }
    super::super::db::lock_admission_relations(tx).await?;
    let fence = context.fence();
    let valid: bool = sqlx::query_scalar("SELECT game_item_action_house_current(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(node.fact().node_id().as_bytes().as_slice())
        .bind(node.fact().registration_revision().to_string()).bind(context.house().world_id().as_bytes().as_slice())
        .bind(context.house().house_key()).fetch_one(&mut **tx).await?;
    if !valid {
        return Ok(false);
    }
    let origin:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(origin_channel_id) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(origin) = origin else {
        return Ok(false);
    };
    let origin: [u8; 16] = origin
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if !super::super::house_hireling::fence::assert_house_inventory_fence(tx, node, &fence, origin)
        .await?
    {
        return Ok(false);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||encode($1,'hex')::uuid::text||':'||$2,0))")
        .bind(context.house().world_id().as_bytes().as_slice())
        .bind(context.house().house_key()).execute(&mut **tx).await?;
    Ok(true)
}

pub(super) async fn path(
    tx: &mut Transaction<'_, Postgres>,
    context: &HouseActionContext,
    item: [u8; 16],
) -> LoopResult<Option<HousePath>> {
    let row = sqlx::query("SELECT root::text,world::text,house,depth,position FROM game_house_container_root(encode($1,'hex')::uuid)")
        .bind(item.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    if uuid_text(row.try_get("world")?)? != *context.house().world_id().as_bytes()
        || row.try_get::<String, _>("house")? != context.house().house_key()
    {
        return Ok(None);
    }
    let at = HouseActionContext::decode_spatial_position(&row.try_get::<Vec<u8>, _>("position")?)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let actor = context.position();
    if at[2] != actor[2]
        || i64::from(at[0]).abs_diff(i64::from(actor[0])) > 1
        || i64::from(at[1]).abs_diff(i64::from(actor[1])) > 1
    {
        return Ok(None);
    }
    Ok(Some(HousePath {
        root: uuid_text(row.try_get("root")?)?,
        depth: row.try_get("depth")?,
    }))
}

pub(super) async fn prepare<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    context: &HouseLoopContext,
    transaction: [u8; 16],
    content: &C,
    request: &LoopMoveRequest,
) -> LoopResult<()> {
    let source = path(tx, context, request.item).await?;
    let destination = match &request.destination {
        LoopDestination::Container(parent) => path(tx, context, *parent).await?,
        LoopDestination::MainBackpack => None,
        LoopDestination::Ground(_) => return refuse(LoopMoveRefusal::NotSupported),
    };
    let mut capacities = serde_json::Map::new();
    capacities.insert(
        "house_capacity_limit".into(),
        json!(context.capacity_limit()),
    );
    let roots: std::collections::BTreeSet<_> = source
        .iter()
        .chain(destination.iter())
        .map(|path| path.root)
        .collect();
    for root in roots {
        let Some((definition, quantity, _)) =
            lock_live_item(tx, root, *context.house().world_id().as_bytes()).await?
        else {
            return refuse(LoopMoveRefusal::Stale);
        };
        if quantity != 1 {
            return refuse(LoopMoveRefusal::Stale);
        }
        let capacity = content
            .loop_facts(&definition)
            .and_then(|facts| facts.container_capacity)
            .ok_or(LoopError::Refused(LoopMoveRefusal::NotSupported))?;
        if capacity > context.capacity_limit() {
            return refuse(LoopMoveRefusal::NotSupported);
        }
        let key: String = root.iter().map(|byte| format!("{byte:02x}")).collect();
        capacities.insert(key, json!(capacity));
    }
    let fence = context.fence();
    let position: Vec<u8> = context
        .position()
        .iter()
        .flat_map(|number| number.to_be_bytes())
        .collect();
    sqlx::query("SELECT game_house_container_loop_prepare(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5::text::numeric,$6::text::numeric,encode($7,'hex')::uuid,$8::text::numeric,encode($9,'hex')::uuid,$10,$11,encode($12,'hex')::uuid,encode($13,'hex')::uuid,$14,$15)")
        .bind(transaction.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(node.fact().node_id().as_bytes().as_slice())
        .bind(node.fact().registration_revision().to_string()).bind(context.house().world_id().as_bytes().as_slice())
        .bind(context.house().house_key()).bind(position).bind(source.map(|path|path.root.to_vec()))
        .bind(destination.map(|path|path.root.to_vec())).bind(serde_json::Value::Object(capacities))
        .bind(node.sql_possession_secret()).execute(&mut **tx).await?;
    Ok(())
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)] // Existing loop owner plus exact named-House proof.
    pub async fn commit_house_item_loop_move<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
    ) -> Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError> {
        self.commit_house_item_loop_move_with_capacity(
            permit, recovery, node, context, command, content, request, false,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_house_item_loop_move_with_capacity<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        house_capacity_v2: bool,
    ) -> Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError> {
        self.commit_loop_with_depot(
            permit,
            recovery,
            node,
            &context.fence(),
            command,
            content,
            request,
            None,
            Some(HouseLoopContext::new(context.clone(), house_capacity_v2)),
        )
        .await
    }
    /// Same-current-House read; the carried-container API never supplies House custody.
    pub async fn read_house_item_loop_container(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        container: [u8; 16],
    ) -> Result<Option<LoopContainer>, DurabilityError> {
        self.read_house_item_loop_container_with_capacity(recovery, node, context, container, false)
            .await
    }
    pub async fn read_house_item_loop_container_with_capacity(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        container: [u8; 16],
        house_capacity_v2: bool,
    ) -> Result<Option<LoopContainer>, DurabilityError> {
        let capacity_limit = if house_capacity_v2 {
            25
        } else {
            GAMEITEM01_CONTAINER_ENTRIES_MAX
        };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let context = context.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            let read: LoopResult<Option<LoopContainer>> = async {
                assert_recovery_fence(&mut tx,&record).await?;
                if !current(&mut tx,&node,&context).await? { return Ok(None); }
                let Some(path) = path(&mut tx,&context,container).await? else { return Ok(None) };
                if !(0..=ITEM_LOOP_DEPTH_MAX).contains(&path.depth)
                    || lock_live_item(&mut tx,path.root,*context.house().world_id().as_bytes()).await?.is_none() { return Ok(None); }
                let Some((definition,quantity,_)) = lock_live_item(&mut tx,container,*context.house().world_id().as_bytes()).await?
                else { return Ok(None) };
                if quantity != 1 { return Ok(None); }
                let parent: Option<String> = sqlx::query_scalar("SELECT parent_item_instance_id::text FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(container.as_slice()).fetch_optional(&mut *tx).await?;
                let entries = load_nested_entries_with_limit(&mut tx,container,capacity_limit).await?;
                let fluids = super::super::item_transfer::load_fluids(&mut tx,&entries).await?;
                Ok(Some(LoopContainer { container: InventoryItem {item_instance_id:container,definition,quantity},
                    parent: parent.as_deref().map(uuid_text).transpose()?,entries,fluids }))
            }.await;
            let result = match read {
                Ok(value) => value,
                Err(LoopError::Durability(error)) => return Err(error),
                Err(LoopError::Database(error)) => return Err(error.into()),
                Err(LoopError::Refused(_)) => None,
            };
            super::super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}
