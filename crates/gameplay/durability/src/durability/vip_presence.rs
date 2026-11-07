//! VIP0 section 8: current consent, privacy and World-session truth in one fenced read.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::CharacterId;
use oteryn_protocol_oteryn::vip_presence::{MAX_ENTRIES, Status};
use sqlx::Row;

/// Server-side routing evidence. The target UUID never reaches the presence wire.
pub struct VipPresenceRow {
    pub target: CharacterId,
    pub entry: u64,
    pub name: String,
    pub notify: bool,
    pub visible: bool,
    pub status: Status,
}

impl DurabilityRoot {
    /// Re-read the list and its statuses together. The watcher's exact current session,
    /// recovery, node, lease and scope fences precede all disclosure, including an empty list.
    pub async fn read_current_vip_presence(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Vec<VipPresenceRow>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let Ok(account) = assert_session_fence(&mut tx, &fence, &node).await? else {
                    return Err(DurabilityError::Unavailable);
                };
                // Account + World is the existing list owner. Consent remains per inviting
                // character even when another character of this Account shares the list.
                // LIMIT 101 detects overflow without truncating a valid 100-entry snapshot.
                let rows = sqlx::query(
                    "SELECT uuid_send(e.character_id) AS target, e.entry_no, r.name, e.notify, \
                     COALESCE(s.vip_status,1)=1 AND EXISTS (SELECT 1 FROM game_vip_consents c \
                       WHERE c.account_id=e.account_id AND c.world_id=e.world_id \
                       AND c.character_id=e.character_id AND c.inviter_character_id=encode($3,'hex')::uuid \
                       AND c.consent=2) AS visible, \
                     CASE WHEN NOT EXISTS (SELECT 1 FROM game_vip_consents c \
                       WHERE c.account_id=e.account_id AND c.world_id=e.world_id \
                       AND c.character_id=e.character_id AND c.inviter_character_id=encode($3,'hex')::uuid \
                       AND c.consent=2) THEN 0 \
                     WHEN COALESCE(s.vip_status,1)=2 THEN 1 \
                     WHEN EXISTS (SELECT 1 FROM game_durability_reconnect_sessions p \
                       WHERE p.character_id=e.character_id AND p.world_id=e.world_id \
                       AND p.session_state IN(1,2)) THEN 2 ELSE 1 END AS presence \
                     FROM game_vip_entries e JOIN game_character_roots r ON r.character_id=e.character_id \
                     LEFT JOIN game_character_social_settings s ON s.character_id=e.character_id \
                     WHERE e.account_id=$1::uuid AND e.world_id=encode($2,'hex')::uuid \
                     ORDER BY e.entry_no LIMIT 101",
                )
                .bind(account)
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(fence.character_id.as_bytes().as_slice())
                .fetch_all(&mut *tx).await?;
                if rows.len() > MAX_ENTRIES {
                    return Err(DurabilityError::Unavailable);
                }
                let mut values = Vec::with_capacity(rows.len());
                for row in rows {
                    let target: Vec<u8> = row.try_get("target")?;
                    let target = CharacterId::decode(&target).map_err(|_| DurabilityError::Unavailable)?;
                    let entry: i64 = row.try_get("entry_no")?;
                    let status = match row.try_get::<i32, _>("presence")? {
                        0 => Status::Unconfirmed,
                        1 => Status::Offline,
                        2 => Status::Online,
                        _ => return Err(DurabilityError::Unavailable),
                    };
                    values.push(VipPresenceRow {
                        target,
                        entry: u64::try_from(entry).map_err(|_| DurabilityError::Unavailable)?,
                        name: row.try_get("name")?,
                        notify: row.try_get("notify")?,
                        visible: row.try_get("visible")?,
                        status,
                    });
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(values)
            })
        }).await
    }
}
