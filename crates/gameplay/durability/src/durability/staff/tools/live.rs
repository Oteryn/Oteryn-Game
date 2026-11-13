//! Current staff enforcement and authority reads.
use super::*;
use crate::domain::AccountId;

impl DurabilityRoot {
    /// A rule exemption inside an already fenced gameplay transaction. The caller
    /// has independently proven the participant's current source session. Staff
    /// authority comes only from this root's configured trust and current roster,
    /// never a caller-supplied group or a historical staff operation.
    pub(crate) async fn staff_group_in_transaction(
        &self,
        tx: &mut PgConnection,
        character: crate::domain::CharacterId,
    ) -> Result<u8, DurabilityError> {
        let Some(trust) = self.staff_trust() else {
            return Ok(0);
        };
        // Signed roster rows are immutable (0170); matching the trusted roster
        // hash needs SELECT only. Lock the mutable group/head, not the roster:
        // runtime intentionally has no UPDATE privilege on roster snapshots.
        let group: Option<i16> = sqlx::query_scalar(
            "SELECT g.group_id FROM game_staff_character_groups g \
             JOIN game_staff_control_heads h ON h.environment=g.environment AND h.activation_sha256=g.activation_sha256 \
             JOIN game_staff_rosters r ON r.environment=g.environment AND r.activation_sha256=g.activation_sha256 AND r.principal=g.principal \
             WHERE g.environment=$1 AND g.character_id=encode($2,'hex')::uuid \
               AND g.activation_sha256=$3 AND r.roster_sha256=$4 \
               AND g.group_id=CASE r.role WHEN 'GM' THEN 4 WHEN 'SENIOR_GM' THEN 5 WHEN 'ADMIN' THEN 6 END \
             ORDER BY g.group_id DESC LIMIT 1 FOR SHARE OF g,h",
        )
        .bind(trust.environment())
        .bind(character.as_bytes().as_slice())
        .bind(trust.activation())
        .bind(trust.current_roster())
        .fetch_optional(tx)
        .await?;
        u8::try_from(group.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)
    }

    pub async fn staff_live_status(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<StaffLiveStatus, DurabilityError> {
        self.with_staff_tools_fence(authority, node, fence, |tx, fence, environment, activation| Box::pin(async move {
            let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM game_staff_bans b JOIN game_character_roots c ON c.account_id=b.account_id WHERE c.character_id=encode($1,'hex')::uuid AND ($3::text IS NULL OR b.environment=$3) AND b.lifted_at_ms IS NULL AND b.starts_at_ms<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint AND b.ends_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint) AS banned, EXISTS(SELECT 1 FROM game_staff_disconnects d WHERE ($3::text IS NULL OR d.environment=$3) AND (d.game_session_id=encode($2,'hex')::uuid OR EXISTS(SELECT 1 FROM game_activity_runtime_returns a WHERE a.destination_game_session_id=encode($2,'hex')::uuid AND a.source_game_session_id=d.game_session_id))) OR game_security_session_revoked(encode($2,'hex')::uuid) AS kicked, EXISTS(SELECT 1 FROM game_staff_actor_flags f WHERE f.game_session_id=encode($2,'hex')::uuid AND f.character_id=encode($1,'hex')::uuid AND ($3::text IS NULL OR f.environment=$3) AND f.ghost) AS ghost, EXISTS(SELECT 1 FROM game_staff_actor_flags f WHERE f.game_session_id=encode($2,'hex')::uuid AND f.character_id=encode($1,'hex')::uuid AND ($3::text IS NULL OR f.environment=$3) AND f.invisible) AS invisible, COALESCE((SELECT g.group_id FROM game_staff_character_groups g JOIN game_staff_control_heads h ON h.environment=g.environment AND h.activation_sha256=g.activation_sha256 WHERE g.character_id=encode($1,'hex')::uuid AND ($3::text IS NULL OR g.environment=$3) AND $4::text IS NOT NULL AND g.activation_sha256=$4 ORDER BY g.group_id DESC LIMIT 1),0)::smallint AS staff_group")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(environment).bind(activation).fetch_one(tx).await?;
            let group = u8::try_from(row.try_get::<i16,_>("staff_group")?).map_err(|_| DurabilityError::InvalidStoredState)?;
            Ok(StaffLiveStatus {
                banned: row.try_get("banned")?, kicked: row.try_get("kicked")?,
                ghost: group >= 4 && row.try_get("ghost")?, invisible: group >= 4 && row.try_get("invisible")?,
                group,
            })
        })).await
    }

    /// A retained signed operation proves its historical outcome only. Acquiring
    /// a new live effect additionally requires the currently configured roster
    /// and this issuer's exact principal binding under the current activation.
    pub async fn staff_tools_actor_authorized(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        principal: &str,
        minimum_group: u8,
    ) -> Result<bool, DurabilityError> {
        if !rules::atom(principal) || !(4..=6).contains(&minimum_group) {
            return Ok(false);
        }
        let principal = principal.to_owned();
        let roster = self
            .staff_trust()
            .map(|trust| trust.current_roster().to_owned());
        self.with_staff_tools_fence(authority,node,fence,move |tx,fence,environment,activation| Box::pin(async move {
            let (Some(environment),Some(activation),Some(roster))=(environment,activation,roster) else { return Ok(false); };
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_character_groups g JOIN game_staff_control_heads h ON h.environment=g.environment JOIN game_staff_rosters r ON r.environment=g.environment AND r.activation_sha256=g.activation_sha256 AND r.principal=g.principal WHERE g.environment=$1 AND g.character_id=encode($2,'hex')::uuid AND g.principal=$3 AND g.activation_sha256=$4 AND h.activation_sha256=$4 AND r.roster_sha256=$5 AND g.group_id>=$6 AND g.group_id=CASE r.role WHEN 'GM' THEN 4 WHEN 'SENIOR_GM' THEN 5 WHEN 'ADMIN' THEN 6 END)")
                .bind(environment).bind(fence.character_id.as_bytes().as_slice()).bind(principal).bind(activation).bind(roster)
                .bind(i16::from(minimum_group)).fetch_one(tx).await.map_err(Into::into)
        })).await
    }

    /// Account-wide Game enforcement cache; Platform identity/account state is not mutated.
    /// A captured disconnect survives unban and belongs to this exact lost
    /// session, not a later fresh session on the same account.
    pub async fn staff_lost_session_kicked(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        account: AccountId,
        epoch: crate::foundation::ControlLossEpochRefV1,
    ) -> Result<bool, DurabilityError> {
        self.with_staff_tools_fence(authority,node,fence,move |tx,fence,environment,_| Box::pin(async move {
            let kicked:Option<bool>=sqlx::query_scalar("SELECT game_security_session_revoked(s.game_session_id) OR EXISTS(SELECT 1 FROM game_staff_disconnects d WHERE ($5::text IS NULL OR d.environment=$5) AND (d.game_session_id=s.game_session_id OR EXISTS(SELECT 1 FROM game_activity_runtime_returns a WHERE a.destination_game_session_id=s.game_session_id AND a.source_game_session_id=d.game_session_id))) FROM game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid AND s.account_id=encode($2,'hex')::uuid AND s.session_state=1 AND s.current_transport_ref IS NULL AND s.control_loss_epoch=$3::text::numeric AND s.current_generation=$4::text::numeric FOR SHARE OF s")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(account.as_bytes().as_slice()).bind(epoch.get().to_string()).bind(fence.connection_generation.get().to_string()).bind(environment).fetch_optional(tx).await?;
            kicked.ok_or(DurabilityError::Unavailable)
        })).await
    }

    pub async fn staff_account_banned(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        account: AccountId,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let environment = self
            .staff_trust()
            .map(|trust| trust.environment().to_owned());
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let banned = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_bans WHERE account_id=encode($1,'hex')::uuid AND ($2::text IS NULL OR environment=$2) AND lifted_at_ms IS NULL AND starts_at_ms<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint AND ends_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
                .bind(account.as_bytes().as_slice()).bind(environment).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(banned)
        })).await
    }
}
