//! Game-owned catalogue binding and DisplayedLook capture. Neither accepts Platform data.
use crate::durability::DurabilityError;
use sha2::{Digest, Sha256};

fn catalogue() -> Result<(String, serde_json::Value), DurabilityError> {
    use oteryn_gameplay_progression::achievement_catalogue::{SHARDS, parse_shards};
    let catalogue = parse_shards(&SHARDS).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut hash = Sha256::new();
    for shard in SHARDS { hash.update(shard.as_bytes()); }
    use std::fmt::Write;
    let mut revision = String::from("sha256:");
    for byte in hash.finalize() {
        write!(revision,"{byte:02x}").map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    let entries: serde_json::Map<String, serde_json::Value> = catalogue.into_iter()
        .map(|(key, entry)| (key, serde_json::json!({"revision":entry.revision, "retired":entry.retired}))).collect();
    Ok((revision, serde_json::Value::Object(entries)))
}

pub(super) async fn assert_achievement_catalogue(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), DurabilityError> {
    let expected = catalogue()?;
    let actual: Option<(String, serde_json::Value)> = sqlx::query_as("SELECT revision,entries FROM game_public_v2_achievement_catalogue WHERE singleton FOR SHARE")
        .fetch_optional(&mut **tx).await?;
    if actual.as_ref() != Some(&expected) { return Err(DurabilityError::Unavailable); }
    Ok(())
}

impl super::DurabilityRoot {
    /// Refresh from Game's own validated embedded catalogue, never caller-provided keys.
    pub async fn sync_public_achievement_catalogue(
        &self, authority: &super::ReconciledCharacterAuthority<'_, '_>,
    ) -> super::Result<()> {
        if self.sync_public_achievement_catalogue_page(authority).await? { Ok(()) }
        else { Err(super::CharacterAuthorityError::from(DurabilityError::Unavailable)) }
    }
    pub(super) async fn sync_public_achievement_catalogue_page(
        &self, authority: &super::ReconciledCharacterAuthority<'_, '_>,
    ) -> super::Result<bool> {
        let recovery = authority.record_for(self)?;
        let (revision, entries) = catalogue()?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::begin_semantic_transaction(holder, deadline).await?;
            super::assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("SELECT 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE")
                .fetch_one(&mut *tx).await?;
            let actual:Option<(String,serde_json::Value)>=sqlx::query_as("SELECT revision,entries FROM game_public_v2_achievement_catalogue WHERE singleton").fetch_optional(&mut *tx).await?;
            if actual.as_ref().is_none_or(|(r,e)| r!=&revision || e!=&entries) {
                sqlx::query("INSERT INTO game_public_v2_achievement_catalogue(singleton,revision,entries) VALUES(true,$1,$2) ON CONFLICT(singleton) DO UPDATE SET revision=EXCLUDED.revision,entries=EXCLUDED.entries WHERE (game_public_v2_achievement_catalogue.revision,game_public_v2_achievement_catalogue.entries) IS DISTINCT FROM (EXCLUDED.revision,EXCLUDED.entries)").bind(revision).bind(entries).execute(&mut *tx).await?;
            }
            // Catalogue updates leave coverage incomplete until every bounded page
            // (including retained suppressions) has been captured under this revision.
            let complete: bool = sqlx::query_scalar("SELECT game_public_v2_catalogue_refresh_page()")
                .fetch_one(&mut *tx).await?;
            super::commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(complete))
        })).await?
    }
}

/// Called only after the appearance owner validates active content and earned unlocks.
/// The proof and selection commit together under the existing complete Character fence.
pub(crate) async fn capture_displayed_look<K: AsRef<str>>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8; 16],
    occurrence: &[u8; 16],
    shown: crate::domain::appearance::DisplayedLook<'_, K>,
) -> Result<(), DurabilityError> {
    let affected: i64 = sqlx::query_scalar("SELECT game_public_v2_write_outfit_proof(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
        .bind(character.as_slice()).bind(occurrence.as_slice()).bind(shown.outfit_key.as_ref())
        .bind(shown.colours.map(i32::from).to_vec()).bind(i16::from(shown.addons))
        .fetch_one(&mut **tx).await?;
    if affected != 1 { return Err(DurabilityError::InvalidStoredState); }
    Ok(())
}
