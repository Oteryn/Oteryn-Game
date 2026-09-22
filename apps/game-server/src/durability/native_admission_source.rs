//! Durable non-rollback state for the authenticated native-admission source.

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;

type Result<T> = std::result::Result<T, DurabilityError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshStoreProvenance {
    pub namespace: String,
    pub authorization: String,
    pub initialized_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorRegistration {
    pub revision: u64,
    pub facts: Vec<u8>,
    pub installed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObservation {
    pub source_authority: String,
    pub operation: String,
    pub semantic_namespace: String,
    pub source_revision: u64,
    pub decision_identity: String,
    pub observed_at: i64,
    pub semantic_facts: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPublication {
    pub slot_id: i16,
    pub operation_binding: Vec<u8>,
    pub checkpointed_at: i64,
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
}
fn valid_descriptor(value: &DescriptorRegistration) -> bool {
    value.revision > 0 && !value.facts.is_empty() && value.installed_at >= 0
}
fn valid_observation(value: &SourceObservation) -> bool {
    valid_text(&value.source_authority)
        && valid_text(&value.operation)
        && valid_text(&value.semantic_namespace)
        && value.source_revision > 0
        && valid_text(&value.decision_identity)
        && value.observed_at >= 0
        && !value.semantic_facts.is_empty()
}

impl DurabilityRoot {
    pub async fn initialize_native_admission_source(
        &self,
        provenance: FreshStoreProvenance,
        descriptor: DescriptorRegistration,
    ) -> Result<()> {
        if !valid_text(&provenance.namespace)
            || !valid_text(&provenance.authorization)
            || provenance.initialized_at < 0
            || !valid_descriptor(&descriptor)
        {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let inserted = sqlx::query("INSERT INTO game_durability_native_source_registration (registration_id,bootstrap_namespace,bootstrap_provenance,descriptor_revision,descriptor_facts,initialized_at) VALUES (1,$1,$2,$3::text::numeric(20,0),$4,$5) ON CONFLICT DO NOTHING")
                .bind(&provenance.namespace).bind(&provenance.authorization).bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(provenance.initialized_at).execute(&mut *tx).await?.rows_affected();
            if inserted == 0 { return Err(DurabilityError::Unavailable); }
            sqlx::query("INSERT INTO game_durability_native_source_descriptor_history VALUES (1,$1::text::numeric(20,0),$2,$3)")
                .bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_durability_native_source_publication_slots (registration_id,slot_id) VALUES (1,1),(1,2)").execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn register_native_admission_descriptor(
        &self,
        descriptor: DescriptorRegistration,
    ) -> Result<()> {
        if !valid_descriptor(&descriptor) {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query("SELECT r.descriptor_revision::text,r.descriptor_facts,h.installed_at FROM game_durability_native_source_registration r JOIN game_durability_native_source_descriptor_history h USING (registration_id,descriptor_revision) WHERE r.registration_id=1 FOR UPDATE OF r").fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            let current: u64 = row.try_get::<String,_>(0).map_err(|_| DurabilityError::InvalidStoredState)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            let facts: Vec<u8> = row.try_get(1).map_err(|_| DurabilityError::InvalidStoredState)?;
            let installed_at: i64 = row.try_get(2).map_err(|_| DurabilityError::InvalidStoredState)?;
            if descriptor.revision < current || (descriptor.revision == current && (descriptor.facts != facts || descriptor.installed_at != installed_at)) { return Err(DurabilityError::Unavailable); }
            if descriptor.revision == current { return commit_semantic_transaction(tx, deadline).await; }
            sqlx::query("INSERT INTO game_durability_native_source_descriptor_history VALUES (1,$1::text::numeric(20,0),$2,$3)").bind(descriptor.revision.to_string()).bind(&descriptor.facts).bind(descriptor.installed_at).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_durability_native_source_registration SET descriptor_revision=$1::text::numeric(20,0),descriptor_facts=$2 WHERE registration_id=1").bind(descriptor.revision.to_string()).bind(&descriptor.facts).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn accept_native_source_observation(
        &self,
        observation: SourceObservation,
    ) -> Result<()> {
        if !valid_observation(&observation) {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query("SELECT source_revision::text,operation,decision_identity,observed_at,semantic_facts FROM game_durability_native_source_floors WHERE registration_id=1 AND source_authority=$1 AND semantic_namespace=$2 FOR UPDATE")
                .bind(&observation.source_authority).bind(&observation.semantic_namespace).fetch_optional(&mut *tx).await?;
            if let Some(row) = row {
                let revision: u64 = row.try_get::<String,_>(0).map_err(|_| DurabilityError::InvalidStoredState)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                let exact = row.try_get::<String,_>(1).ok().as_deref() == Some(observation.operation.as_str())
                    && row.try_get::<String,_>(2).ok().as_deref() == Some(observation.decision_identity.as_str())
                    && row.try_get::<i64,_>(3).ok() == Some(observation.observed_at)
                    && row.try_get::<Vec<u8>,_>(4).ok().as_deref() == Some(observation.semantic_facts.as_slice());
                if observation.source_revision < revision || (observation.source_revision == revision && !exact) { return Err(DurabilityError::Unavailable); }
                if observation.source_revision == revision { return commit_semantic_transaction(tx, deadline).await; }
                sqlx::query("INSERT INTO game_durability_native_source_observation_history VALUES (1,$1,$2,$3,$4::text::numeric(20,0),$5,$6,$7)").bind(&observation.source_authority).bind(&observation.operation).bind(&observation.semantic_namespace).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_durability_native_source_floors SET operation=$3,source_revision=$4::text::numeric(20,0),decision_identity=$5,observed_at=$6,semantic_facts=$7 WHERE registration_id=1 AND source_authority=$1 AND semantic_namespace=$2").bind(&observation.source_authority).bind(&observation.semantic_namespace).bind(&observation.operation).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO game_durability_native_source_observation_history VALUES (1,$1,$2,$3,$4::text::numeric(20,0),$5,$6,$7)").bind(&observation.source_authority).bind(&observation.operation).bind(&observation.semantic_namespace).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_durability_native_source_floors VALUES (1,$1,$2,$3,$4::text::numeric(20,0),$5,$6,$7)").bind(&observation.source_authority).bind(&observation.operation).bind(&observation.semantic_namespace).bind(observation.source_revision.to_string()).bind(&observation.decision_identity).bind(observation.observed_at).bind(&observation.semantic_facts).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn checkpoint_native_source_publication(
        &self,
        operation_binding: Vec<u8>,
        checkpointed_at: i64,
    ) -> Result<i16> {
        if operation_binding.is_empty() || checkpointed_at < 0 {
            return Err(DurabilityError::Unavailable);
        }
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            sqlx::query("SELECT registration_id FROM game_durability_native_source_registration WHERE registration_id=1 FOR UPDATE").fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            if let Some(slot) = sqlx::query_scalar::<_,i16>("SELECT slot_id FROM game_durability_native_source_publication_slots WHERE registration_id=1 AND operation_binding=$1").bind(&operation_binding).fetch_optional(&mut *tx).await? { commit_semantic_transaction(tx, deadline).await?; return Ok(slot); }
            let slot = sqlx::query_scalar::<_,i16>("SELECT slot_id FROM game_durability_native_source_publication_slots WHERE registration_id=1 AND operation_binding IS NULL ORDER BY slot_id FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=$1,checkpointed_at=$2 WHERE registration_id=1 AND slot_id=$3 AND operation_binding IS NULL").bind(&operation_binding).bind(checkpointed_at).bind(slot).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?; Ok(slot)
        })).await
    }

    pub async fn clear_native_source_publication(
        &self,
        slot_id: i16,
        operation_binding: Vec<u8>,
    ) -> Result<()> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let changed = sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=NULL,checkpointed_at=NULL WHERE registration_id=1 AND slot_id=$1 AND operation_binding=$2").bind(slot_id).bind(operation_binding).execute(&mut *tx).await?.rows_affected();
            if changed != 1 { return Err(DurabilityError::Unavailable); }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    pub async fn pending_native_source_publications(&self) -> Result<Vec<PendingPublication>> {
        self.try_issue_semantic_pass()?.run(|holder, _| Box::pin(async move {
            sqlx::query_scalar::<_, i16>("SELECT registration_id FROM game_durability_native_source_registration WHERE registration_id=1")
                .fetch_optional(&mut **holder).await?.ok_or(DurabilityError::Unavailable)?;
            let rows = sqlx::query("SELECT slot_id,operation_binding,checkpointed_at FROM game_durability_native_source_publication_slots WHERE registration_id=1 AND operation_binding IS NOT NULL ORDER BY slot_id").fetch_all(&mut **holder).await?;
            rows.into_iter().map(|row| Ok(PendingPublication { slot_id: row.try_get(0).map_err(|_| DurabilityError::InvalidStoredState)?, operation_binding: row.try_get(1).map_err(|_| DurabilityError::InvalidStoredState)?, checkpointed_at: row.try_get(2).map_err(|_| DurabilityError::InvalidStoredState)? })).collect()
        })).await
    }
}
