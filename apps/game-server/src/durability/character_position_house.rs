//! HOUSE-RUNTIME-1c native last-position storage component.
//!
//! The optional House belongs to the existing 0080/0084 row, with its cell, vitals and order
//! key. It is historical projection data, not an entry/exit or physical-placement permission.
//! Capture, admission placement and atomic reverse-handoff callers remain separate owners.
//! The uninstalled schema proposal must ship with ALL native position writers changed to set
//! the House fields; the old Channel upsert otherwise retains an existing House discriminator.

use super::admission_authority_guards::AdmissionGuardStore;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{
    LastPositionKey, LastPositionRecord, LastPositionWrite, LastVitals, StoredLastPosition,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_session_fence, numeric_u64,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::RuntimeScopeRefV1;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1 as GuardKey, AdmissionAuthorityGuardStateV1 as GuardState,
};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type StorageResult<T> = std::result::Result<T, DurabilityError>;

/// One capture supplied by the actual position owner, using the existing native record.
/// `house` is expected binding only; the writer independently resolves the current named House.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousePositionRecord {
    pub position: LastPositionRecord,
    pub house: Option<HouseId>,
}

/// The same historical row and native order key. Reading it grants no placement authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredHousePosition {
    pub position: StoredLastPosition,
    pub house: Option<HouseId>,
}

/// Private implementing proposal, applied ONLY in isolated qualification databases.
/// No migration installer, grants or runtime activation are provided here.
pub const PROPOSED_SCHEMA: &str = "\
ALTER TABLE game_character_last_positions \
 ADD COLUMN house_world_id UUID NULL, \
 ADD COLUMN house_key TEXT NULL, \
 ADD CONSTRAINT game_character_last_positions_house CHECK (\
  (house_world_id IS NULL AND house_key IS NULL) OR \
  (house_world_id IS NOT NULL AND house_key IS NOT NULL \
   AND house_world_id=world_id AND game_house_scope_key_valid(house_key)));";

/// SYNTHETIC INDEPENDENT GUARD DATA for isolated PostgreSQL STORAGE fixtures only.
/// The sealed lifecycle/publication predicate is NOT admitted by this helper. Installing a claimed projection here
/// does not prove an admission/claim transition or create a production publisher capability.
/// Native ordinary publication correctly forbids bootstrapping or changing holder/presence.
/// Preserve its canonical encoder, SQL mirrors/history and relation locks without weakening it.
#[cfg(test)]
pub async fn install_synthetic_independent_guard_data(
    root: &DurabilityRoot,
    change: &crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationChangeV1,
) -> StorageResult<()> {
    let owner = root.clone();
    let change = change.clone();
    root.try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                lock_admission_relations(&mut tx).await?;
                let store = AdmissionGuardStore::from_root(owner);
                let current = store.load_locked(&mut tx, &change.key).await?;
                let encoded = super::admission_authority_guards::encode_guard(
                    &change,
                    super::MAX_ADMISSION_GUARD_BYTES,
                )?;
                if !store
                    .successor_history_available(
                        &mut tx,
                        std::slice::from_ref(&change),
                        std::slice::from_ref(&current),
                    )
                    .await?
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
                store
                    .persist_locked(&mut tx, &[change], &[current], &[encoded])
                    .await?;
                commit_semantic_transaction(tx, deadline).await
            })
        })
        .await
}

impl DurabilityRoot {
    /// Store one living-owner capture under the actual current rule-2 fence, outside the
    /// CharacterRevision chain. An ordinary Channel capture explicitly clears both House fields.
    /// A generic Instance cannot write: the current session must independently name its House.
    pub async fn write_character_position_house(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        write_sequence: u64,
        record: HousePositionRecord,
    ) -> Result<LastPositionWrite> {
        if write_sequence == 0 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let root = self.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let outcome =
                        write_in_transaction(&root, &mut tx, &node, fence, write_sequence, &record)
                            .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcome)
                })
            })
            .await?
    }

    /// Read the existing row under the independently current Channel/named-House session.
    /// Saved House/cell/markers remain historical data for the actual admission owner to check.
    pub async fn read_character_position_house(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<StoredHousePosition>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let root = self.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = current_house(&root, &mut tx, &node, fence).await? {
                        return Ok(Err(error));
                    }
                    let row = read_in_transaction(&mut tx, fence.character_id).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(row))
                })
            })
            .await?
    }
}

/// Same-transaction storage hook for the owning native terminal/admission/handoff callers.
/// The caller must assert its recovery fence and take native admission relation locks first.
/// This hook independently rechecks session/assignment/incarnation/guards; it does not authorize
/// an exit or select a tile. Supply the actual destination capture and destination lease fence.
pub(super) async fn write_in_transaction(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    write_sequence: u64,
    record: &HousePositionRecord,
) -> StorageResult<Result<LastPositionWrite>> {
    if write_sequence == 0 || fence.runtime_scope.world_id() != record.position.world_id {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    let current = match current_house(root, tx, node, fence).await? {
        Ok(house) => house,
        Err(error) => return Ok(Err(error)),
    };
    if current != record.house {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let key = LastPositionKey {
        character_lease_generation: fence.character_lease_generation,
        write_sequence,
    };
    upsert(tx, fence.character_id, key, record).await.map(Ok)
}

/// Resolve current facts independently of the capture/stored row. This is the House-specific
/// counterpart of the native Channel-only assert_session_fence, not a change to that fence.
async fn current_house(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
) -> StorageResult<Result<Option<HouseId>>> {
    if fence.character_lease_generation == 0 {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = fence.runtime_scope
    else {
        return assert_session_fence(tx, &fence.item_fence(), node)
            .await
            .map(|r| r.map(|_| None));
    };
    let row = sqlx::query(
        "SELECT account_id::text AS account, runtime_scope_house_key AS house \
         FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid \
          AND character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid \
          AND runtime_scope_kind=2 AND runtime_scope_world_id=encode($3,'hex')::uuid \
          AND runtime_scope_channel_id IS NULL \
          AND runtime_scope_instance_id=encode($4,'hex')::uuid \
          AND runtime_scope_house_key IS NOT NULL AND origin_channel_id IS NOT NULL \
          AND current_generation=$5::text::numeric(20,0) \
          AND character_lease_generation=$6::text::numeric(20,0) \
          AND scope_ownership_generation=$7::text::numeric(20,0) \
          AND session_state IN(1,2) FOR SHARE",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(instance_id.as_slice())
    .bind(fence.connection_generation.get().to_string())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    };
    let account: String = row.try_get("account")?;
    let key: String = row.try_get("house")?;
    let house = HouseId::new(world_id, &key).ok_or(DurabilityError::InvalidStoredState)?;
    if house.runtime_scope()? != fence.runtime_scope {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let fact = node.fact();
    let assigned = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key=$1 AND scope_kind=2 AND world_id=encode($2,'hex')::uuid \
          AND channel_id IS NULL AND house_key=$3 AND state=1 \
          AND ownership_generation=$4::text::numeric(20,0) \
          AND holder_node_id=encode($5,'hex')::uuid \
          AND holder_registration_revision=$6::text::numeric(20,0) FOR SHARE",
    )
    .bind(house.scope_key())
    .bind(world_id.as_bytes().as_slice())
    .bind(house.house_key())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    if assigned.is_none() || !prove_current_incarnation(tx, node).await? {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    // The native guard loader validates actual projection mirrors AND immutable publication
    // history. A row-derived matching claim or the saved position cannot supply these facts.
    let guarded_character = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let guards = AdmissionGuardStore::from_root(root.clone());
    let character = guards
        .load_locked(tx, &GuardKey::Character(guarded_character))
        .await?;
    let character_ok = matches!(character.as_ref().map(|v| &v.state),
        Some(GuardState::Character {account_id, world_id: w, eligible: true, lease_generation, holder: Some(s)})
        if *account_id==account && *w==world_id && *lease_generation==fence.character_lease_generation && *s==fence.game_session_id);
    let presence = guards
        .load_locked(
            tx,
            &GuardKey::Account {
                account_id: account,
            },
        )
        .await?;
    let presence_ok = matches!(presence.as_ref().map(|v| &v.state),
        Some(GuardState::Account {presence: Some((c,s)), ..}) if *c==guarded_character && *s==fence.game_session_id);
    let runtime = guards
        .load_locked(tx, &GuardKey::Runtime(fence.runtime_scope))
        .await?;
    let runtime_ok = matches!(runtime.as_ref().map(|v| &v.state),
        Some(GuardState::Runtime {ownership_generation, ready: true, ..}) if *ownership_generation==fence.scope_ownership_generation.get());
    if !(character_ok && presence_ok && runtime_ok) {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    Ok(Ok(Some(house)))
}

async fn upsert(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    key: LastPositionKey,
    record: &HousePositionRecord,
) -> StorageResult<LastPositionWrite> {
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns \
         WHERE character_id=encode($1,'hex')::uuid)",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Ok(LastPositionWrite::PendingObligation);
    }
    let p = &record.position;
    let (health, mana) = match p.vitals {
        Some(v) => (
            Some(i32::try_from(v.health()).map_err(|_| DurabilityError::InvalidStoredState)?),
            Some(i32::try_from(v.mana()).map_err(|_| DurabilityError::InvalidStoredState)?),
        ),
        None => (None, None),
    };
    let written = sqlx::query(
        "INSERT INTO game_character_last_positions AS p \
         (character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest, \
          map_revision_digest,server_artifact_digest,character_lease_generation, \
          write_sequence,health,mana,house_world_id,house_key) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8, \
          $9::text::numeric(20,0),$10::text::numeric(20,0),$11,$12, \
          CASE WHEN $13::bytea IS NULL THEN NULL ELSE encode($13,'hex')::uuid END,$14) \
         ON CONFLICT(character_id) DO UPDATE SET world_id=EXCLUDED.world_id, \
          tile_x=EXCLUDED.tile_x,tile_y=EXCLUDED.tile_y,tile_floor=EXCLUDED.tile_floor, \
          coordinate_frame_digest=EXCLUDED.coordinate_frame_digest, \
          map_revision_digest=EXCLUDED.map_revision_digest,server_artifact_digest=EXCLUDED.server_artifact_digest, \
          character_lease_generation=EXCLUDED.character_lease_generation,write_sequence=EXCLUDED.write_sequence, \
          health=EXCLUDED.health,mana=EXCLUDED.mana,house_world_id=EXCLUDED.house_world_id,house_key=EXCLUDED.house_key \
         WHERE(p.character_lease_generation,p.write_sequence)<(EXCLUDED.character_lease_generation,EXCLUDED.write_sequence)",
    )
    .bind(character.as_bytes().as_slice())
    .bind(p.world_id.as_bytes().as_slice())
    .bind(p.cell.x).bind(p.cell.y).bind(p.cell.floor)
    .bind(p.coordinate_frame_digest.as_slice()).bind(p.map_revision_digest.as_slice())
    .bind(p.server_artifact_digest.as_slice()).bind(key.character_lease_generation.to_string())
    .bind(key.write_sequence.to_string()).bind(health).bind(mana)
    .bind(record.house.as_ref().map(|h| h.world_id().as_bytes().to_vec()))
    .bind(record.house.as_ref().map(HouseId::house_key))
    .execute(&mut **tx).await?;
    Ok(if written.rows_affected() == 1 {
        LastPositionWrite::Written
    } else {
        LastPositionWrite::Superseded
    })
}

async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
) -> StorageResult<Option<StoredHousePosition>> {
    let row = sqlx::query(
        "SELECT uuid_send(world_id) AS world,tile_x,tile_y,tile_floor,coordinate_frame_digest, \
         map_revision_digest,server_artifact_digest,character_lease_generation::text, \
         write_sequence::text,health,mana,uuid_send(house_world_id) AS house_world,house_key \
         FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    row.as_ref().map(decode).transpose()
}

fn decode(row: &sqlx::postgres::PgRow) -> StorageResult<StoredHousePosition> {
    let world: Vec<u8> = row.try_get("world")?;
    let world = crate::foundation::WorldId::decode(&world)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let house = match (
        row.try_get::<Option<Vec<u8>>, _>("house_world")?,
        row.try_get::<Option<String>, _>("house_key")?,
    ) {
        (None, None) => None,
        (Some(w), Some(k)) => {
            let w = crate::foundation::WorldId::decode(&w)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if w != world {
                return Err(DurabilityError::InvalidStoredState);
            }
            Some(HouseId::new(w, &k).ok_or(DurabilityError::InvalidStoredState)?)
        }
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let vitals = match (
        row.try_get::<Option<i32>, _>("health")?,
        row.try_get::<Option<i32>, _>("mana")?,
    ) {
        (None, None) => None,
        (Some(h), Some(m)) => Some(
            LastVitals::new(
                u32::try_from(h).map_err(|_| DurabilityError::InvalidStoredState)?,
                u32::try_from(m).map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?,
        ),
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let digest = |name| -> StorageResult<[u8; 32]> {
        row.try_get::<Vec<u8>, _>(name)?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let key = LastPositionKey {
        character_lease_generation: numeric_u64(row, "character_lease_generation")?,
        write_sequence: numeric_u64(row, "write_sequence")?,
    };
    if key.character_lease_generation == 0 || key.write_sequence == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(StoredHousePosition {
        position: StoredLastPosition {
            record: LastPositionRecord {
                world_id: world,
                cell: super::character_last_position::LastPositionCell {
                    x: row.try_get("tile_x")?,
                    y: row.try_get("tile_y")?,
                    floor: row.try_get("tile_floor")?,
                },
                coordinate_frame_digest: digest("coordinate_frame_digest")?,
                map_revision_digest: digest("map_revision_digest")?,
                server_artifact_digest: digest("server_artifact_digest")?,
                vitals,
            },
            key,
        },
        house,
    })
}
