//! NX-1 extends the existing signed transaction; no second operator authority.
use super::*;
use oteryn_protocol_oteryn::staff::StaffCode;
use sqlx::{Postgres, Transaction};

pub(super) async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    trust: &StaffTrust,
    auth: &Authenticated,
    r: &rules::Request,
    fence: CurrentCharacterItemFence,
    exists: bool,
) -> Result<Option<StaffCode>, DurabilityError> {
    use rules::Action;
    if !r.valid_tools(auth.role) {
        return Ok(Some(StaffCode::TierInsufficient));
    }
    if !exists {
        return Ok(Some(StaffCode::TargetNotFound));
    }
    let target = r.action.character_id();
    // The signed self-inspection binds the off-host principal to this exact GM
    // character. Transporting another operator's signed request never grants a group.
    if auth.tools
        && !(matches!(r.action, Action::Inspect { .. })
            && target == executor::hex_id(fence.character_id.as_bytes()))
    {
        let bound: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_character_groups WHERE environment=$1 AND character_id=encode($2,'hex')::uuid AND activation_sha256=$3 AND principal=$4)")
            .bind(trust.environment()).bind(fence.character_id.as_bytes().as_slice()).bind(trust.activation()).bind(&auth.envelope.principal).fetch_one(&mut **tx).await?;
        if !bound {
            return Ok(Some(StaffCode::TierInsufficient));
        }
    }
    if matches!(
        r.action,
        Action::Teleport { .. }
            | Action::TeleportToPlayer { .. }
            | Action::Ghost { .. }
            | Action::Invisible { .. }
            | Action::Broadcast { .. }
    ) && target != executor::hex_id(fence.character_id.as_bytes())
    {
        return Ok(Some(StaffCode::TierInsufficient));
    }
    if matches!(
        r.action,
        Action::Ban { .. } | Action::Kick { .. } | Action::Mute { .. }
    ) {
        let protected: bool = if matches!(r.action, Action::Ban { .. }) {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_character_groups g JOIN game_character_roots c USING(character_id) WHERE g.environment=$1 AND g.activation_sha256=$3 AND c.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$2::uuid))")
                .bind(trust.environment()).bind(target).bind(trust.activation()).fetch_one(&mut **tx).await?
        } else {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_character_groups WHERE environment=$1 AND character_id=$2::uuid AND activation_sha256=$3)")
                .bind(trust.environment()).bind(target).bind(trust.activation()).fetch_one(&mut **tx).await?
        };
        if protected {
            return Ok(Some(StaffCode::TierInsufficient));
        }
    }
    if matches!(r.action, Action::Ban { .. }) {
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_bans WHERE environment=$1 AND account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$2::uuid) AND lifted_at_ms IS NULL AND ends_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
            .bind(trust.environment()).bind(target).fetch_one(&mut **tx).await?;
        if active {
            return Ok(Some(StaffCode::AlreadyBanned));
        }
    }
    if matches!(r.action, Action::Unban { .. }) {
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_bans WHERE environment=$1 AND account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$2::uuid) AND lifted_at_ms IS NULL AND ends_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
            .bind(trust.environment()).bind(target).fetch_one(&mut **tx).await?;
        if !active {
            return Ok(Some(StaffCode::AlreadyReleased));
        }
    }
    if matches!(
        r.action,
        Action::Mute { .. } | Action::Unmute { .. } | Action::Kick { .. }
    ) {
        // Moderation follows the character across this World's channels/Activities.
        let online: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND world_id=$2::uuid AND session_state IN(1,2))")
            .bind(target).bind(&r.world_id).fetch_one(&mut **tx).await?;
        if !online {
            return Ok(Some(StaffCode::TargetNotInChannel));
        }
    }
    if world_action(&r.action) {
        let online: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND world_id=$2::uuid AND runtime_scope_channel_id=$3::uuid AND session_state=2)")
            .bind(target).bind(&r.world_id).bind(&r.channel_id).fetch_one(&mut **tx).await?;
        if !online {
            return Ok(Some(StaffCode::TargetNotInChannel));
        }
    }
    Ok(None)
}
fn world_action(action: &rules::Action) -> bool {
    !matches!(
        action,
        rules::Action::Inspect { .. }
            | rules::Action::Mute { .. }
            | rules::Action::Unmute { .. }
            | rules::Action::Kick { .. }
            | rules::Action::Ban { .. }
            | rules::Action::Unban { .. }
    )
}
pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    trust: &StaffTrust,
    auth: &Authenticated,
    r: &rules::Request,
    fence: CurrentCharacterItemFence,
    now: u64,
) -> Result<(), DurabilityError> {
    use rules::Action;
    let env = trust.environment();
    let target = r.action.character_id();
    match &r.action {
        Action::Ban { .. } => {
            sqlx::query("INSERT INTO game_staff_bans(environment,sanction_id,account_id,character_id,starts_at_ms,ends_at_ms) SELECT $1,$2::uuid,account_id,character_id,$4,$5 FROM game_character_roots WHERE character_id=$3::uuid")
                .bind(env).bind(&r.operation_id).bind(target).bind(now as i64).bind((now+r.action.duration_ms()) as i64).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_staff_disconnects(environment,operation_id,game_session_id) SELECT $1,$2::uuid,game_session_id FROM game_durability_reconnect_sessions WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$3::uuid) AND session_state IN(1,2)")
                .bind(env).bind(&r.operation_id).bind(target).execute(&mut **tx).await?;
        }
        Action::Unban { .. } => {
            // Stable row order serializes lifts with concurrent moderation writers.
            sqlx::query("SELECT sanction_id FROM game_staff_bans WHERE environment=$1 AND account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$2::uuid) AND lifted_at_ms IS NULL AND ends_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint ORDER BY sanction_id LIMIT 1 FOR UPDATE")
                .bind(env).bind(target).fetch_optional(&mut **tx).await?;
            sqlx::query("UPDATE game_staff_bans SET lift_operation_id=$3::uuid,lifted_at_ms=$4 WHERE environment=$1 AND account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$2::uuid) AND lifted_at_ms IS NULL AND ends_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .bind(env).bind(target).bind(&r.operation_id).bind(now as i64).execute(&mut **tx).await?;
        }
        Action::Kick { .. } => {
            sqlx::query("INSERT INTO game_staff_disconnects(environment,operation_id,game_session_id) SELECT $1,$2::uuid,game_session_id FROM game_durability_reconnect_sessions WHERE character_id=$3::uuid AND session_state IN(1,2)")
                .bind(env).bind(&r.operation_id).bind(target).execute(&mut **tx).await?;
        }
        action if world_action(action) => {
            sqlx::query("INSERT INTO game_staff_world_effects(environment,operation_id,target_character_id,issuer_character_id,target_game_session_id,target_connection_generation,target_scope_ownership_generation,action) SELECT $1,$2::uuid,$3::uuid,encode($4,'hex')::uuid,game_session_id,current_generation,scope_ownership_generation,$5 FROM game_durability_reconnect_sessions WHERE character_id=$3::uuid AND world_id=$6::uuid AND runtime_scope_channel_id=$7::uuid AND session_state=2")
                .bind(env).bind(&r.operation_id).bind(target).bind(fence.character_id.as_bytes().as_slice()).bind(serde_json::to_value(action).map_err(|_|DurabilityError::InvalidStoredState)?).bind(&r.world_id).bind(&r.channel_id).execute(&mut **tx).await?;
        }
        _ => {}
    }
    if auth.tools
        && target == executor::hex_id(fence.character_id.as_bytes())
        && (matches!(r.action, Action::Inspect { .. }) || world_action(&r.action))
    {
        let group: i16 = match auth.role {
            rules::StaffRole::Gm => 4,
            rules::StaffRole::SeniorGm => 5,
            rules::StaffRole::Admin => 6,
        };
        sqlx::query("INSERT INTO game_staff_character_groups(environment,character_id,group_id,activation_sha256,principal,operation_id) VALUES($1,$2::uuid,$3,$4,$5,$6::uuid) ON CONFLICT(environment,character_id) DO UPDATE SET group_id=EXCLUDED.group_id,activation_sha256=EXCLUDED.activation_sha256,principal=EXCLUDED.principal,operation_id=EXCLUDED.operation_id")
            .bind(env).bind(target).bind(group).bind(auth.activation()).bind(&auth.envelope.principal).bind(&r.operation_id).execute(&mut **tx).await?;
    }
    Ok(())
}
pub(super) async fn effect_matches(
    tx: &mut sqlx::PgConnection,
    trust: &StaffTrust,
    r: &rules::Request,
    starts: i64,
) -> Result<bool, DurabilityError> {
    Ok(match &r.action {
        rules::Action::Ban {..} => sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_staff_bans WHERE environment=$1 AND sanction_id=$2::uuid AND character_id=$3::uuid AND starts_at_ms=$4 AND ends_at_ms-starts_at_ms=$5)")
            .bind(trust.environment()).bind(&r.operation_id).bind(r.action.character_id()).bind(starts).bind(r.action.duration_ms() as i64).fetch_one(&mut *tx).await?,
        rules::Action::Unban {..} => sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_staff_bans WHERE environment=$1 AND lift_operation_id=$2::uuid AND account_id=(SELECT account_id FROM game_character_roots WHERE character_id=$3::uuid) AND lifted_at_ms=$4)")
            .bind(trust.environment()).bind(&r.operation_id).bind(r.action.character_id()).bind(starts).fetch_one(&mut *tx).await?,
        action if world_action(action) => sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_staff_world_effects WHERE environment=$1 AND operation_id=$2::uuid AND target_character_id=$3::uuid AND action=$4)")
            .bind(trust.environment()).bind(&r.operation_id).bind(r.action.character_id()).bind(serde_json::to_value(action).map_err(|_|DurabilityError::InvalidStoredState)?).fetch_one(&mut *tx).await?,
        _=>false,
    })
}
