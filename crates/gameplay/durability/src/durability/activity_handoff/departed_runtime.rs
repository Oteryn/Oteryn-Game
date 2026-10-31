//! Origin-owner recovery after the native Instance process has disappeared.
//! Discovery is inert. Every mutation repeats the existing departed-source,
//! current origin, Character claim and immutable activity lineage checks.
use super::*;
use crate::foundation::{ConnectionGeneration, GameSessionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepartedActivityCursor {
    handoff: [u8; 16],
    character: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartedActivity {
    cursor: DepartedActivityCursor,
    source: CurrentCharacterItemFence,
    account: String,
    origin: RuntimeScopeRefV1,
    origin_generation: ScopeOwnershipGeneration,
    returned: Option<ActivityReturnReceipt>,
}

impl DepartedActivity {
    pub const fn handoff_id(&self) -> [u8; 16] { self.cursor.handoff }
    pub const fn source(&self) -> CurrentCharacterItemFence { self.source }
    pub fn account(&self) -> &str { &self.account }
    pub const fn origin_scope(&self) -> RuntimeScopeRefV1 { self.origin }
    pub const fn origin_generation(&self) -> ScopeOwnershipGeneration { self.origin_generation }
    pub const fn returned(&self) -> Option<&ActivityReturnReceipt> { self.returned.as_ref() }
}

#[derive(Debug, Clone)]
pub struct DepartedActivityPage {
    pub activities: Vec<DepartedActivity>,
    /// Based on inspected rows, including refused ones, so a blocked prefix
    /// cannot hide later recoverable participants.
    pub next_after: Option<DepartedActivityCursor>,
}

impl ActivityHandoffOwner {
    /// The current origin Channel can discover departed participants without a
    /// retained native directory. No credential, assignment or actor is issued.
    pub async fn departed_activities_after(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        origin: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        after: Option<DepartedActivityCursor>,
    ) -> Result<DepartedActivityPage> {
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = origin else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let recovery = authority.record_for(&self.root).map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !origin_current(&mut tx, &root, &node, origin, generation).await? {
                return Err(DurabilityError::Unavailable);
            }
            let rows = sqlx::query("SELECT uuid_send(p.handoff_id) AS handoff,uuid_send(p.character_id) AS character \
                FROM game_activity_handoff_participants p JOIN game_activity_handoff_groups g USING(handoff_id) \
                JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.destination_game_session_id \
                LEFT JOIN game_activity_runtime_returns r ON (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id) \
                LEFT JOIN game_durability_reconnect_sessions d ON d.game_session_id=r.destination_game_session_id \
                WHERE p.world_id=encode($1,'hex')::uuid AND p.origin_channel_id=encode($2,'hex')::uuid AND g.state=2 \
                  AND ((r.return_id IS NULL AND s.session_state IN(1,2)) \
                    OR (r.reason IN(4,5) AND r.destination_transport_ref IS NULL AND d.session_state=1 \
                        AND d.current_generation=1 AND d.current_transport_ref IS NULL)) \
                  AND ($3::bytea IS NULL OR (p.handoff_id,p.character_id)>(encode($3,'hex')::uuid,encode($4,'hex')::uuid)) \
                ORDER BY p.handoff_id,p.character_id LIMIT 15")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                .bind(after.map(|cursor| cursor.handoff.to_vec())).bind(after.map(|cursor| cursor.character.to_vec()))
                .fetch_all(&mut *tx).await?;
            let next_after = if rows.len() == ACTIVITY_PARTICIPANTS_MAX {
                rows.last().map(decode_cursor).transpose()?
            } else { None };
            let mut activities = Vec::with_capacity(rows.len());
            for row in rows {
                if let Some(activity) = load_candidate(&mut tx, &root, &node, origin, generation, decode_cursor(&row)?).await? {
                    activities.push(activity);
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(DepartedActivityPage { activities, next_after })
        })).await
    }

    /// Finish only the existing offline return of a proven departed owner.
    /// The ordinary terminal-release writer owns claims and its receipt. Its
    /// original grace deadline is respected and no loss receipt is fabricated.
    pub async fn release_departed_return(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        activity: &DepartedActivity,
    ) -> Result<super::super::fresh_admission::ExpiredLossReleaseV1> {
        let Some(receipt) = activity.returned() else {
            return Ok(super::super::fresh_admission::ExpiredLossReleaseV1::NotApplicable);
        };
        let proof = DepartedActivityRelease {
            root: self.root.clone(),
            recovery: authority.record_for(&self.root).map_err(|_| DurabilityError::Unavailable)?,
            node: node.clone(),
            activity: activity.clone(),
        };
        FreshAdmissionStore::from_root(self.root.clone())
            .with_departed_activity_release(proof)
            .release_expired_loss(receipt.destination_game_session_id, activity.account())
            .await
    }
}

fn decode_cursor(row: &sqlx::postgres::PgRow) -> Result<DepartedActivityCursor> {
    Ok(DepartedActivityCursor {
        handoff: bytes16(row.try_get("handoff")?)?,
        character: bytes16(row.try_get("character")?)?,
    })
}

pub(super) async fn origin_current(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    origin: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
) -> Result<bool> {
    if !matches!(origin, RuntimeScopeRefV1::Channel { .. })
        || AdmissionGuardStore::from_root(root.clone())
            .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(origin)).await?.is_none()
    { return Ok(false); }
    assert_runtime_scope_fence(tx, origin, generation, node).await
}

async fn load_candidate(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    origin: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    cursor: DepartedActivityCursor,
) -> Result<Option<DepartedActivity>> {
    let Some(channel) = origin.channel_id() else { return Ok(None); };
    let row = sqlx::query("SELECT p.account_id::text,uuid_send(p.destination_game_session_id) AS source,uuid_send(g.instance_id) AS instance, \
        g.ownership_generation::text AS scope_generation,g.content_revision,s.current_generation::text AS connection, \
        s.character_lease_generation::text AS lease,s.session_state,r.reason,r.destination_transport_ref, \
        r.source_connection_generation::text AS return_connection,r.source_lease_generation::text AS return_lease \
        FROM game_activity_handoff_participants p JOIN game_activity_handoff_groups g USING(handoff_id) \
        JOIN game_activity_runtime_entries e USING(handoff_id) \
        JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.destination_game_session_id \
        LEFT JOIN game_activity_runtime_returns r ON (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id) \
        WHERE p.handoff_id=encode($1,'hex')::uuid AND p.character_id=encode($2,'hex')::uuid AND g.state=2 \
          AND p.world_id=encode($3,'hex')::uuid AND p.origin_channel_id=encode($4,'hex')::uuid \
          AND s.character_id=p.character_id AND s.account_id=p.account_id AND s.world_id=p.world_id \
          AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=g.world_id AND s.runtime_scope_instance_id=g.instance_id \
          AND s.scope_ownership_generation=g.ownership_generation AND s.character_lease_generation=p.source_lease_generation+1 \
        FOR SHARE OF p,g,e FOR UPDATE OF s")
        .bind(cursor.handoff.as_slice()).bind(cursor.character.as_slice())
        .bind(origin.world_id().as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None); };
    let source = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(cursor.character).map_err(|_| DurabilityError::InvalidStoredState)?,
        game_session_id: GameSessionId::decode(&bytes16(row.try_get("source")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
        connection_generation: ConnectionGeneration::new(parse_u64(row.try_get("connection")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
        character_lease_generation: parse_u64(row.try_get("lease")?)?,
        runtime_scope: RuntimeScopeRefV1::instance(origin.world_id(), bytes16(row.try_get("instance")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(parse_u64(row.try_get("scope_generation")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    let account: String = row.try_get("account_id")?;
    let returned = match row.try_get::<Option<i16>, _>("reason")? {
        None => {
            if !super::returning::recovery_source(tx, root, node, source, &account, &row.try_get::<String,_>("content_revision")?).await? {
                return Ok(None);
            }
            None
        }
        Some(4 | 5) => {
            if row.try_get::<i16,_>("session_state")? != 3
                || row.try_get::<Option<Vec<u8>>,_>("destination_transport_ref")?.is_some()
                || row.try_get::<Option<String>,_>("return_connection")?.map(parse_u64).transpose()? != Some(source.connection_generation.get())
                || row.try_get::<Option<String>,_>("return_lease")?.map(parse_u64).transpose()? != Some(source.character_lease_generation)
                || !super::returning::departed_scope(tx, source.runtime_scope, source.scope_ownership_generation).await?
            { return Ok(None); }
            let character = foundation_character(source)?;
            let store = FreshAdmissionStore::from_root(root.clone());
            if !store.committed_session_use(tx, character, source.game_session_id).await? { return Ok(None); }
            let receipt = super::returning::load_receipt(tx, cursor.handoff, character).await?;
            let current = store.current_session_in_transaction(tx, receipt.destination_game_session_id).await?;
            if current.session_state() != GameSessionState::Reconnectable
                || current.current_transport().is_some()
                || current.current_connection_generation().get() != 1
                || current.current_runtime_scope() != origin
                || current.current_scope_generation() != receipt.destination_generation
                || current.current_character_lease().generation() != receipt.destination_lease_generation
                || current.current_control_loss_epoch().is_none()
                || current.current_original_grace_deadline().is_none()
            { return Ok(None); }
            Some(receipt)
        }
        Some(_) => return Ok(None),
    };
    Ok(Some(DepartedActivity { cursor, source, account, origin, origin_generation: generation, returned }))
}

/// Held by the existing release writer, not by a native runtime. Revalidated
/// after its session/claim locks, before any terminal state or claim mutation.
#[derive(Clone)]
pub(crate) struct DepartedActivityRelease {
    root: DurabilityRoot,
    recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    activity: DepartedActivity,
}

impl DepartedActivityRelease {
    pub(crate) async fn validate(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        current: crate::foundation::GameSessionAuthoritySnapshot<crate::foundation::AuthenticatedTransportRefV1>,
    ) -> Result<bool> {
        assert_recovery_fence(tx, &self.recovery).await?;
        if self.activity.returned().is_none_or(|receipt| receipt.destination_game_session_id != current.current_game_session_id())
            || !origin_current(tx, &self.root, &self.node, self.activity.origin, self.activity.origin_generation).await?
        { return Ok(false); }
        Ok(load_candidate(tx, &self.root, &self.node, self.activity.origin, self.activity.origin_generation, self.activity.cursor).await?.as_ref() == Some(&self.activity))
    }
}
