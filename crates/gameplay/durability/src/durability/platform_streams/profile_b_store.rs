//! Profile-B account-before-product fences. B1 and B2 share the same deciding account lock.
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::profile_b_wire::{Answer, CommercialState, Hint, Item, Message, Request, time};
use sqlx::Row;
type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
#[derive(Debug, Clone, Default)]
pub struct View {
    pub answer: Option<Answer>,
    pub hints: Vec<Hint>,
}
#[derive(Debug, Clone)]
pub enum Acceptance {
    Accepted(View),
    Ignored(View),
    Conflict(View),
}
async fn account(tx: &mut Tx<'_>, id: &str) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_profile_b_accounts(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "SELECT account_id FROM game_profile_b_accounts WHERE account_id=$1::uuid FOR UPDATE",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(())
}
fn life(e: &Item) -> Result<Vec<u8>, DurabilityError> {
    // ACTIVE/NOT_YET_EFFECTIVE/EXPIRED are projections of the same stored interval;
    // elapsed-time transitions are checked separately, as in the accepted Premium fence.
    serde_json::to_vec(&(
        &e.entitlement_id,
        e.entitlement_state == CommercialState::Revoked,
        &e.effective_from,
        &e.effective_until,
    ))
    .map_err(|_| DurabilityError::InvalidStoredState)
}
async fn view(tx: &mut Tx<'_>, id: &str) -> Result<View, DurabilityError> {
    let raw: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT answer_bytes FROM game_profile_b_accounts WHERE account_id=$1::uuid FOR SHARE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .flatten();
    let answer = match raw {
        Some(raw) => match Message::parse(&raw) {
            Some(Message::Answer(a)) => Some(a),
            _ => return Err(DurabilityError::InvalidStoredState),
        },
        None => None,
    };
    let mut hints = Vec::new();
    if let Some(a) = &answer {
        for e in &a.entitlements {
            let revision:Option<i64>=sqlx::query_scalar("SELECT hint_revision FROM game_profile_b_products WHERE account_id=$1::uuid AND product_id=$2 AND product_version=$3").bind(id).bind(&e.product_id).bind(e.product_version as i64).fetch_optional(&mut **tx).await?;
            if let Some(revision) = revision.filter(|r| *r > 0) {
                hints.push(Hint {
                    account_id: id.into(),
                    product_id: e.product_id.clone(),
                    product_version: e.product_version,
                    lifecycle_revision: revision as u64,
                });
            }
        }
    }
    Ok(View { answer, hints })
}
impl DurabilityRoot {
    pub async fn load_profile_b(&self, id: &str) -> Result<View, DurabilityError> {
        if !super::enforcement_wire::uuid7(id) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let id = id.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = view(&mut tx, &id).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
    pub async fn accept_profile_b_answer(
        &self,
        request: &Request,
        products: &[(String, u64)],
        raw: &[u8],
    ) -> Result<Acceptance, DurabilityError> {
        let Some(Message::Answer(answer)) = Message::parse(raw) else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if !answer.bound_to(request, products) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let raw = raw.to_vec();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;account(&mut tx,&answer.account_id).await?;
            let high:i64=sqlx::query_scalar("SELECT authority_revision FROM game_profile_b_accounts WHERE account_id=$1::uuid").bind(&answer.account_id).fetch_one(&mut *tx).await?;
            if answer.authority_revision<=high as u64 {let v=view(&mut tx,&answer.account_id).await?;commit_semantic_transaction(tx,deadline).await?;return Ok(Acceptance::Ignored(v));}
            let previous=view(&mut tx,&answer.account_id).await?;
            let mut conflicting=false;
            for e in &answer.entitlements {
                let row=sqlx::query("SELECT lifecycle_revision,lifecycle_bytes,hint_revision FROM game_profile_b_products WHERE account_id=$1::uuid AND product_id=$2 AND product_version=$3").bind(&answer.account_id).bind(&e.product_id).bind(e.product_version as i64).fetch_optional(&mut *tx).await?;
                if let Some(row)=row {
                    let old:i64=row.try_get("lifecycle_revision")?;let hint:i64=row.try_get("hint_revision")?;let bytes:Option<Vec<u8>>=row.try_get("lifecycle_bytes")?;
                    if let Some(bytes)=&bytes {
                        let (old_id,_,_,_):(Option<String>,bool,Option<String>,Option<String>)=serde_json::from_slice(bytes).map_err(|_|DurabilityError::InvalidStoredState)?;
                        conflicting |= old_id.is_some() && old_id!=e.entitlement_id;
                    }
                    conflicting |= e.lifecycle_revision<(old.max(hint) as u64) || e.lifecycle_revision==old as u64 && bytes.is_some_and(|b|life(e).is_ok_and(|new|b!=new));
                }
                if let Some(old)=previous.answer.as_ref().and_then(|a|a.entitlements.iter().find(|p|p.product_id==e.product_id&&p.product_version==e.product_version))
                    && old.lifecycle_revision==e.lifecycle_revision {
                    let rank=|s|match s {CommercialState::NotYetEffective=>0,CommercialState::Active=>1,_=>2};
                    conflicting |= rank(e.entitlement_state)<rank(old.entitlement_state)
                        || time(&answer.authority_issued_at)<previous.answer.as_ref().and_then(|a|time(&a.authority_issued_at));
                }
            }
            if conflicting {commit_semantic_transaction(tx,deadline).await?;return Ok(Acceptance::Conflict(previous));}
            for e in &answer.entitlements {
                sqlx::query("INSERT INTO game_profile_b_products(account_id,product_id,product_version,lifecycle_revision,lifecycle_bytes) VALUES($1::uuid,$2,$3,$4,$5) ON CONFLICT(account_id,product_id,product_version) DO UPDATE SET lifecycle_revision=EXCLUDED.lifecycle_revision,lifecycle_bytes=EXCLUDED.lifecycle_bytes")
                    .bind(&answer.account_id).bind(&e.product_id).bind(e.product_version as i64).bind(e.lifecycle_revision as i64).bind(life(e)?).execute(&mut *tx).await?;
            }
            sqlx::query("UPDATE game_profile_b_accounts SET authority_revision=$2,answer_bytes=$3 WHERE account_id=$1::uuid").bind(&answer.account_id).bind(answer.authority_revision as i64).bind(raw).execute(&mut *tx).await?;
            let result=view(&mut tx,&answer.account_id).await?;commit_semantic_transaction(tx,deadline).await?;Ok(Acceptance::Accepted(result))
        })).await
    }
    /// A shared cursor may be advanced by another node: live consumers also read captured floors.
    pub async fn profile_b_known_hints(
        &self,
        accounts: &[String],
        products: &[(String, u64)],
    ) -> Result<Vec<Hint>, DurabilityError> {
        if accounts.len() > 500
            || products.len() > 17
            || accounts
                .iter()
                .any(|id| !super::enforcement_wire::uuid7(id))
            || products.iter().any(|(p, v)| {
                !super::profile_b_wire::product(p)
                    || !(1..=super::profile_b_wire::INT53_MAX).contains(v)
            })
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let accounts = sqlx::types::Json(accounts.to_vec());
        let products=sqlx::types::Json(products.iter().map(|(product_id,product_version)|serde_json::json!({"product_id":product_id,"product_version":product_version})).collect::<Vec<_>>());
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let rows=sqlx::query("SELECT p.account_id::text,p.product_id,p.product_version,p.hint_revision FROM jsonb_array_elements_text($1::jsonb) a(account_id) CROSS JOIN jsonb_to_recordset($2::jsonb) AS k(product_id TEXT,product_version BIGINT) JOIN game_profile_b_products p ON p.account_id=a.account_id::uuid AND p.product_id=k.product_id AND p.product_version=k.product_version WHERE p.hint_revision>0 ORDER BY p.account_id,p.product_id,p.product_version LIMIT 8500")
                .bind(accounts).bind(products).fetch_all(&mut *tx).await?;
            let result=rows.into_iter().map(|r|Ok::<_,sqlx::Error>(Hint{account_id:r.try_get("account_id")?,product_id:r.try_get("product_id")?,product_version:r.try_get::<i64,_>("product_version")? as u64,lifecycle_revision:r.try_get::<i64,_>("hint_revision")? as u64})).collect::<Result<Vec<_>,_>>()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }
    pub async fn profile_b_hint_revision(
        &self,
        id: &str,
        product: &str,
        version: u64,
    ) -> Result<u64, DurabilityError> {
        if !super::enforcement_wire::uuid7(id)
            || !super::profile_b_wire::product(product)
            || !(1..=super::profile_b_wire::INT53_MAX).contains(&version)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let (id, product) = (id.to_owned(), product.to_owned());
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let revision:Option<i64>=sqlx::query_scalar("SELECT hint_revision FROM game_profile_b_products WHERE account_id=$1::uuid AND product_id=$2 AND product_version=$3").bind(id).bind(product).bind(version as i64).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(revision.unwrap_or(0) as u64)
        })).await
    }
    pub async fn profile_b_changes_cursor(
        &self,
    ) -> Result<(Option<String>, bool), DurabilityError> {
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let row:(Option<String>,bool)=sqlx::query_as("SELECT stream_cursor,stopped FROM game_profile_b_changes_cursor WHERE singleton").fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(row)
        })).await
    }
    pub async fn stop_profile_b_changes(&self) -> Result<(), DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    sqlx::query(
                        "UPDATE game_profile_b_changes_cursor SET stopped=true WHERE singleton",
                    )
                    .execute(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(())
                })
            })
            .await
    }
    pub async fn apply_profile_b_changes(
        &self,
        expected: Option<&str>,
        raw: &[u8],
    ) -> Result<bool, DurabilityError> {
        let Some(Message::Page(page)) = Message::parse(raw) else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let expected = expected.map(str::to_owned);
        if let Some(next) = &page.next_cursor {
            let position = |s: &str| s.strip_prefix("v1:").and_then(|n| n.parse::<i64>().ok());
            if expected
                .as_deref()
                .is_some_and(|old| position(old) >= position(next))
            {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let current:(Option<String>,bool)=sqlx::query_as("SELECT stream_cursor,stopped FROM game_profile_b_changes_cursor WHERE singleton FOR UPDATE").fetch_one(&mut *tx).await?;
            if current.0!=expected || current.1 {commit_semantic_transaction(tx,deadline).await?;return Ok(false);}
            if !page.items.is_empty() {
                let rows=sqlx::types::Json(&page.items);
                sqlx::query("INSERT INTO game_profile_b_accounts(account_id) SELECT DISTINCT account_id FROM jsonb_to_recordset($1::jsonb) AS h(account_id UUID) ORDER BY account_id ON CONFLICT DO NOTHING").bind(rows).execute(&mut *tx).await?;
                sqlx::query("SELECT a.account_id FROM game_profile_b_accounts a JOIN (SELECT DISTINCT account_id FROM jsonb_to_recordset($1::jsonb) AS h(account_id UUID)) h USING(account_id) ORDER BY a.account_id FOR UPDATE OF a").bind(rows).fetch_all(&mut *tx).await?;
                // Hint effects commute (maximum revision only); this preserves every reduction
                // while avoiding hundreds of round trips in the deciding transaction.
                sqlx::query("INSERT INTO game_profile_b_products(account_id,product_id,product_version,hint_revision) SELECT account_id,product_id,product_version,max(lifecycle_revision) FROM jsonb_to_recordset($1::jsonb) AS h(account_id UUID,product_id TEXT,product_version BIGINT,lifecycle_revision BIGINT) GROUP BY account_id,product_id,product_version ORDER BY account_id,product_id,product_version ON CONFLICT(account_id,product_id,product_version) DO UPDATE SET hint_revision=greatest(game_profile_b_products.hint_revision,EXCLUDED.hint_revision)").bind(rows).execute(&mut *tx).await?;
            }
            sqlx::query("UPDATE game_profile_b_changes_cursor SET stream_cursor=coalesce($1,stream_cursor) WHERE singleton").bind(page.next_cursor).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(true)
        })).await
    }
}
