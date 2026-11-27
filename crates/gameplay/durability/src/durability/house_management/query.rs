use super::*;

#[derive(Debug, Clone)]
pub struct HouseManagementSummary {
    pub house: String,
    pub kind: String,
    pub state: u8,
    pub rent: u64,
    pub revision: u64,
    pub acl_revision: u64,
    pub owned: bool,
    pub move_out_ms: Option<u64>,
    pub price: u64,
    pub ends_ms: Option<u64>,
    pub own_maximum: Option<u64>,
    pub auction_sort_end: i64,
}
#[derive(Clone)]
pub struct HouseManagementPage {
    pub houses: Vec<QualifiedHouse>,
    pub cursor: Option<String>,
    pub auction_order: bool,
}

impl DurabilityRoot {
    /// Bounded keyset read. Keys come from the server catalogue for the requested town;
    /// actor-private notices are returned only to the current owner.
    pub async fn read_house_management(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        page: HouseManagementPage,
        content_digest: [u8; 32],
    ) -> Result<Vec<HouseManagementSummary>> {
        let HouseManagementPage {
            houses,
            cursor,
            auction_order,
        } = page;
        let (cursor, cursor_end) = match (auction_order, cursor.as_deref()) {
            (true, Some(cursor)) => {
                let (end, key) = cursor
                    .split_once('/')
                    .ok_or(HouseManagementError::Rejected)?;
                let end: i64 = end.parse().map_err(|_| HouseManagementError::Rejected)?;
                if end < 0 {
                    return Err(HouseManagementError::Rejected);
                }
                (Some(key), end)
            }
            (_, cursor) => (cursor, i64::MAX),
        };
        let cursor = cursor.map(str::to_owned);
        if houses.len() > 1024
            || houses.iter().any(|house| {
                HouseId::new(fence.runtime_scope.world_id(), &house.key).is_none()
                    || !matches!(house.kind, "private_house" | "shop" | "guildhall")
            })
            || cursor
                .as_ref()
                .is_some_and(|key| !houses.iter().any(|house| &house.key == key))
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
                    let keys: Vec<_> = houses.iter().map(|house| house.key.as_str()).collect();
                    let kinds: Vec<_> = houses.iter().map(|house| house.kind).collect();
                    let rows = sqlx::query(PAGE_SQL)
                        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                        .bind(&keys)
                        .bind(cursor)
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(content_digest.as_slice())
                        .bind(&kinds)
                        .bind(auction_order)
                        .bind(cursor_end)
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
        kind: row.try_get("kind")?,
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
        auction_sort_end: row.try_get("sort_end")?,
    })
}
const PAGE_SQL: &str = r#"
WITH eligible AS (
 SELECT p.*, a.auction_id, a.ends_at,
  CASE WHEN $7 AND p.state=2 AND a.ends_at > statement_timestamp()
       THEN (extract(epoch FROM a.ends_at)*1000)::bigint
       ELSE 9223372036854775807::bigint END AS sort_end
 FROM game_house_properties p
 LEFT JOIN game_house_auctions a ON a.world_id=p.world_id AND a.house_key=p.house_key AND NOT a.settled
 WHERE p.world_id=encode($1,'hex')::uuid AND (p.house_key,p.kind) IN
  (SELECT * FROM unnest($2::text[],$6::text[]))
  AND p.content_digest=$5
), page AS (
 SELECT * FROM eligible
 WHERE $3::text IS NULL
    OR (NOT $7 AND house_key>$3)
    OR ($7 AND house_key<>$3 AND (sort_end,house_key) > ($8::bigint,$3))
 ORDER BY sort_end,house_key LIMIT 21
)
SELECT p.house_key,p.kind,p.state,p.rent_gold,p.revision::text,p.acl_revision::text,p.sort_end,
 (SELECT CASE count(*) WHEN 0 THEN 0 WHEN 1 THEN 0 ELSE least(max(maximum),min(maximum)+1) END FROM (SELECT maximum FROM game_house_bids b WHERE b.auction_id=p.auction_id AND b.state=1 ORDER BY maximum DESC,reached_order LIMIT 2) bids) AS price,(extract(epoch FROM p.ends_at)*1000)::bigint AS ends,
 (SELECT maximum FROM game_house_bids b WHERE b.auction_id=p.auction_id AND (b.guild_id IS NULL AND b.character_id=encode($4,'hex')::uuid OR b.guild_id=(SELECT guild_id FROM game_guild_members WHERE character_id=encode($4,'hex')::uuid AND rank_level=1)) AND b.state=1 LIMIT 1) AS own_maximum,game_house_effective_owner(p.world_id,p.house_key)=encode($4,'hex')::uuid AS owned,CASE WHEN game_house_effective_owner(p.world_id,p.house_key)=encode($4,'hex')::uuid AND n.ownership_revision=p.ownership_generation THEN n.due_at_unix_ms END AS due
 FROM page p
 LEFT JOIN game_house_move_out_notices n USING(world_id,house_key)
 ORDER BY p.sort_end,p.house_key
"#;
