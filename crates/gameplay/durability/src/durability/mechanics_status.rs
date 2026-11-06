//! CL6 read-only projections; existing durable owners and admission remain authoritative.
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{GameSessionId, GameSessionState, PartyPresence, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::mechanics_status::{
    DeathConsumption, EffectTimer, MechanicsStatus, MemberEffects,
};
use sqlx::Row;

/// These live bindings are observations, not tokens authorizing a write or a cast.
pub struct MechanicsObservation {
    pub status: MechanicsStatus,
    pub live_members: Vec<Option<(GameSessionId, u64)>>,
}
impl DurabilityRoot {
    pub async fn read_mechanics_status(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<MechanicsObservation, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let dust = super::character_forge_dust::read_forge_dust(&mut tx, fence.character_id).await?;
            let rows = sqlx::query("SELECT committed_at AS at, blessings_before, blessings_after FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid ORDER BY committed_at DESC,death_occurrence_id DESC LIMIT 5")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let mut deaths = Vec::with_capacity(rows.len());
            for row in rows {
                let before: Vec<String> = row.try_get("blessings_before")?;
                let after: Vec<String> = row.try_get("blessings_after")?;
                if after.iter().any(|key| !before.contains(key)) { return Err(DurabilityError::InvalidStoredState); }
                let mut consumed: Vec<_> = before.into_iter().filter(|key| !after.contains(key)).collect();
                consumed.sort();
                consumed.dedup();
                deaths.push(DeathConsumption { at_unix_ms: row.try_get("at")?, consumed });
            }
            // The existing owner counts online intervals, including reconnect/terminal stamps.
            // expires_at alone is not an authoritative Wealth Duplex timer.
            let remaining: i64 = sqlx::query_scalar("WITH instant AS (SELECT public.game_daily_effect_now() AS at), latest AS (SELECT character_id,started_at FROM game_character_concoction_effects,instant WHERE character_id=encode($1,'hex')::uuid AND concoction='wealth_duplex' AND started_at<=instant.at ORDER BY started_at DESC LIMIT 1) SELECT COALESCE((SELECT (greatest(0,$2::bigint-public.game_character_online_us(latest.character_id,latest.started_at,instant.at))+999)/1000 FROM latest,instant),0)::bigint")
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(super::item_use::concoction::WEALTH_DUPLEX_ONLINE_MICROS)
                .fetch_one(&mut *tx).await?;
            let timers = if remaining > 0 { vec![EffectTimer { key: "wealth_duplex".into(), remaining_ms: u64::try_from(remaining).map_err(|_| DurabilityError::InvalidStoredState)?, online_only: true }] } else { vec![] };
            let world = fence.runtime_scope.world_id();
            let world_hex = world.as_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>();
            sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))").bind(world_hex).execute(&mut *tx).await?;
            let party = super::world_party::observe_world_party(&mut tx, *world.as_bytes(), *fence.character_id.as_bytes()).await.map_err(|_| DurabilityError::Unavailable)?;
            let mut members = Vec::with_capacity(party.members.len());
            let mut live_members = Vec::with_capacity(party.members.len());
            let sessions = super::fresh_admission::FreshAdmissionStore::from_root(root);
            for member in party.members {
                let name: String = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
                    .bind(member.character.as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let colocated = member.presence == PartyPresence::Channel && matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { channel_id, .. } if member.channel == Some(*channel_id.as_bytes()));
                let held: Option<Vec<u8>> = if colocated {
                    sqlx::query_scalar("SELECT uuid_send(holder_game_session_id) FROM game_durability_admission_character_guards WHERE character_id=encode($1,'hex')::uuid AND eligible AND holder_game_session_id IS NOT NULL")
                        .bind(member.character.as_slice()).fetch_optional(&mut *tx).await?
                } else { None };
                let live = if let Some(held) = held {
                    let session = GameSessionId::decode(&held).map_err(|_| DurabilityError::InvalidStoredState)?;
                    let current = sessions.current_session_in_transaction(&mut tx, session).await?;
                    if current.session_state() == GameSessionState::Active
                        && current.current_control_loss_epoch().is_none()
                        && current.current_transport().is_some()
                        && current.commit().character_id().as_bytes() == &member.character
                        && current.current_runtime_scope() == fence.runtime_scope
                        && current.current_scope_generation() == fence.scope_ownership_generation
                    { Some((current.current_game_session_id(), current.current_character_lease().generation())) } else { None }
                } else { None };
                members.push(MemberEffects { bonuses: [0; 5], character: member.character, name, conditions: None, party_spell_remaining_ms: [0;5] });
                live_members.push(live);
            }
            let observed_unix_ms = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            let status = MechanicsStatus { observed_unix_ms, dust: u32::from(dust.balance()), dust_limit: u32::from(dust.limit().get()), deaths, timers, members };
            status.encode().ok_or(DurabilityError::InvalidStoredState)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(MechanicsObservation { status, live_members }))
        })).await?
    }
}
