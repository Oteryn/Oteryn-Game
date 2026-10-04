//! Actual current Character/Item reads for a cast, inside its owner's physical SQL transaction.
//! Premium/learning/Wheel producers are explicit ports. Unavailable authority never becomes a grant.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use crate::content::ActiveGeneration;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_equipment::{
    EquipmentError, assert_equipment_authority_in_transaction,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use crate::spell::cast::PlayerSpellState;
use crate::spell::owned_cast_facts::{
    AccessProjections, CastFactsBinding, OwnedCastFacts, OwnedFactsError,
};
use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub(crate) enum AccessFactsError {
    Equipment(EquipmentError),
    Database(sqlx::Error),
    Durability(DurabilityError),
    Facts(OwnedFactsError),
    Unavailable(&'static str),
}
impl From<EquipmentError> for AccessFactsError {
    fn from(e: EquipmentError) -> Self {
        Self::Equipment(e)
    }
}
impl From<sqlx::Error> for AccessFactsError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
impl From<OwnedFactsError> for AccessFactsError {
    fn from(e: OwnedFactsError) -> Self {
        Self::Facts(e)
    }
}
/// A producer implementation is an explicit, reviewed in-crate owner registration. The wire/client
/// cannot implement this sealed port. It must read authenticated Premium evidence with its durable
/// revision/conflict fence, or Character learning/Wheel receipts under the same exact binding.
pub(crate) mod owner_registration {
    pub(crate) trait Registered {}
}
pub(crate) trait CurrentSpellAccessOwner: owner_registration::Registered {
    fn account_id(&self) -> Option<[u8; 16]> {
        None
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now_micros: u64,
    ) -> Result<AccessProjections, AccessFactsError>;
}
/// Explicit missing external producer. This supplies no known-negative or zero-stage projection.
pub(crate) struct UnavailableAccessOwner;
impl owner_registration::Registered for UnavailableAccessOwner {}
impl CurrentSpellAccessOwner for UnavailableAccessOwner {
    fn read_current(
        &self,
        _: &CastFactsBinding,
        _: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        Ok(AccessProjections::default())
    }
}
/// The caller holds the actual Channel owner turn and SQL transaction until the whole cast stages
/// and commits. This helper never issues session, lease, scope or Content authority; each is checked
/// independently from the live owner or actual durable fence. No fixture map substitutes for Item.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_owned_cast_facts_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    state: &PlayerSpellState,
    active: &ActiveGeneration,
    access_owner: &impl CurrentSpellAccessOwner,
    now_micros: u64,
) -> Result<OwnedCastFacts, AccessFactsError> {
    runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| AccessFactsError::Unavailable("current player owner"))?;
    let native = active
        .native_gameplay()
        .ok_or(AccessFactsError::Unavailable("active gameplay artifact"))?;
    if active.identity().server_artifact_digest() != native.source_digest()
        || runtime.content_pin().server_artifact_digest() != native.source_digest()
    {
        return Err(AccessFactsError::Unavailable("current native Content pin"));
    }
    let authority = assert_equipment_authority_in_transaction(
        tx,
        root,
        recovery,
        node,
        fence,
        command,
        native.source_digest(),
    )
    .await?;
    // An uninitialized equipment owner is unavailable. Initialization belongs to its independent
    // admission/Item command, never to a rejected cast's hidden side effects.
    let raw =
        crate::durability::character_equipment::read_raw_cast_facts_in_transaction(tx, &authority)
            .await?;
    qualify_raw_owned_cast_facts(
        &raw,
        command,
        fence,
        runtime,
        actor,
        state,
        active,
        access_owner,
        now_micros,
    )
}
/// Qualifies a real SQL snapshot only after the caller rechecks the current actual owner turn.
/// This data producer grants no authority; final transaction staging still compares every revision.
#[allow(clippy::too_many_arguments)]
pub(crate) fn qualify_raw_owned_cast_facts(
    raw: &crate::durability::character_equipment::RawCastDurableFacts,
    command: CommandRef,
    fence: &CurrentCharacterItemFence,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    state: &PlayerSpellState,
    active: &ActiveGeneration,
    access_owner: &impl CurrentSpellAccessOwner,
    now_micros: u64,
) -> Result<OwnedCastFacts, AccessFactsError> {
    if raw.command != command
        || &raw.fence != fence
        || access_owner
            .account_id()
            .is_some_and(|account| account != raw.account)
    {
        return Err(AccessFactsError::Unavailable(
            "raw cast owner binding changed",
        ));
    }
    runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| AccessFactsError::Unavailable("current raw cast actor"))?;
    let native = active
        .native_gameplay()
        .ok_or(AccessFactsError::Unavailable("active gameplay artifact"))?;
    if active.identity().server_artifact_digest() != raw.equipment.content_digest
        || runtime.content_pin().server_artifact_digest() != raw.equipment.content_digest
        || native.source_digest() != raw.equipment.content_digest
    {
        return Err(AccessFactsError::Unavailable(
            "raw cast Content binding changed",
        ));
    }
    let equipment = raw.equipment.clone();
    let build = raw.build.clone();
    let level = raw.level;
    let binding = CastFactsBinding {
        actor,
        session: fence.game_session_id,
        character: *fence.character_id.as_bytes(),
        character_revision: equipment.character_revision,
        lease_generation: fence.character_lease_generation,
        connection_generation: fence.connection_generation.get(),
        player_revision: state.revision(),
        content_digest: native.source_digest(),
        equipment_revision: equipment.revision,
    };
    let mut projections = access_owner.read_current(&binding, now_micros)?;
    // Main's Wheel owner (0070) keeps runtime effects unadmitted until SPELL-WHEEL-GATE-1;
    // no Wheel stage projection exists, so Wheel-gated spells stay refused.
    projections.wheel = None;
    projections.magnitude = super::spell_magnitude_facts::project(raw, &binding, native)?;
    let facts = OwnedCastFacts::from_owner_reads(binding, build, level, equipment, projections)?;
    // No await follows this final independent owner recheck; the caller keeps the same lock/tx.
    runtime
        .player_control_facts(actor, fence.game_session_id)
        .map_err(|_| AccessFactsError::Unavailable("player owner changed"))?;
    Ok(facts)
}
