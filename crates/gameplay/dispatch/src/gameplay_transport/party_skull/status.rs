use super::*;
use oteryn_gameplay_combat_rules::pvp::skull::{BLACK_THRESHOLDS_MILLI, RED_THRESHOLDS_MILLI};
use oteryn_protocol_oteryn::party_skull::{CAPABILITY_PARTY_SKULL_V1, PvpSituation, PvpStatus};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_own_pvp(
        &self,
        admitted: AdmittedSession,
    ) -> Option<PvpStatus> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_SKULL_V1)
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        runtime.assert_actor_spell_unreserved(actor).ok()?;
        if !crate::gameplay_transport::battle_instinct_status::actor_is_current(&runtime, admitted)
        {
            return None;
        }
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, &*runtime, &permit);
        pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime, permit) = ctx;
                let unavailable = || DurabilityError::Unavailable;
                let binding = runtime.binding();
                let scope = binding.runtime_scope();
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_scope_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder, scope, binding.scope_generation().get()).await.map_err(|_| unavailable())?;
                let observer = crate::durability::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(&mut tx, owner.root, &authority, runtime, actor, admitted.game_session_id).await.map_err(|_| unavailable())?.ok_or_else(unavailable)?;
                let connection = crate::durability::admission_journal::party_target_binding::read_connection_generation_for_share_in_transaction(&mut tx, admitted.game_session_id).await?;
                if connection != admitted.continuity.connection_generation.to_string() { return Err(unavailable()); }
                let row = sqlx::query("SELECT COALESCE(sum(points_milli) FILTER (WHERE committed_at > transaction_timestamp()-interval '24 hours'),0)::bigint AS daily, COALESCE(sum(points_milli) FILTER (WHERE committed_at > transaction_timestamp()-interval '168 hours'),0)::bigint AS weekly, COALESCE(sum(points_milli),0)::bigint AS monthly FROM game_character_unjustified_points WHERE character_id=encode($1,'hex')::uuid AND committed_at > transaction_timestamp()-interval '720 hours' AND committed_at <= transaction_timestamp()")
                    .bind(observer.as_slice()).fetch_one(&mut *tx).await?;
                let points_milli = [row.try_get::<i64,_>("daily")?, row.try_get::<i64,_>("weekly")?, row.try_get::<i64,_>("monthly")?].map(u64::try_from);
                let [daily, weekly, monthly] = points_milli;
                let points_milli = [daily.map_err(|_| unavailable())?, weekly.map_err(|_| unavailable())?, monthly.map_err(|_| unavailable())?];
                let skull = sqlx::query("SELECT skull, COALESCE(GREATEST(0,ceil(extract(epoch FROM (until_at-transaction_timestamp()))*1000)),0)::bigint AS remaining_ms FROM (SELECT CASE WHEN skull_until>transaction_timestamp() THEN CASE skull WHEN 2 THEN 5 WHEN 1 THEN 4 ELSE 0 END WHEN white_skull_until>transaction_timestamp() THEN 3 ELSE 0 END AS skull, CASE WHEN skull_until>transaction_timestamp() THEN skull_until WHEN white_skull_until>transaction_timestamp() THEN white_skull_until END AS until_at FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid) AS pvp")
                    .bind(observer.as_slice()).bind(admitted.world_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let (skull, remaining_ms) = match skull {
                    Some(row) => (u8::try_from(row.try_get::<i32,_>("skull")?).map_err(|_| unavailable())?, u64::try_from(row.try_get::<i64,_>("remaining_ms")?).map_err(|_| unavailable())?),
                    None => (0,0),
                };
                let rows = sqlx::query("SELECT target=encode($1,'hex')::uuid AS incoming, ceil(extract(epoch FROM (until_at-transaction_timestamp()))*1000)::bigint AS remaining_ms FROM game_character_pvp_aggression WHERE scope_generation=$2::text::numeric AND ((attacker=encode($1,'hex')::uuid AND attacker_placement=$3) OR (target=encode($1,'hex')::uuid AND target_placement=$3)) AND until_at>transaction_timestamp() ORDER BY until_at,attacker,target LIMIT 65")
                    .bind(observer.as_slice()).bind(binding.scope_generation().get().to_string()).bind(actor.placement_identity().as_slice()).fetch_all(&mut *tx).await?;
                if rows.len() > 64 { return Err(unavailable()); }
                let situations = rows.iter().map(|row| Ok(PvpSituation {
                    incoming: row.try_get("incoming")?, remaining_ms: u64::try_from(row.try_get::<i64,_>("remaining_ms")?).map_err(|_| unavailable())?,
                })).collect::<Result<Vec<_>, DurabilityError>>()?;
                let status = PvpStatus {points_milli, red_limits_milli: RED_THRESHOLDS_MILLI, black_limits_milli: BLACK_THRESHOLDS_MILLI, skull, remaining_ms, situations};
                status.encode().map_err(|_| unavailable())?;
                tx.commit().await?;
                Ok(status)
            })
        }).await.ok()
    }
}
