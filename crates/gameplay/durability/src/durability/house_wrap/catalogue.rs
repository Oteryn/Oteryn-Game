//! A bounded projection, never a future mutation grant.
use super::super::item_mint::TypedDefinitionRef;
use super::*;

pub struct HouseFurnitureCandidate {
    pub item: [u8; 16],
    pub definition: String,
    pub revision: u64,
    pub action: oteryn_protocol_oteryn::house_wrap::Action,
    pub target: Option<TypedDefinitionRef>,
}
pub struct HouseFurnitureCatalogue {
    pub acl_revision: u64,
    pub items: Vec<HouseFurnitureCandidate>,
}
impl DurabilityRoot {
    pub async fn house_furniture_catalogue(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
    ) -> Result<Result<HouseFurnitureCatalogue, Refusal>, DurabilityError> {
        if fence.runtime_scope != house.runtime_scope()? {
            return Ok(Err(Refusal::NotInHouse));
        }
        // Crystal src/items/decay/decay.cpp:29-54: an idle House's elapsed
        // timers must advance before its current furniture actions are projected.
        if !self
            .settle_house_timed_decorations(node, fence, house.clone())
            .await?
        {
            return Ok(Err(Refusal::Stale));
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await? { return Ok(Err(Refusal::Stale)); }
            let fact = node.fact();
            let rows = sqlx::query("SELECT uuid_send(item) AS item,definition,revision::text,action,acl::text,target_key,target_revision FROM game_house_furniture_catalogue(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
                .bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(house.world_id().as_bytes().as_slice())
                .bind(house.house_key()).fetch_all(&mut *tx).await;
            let rows = match rows {
                Ok(rows) => rows,
                Err(error) if error.as_database_error().is_some_and(|e|e.message()=="WRAP_NO_ACCESS") => return Ok(Err(Refusal::NoAccess)),
                Err(error) if error.as_database_error().is_some_and(|e|e.message()=="PODIUM_STALE_SESSION") => return Ok(Err(Refusal::Stale)),
                Err(error) => return Err(error.into()),
            };
            if rows.len()>2020 { return Err(DurabilityError::InvalidStoredState); }
            let acl: String = rows.first().ok_or(DurabilityError::InvalidStoredState)?.try_get("acl")?;
            let acl_revision = acl.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
            let mut items = Vec::new();
            for row in rows {
                let Some(item) = row.try_get::<Option<Vec<u8>>,_>("item")? else {continue};
                let revision: String = row.try_get("revision")?;
                let target_key: Option<String> = row.try_get("target_key")?;
                let target = match target_key {
                    Some(production_key) => Some(TypedDefinitionRef {family:"Item".into(),production_key,revision_ref:row.try_get("target_revision")?}),
                    None => None,
                };
                items.push(HouseFurnitureCandidate {item:item.try_into().map_err(|_|DurabilityError::InvalidStoredState)?,definition:row.try_get("definition")?,
                    revision:revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?,
                    action:match row.try_get::<i16,_>("action")? {1=>oteryn_protocol_oteryn::house_wrap::Action::Wrap,2=>oteryn_protocol_oteryn::house_wrap::Action::Unwrap,_=>return Err(DurabilityError::InvalidStoredState)},target});
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(HouseFurnitureCatalogue {acl_revision,items}))
        })).await
    }
}
