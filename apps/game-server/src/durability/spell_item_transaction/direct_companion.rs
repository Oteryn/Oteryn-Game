//! A direct summon/convince source receipt has no corpse or Familiar identity.
//! Only actual private physical reservations implement the sealed source ABI.
use super::*;
use crate::domain::CharacterId;
use crate::foundation::{ExactActorRef, GameSessionId};

pub(crate) mod direct_companion_source_seal {
    pub(crate) trait Sealed {}
}
pub(crate) trait DirectCompanionAcquisitionSource:
    direct_companion_source_seal::Sealed
{
    fn actor(&self) -> ExactActorRef;
    fn master_actor(&self) -> ExactActorRef;
    fn character_id(&self) -> CharacterId;
    fn game_session_id(&self) -> GameSessionId;
    fn cell(&self) -> SpellItemCell;
    fn creature_definition_key(&self) -> &str;
    fn creature_definition_revision(&self) -> &str;
    fn source(&self) -> &DirectCompanionSource;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectCompanionSource {
    Named {
        requested_name: String,
    },
    Target {
        before_snapshot_digest: [u8; 32],
        prior_master: Option<ExactActorRef>,
        prior_master_session: Option<GameSessionId>,
    },
}
fn actor_json(a: ExactActorRef) -> serde_json::Value {
    serde_json::json!({"world":a.world_id().as_bytes(),"channel":a.channel_id().as_bytes(),
      "scope_generation":a.scope_generation().get(),"local_id":a.actor_local_id(),
      "local_generation":a.actor_local_generation(),"placement":a.placement_identity()})
}
impl DirectCompanionSource {
    fn valid(&self) -> bool {
        match self {
            Self::Named { requested_name } => {
                !requested_name.trim().is_empty() && requested_name.len() <= 512
            }
            Self::Target {
                before_snapshot_digest,
                prior_master,
                prior_master_session,
            } => {
                *before_snapshot_digest != [0; 32]
                    && prior_master.is_some() == prior_master_session.is_some()
            }
        }
    }
    fn json(&self) -> serde_json::Value {
        match self {
            Self::Named { requested_name } => {
                serde_json::json!({"kind":"named","requested_name":requested_name})
            }
            Self::Target {
                before_snapshot_digest,
                prior_master,
                prior_master_session,
            } => {
                serde_json::json!({"kind":"target","before_snapshot_digest":before_snapshot_digest,
                    "prior_master":prior_master.map(actor_json),
                    "prior_master_session":prior_master_session.map(|s|*s.as_bytes())})
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DirectBinding {
    transaction: [u8; 16],
    actor: ExactActorRef,
    master: ExactActorRef,
    character: [u8; 16],
    session: [u8; 16],
    cell: SpellItemCell,
    creature_key: String,
    creature_revision: String,
    source: DirectCompanionSource,
}
impl DirectBinding {
    pub(super) fn json(&self) -> serde_json::Value {
        serde_json::json!({"transaction":self.transaction,"actor":actor_json(self.actor),
            "master":actor_json(self.master),"character":self.character,"session":self.session,
            "cell":[self.cell.x,self.cell.y,self.cell.z],"creature_key":self.creature_key,
            "creature_revision":self.creature_revision,"source":self.source.json()})
    }
    fn valid(&self) -> bool {
        self.actor != self.master
            && self.actor.world_id() == self.master.world_id()
            && self.actor.channel_id() == self.master.channel_id()
            && self.actor.scope_generation() == self.master.scope_generation()
            && !self.creature_key.is_empty()
            && self.creature_key.len() <= 512
            && !self.creature_revision.is_empty()
            && self.creature_revision.len() <= 512
            && self.source.valid()
    }
    fn current(&self, a: &SpellItemAuthority, r: &SpellItemTransactionRequest) -> bool {
        self.valid()
            && self.transaction == r.transaction_id
            && *self.actor.world_id().as_bytes() == a.world
            && *self.actor.channel_id().as_bytes() == a.channel
            && self.actor.scope_generation().get() == a.ownership_generation
            && self.character == a.character
            && self.session == *a.command.game_session_id().as_bytes()
            && r.command == a.command
            && r.catalog_digest == a.compatible_content_digest
            && r.caster_origin
                .as_ref()
                .is_some_and(|o| o.actor == self.master)
            && r.companion.is_none()
    }
}
fn binding(transaction: [u8; 16], s: &impl DirectCompanionAcquisitionSource) -> DirectBinding {
    DirectBinding {
        transaction,
        actor: s.actor(),
        master: s.master_actor(),
        character: *s.character_id().as_bytes(),
        session: *s.game_session_id().as_bytes(),
        cell: s.cell(),
        creature_key: s.creature_definition_key().into(),
        creature_revision: s.creature_definition_revision().into(),
        source: s.source().clone(),
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedDirectCompanionAcquisition {
    pub(super) binding: DirectBinding,
}
impl PreparedDirectCompanionAcquisition {
    pub(crate) fn from_reservation(
        transaction: [u8; 16],
        source: &impl DirectCompanionAcquisitionSource,
    ) -> Result<Self> {
        let binding = binding(transaction, source);
        if !binding.valid() {
            return Err(SpellItemError::Rejected(
                "direct companion reservation source/scope",
            ));
        }
        Ok(Self { binding })
    }
    pub(super) fn valid_intent(&self, r: &SpellItemTransactionRequest) -> bool {
        self.binding.transaction == r.transaction_id
            && self.binding.valid()
            && r.companion.is_none()
            && r.caster_origin
                .as_ref()
                .is_some_and(|o| o.actor == self.binding.master)
    }
}
pub(crate) struct PendingDirectCompanionAcquisition {
    binding: DirectBinding,
}
impl PendingDirectCompanionAcquisition {
    pub(in crate::durability) fn matches_transaction(&self, t: &[u8; 16]) -> bool {
        &self.binding.transaction == t
    }
    pub(in crate::durability) async fn verify_staged_row(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> std::result::Result<(), DurabilityError> {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_direct_companion_acquisition_receipts d JOIN game_spell_item_receipts r USING(transaction_id) WHERE d.transaction_id=encode($1,'hex')::uuid AND d.binding_json=$2::text::jsonb AND d.created_xact_id=pg_current_xact_id() AND r.created_xact_id=d.created_xact_id AND convert_from(r.intent,'UTF8')::jsonb->'direct_companion'=d.binding_json)")
            .bind(self.binding.transaction.as_slice()).bind(self.binding.json().to_string()).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }
}
pub(crate) struct CommittedDirectCompanionAcquisition {
    binding: DirectBinding,
}
impl CommittedDirectCompanionAcquisition {
    pub(in crate::durability) fn after_successful_commit(
        p: PendingDirectCompanionAcquisition,
    ) -> Self {
        Self { binding: p.binding }
    }
    pub(crate) fn matches_reservation(&self, s: &impl DirectCompanionAcquisitionSource) -> bool {
        // This comparison is also used during infallible physical installation
        // after COMMIT. Borrow the exact prepared data instead of constructing
        // a new binding (which cloned definition/name strings).
        let b = &self.binding;
        b.actor == s.actor()
            && b.master == s.master_actor()
            && b.character == *s.character_id().as_bytes()
            && b.session == *s.game_session_id().as_bytes()
            && b.cell == s.cell()
            && b.creature_key == s.creature_definition_key()
            && b.creature_revision == s.creature_definition_revision()
            && &b.source == s.source()
    }
    pub(in crate::durability) async fn verify_committed_row(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        transaction: &[u8; 16],
        physical: &str,
    ) -> std::result::Result<(), DurabilityError> {
        if &self.binding.transaction != transaction {
            return Err(DurabilityError::InvalidStoredState);
        }
        let valid: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_direct_companion_acquisition_receipts d JOIN game_spell_item_receipts r USING(transaction_id) WHERE d.transaction_id=encode($1,'hex')::uuid AND d.binding_json=$2::text::jsonb AND d.created_xact_id::text=$3 AND d.created_xact_id<>pg_current_xact_id() AND pg_xact_status(d.created_xact_id)='committed' AND r.created_xact_id=d.created_xact_id AND convert_from(r.intent,'UTF8')::jsonb->'direct_companion'=d.binding_json)")
            .bind(transaction.as_slice()).bind(self.binding.json().to_string()).bind(physical)
            .fetch_one(&mut **tx).await?;
        if !valid {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }
}
pub(crate) async fn record_prepared_direct_companion_acquisition_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
) -> Result<PendingDirectCompanionAcquisition> {
    check_transaction(tx, authority).await?;
    if authority.due_read_only {
        return Err(SpellItemError::Rejected(
            "due-only direct acquisition mutation",
        ));
    }
    let b = request
        .direct_companion
        .as_ref()
        .ok_or(SpellItemError::Rejected(
            "missing sealed direct acquisition intent",
        ))?
        .binding
        .clone();
    if !b.current(authority, request) {
        return Err(SpellItemError::Rejected(
            "direct acquisition current source/caster binding",
        ));
    }
    sqlx::query("INSERT INTO game_spell_direct_companion_acquisition_receipts(transaction_id,world_id,channel_id,ownership_generation,actor_local_id,actor_local_generation,master_character_id,master_game_session_id,master_actor_local_id,master_actor_local_generation,creature_key,creature_revision,cell_x,cell_y,cell_z,source_kind,binding_json,actor_placement_identity,master_placement_identity,source_json) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5,$6::text::numeric(20,0),encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,$10::text::numeric(20,0),$11,$12,$13,$14,$15,$16,$17::text::jsonb,encode($18,'hex')::uuid,encode($19,'hex')::uuid,$20::text::jsonb)")
        .bind(b.transaction.as_slice()).bind(b.actor.world_id().as_bytes().as_slice()).bind(b.actor.channel_id().as_bytes().as_slice())
        .bind(b.actor.scope_generation().get().to_string()).bind(i64::from(b.actor.actor_local_id())).bind(b.actor.actor_local_generation().to_string())
        .bind(b.character.as_slice()).bind(b.session.as_slice()).bind(i64::from(b.master.actor_local_id())).bind(b.master.actor_local_generation().to_string())
        .bind(&b.creature_key).bind(&b.creature_revision).bind(b.cell.x).bind(b.cell.y).bind(b.cell.z)
        .bind(match &b.source {DirectCompanionSource::Named{..}=>1i16,DirectCompanionSource::Target{..}=>2i16})
        .bind(b.json().to_string()).bind(b.actor.placement_identity().as_slice()).bind(b.master.placement_identity().as_slice()).bind(b.source.json().to_string()).execute(&mut **tx).await?;
    Ok(PendingDirectCompanionAcquisition { binding: b })
}
pub(crate) async fn reconcile_committed_direct_companion_acquisition_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
) -> Result<Option<CommittedDirectCompanionAcquisition>> {
    check_transaction(tx, authority).await?;
    let b = request
        .direct_companion
        .as_ref()
        .ok_or(SpellItemError::Rejected(
            "missing exact historical direct acquisition request",
        ))?
        .binding
        .clone();
    if !b.current(authority, request) {
        return Err(SpellItemError::Rejected(
            "historical direct acquisition/current binding",
        ));
    }
    let actual:Option<serde_json::Value>=sqlx::query_scalar("SELECT d.binding_json FROM game_spell_direct_companion_acquisition_receipts d JOIN game_spell_item_receipts r USING(transaction_id) WHERE d.transaction_id=encode($1,'hex')::uuid AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric(20,0) AND d.created_xact_id<>pg_current_xact_id() AND pg_xact_status(d.created_xact_id)='committed' AND r.created_xact_id=d.created_xact_id AND convert_from(r.intent,'UTF8')::jsonb->'direct_companion'=d.binding_json")
        .bind(b.transaction.as_slice()).bind(authority.command.game_session_id().as_bytes().as_slice()).bind(authority.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    match actual {
        None => Ok(None),
        Some(value) if value == b.json() => {
            Ok(Some(CommittedDirectCompanionAcquisition { binding: b }))
        }
        Some(_) => Err(SpellItemError::Rejected(
            "substituted committed direct acquisition source",
        )),
    }
}
