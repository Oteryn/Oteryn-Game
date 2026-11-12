//! Durable cross-worker consent revocations. A withdrawal commits only after
//! every live media worker applied its prefix or its short media lease expired.
//! Preference truth stays in `cast_consent`; this module can only deny delivery.
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_mint_audit::check_uuid_v7;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use oteryn_gameplay_domain::domain::CharacterId;
use std::time::{Duration, Instant};

pub struct CastConsentWorker {
    instance: [u8; 16],
    cursor: i64,
    deadline: Instant,
}
impl CastConsentWorker {
    pub fn version(&self) -> i64 {
        self.cursor
    }
    pub fn valid_until(&self) -> Instant {
        self.deadline
    }
    pub fn valid(&self) -> bool {
        Instant::now() < self.deadline
    }
}
pub struct CastConsentInvalidation {
    pub character: CharacterId,
    pub minimum_revision: i64,
}
pub struct CastConsentBatch {
    instance: [u8; 16],
    from: i64,
    through: i64,
    events: Vec<CastConsentInvalidation>,
}
impl CastConsentBatch {
    pub fn events(&self) -> &[CastConsentInvalidation] {
        &self.events
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
impl DurabilityRoot {
    // Refresh and ACK share the worker's existing media lease. Keep a FIFO
    // ticket while its holder is busy, without extending that lease or retrying
    // storage/authority failures. An expired worker retains only the original
    // immediate recovery attempt; its caller has already purged live media.
    async fn cast_privacy_worker_pass(
        &self,
        until: Instant,
    ) -> Result<super::db::IssuedSemanticPass, DurabilityError> {
        if until <= Instant::now() {
            return self.try_issue_semantic_pass();
        }
        let mut waiter = None;
        loop {
            match self.try_issue_semantic_pass_by(until) {
                Ok(pass) => return Ok(pass),
                Err(DurabilityError::RootUnavailable) if Instant::now() < until => {
                    if waiter.is_none() {
                        waiter = self.holder_waiter();
                    }
                    let now = tokio::time::Instant::now();
                    let latest = (now + Duration::from_millis(50))
                        .min(tokio::time::Instant::from_std(until));
                    if !self.wait_holder_turn(now, latest).await {
                        tokio::time::sleep_until(latest).await;
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Called exactly once on a new empty worker, before accepting any operation.
    /// Root reads deny pending floors, so a new worker needs no historic media replay.
    pub async fn register_cast_consent_worker(
        &self,
        node: &NodeIncarnationProof,
        instance: [u8; 16],
    ) -> Result<CastConsentWorker, DurabilityError> {
        check_uuid_v7(&instance).map_err(|_| DurabilityError::InvalidStoredState)?;
        let started = Instant::now();
        let node = node.clone();
        let cursor=self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            current(&mut tx,&node).await?;
            let cursor=control(&mut tx).await?;
            let inserted=sqlx::query("INSERT INTO game_cast_consent_workers(instance,node_id,registration_revision,applied_seq,expires_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+2000) ON CONFLICT DO NOTHING")
                .bind(instance.as_slice()).bind(node.fact().node_id().as_bytes().as_slice())
                .bind(node.fact().registration_revision().to_string()).bind(cursor).execute(&mut *tx).await?;
            if inserted.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
            commit_semantic_transaction(tx,deadline).await?;
            Ok(cursor)
        })).await?;
        // Conservative local deadline starts BEFORE IO, one second earlier than
        // the DB lease. A busy/disconnected worker loses media authority first.
        Ok(CastConsentWorker {
            instance,
            cursor,
            deadline: started + Duration::from_secs(1),
        })
    }
    pub async fn refresh_cast_consent_worker(
        &self,
        node: &NodeIncarnationProof,
        worker: &mut CastConsentWorker,
    ) -> Result<CastConsentBatch, DurabilityError> {
        self.refresh_cast_consent_worker_with_wait(node, worker, true)
            .await
    }

    /// The cast service must keep polling its pending observer command's existing
    /// rejection boundary while that command's caller may hold the busy holder.
    /// Try this same refresh without waiting; media still requires the current lease.
    pub async fn try_refresh_cast_consent_worker(
        &self,
        node: &NodeIncarnationProof,
        worker: &mut CastConsentWorker,
    ) -> Result<CastConsentBatch, DurabilityError> {
        self.refresh_cast_consent_worker_with_wait(node, worker, false)
            .await
    }

    async fn refresh_cast_consent_worker_with_wait(
        &self,
        node: &NodeIncarnationProof,
        worker: &mut CastConsentWorker,
        wait_for_holder: bool,
    ) -> Result<CastConsentBatch, DurabilityError> {
        let started = Instant::now();
        let pass = if wait_for_holder {
            self.cast_privacy_worker_pass(worker.valid_until()).await?
        } else if worker.valid() {
            self.try_issue_semantic_pass_by(worker.valid_until())?
        } else {
            self.try_issue_semantic_pass()?
        };
        let instance = worker.instance;
        let from = worker.cursor;
        let node = node.clone();
        let (through,events,renewed)=pass.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            current(&mut tx,&node).await?;
            let latest=control(&mut tx).await?;
            assert_worker(&mut tx,&node,instance,from).await?;
            let rows:Vec<(i64,Vec<u8>,i64)>=sqlx::query_as("SELECT seq,uuid_send(character_id),minimum_revision FROM game_cast_consent_invalidations WHERE seq>$1 ORDER BY seq LIMIT 128")
                .bind(from).fetch_all(&mut *tx).await?;
            let mut through=from; let mut events=Vec::new();
            for (seq,id,revision) in rows {
                if through.checked_add(1)!=Some(seq) || seq>latest || revision<1 {return Err(DurabilityError::InvalidStoredState);}
                let id:[u8;16]=id.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
                let character=CharacterId::from_bytes(id).map_err(|_|DurabilityError::InvalidStoredState)?;
                events.push(CastConsentInvalidation{character,minimum_revision:revision});through=seq;
            }
            if through==from && through!=latest {return Err(DurabilityError::InvalidStoredState);}
            let renewed=from==latest;
            if renewed {renew(&mut tx,instance).await?;}
            commit_semantic_transaction(tx,deadline).await?;
            Ok((through,events,renewed))
        })).await?;
        if renewed {
            worker.deadline = started + Duration::from_secs(1);
        }
        Ok(CastConsentBatch {
            instance,
            from,
            through,
            events,
        })
    }
    /// Only acknowledge the opaque batch AFTER applying every invalidation under
    /// the same owner that performs the final nonblocking media writes.
    pub async fn acknowledge_cast_consent(
        &self,
        node: &NodeIncarnationProof,
        worker: &mut CastConsentWorker,
        batch: CastConsentBatch,
    ) -> Result<(), DurabilityError> {
        if batch.instance != worker.instance
            || batch.from != worker.cursor
            || batch.through < batch.from
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let started = Instant::now();
        let node = node.clone();
        let instance = worker.instance;
        let from = batch.from;
        let through = batch.through;
        let renewed=self.cast_privacy_worker_pass(worker.valid_until()).await?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            current(&mut tx,&node).await?;
            let latest=control(&mut tx).await?;
            assert_worker(&mut tx,&node,instance,from).await?;
            if through>latest {return Err(DurabilityError::InvalidStoredState);}
            sqlx::query("UPDATE game_cast_consent_workers SET applied_seq=$2 WHERE instance=encode($1,'hex')::uuid")
                .bind(instance.as_slice()).bind(through).execute(&mut *tx).await?;
            let renewed=through==latest;
            if renewed {renew(&mut tx,instance).await?;}
            commit_semantic_transaction(tx,deadline).await?;
            Ok(renewed)
        })).await?;
        worker.cursor = through;
        if renewed {
            worker.deadline = started + Duration::from_secs(1);
        }
        Ok(())
    }
    pub async fn close_cast_consent_worker(
        &self,
        node: &NodeIncarnationProof,
        worker: CastConsentWorker,
    ) -> Result<(), DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            current(&mut tx,&node).await?;
            assert_worker(&mut tx,&node,worker.instance,worker.cursor).await?;
            sqlx::query("UPDATE game_cast_consent_workers SET expires_at_ms=0 WHERE instance=encode($1,'hex')::uuid")
                .bind(worker.instance.as_slice()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(())
        })).await
    }
}
async fn current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
) -> Result<(), DurabilityError> {
    if !prove_current_incarnation(tx, node).await? {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}
pub(super) async fn control(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i64, DurabilityError> {
    let seq: i64 = sqlx::query_scalar(
        "SELECT last_seq FROM game_cast_consent_control WHERE singleton=TRUE FOR UPDATE",
    )
    .fetch_one(&mut **tx)
    .await?;
    if seq < 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(seq)
}
async fn assert_worker(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    instance: [u8; 16],
    cursor: i64,
) -> Result<(), DurabilityError> {
    let row:Option<(Vec<u8>,String,i64)>=sqlx::query_as("SELECT uuid_send(node_id),registration_revision::text,applied_seq FROM game_cast_consent_workers WHERE instance=encode($1,'hex')::uuid FOR UPDATE")
        .bind(instance.as_slice()).fetch_optional(&mut **tx).await?;
    if !matches!(row,Some((id,revision,seq)) if id.as_slice()==node.fact().node_id().as_bytes() && revision==node.fact().registration_revision().to_string() && seq==cursor)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
async fn renew(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    instance: [u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query("UPDATE game_cast_consent_workers SET expires_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+2000 WHERE instance=encode($1,'hex')::uuid")
        .bind(instance.as_slice()).execute(&mut **tx).await?;
    Ok(())
}
pub(super) async fn prepare(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    minimum: i64,
) -> Result<i64, DurabilityError> {
    let last = control(tx).await?;
    let old:Option<(i64,i64)>=sqlx::query_as("SELECT minimum_revision,seq FROM game_cast_consent_floors WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if let Some((revision, seq)) = old
        && revision >= minimum
    {
        return Ok(seq);
    }
    let seq = last
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_cast_consent_invalidations(seq,character_id,minimum_revision) VALUES($1,encode($2,'hex')::uuid,$3)")
        .bind(seq).bind(character.as_bytes().as_slice()).bind(minimum).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_cast_consent_floors(character_id,minimum_revision,seq) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT(character_id) DO UPDATE SET minimum_revision=EXCLUDED.minimum_revision,seq=EXCLUDED.seq")
        .bind(character.as_bytes().as_slice()).bind(minimum).bind(seq).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_cast_consent_control SET last_seq=$1 WHERE singleton=TRUE")
        .bind(seq)
        .execute(&mut **tx)
        .await?;
    Ok(seq)
}
pub(super) async fn all_applied(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seq: i64,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_cast_consent_workers WHERE applied_seq<$1 AND expires_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
        .bind(seq).fetch_one(&mut **tx).await?)
}
