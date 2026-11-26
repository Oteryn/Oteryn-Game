//! Native entry Content activation issuances (`NATIVE_ENTRY_CONTENT_ACTIVATION_V1`, #935).
//!
//! The control plane records one immutable issuance per Channel scope and monotonic sequence;
//! the newest row of a scope is its current activation and floor. The node reads only that row.

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, WorldId};
use serde::Deserialize;
use sqlx::Row;
use std::sync::OnceLock;

type Result<T> = std::result::Result<T, DurabilityError>;

/// One control-plane activation request for a Channel scope. `previous_sequence` is the scope's
/// current sequence the issuer observed, or `None` for the scope's first activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentActivationRequest {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub activation_sequence: u64,
    pub previous_sequence: Option<u64>,
    pub server_artifact_digest: [u8; 32],
    pub client_artifact_digest: [u8; 32],
    pub frame_binding_digest: [u8; 32],
}

/// The scope's current issuance as recorded. The node binds it to the Channel scope's WorldId
/// before activation; this record carries no Content bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentActivationRecord {
    pub activation_sequence: u64,
    pub server_artifact_digest: [u8; 32],
    pub client_artifact_digest: [u8; 32],
    pub frame_binding_digest: [u8; 32],
}

#[derive(Deserialize)]
struct HouseCatalogue {
    houses: Vec<AuthoredHouse>,
}

#[derive(Deserialize)]
struct AuthoredHouse {
    identity: HouseIdentity,
    kind: String,
    rent_gold: i64,
    #[serde(default)]
    retired: bool,
}

#[derive(Deserialize)]
struct HouseIdentity {
    key: String,
}

/// The shipped content catalogue also supplies the keys and kinds for HOUSE QUERY.
fn authored_houses() -> Result<&'static [AuthoredHouse]> {
    static HOUSES: OnceLock<Option<Vec<AuthoredHouse>>> = OnceLock::new();
    HOUSES
        .get_or_init(|| {
            let mut houses = Vec::new();
            for shard in [
                include_str!("../../../../../content/houses/houses-00000-00499.json"),
                include_str!("../../../../../content/houses/houses-00500-00994.json"),
            ] {
                let catalogue: HouseCatalogue = serde_json::from_str(shard).ok()?;
                houses.extend(catalogue.houses.into_iter().filter(|house| !house.retired));
            }
            Some(houses)
        })
        .as_deref()
        .ok_or(DurabilityError::Unavailable)
}

impl DurabilityRoot {
    /// Control-plane: durably record one activation issuance. An exact replay succeeds; a
    /// conflicting replay, stale predecessor, non-newer sequence or missing exact-scope grant
    /// is a definitive refusal, `Ok(false)`. The current issuance registers its authored houses
    /// in the same transaction.
    pub async fn record_content_activation(
        &self,
        request: &ContentActivationRequest,
    ) -> Result<bool> {
        if request.activation_sequence == 0
            || request.previous_sequence == Some(0)
            || request
                .previous_sequence
                .is_some_and(|previous| previous >= request.activation_sequence)
        {
            return Err(DurabilityError::Unavailable);
        }
        let houses = authored_houses()?;
        let keys: Vec<_> = houses.iter().map(|house| house.identity.key.as_str()).collect();
        let kinds: Vec<_> = houses.iter().map(|house| house.kind.as_str()).collect();
        let rents: Vec<_> = houses.iter().map(|house| house.rent_gold).collect();
        let request = request.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let recorded = sqlx::query(
                        "SELECT game_content_record_activation_with_houses(encode($1::bytea, 'hex')::uuid, \
                         encode($2::bytea, 'hex')::uuid, $3::text::numeric(20,0), \
                         $4::text::numeric(20,0), $5, $6, $7, $8, $9, $10)",
                    )
                    .bind(request.world_id.as_bytes().as_slice())
                    .bind(request.channel_id.as_bytes().as_slice())
                    .bind(request.activation_sequence.to_string())
                    .bind(request.previous_sequence.map(|value| value.to_string()))
                    .bind(request.server_artifact_digest.as_slice())
                    .bind(request.client_artifact_digest.as_slice())
                    .bind(request.frame_binding_digest.as_slice())
                    .bind(&keys)
                    .bind(&kinds)
                    .bind(&rents)
                    .execute(&mut *tx)
                    .await;
                    match recorded {
                        Ok(_) => {}
                        Err(sqlx::Error::Database(error))
                            if matches!(error.code().as_deref(), Some("OTC01" | "42501")) =>
                        {
                            return Ok(false);
                        }
                        Err(error) => return Err(error.into()),
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(true)
                })
            })
            .await
    }

    /// The scope's current (newest) activation issuance, or `None` when none was issued.
    pub async fn read_current_content_activation(
        &self,
        world_id: WorldId,
        channel_id: ChannelId,
    ) -> Result<Option<ContentActivationRecord>> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let row = sqlx::query(
                        "SELECT activation_sequence::text, server_artifact_digest, \
                         client_artifact_digest, frame_binding_digest \
                         FROM game_content_activations \
                         WHERE world_id = encode($1::bytea, 'hex')::uuid \
                           AND channel_id = encode($2::bytea, 'hex')::uuid \
                         ORDER BY activation_sequence DESC LIMIT 1",
                    )
                    .bind(world_id.as_bytes().as_slice())
                    .bind(channel_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    let Some(row) = row else { return Ok(None) };
                    let invalid = |_| DurabilityError::InvalidStoredState;
                    let digest = |index: usize| -> Result<[u8; 32]> {
                        row.try_get::<Vec<u8>, _>(index)
                            .map_err(invalid)?
                            .try_into()
                            .map_err(|_| DurabilityError::InvalidStoredState)
                    };
                    Ok(Some(ContentActivationRecord {
                        activation_sequence: row
                            .try_get::<String, _>(0)
                            .map_err(invalid)?
                            .parse()
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        server_artifact_digest: digest(1)?,
                        client_artifact_digest: digest(2)?,
                        frame_binding_digest: digest(3)?,
                    }))
                })
            })
            .await
    }

    /// Control-plane: activate `revision` of a `ProficiencyShaping` definition and retain
    /// exactly `retained` plus `revision` (PROFICIENCY-1B §8). The activation takes the key's
    /// exclusive retention lock, which every modification write holds shared until commit; a
    /// revision some modification row still references is never dropped, and the refusal,
    /// `Ok(false)`, changes nothing. A missing grant is also `Ok(false)`.
    pub async fn record_proficiency_shaping_activation(
        &self,
        shaping_key: &str,
        revision: &str,
        retained: &[String],
    ) -> Result<bool> {
        let (shaping_key, revision, retained) = (
            shaping_key.to_owned(),
            revision.to_owned(),
            retained.to_vec(),
        );
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let recorded =
                        sqlx::query("SELECT game_proficiency_shaping_activate($1, $2, $3)")
                            .bind(&shaping_key)
                            .bind(&revision)
                            .bind(&retained)
                            .execute(&mut *tx)
                            .await;
                    match recorded {
                        Ok(_) => {}
                        Err(sqlx::Error::Database(error))
                            if matches!(error.code().as_deref(), Some("OTC01" | "42501")) =>
                        {
                            return Ok(false);
                        }
                        Err(error) => return Err(error.into()),
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(true)
                })
            })
            .await
    }
}
