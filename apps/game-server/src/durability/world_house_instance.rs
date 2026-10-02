//! Actual World-global House-to-Instance identity owner, separate from Channels.
//! Fresh admission supplies the current Instance and ownership generation. This
//! read-only observation is deliberately not a physical HousePresenceProof:
//! assignment/handoff and the actual Instance position carrier are still required.
use super::{DurabilityError, fresh_admission::FreshAdmissionStore};
use crate::foundation::{GameSessionId, GameSessionState, RuntimeScopeRefV1};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug)]
pub(crate) enum Error {
    Database(sqlx::Error),
    Admission(DurabilityError),
    ChannelScope,
    InactiveSession,
    MissingBinding,
    ContentMismatch,
    InvalidBinding,
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
#[derive(Debug)]
pub(crate) struct CurrentWorldHouseAdmission {
    scope: RuntimeScopeRefV1,
    session: GameSessionId,
    character: [u8; 16],
    connection_generation: u64,
    lease_generation: u64,
    scope_generation: u64,
    house_key: String,
    house_revision: String,
    content_digest: [u8; 32],
    placement_digest: [u8; 32],
}
impl CurrentWorldHouseAdmission {
    pub(crate) fn scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }
    pub(crate) fn session(&self) -> GameSessionId {
        self.session
    }
    pub(crate) fn character(&self) -> [u8; 16] {
        self.character
    }
    pub(crate) fn connection_generation(&self) -> u64 {
        self.connection_generation
    }
    pub(crate) fn lease_generation(&self) -> u64 {
        self.lease_generation
    }
    pub(crate) fn scope_generation(&self) -> u64 {
        self.scope_generation
    }
    pub(crate) fn house_key(&self) -> &str {
        &self.house_key
    }
    pub(crate) fn house_revision(&self) -> &str {
        &self.house_revision
    }
    pub(crate) fn content_digest(&self) -> [u8; 32] {
        self.content_digest
    }
    pub(crate) fn placement_digest(&self) -> [u8; 32] {
        self.placement_digest
    }
}
/// No caller-selected Instance or nativeHouse key. Both come from independently
/// current admitted scope and the control-owned immutable allocation binding.
/// A Channel session explicitly refuses before querying any House allocation.
pub(crate) async fn read_current_world_house_admission_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    admissions: &FreshAdmissionStore,
    session: GameSessionId,
    current_content_digest: [u8; 32],
) -> Result<CurrentWorldHouseAdmission, Error> {
    let current = admissions
        .current_session_in_transaction(tx, session)
        .await
        .map_err(Error::Admission)?;
    if current.session_state() != GameSessionState::Active
        || current.current_transport().is_none()
        || current.current_control_loss_epoch().is_some()
    {
        return Err(Error::InactiveSession);
    }
    let scope = current.current_runtime_scope();
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = scope
    else {
        return Err(Error::ChannelScope);
    };
    let row=sqlx::query("SELECT h.house_key,h.house_revision,h.compatible_content_digest,h.qualified_placement_digest FROM game_world_house_instances h JOIN game_house_ownership o USING(world_id,house_key) WHERE h.world_id=encode($1,'hex')::uuid AND h.instance_id=encode($2,'hex')::uuid FOR SHARE OF h,o")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).fetch_optional(&mut **tx).await?.ok_or(Error::MissingBinding)?;
    let content_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("compatible_content_digest")?
        .try_into()
        .map_err(|_| Error::InvalidBinding)?;
    if content_digest != current_content_digest || content_digest == [0; 32] {
        return Err(Error::ContentMismatch);
    }
    let placement_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("qualified_placement_digest")?
        .try_into()
        .map_err(|_| Error::InvalidBinding)?;
    if placement_digest == [0; 32] {
        return Err(Error::InvalidBinding);
    }
    let house_key: String = row.try_get("house_key")?;
    let house_revision: String = row.try_get("house_revision")?;
    if !house_key.starts_with("oteryn:content.house.") || house_revision.is_empty() {
        return Err(Error::InvalidBinding);
    }
    Ok(CurrentWorldHouseAdmission {
        scope,
        session,
        character: *current.commit().character_id().as_bytes(),
        connection_generation: current.current_connection_generation().get(),
        lease_generation: current.current_character_lease().generation(),
        scope_generation: current.current_scope_generation().get(),
        house_key,
        house_revision,
        content_digest,
        placement_digest,
    })
}
