use oteryn_admin_contracts::{ErrorCode as E, Request, Response, signed_bytes};
use sqlx::{PgConnection, PgPool, Row};
use std::future::Future;

pub trait NonceStore {
    /// Atomic claim, including expired-row replacement. False is a replay.
    fn claim_nonce(
        &mut self,
        nonce: &str,
        now: i64,
    ) -> impl Future<Output = Result<bool, E>> + Send;
}
pub trait IdempotencyStore {
    /// Rejections have no retained idempotent result, but remain audit evidence.
    fn audit(
        &mut self,
        request: &Request,
        digest: &str,
        response: &Response,
        now: i64,
    ) -> impl Future<Output = Result<(), E>> + Send;
    fn lookup(
        &mut self,
        operation_id: &str,
    ) -> impl Future<Output = Result<Option<(String, Response)>, E>> + Send;
    /// Atomically retains the result and ledger; concurrent callers receive the winner.
    fn finish(
        &mut self,
        request: &Request,
        digest: &str,
        response: &Response,
        now: i64,
    ) -> impl Future<Output = Result<Response, E>> + Send;
}
#[derive(Clone)]
pub struct PostgresStore {
    pub pool: PgPool,
}
impl NonceStore for PostgresStore {
    async fn claim_nonce(&mut self, nonce: &str, now: i64) -> Result<bool, E> {
        let row = sqlx::query("INSERT INTO admin_request_nonces(nonce,expires_at) VALUES($1,to_timestamp($2::double precision)+interval '24 hours') ON CONFLICT(nonce) DO UPDATE SET expires_at=EXCLUDED.expires_at WHERE admin_request_nonces.expires_at<=to_timestamp($2::double precision) RETURNING nonce")
            .bind(nonce).bind(now as f64).fetch_optional(&self.pool).await.map_err(|_| E::AdmTargetUnavailable)?;
        Ok(row.is_some())
    }
}
impl IdempotencyStore for PostgresStore {
    async fn audit(
        &mut self,
        request: &Request,
        digest: &str,
        response: &Response,
        now: i64,
    ) -> Result<(), E> {
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|_| E::AdmTargetUnavailable)?;
        append_ledger(&mut connection, request, digest, response, now).await
    }
    async fn lookup(&mut self, operation_id: &str) -> Result<Option<(String, Response)>, E> {
        let row = sqlx::query(
            "SELECT params_digest,result FROM admin_idempotency WHERE admin_operation_id=$1::uuid",
        )
        .bind(operation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| E::AdmTargetUnavailable)?;
        row.map(|r| {
            let digest = r.try_get("params_digest").map_err(|_| E::AdmInternal)?;
            let result: serde_json::Value = r.try_get("result").map_err(|_| E::AdmInternal)?;
            Ok((
                digest,
                serde_json::from_value(result).map_err(|_| E::AdmInternal)?,
            ))
        })
        .transpose()
    }
    async fn finish(
        &mut self,
        request: &Request,
        digest: &str,
        response: &Response,
        now: i64,
    ) -> Result<Response, E> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| E::AdmTargetUnavailable)?;
        let result = serde_json::to_value(response).map_err(|_| E::AdmInternal)?;
        let inserted = sqlx::query("INSERT INTO admin_idempotency(admin_operation_id,params_digest,result) VALUES($1::uuid,$2,$3) ON CONFLICT DO NOTHING RETURNING admin_operation_id")
            .bind(&request.admin_operation_id).bind(digest).bind(result).fetch_optional(&mut *tx).await.map_err(|_| E::AdmInternal)?;
        if inserted.is_none() {
            let row = sqlx::query("SELECT params_digest,result FROM admin_idempotency WHERE admin_operation_id=$1::uuid").bind(&request.admin_operation_id).fetch_one(&mut *tx).await.map_err(|_| E::AdmInternal)?;
            let prior: String = row.try_get("params_digest").map_err(|_| E::AdmInternal)?;
            if prior != digest {
                return Err(E::AdmIdempotencyConflict);
            }
            let value: serde_json::Value = row.try_get("result").map_err(|_| E::AdmInternal)?;
            return serde_json::from_value(value).map_err(|_| E::AdmInternal);
        }
        append_ledger(&mut tx, request, digest, response, now).await?;
        tx.commit().await.map_err(|_| E::AdmInternal)?;
        Ok(response.clone())
    }
}
/// Caller-owned transaction: future DB effects must use this same connection.
pub async fn append_ledger(
    tx: &mut PgConnection,
    r: &Request,
    digest: &str,
    response: &Response,
    now: i64,
) -> Result<(), E> {
    let ledger_id = response.game_ledger_id.as_deref().ok_or(E::AdmInternal)?;
    let target = serde_json::to_value(&r.target).map_err(|_| E::AdmInternal)?;
    let risk = serde_json::to_value(r.risk_class).map_err(|_| E::AdmInternal)?;
    let status = serde_json::to_value(response.status).map_err(|_| E::AdmInternal)?;
    let error = response
        .error
        .as_ref()
        .map(|e| serde_json::to_value(e.code))
        .transpose()
        .map_err(|_| E::AdmInternal)?;
    sqlx::query("INSERT INTO admin_command_ledger(ledger_id,request_id,admin_operation_id,operation,operation_version,params_digest,canonical_request,signer_key_id,admin_id,risk_class,target,status,error_code,effects,started_at,finished_at,agent_id) VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'[]'::jsonb,to_timestamp($14::double precision),to_timestamp($14::double precision),$15)")
        .bind(ledger_id).bind(&r.request_id).bind(&r.admin_operation_id).bind(&r.operation).bind(i32::try_from(r.operation_version).map_err(|_| E::AdmInternal)?)
        .bind(digest).bind(signed_bytes(r).map_err(|_| E::AdmInternal)?).bind(&r.signer_key_id).bind(&r.actor.admin_id).bind(risk.as_str().ok_or(E::AdmInternal)?)
        .bind(target).bind(status.as_str().ok_or(E::AdmInternal)?).bind(error.as_ref().and_then(|v| v.as_str())).bind(now as f64).bind(&response.agent_id)
        .execute(tx).await.map_err(|_| E::AdmInternal)?;
    Ok(())
}
