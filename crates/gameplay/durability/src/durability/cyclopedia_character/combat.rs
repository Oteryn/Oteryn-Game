//! P12c durable combat observations. Actual combat remains owned by the existing runtime;
//! the snapshot cannot grant a bonus, spend an item or authorize a hit.

use super::super::DurabilityRoot;
use super::super::character_authority::{CharacterAuthorityError, ReconciledCharacterAuthority};
use super::super::character_equipment::{
    EquipmentSnapshot, assert_current_equipment_in_transaction, read_equipment_in_transaction,
};
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use oteryn_protocol_oteryn::cyclopedia_character::combat::{CombatStat, CombatStats};
use oteryn_protocol_oteryn::cyclopedia_character::{CharacterTab, TabView};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatSnapshot {
    pub observed_owner_micros: u64,
    pub equipment: EquipmentSnapshot,
    pub offence: CombatStats,
    pub defence: CombatStats,
    pub misc: CombatStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatSnapshotReceipt {
    pub sample_revision: u64,
    pub replayed: bool,
}

impl CombatSnapshot {
    fn encoded(&self) -> Result<[Vec<u8>; 3], CharacterAuthorityError> {
        if [
            (&self.offence, CharacterTab::Offence),
            (&self.defence, CharacterTab::Defence),
            (&self.misc, CharacterTab::Misc),
        ]
        .into_iter()
        .any(|(value, tab)| {
            value.tab() != tab
                || value.observed_at_unix_ms != self.offence.observed_at_unix_ms
                || value.character_revision != self.equipment.character_revision
                || value.equipment_revision != self.equipment.revision
        }) {
            return Err(CharacterAuthorityError::Rejected);
        }
        let encode = |value: &CombatStats| {
            TabView::Combat(value.clone())
                .encode()
                .map_err(|_| CharacterAuthorityError::Rejected)
        };
        Ok([
            encode(&self.offence)?,
            encode(&self.defence)?,
            encode(&self.misc)?,
        ])
    }
}

fn stored_u64(row: &sqlx::postgres::PgRow, name: &str) -> Result<u64, sqlx::Error> {
    row.try_get::<String, _>(name)?
        .parse()
        .map_err(|_| sqlx::Error::Decode("cyclopedia combat observation number".into()))
}

/// Misc values with an existing durable owner are taken inside the snapshot transaction.
/// Crystal `protocolgame.cpp:5291-5320`: ignore Twist of Fate; list active concoctions.
async fn fill_misc(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    now_ms: i64,
    misc: &mut CombatStats,
) -> Result<(), sqlx::Error> {
    let row = sqlx::query(
        "SELECT (SELECT count(*) FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key <> 'oteryn:blessing.twist_of_fate') AS blessings, \
         COALESCE((SELECT greatest(0,ceil((extract(epoch FROM expires_at)*1000-$2::bigint)/1000))::bigint FROM game_character_concoction_effects \
         WHERE character_id=encode($1,'hex')::uuid AND concoction='wealth_duplex' AND started_at<=to_timestamp($2::double precision/1000) AND expires_at>to_timestamp($2::double precision/1000) \
         ORDER BY started_at DESC LIMIT 1),0) AS wealth_seconds",
    )
    .bind(character.as_slice())
    .bind(now_ms)
    .fetch_one(&mut **tx)
    .await?;
    for (key, value) in [
        (CombatStat::Blessings, row.try_get::<i64, _>("blessings")?),
        (
            CombatStat::MaximumBlessings,
            i64::from(crate::domain::death::MAX_REGULAR_BLESSINGS),
        ),
        (
            CombatStat::WealthDuplexSeconds,
            row.try_get::<i64, _>("wealth_seconds")?,
        ),
    ] {
        misc.set(key, value)
            .map_err(|_| sqlx::Error::Decode("cyclopedia misc stat".into()))?;
    }
    Ok(())
}

impl DurabilityRoot {
    /// Persist one server observation after re-proving the session, lease, content and exact
    /// equipment snapshot in the same transaction. Identical sample retries return their
    /// stored receipt; a changed retry or an older sample cannot overwrite a newer one.
    pub async fn record_cyclopedia_combat(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        mut snapshot: CombatSnapshot,
    ) -> Result<CombatSnapshotReceipt, CharacterAuthorityError> {
        if snapshot.equipment.character != *fence.character_id.as_bytes() {
            return Err(CharacterAuthorityError::Rejected);
        }
        let encoded = snapshot.encoded()?;
        let mut fingerprint = Sha256::new();
        fingerprint.update(b"OTERYN_CYCLOPEDIA_COMBAT/v1");
        fingerprint.update(snapshot.equipment.character);
        fingerprint.update(snapshot.equipment.content_digest);
        use super::super::character_equipment::CombatMode;
        fingerprint.update([match snapshot.equipment.combat_mode {
            None => 0,
            Some(CombatMode::Attack) => 1,
            Some(CombatMode::Balanced) => 2,
            Some(CombatMode::Defense) => 3,
        }]);
        for item in &snapshot.equipment.items {
            fingerprint.update([item.slot, item.tier]);
            fingerprint.update(item.item_instance_id);
            fingerprint.update(item.state_revision.to_le_bytes());
            fingerprint.update(item.quantity.to_le_bytes());
            for field in [
                &item.definition.family,
                &item.definition.production_key,
                &item.definition.revision_ref,
            ] {
                fingerprint.update(
                    u64::try_from(field.len())
                        .map_err(|_| CharacterAuthorityError::Rejected)?
                        .to_le_bytes(),
                );
                fingerprint.update(field.as_bytes());
            }
        }
        for bytes in &encoded {
            fingerprint.update(bytes);
        }
        let fingerprint: [u8; 32] = fingerprint.finalize().into();
        let observed_ms = i64::try_from(snapshot.offence.observed_at_unix_ms)
            .map_err(|_| CharacterAuthorityError::Rejected)?;
        let lease = i64::try_from(fence.character_lease_generation)
            .map_err(|_| CharacterAuthorityError::Rejected)?;
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let current = match assert_current_equipment_in_transaction(
                &mut tx, &recovery, &node, &fence, None, snapshot.equipment.content_digest,
            ).await {
                Ok(current) => current,
                Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)),
            };
            let actual = match read_equipment_in_transaction(&mut tx, &current).await {
                Ok(actual) => actual,
                Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)),
            };
            if actual != snapshot.equipment {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            let before = sqlx::query(
                "SELECT uuid_send(game_session_id) AS session,character_lease_generation,connection_generation::text,observed_owner_micros::text,input_sha256,sample_revision \
                 FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
            ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let mut sample_revision = 1_i64;
            if let Some(before) = before {
                sample_revision = before.try_get::<i64, _>("sample_revision")?;
                let same_session = before.try_get::<Vec<u8>, _>("session")? == fence.game_session_id.as_bytes().as_slice()
                    && before.try_get::<i64, _>("character_lease_generation")? == lease
                    && stored_u64(&before, "connection_generation")? == fence.connection_generation.get();
                if same_session {
                    let previous = stored_u64(&before, "observed_owner_micros")?;
                    if snapshot.observed_owner_micros < previous {
                        return Ok(Err(CharacterAuthorityError::Conflict));
                    }
                    if snapshot.observed_owner_micros == previous {
                        if before.try_get::<Vec<u8>, _>("input_sha256")? != fingerprint.as_slice() {
                            return Ok(Err(CharacterAuthorityError::Conflict));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(CombatSnapshotReceipt {
                            sample_revision: u64::try_from(sample_revision).map_err(|_| sqlx::Error::Decode("cyclopedia sample revision".into()))?,
                            replayed: true,
                        }));
                    }
                }
                let Some(next) = sample_revision.checked_add(1) else {
                    return Ok(Err(CharacterAuthorityError::Rejected));
                };
                sample_revision = next;
            }
            fill_misc(&mut tx, fence.character_id.as_bytes(), observed_ms, &mut snapshot.misc).await?;
            let encoded = match snapshot.encoded() {
                Ok(encoded) => encoded,
                Err(error) => return Ok(Err(error)),
            };
            sqlx::query(
                "INSERT INTO game_character_cyclopedia_combat(character_id,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc) \
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric,$5::text::numeric,$6,$7::text::numeric,$8::text::numeric,$9,$10,$11,$12,$13,$14) \
                 ON CONFLICT(character_id) DO UPDATE SET game_session_id=EXCLUDED.game_session_id,character_lease_generation=EXCLUDED.character_lease_generation,connection_generation=EXCLUDED.connection_generation, \
                 observed_owner_micros=EXCLUDED.observed_owner_micros,observed_at_unix_ms=EXCLUDED.observed_at_unix_ms,character_revision=EXCLUDED.character_revision,equipment_revision=EXCLUDED.equipment_revision, \
                 content_digest=EXCLUDED.content_digest,input_sha256=EXCLUDED.input_sha256,sample_revision=EXCLUDED.sample_revision,offence=EXCLUDED.offence,defence=EXCLUDED.defence,misc=EXCLUDED.misc",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(fence.game_session_id.as_bytes().as_slice()).bind(lease)
            .bind(fence.connection_generation.get().to_string())
            .bind(snapshot.observed_owner_micros.to_string()).bind(observed_ms)
            .bind(snapshot.equipment.character_revision.to_string())
            .bind(snapshot.equipment.revision.to_string())
            .bind(snapshot.equipment.content_digest.as_slice()).bind(fingerprint.as_slice())
            .bind(sample_revision).bind(&encoded[0]).bind(&encoded[1]).bind(&encoded[2])
            .execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CombatSnapshotReceipt {
                sample_revision: u64::try_from(sample_revision).map_err(|_| sqlx::Error::Decode("cyclopedia sample revision".into()))?,
                replayed: false,
            }))
        })).await?
    }
}
