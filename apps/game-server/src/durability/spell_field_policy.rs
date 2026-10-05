//! A current admission policy revision binds an explicitly control-owned field
//! World config. No missing row, ready bit, or source default grants PvP.
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use crate::foundation::{RuntimeScopeRefV1, WorldId};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldWorldType {
    NoPvp,
    Pvp,
    PvpEnforced,
}
#[derive(Debug)]
pub(crate) struct WorldFieldPolicyRead<'read, 'holder> {
    _transaction: &'read Transaction<'holder, Postgres>,
    world: WorldId,
    revision: String,
    mode: FieldWorldType,
    protection_level: u32,
    in_fight_ms: u32,
}
impl WorldFieldPolicyRead<'_, '_> {
    pub(crate) fn world(&self) -> WorldId {
        self.world
    }
    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }
    pub(crate) fn mode(&self) -> FieldWorldType {
        self.mode
    }
    pub(crate) fn protection_level(&self) -> u32 {
        self.protection_level
    }
    pub(crate) fn in_fight_ms(&self) -> u32 {
        self.in_fight_ms
    }
}
pub(crate) struct WorldFieldPolicyData {
    world: WorldId,
    revision: String,
    mode: FieldWorldType,
    protection_level: u32,
    in_fight_ms: u32,
}
impl WorldFieldPolicyData {
    /// Borrow binds the actual same SQL transaction retaining readiness and
    /// immutable config locks through field's physical owner install.
    pub(crate) fn bind<'r, 'h>(
        self,
        tx: &'r Transaction<'h, Postgres>,
    ) -> WorldFieldPolicyRead<'r, 'h> {
        WorldFieldPolicyRead {
            _transaction: tx,
            world: self.world,
            revision: self.revision,
            mode: self.mode,
            protection_level: self.protection_level,
            in_fight_ms: self.in_fight_ms,
        }
    }
}
/// Caller already holds the actual scope/session fence locks in this TX.
/// The existing guard reader compares every mirror plus retained source history.
pub(crate) async fn read_world_field_policy_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: RuntimeScopeRefV1,
    generation: u64,
) -> Result<Option<WorldFieldPolicyData>, DurabilityError> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = scope else {
        return Err(DurabilityError::Unavailable);
    };
    let guard = super::admission_authority_guards::AdmissionGuardStore::from_root(root.clone())
        .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope))
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready: true,
        world_policy_revision,
        ..
    } = guard.state
    else {
        return Err(DurabilityError::Unavailable);
    };
    if ownership_generation != generation {
        return Err(DurabilityError::Unavailable);
    }
    // Migration 0048's insert guard binds `control_role` to `session_user`, not
    // `current_user`: the guard is SECURITY DEFINER, so `current_user` inside it
    // is always the function owner, and `session_user` is the authenticated login
    // that a `SET ROLE` cannot change. A row here was published by the exact
    // control login holding a World/revision grant, never by a role it assumed.
    let row=sqlx::query("SELECT world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity FROM game_spell_field_world_policies WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision=$2 FOR SHARE")
        .bind(world_id.as_bytes().as_slice()).bind(&world_policy_revision).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    if row.try_get::<String, _>("source_pin")? != "99902524e052f37574194466c2949c576e4ab269"
        || row.try_get::<String, _>("control_role")?.is_empty()
        || row.try_get::<String, _>("decision_identity")?.is_empty()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mode = match row.try_get::<i16, _>("world_type")? {
        1 => FieldWorldType::NoPvp,
        2 => FieldWorldType::Pvp,
        3 => FieldWorldType::PvpEnforced,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(Some(WorldFieldPolicyData {
        world: world_id,
        revision: world_policy_revision,
        mode,
        protection_level: u32::try_from(row.try_get::<i64, _>("protection_level")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        in_fight_ms: u32::try_from(row.try_get::<i64, _>("in_fight_ms")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    }))
}
