use super::*;

#[derive(Debug, Clone)]
pub struct HouseManagementSummary {
    pub house: String,
    pub state: u8,
    pub rent: u64,
    pub revision: u64,
    pub acl_revision: u64,
    pub owned: bool,
    pub move_out_ms: Option<u64>,
    pub price: u64,
    pub ends_ms: Option<u64>,
    pub own_maximum: Option<u64>,
}
impl DurabilityRoot {
    /// Bounded keyset read. Keys come from the server catalogue for the requested town;
    /// actor-private notices are returned only to the current owner.
    pub async fn read_house_management(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        houses: Vec<String>,
        cursor: Option<String>,
        content_digest: [u8; 32],
    ) -> Result<Vec<HouseManagementSummary>> {
        if houses.len() > 1024
            || houses
                .iter()
                .any(|key| HouseId::new(fence.runtime_scope.world_id(), key).is_none())
            || cursor.as_ref().is_some_and(|key| !houses.contains(key))
        {
            return Err(HouseManagementError::Rejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseManagementError::Rejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(Err(HouseManagementError::Rejected));
                    }
                    let rows = sqlx::query(PAGE_SQL)
                        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                        .bind(&houses)
                        .bind(cursor)
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(content_digest.as_slice())
                        .fetch_all(&mut *tx)
                        .await?;
                    let rows = rows
                        .iter()
                        .map(summary)
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    Ok(Ok(rows))
                })
            })
            .await?
    }
}
fn summary(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<HouseManagementSummary, DurabilityError> {
    let unsigned = |name| -> std::result::Result<u64, DurabilityError> {
        u64::try_from(row.try_get::<i64, _>(name)?).map_err(|_| DurabilityError::InvalidStoredState)
    };
    let optional = |name| -> std::result::Result<Option<u64>, DurabilityError> {
        row.try_get::<Option<i64>, _>(name)?
            .map(u64::try_from)
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(HouseManagementSummary {
        house: row.try_get("house_key")?,
        state: u8::try_from(row.try_get::<i16, _>("state")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        rent: unsigned("rent_gold")?,
        revision: numeric_u64(row, "revision")?,
        acl_revision: numeric_u64(row, "acl_revision")?,
        owned: row.try_get::<Option<bool>, _>("owned")?.unwrap_or(false),
        move_out_ms: optional("due")?,
        price: unsigned("price")?,
        ends_ms: optional("ends")?,
        own_maximum: optional("own_maximum")?,
    })
}
const PAGE_SQL: &str = r#"
SELECT p.house_key,p.state,p.rent_gold,p.revision::text,p.acl_revision::text,
 (SELECT CASE count(*) WHEN 0 THEN 0 WHEN 1 THEN 0 ELSE least(max(maximum),min(maximum)+1) END FROM (SELECT maximum FROM game_house_bids b WHERE b.auction_id=a.auction_id AND b.state=1 ORDER BY maximum DESC,reached_order LIMIT 2) bids) AS price,(extract(epoch FROM a.ends_at)*1000)::bigint AS ends,
 (SELECT maximum FROM game_house_bids b WHERE b.auction_id=a.auction_id AND b.character_id=encode($4,'hex')::uuid AND b.state=1 LIMIT 1) AS own_maximum,p.owner_character_id=encode($4,'hex')::uuid AS owned,CASE WHEN p.owner_character_id=encode($4,'hex')::uuid AND n.ownership_revision=p.ownership_generation THEN n.due_at_unix_ms END AS due
 FROM game_house_properties p
 LEFT JOIN game_house_move_out_notices n USING(world_id,house_key)
 LEFT JOIN game_house_auctions a ON a.world_id=p.world_id AND a.house_key=p.house_key AND NOT a.settled
 WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=ANY($2) AND p.house_key>coalesce($3,'') AND p.content_digest=$5
 ORDER BY p.house_key LIMIT 21
"#;
