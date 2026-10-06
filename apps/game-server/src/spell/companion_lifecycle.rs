//! Companion execution against the real Channel fixed-slot owner. Prepared snapshots are
//! immutable intent only: the root's fenced caster/cost/Item/timer compositor owns admission
//! and commits all participants in one owner turn. This module never admits client prototypes.
use super::delayed_execution::CastBinding;
use super::native::{CompiledNativeSpell, Facts, Plan};
use super::native_companions::{
    AcquisitionFacts, AcquisitionKind, AcquisitionPlan, CompanionFacts,
};
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, CompanionMaster, CompanionSnapshot, ExactActorRef,
    GameSessionId, MovementLocalPosition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompanionExecutionError {
    Actor(CarrierError),
    UnqualifiedProfile,
    InvalidOwnerFacts,
    InvalidMap,
    PlacementUnavailable,
    SnapshotMismatch,
    ArithmeticBounds,
    Refused(String),
}
impl From<CarrierError> for CompanionExecutionError {
    fn from(value: CarrierError) -> Self {
        Self::Actor(value)
    }
}

/// Current Character/account-owned caster facts. They are independently resolved by the root
/// both while preparing and before applying, never reconstructed from this prepared command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcquisitionOwnerFacts {
    pub(crate) mana: u64,
    pub(crate) has_infinite_mana: bool,
    pub(crate) can_summon_all: bool,
    pub(crate) can_convince_all: bool,
    pub(crate) black_skull: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AcquisitionSource {
    Named(String),
    Target(ExactActorRef),
}
#[derive(Debug, Clone)]
pub(crate) struct PreparedAcquisition {
    owner: ExactActorRef,
    session: GameSessionId,
    source: AcquisitionSource,
    owner_facts: AcquisitionOwnerFacts,
    owned: Vec<ExactActorRef>,
    target: Option<CompanionSnapshot>,
    position: Option<MovementLocalPosition>,
    plan: AcquisitionPlan,
    mana_cost: u32,
    definition_key: String,
}
impl PreparedAcquisition {
    pub(crate) fn mana_cost(&self) -> u32 {
        self.mana_cost
    }
    pub(crate) fn mana_to_deduct(&self) -> u32 {
        if self.owner_facts.has_infinite_mana {
            0
        } else {
            self.mana_cost
        }
    }
    pub(crate) fn mana_spent(&self) -> u32 {
        self.mana_cost
    }
    pub(crate) fn consume_rune_charge(&self) -> bool {
        self.plan.consume_rune_charge_on_success
    }
}

/// A direct acquisition joins a genuine physical reservation to the source
/// mana decision. It has no corpse or Familiar-state placeholder.
#[derive(Debug)]
pub(crate) struct DirectCompanionReservation {
    physical: DirectCompanionPhysical,
    owner: ExactActorRef,
    session: GameSessionId,
    character: crate::domain::CharacterId,
    current: AcquisitionOwnerFacts,
    owned: Vec<ExactActorRef>,
    source: crate::durability::spell_item_transaction::DirectCompanionSource,
    profile: CompiledNativeSpell,
    mana_cost: u32,
    consume_rune_charge: bool,
}
#[derive(Debug)]
enum DirectCompanionPhysical {
    Named(crate::foundation::PreparedCompanionSpawn),
    Target(crate::foundation::PreparedCompanionAssignment),
}
impl DirectCompanionReservation {
    pub(crate) fn qualifies_payment(
        &self,
        profile: &CompiledNativeSpell,
        current_mana: u32,
    ) -> bool {
        &self.profile == profile && self.current.mana == u64::from(current_mana)
    }
    pub(crate) fn restrictions(&self) -> &AcquisitionOwnerFacts {
        &self.current
    }
    pub(crate) fn mana_to_deduct(&self) -> u32 {
        if self.current.has_infinite_mana {
            0
        } else {
            self.mana_cost
        }
    }
    /// Both source scripts call addManaSpent(cost), including InfiniteMana.
    pub(crate) fn mana_spent(&self) -> u32 {
        self.mana_cost
    }
    pub(crate) fn consume_rune_charge(&self) -> bool {
        self.consume_rune_charge
    }
    pub(crate) fn assignment(&self) -> Option<&crate::foundation::PreparedCompanionAssignment> {
        match &self.physical {
            DirectCompanionPhysical::Target(value) => Some(value),
            _ => None,
        }
    }
    pub(crate) fn actor(&self) -> ExactActorRef {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => p.actor(),
            DirectCompanionPhysical::Target(p) => p.actor(),
        }
    }
    pub(crate) fn master_actor(&self) -> ExactActorRef {
        self.owner
    }
    pub(crate) fn character_id(&self) -> crate::domain::CharacterId {
        self.character
    }
    pub(crate) fn game_session_id(&self) -> GameSessionId {
        self.session
    }
    pub(crate) fn position(&self) -> MovementLocalPosition {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => p.position(),
            DirectCompanionPhysical::Target(p) => p.position(),
        }
    }
    pub(crate) fn creature_definition_key(&self) -> &str {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => &p.policy().definition_key,
            DirectCompanionPhysical::Target(p) => &p.policy().definition_key,
        }
    }
    pub(crate) fn creature_definition_revision(&self) -> &str {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => &p.policy().definition_revision,
            DirectCompanionPhysical::Target(p) => &p.policy().definition_revision,
        }
    }
    pub(crate) fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        current: &AcquisitionOwnerFacts,
    ) -> Result<(), CompanionExecutionError> {
        if current != &self.current || owned_ids(runtime, self.owner, self.session)? != self.owned {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        match &self.physical {
            DirectCompanionPhysical::Named(p) => runtime.validate_companion_spawn(p)?,
            DirectCompanionPhysical::Target(p) => runtime.validate_companion_assignment(p)?,
        }
        Ok(())
    }
    pub(crate) fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.current)?;
        if let DirectCompanionPhysical::Named(p) = &mut self.physical {
            runtime.reserve_companion_spawn(p)?;
        }
        Ok(())
    }
    pub(crate) fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        if let DirectCompanionPhysical::Named(p) = &self.physical {
            runtime.rollback_companion_spawn(p)?;
        }
        Ok(())
    }
    /// Target master changes are installed by the shared combat batch. The
    /// Named branch moves its already reserved slot only after actual COMMIT.
    pub(crate) fn install_prevalidated(
        self,
        runtime: &mut ChannelRuntimeV1,
        receipt: &crate::durability::spell_item_transaction::CommittedDirectCompanionAcquisition,
    ) -> Result<(), CompanionExecutionError> {
        if !receipt.matches_reservation(&self) {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        if let DirectCompanionPhysical::Named(p) = self.physical {
            runtime.install_companion_spawn(p)?;
        }
        Ok(())
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
pub(crate) fn prepare_direct_acquisition(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    owner: ExactActorRef,
    session: GameSessionId,
    character: crate::domain::CharacterId,
    spell: &CompiledNativeSpell,
    source: AcquisitionSource,
    current: &AcquisitionOwnerFacts,
) -> Result<DirectCompanionReservation, CompanionExecutionError> {
    prepare_direct_acquisition_excluding(
        runtime,
        cells,
        owner,
        session,
        character,
        spell,
        source,
        current,
        &[],
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
pub(crate) fn prepare_direct_acquisition_excluding(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    owner: ExactActorRef,
    session: GameSessionId,
    character: crate::domain::CharacterId,
    spell: &CompiledNativeSpell,
    source: AcquisitionSource,
    current: &AcquisitionOwnerFacts,
    excluded: &[MovementLocalPosition],
) -> Result<DirectCompanionReservation, CompanionExecutionError> {
    let p = parameters(spell, "acquire_summon")?;
    if p["refuse_black_skull"] != false
        || !matches!(
            (p["source"].as_str(), &source),
            (Some("named_creature"), AcquisitionSource::Named(_))
                | (Some("target_creature"), AcquisitionSource::Target(_))
        )
    {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    let prepared = prepare_acquisition_excluding(
        runtime, cells, owner, session, spell, source, current, excluded,
    )?;
    let (physical, source) = match prepared.source {
        AcquisitionSource::Named(name) => {
            let position = prepared
                .position
                .ok_or(CompanionExecutionError::PlacementUnavailable)?;
            let spawn = runtime.prepare_companion_spawn(
                owner,
                session,
                &prepared.definition_key,
                position,
                None,
                None,
                0,
                0,
                false,
            )?;
            (
                DirectCompanionPhysical::Named(spawn),
                crate::durability::spell_item_transaction::DirectCompanionSource::Named {
                    requested_name: name,
                },
            )
        }
        AcquisitionSource::Target(_) => {
            let target = prepared
                .target
                .ok_or(CompanionExecutionError::SnapshotMismatch)?;
            use sha2::{Digest, Sha256};
            let digest = Sha256::digest(format!("{target:?}").as_bytes());
            let mut before_snapshot_digest = [0; 32];
            before_snapshot_digest.copy_from_slice(&digest);
            let prior = target.state.master;
            let assignment = runtime.prepare_companion_assignment(owner, session, &target)?;
            (
                DirectCompanionPhysical::Target(assignment),
                crate::durability::spell_item_transaction::DirectCompanionSource::Target {
                    before_snapshot_digest,
                    prior_master: prior.map(|m| m.actor),
                    prior_master_session: prior.map(|m| m.session),
                },
            )
        }
    };
    Ok(DirectCompanionReservation {
        physical,
        owner,
        session,
        character,
        current: current.clone(),
        owned: prepared.owned,
        source,
        profile: spell.clone(),
        mana_cost: prepared.mana_cost,
        consume_rune_charge: prepared.plan.consume_rune_charge_on_success,
    })
}
impl crate::durability::spell_item_transaction::direct_companion_source_seal::Sealed
    for DirectCompanionReservation
{
}
impl crate::durability::spell_item_transaction::DirectCompanionAcquisitionSource
    for DirectCompanionReservation
{
    fn actor(&self) -> ExactActorRef {
        self.actor()
    }
    fn master_actor(&self) -> ExactActorRef {
        self.owner
    }
    fn character_id(&self) -> crate::domain::CharacterId {
        self.character
    }
    fn game_session_id(&self) -> GameSessionId {
        self.session
    }
    fn cell(&self) -> crate::durability::spell_items_abi::SpellItemCell {
        let p = self.position();
        crate::durability::spell_items_abi::SpellItemCell {
            x: p.x,
            y: p.y,
            z: i32::from(p.floor),
        }
    }
    fn creature_definition_key(&self) -> &str {
        self.creature_definition_key()
    }
    fn creature_definition_revision(&self) -> &str {
        self.creature_definition_revision()
    }
    fn source(&self) -> &crate::durability::spell_item_transaction::DirectCompanionSource {
        &self.source
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcquisitionReceipt {
    pub(crate) creature: ExactActorRef,
    pub(crate) mana_cost: u32,
    pub(crate) mana_spent: u32,
    pub(crate) consume_rune_charge: bool,
}

fn parameters<'a>(
    spell: &'a CompiledNativeSpell,
    key: &str,
) -> Result<&'a serde_json::Value, CompanionExecutionError> {
    let behavior = &spell.spell()["execution"]["native_behavior"];
    if behavior["key"] != key {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    Ok(&behavior["parameters"])
}
fn owned_ids(
    runtime: &ChannelRuntimeV1,
    owner: ExactActorRef,
    session: GameSessionId,
) -> Result<Vec<ExactActorRef>, CompanionExecutionError> {
    Ok(runtime
        .owned_companions(owner, session)?
        .into_iter()
        .map(|v| v.actor)
        .collect())
}
/// Actual map collision + active artifact identity + physical occupancy. Creation tries the
/// source cell then adjacent cells for extended placement; no unknown cell becomes walkable.
fn placement(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    anchor: MovementLocalPosition,
    extended: bool,
) -> Result<Option<MovementLocalPosition>, CompanionExecutionError> {
    placement_excluding(runtime, cells, anchor, extended, &[])
}
fn placement_excluding(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    anchor: MovementLocalPosition,
    extended: bool,
    excluded: &[MovementLocalPosition],
) -> Result<Option<MovementLocalPosition>, CompanionExecutionError> {
    let scope = cells.scope();
    if scope.world_id != runtime.binding().world_id()
        || scope.generation_digest != runtime.content_pin().server_artifact_digest()
    {
        return Err(CompanionExecutionError::InvalidMap);
    }
    let offsets: &[(i32, i32)] = if extended {
        &[
            (0, 0),
            (-1, 0),
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ]
    } else {
        &[(0, 0)]
    };
    for &(dx, dy) in offsets {
        let Some(x) = anchor.x.checked_add(dx) else {
            continue;
        };
        let Some(y) = anchor.y.checked_add(dy) else {
            continue;
        };
        let position = MovementLocalPosition {
            x,
            y,
            floor: anchor.floor,
        };
        if excluded.contains(&position) {
            continue;
        }
        if matches!(
            cells.index().lookup(
                scope,
                LogicalCell {
                    x,
                    y,
                    z: i32::from(anchor.floor)
                }
            ),
            Ok(CollisionClass::Walkable)
        ) && runtime.companion_creation_available(position)?
        {
            return Ok(Some(position));
        }
    }
    Ok(None)
}

pub(crate) fn prepare_acquisition(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    owner: ExactActorRef,
    session: GameSessionId,
    spell: &CompiledNativeSpell,
    source: AcquisitionSource,
    current: &AcquisitionOwnerFacts,
) -> Result<PreparedAcquisition, CompanionExecutionError> {
    prepare_acquisition_excluding(runtime, cells, owner, session, spell, source, current, &[])
}
#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
fn prepare_acquisition_excluding(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    owner: ExactActorRef,
    session: GameSessionId,
    spell: &CompiledNativeSpell,
    source: AcquisitionSource,
    current: &AcquisitionOwnerFacts,
    excluded: &[MovementLocalPosition],
) -> Result<PreparedAcquisition, CompanionExecutionError> {
    let p = parameters(spell, "acquire_summon")?;
    let owned = owned_ids(runtime, owner, session)?;
    let (kind, policy, target, position) = match &source {
        AcquisitionSource::Named(name) => {
            let policy = runtime.companion_policy(name)?;
            let anchor = runtime.read_actor_position(owner)?.position();
            let position = placement_excluding(
                runtime,
                cells,
                anchor,
                p["spawn"]["extended"]
                    .as_bool()
                    .ok_or(CompanionExecutionError::UnqualifiedProfile)?,
                excluded,
            )?;
            (AcquisitionKind::NamedCreature, policy, None, position)
        }
        AcquisitionSource::Target(actor) => {
            let target = runtime.companion_snapshot(*actor)?;
            (
                AcquisitionKind::TargetCreature,
                target.state.policy.clone(),
                Some(target),
                None,
            )
        }
    };
    // An absent cost is unknown. Bypass permissions do not invent zero-cost definitions.
    let cost = policy
        .mana_cost
        .ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
    let facts = AcquisitionFacts {
        kind,
        creature_name: policy.display_name.clone(),
        type_found: true,
        summonable: policy.summonable,
        target_is_monster: target.is_some(),
        convinceable: policy.convinceable,
        target_master_name: target
            .as_ref()
            .and_then(|t| t.state.master)
            .map(|_| "Player".to_owned()),
        tile_present: false,
        top_down_item_present: false,
        is_corpse: false,
        corpse_movable: false,
        owned_summons: owned.len(),
        can_summon_all: current.can_summon_all,
        can_convince_all: current.can_convince_all,
        black_skull: current.black_skull,
        mana: current.mana,
        creature_mana_cost: cost,
        has_infinite_mana: current.has_infinite_mana,
        spawn_room: position.is_some(),
    };
    let plan = match spell
        .plan(
            Facts::Companion(&CompanionFacts::Acquisition(facts)),
            &mut |min, _| min,
        )
        .map_err(|e| CompanionExecutionError::Refused(e.0))?
    {
        Plan::Companion(plan) => match *plan {
            super::native_companions::CompanionPlan::Acquisition(plan) => plan,
            _ => return Err(CompanionExecutionError::UnqualifiedProfile),
        },
        _ => return Err(CompanionExecutionError::UnqualifiedProfile),
    };
    Ok(PreparedAcquisition {
        owner,
        session,
        source,
        owner_facts: current.clone(),
        owned,
        target,
        position,
        plan,
        mana_cost: cost,
        definition_key: policy.definition_key.clone(),
    })
}
/// Root calls this only after common costs/training/Item preflight and under its exclusive
/// current owner turn. The typed receipt carries dynamic mana once; generic Summon cost zero
/// must be replaced with this qualified definition cost by the root, never charged twice.
pub(crate) fn commit_acquisition(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    prepared: PreparedAcquisition,
    current: &AcquisitionOwnerFacts,
) -> Result<AcquisitionReceipt, CompanionExecutionError> {
    if current != &prepared.owner_facts
        || owned_ids(runtime, prepared.owner, prepared.session)? != prepared.owned
    {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    let creature = match prepared.source {
        AcquisitionSource::Named(_) => {
            let position = prepared
                .position
                .ok_or(CompanionExecutionError::PlacementUnavailable)?;
            if placement(runtime, cells, position, false)? != Some(position) {
                return Err(CompanionExecutionError::PlacementUnavailable);
            }
            runtime.create_companion(
                prepared.owner,
                prepared.session,
                &prepared.definition_key,
                position,
                None,
                None,
                0,
            )?
        }
        AcquisitionSource::Target(_) => {
            let target = prepared
                .target
                .ok_or(CompanionExecutionError::SnapshotMismatch)?;
            runtime
                .compare_assign_companion(prepared.owner, prepared.session, &target)?
                .actor
        }
    };
    Ok(AcquisitionReceipt {
        creature,
        mana_cost: prepared.mana_cost,
        mana_spent: prepared.mana_cost,
        consume_rune_charge: prepared.plan.consume_rune_charge_on_success,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FamiliarTimerEvent {
    Expiry,
    Warning { index: usize },
}
#[derive(Debug, Clone)]
pub(crate) struct SavedFamiliarTimer {
    pub(crate) binding: CastBinding,
    pub(crate) creature: ExactActorRef,
    pub(crate) lifecycle_epoch: u64,
    pub(crate) event: FamiliarTimerEvent,
}
#[derive(Debug, Clone)]
pub(crate) struct StagedFamiliarOperation {
    owner: ExactActorRef,
    session: GameSessionId,
    expected: CompanionSnapshot,
    event: FamiliarTimerEvent,
    message: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamiliarTimerReceipt {
    pub(crate) creature: ExactActorRef,
    pub(crate) removed: bool,
    pub(crate) message: Option<String>,
}

pub(crate) fn stage_familiar_timer(
    runtime: &ChannelRuntimeV1,
    saved: &SavedFamiliarTimer,
) -> Result<Option<StagedFamiliarOperation>, CompanionExecutionError> {
    stage_familiar_timer_parts(
        runtime,
        &saved.binding.spell,
        saved.binding.caster,
        saved.binding.command.game_session_id(),
        saved.creature,
        saved.lifecycle_epoch,
        saved.event,
    )
}
fn stage_familiar_timer_parts(
    runtime: &ChannelRuntimeV1,
    spell: &CompiledNativeSpell,
    owner: ExactActorRef,
    session: GameSessionId,
    creature: ExactActorRef,
    epoch: u64,
    event: FamiliarTimerEvent,
) -> Result<Option<StagedFamiliarOperation>, CompanionExecutionError> {
    let p = parameters(spell, "familiar_summon")?;
    if runtime.player_control_facts(owner, session).is_err() {
        return Ok(None);
    }
    let snapshot = match runtime.companion_snapshot(creature) {
        Ok(value) => value,
        Err(
            CarrierError::StaleActorGeneration
            | CarrierError::CreatureNotActionable
            | CarrierError::NotCreature,
        ) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    // Foreign pending SQL owns this actual target. Return a blocked event so
    // the one real timer lane retains this occurrence for a later owner cycle.
    runtime.assert_actor_spell_unreserved(creature)?;
    if snapshot.state.master
        != Some(CompanionMaster {
            actor: owner,
            session,
        })
        || !snapshot.state.policy.is_familiar
        || snapshot.state.lifecycle_epoch != epoch
    {
        return Ok(None);
    }
    if p["creature_name"].as_str() != Some(snapshot.state.policy.display_name.as_str()) {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    let message = match event {
        FamiliarTimerEvent::Expiry => None,
        FamiliarTimerEvent::Warning { index } => {
            let warning = p["warnings"]
                .as_array()
                .and_then(|v| v.get(index))
                .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
            Some(format!(
                "{}{}",
                p["warning_dispatch"]["prefix"]
                    .as_str()
                    .ok_or(CompanionExecutionError::UnqualifiedProfile)?,
                warning["message"]
                    .as_str()
                    .ok_or(CompanionExecutionError::UnqualifiedProfile)?
            ))
        }
    };
    Ok(Some(StagedFamiliarOperation {
        owner,
        session,
        expected: snapshot,
        event,
        message,
    }))
}
pub(crate) fn commit_familiar_timer(
    runtime: &mut ChannelRuntimeV1,
    staged: StagedFamiliarOperation,
) -> Result<FamiliarTimerReceipt, CompanionExecutionError> {
    runtime.player_control_facts(staged.owner, staged.session)?;
    runtime.assert_actor_spell_unreserved(staged.expected.actor)?;
    runtime.validate_companion_snapshot(&staged.expected)?;
    let removed = staged.event == FamiliarTimerEvent::Expiry;
    if removed {
        runtime.despawn_companion(staged.owner, staged.session, &staged.expected)?;
    }
    Ok(FamiliarTimerReceipt {
        creature: staged.expected.actor,
        removed,
        message: staged.message,
    })
}

use super::native_companions::{FamiliarAction, FamiliarEvent, FamiliarFacts};
use crate::durability::character_familiar::{
    CommittedCharacterFamiliar, DurableFamiliarState, FamiliarStateOccurrence,
    FamiliarStateRequest, FamiliarUnixTime,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::PreparedCompanionSpawn;
use crate::foundation::owner_timer::SemanticTimeMicros;

/// These values come from the current Character/account/config owners. The source's Unix
/// lifetime is a separately sealed database-clock sample, never the monotonic timer clock.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FamiliarOwnerFacts {
    pub(crate) vocation: String,
    pub(crate) premium: bool,
    pub(crate) level: u32,
    pub(crate) account_at_least_god: bool,
    pub(crate) current_speed: i32,
    pub(crate) familiar_minutes: i64,
    pub(crate) vip: bool,
    pub(crate) vip_reduction_minutes: i64,
    pub(crate) cooldown_rate: f32,
    pub(crate) state: DurableFamiliarState,
}
#[derive(Debug, Clone)]
pub(crate) struct PreparedFamiliar {
    binding: CastBinding,
    event: FamiliarEvent,
    owner_facts: FamiliarOwnerFacts,
    owned: Vec<CompanionSnapshot>,
    death: Option<CompanionSnapshot>,
    spawn: Option<PreparedCompanionSpawn>,
    actions: Vec<FamiliarAction>,
    facts: FamiliarFacts,
    after: DurableFamiliarState,
}
impl PreparedFamiliar {
    /// Frozen real owner census for metadata reservations. Binding these reads grants
    /// no current authority; the physical batch independently compares each actual slot.
    pub(crate) fn reservation_snapshots(&self) -> &[CompanionSnapshot] {
        &self.owned
    }
    pub(crate) fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.owner_facts)?;
        for owned in &self.owned {
            runtime.assert_actor_spell_unreserved(owned.actor)?;
        }
        if let Some(dead) = &self.death {
            runtime.assert_actor_spell_unreserved(dead.actor)?;
        }
        if let Some(spawn) = self.spawn.as_mut() {
            runtime.reserve_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub(crate) fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        if let Some(spawn) = self.spawn.as_ref() {
            runtime.rollback_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub(crate) fn matches_saved(&self, saved: &SavedFamiliarTimer) -> bool {
        self.spawn
            .as_ref()
            .is_some_and(|spawn| spawn.actor() == saved.creature)
            && self.after.lifecycle_epoch == saved.lifecycle_epoch
            && same_binding(&self.binding, &saved.binding)
            && match saved.event {
                FamiliarTimerEvent::Expiry => self
                    .actions
                    .iter()
                    .any(|a| matches!(a, FamiliarAction::ScheduleExpiry { .. })),
                FamiliarTimerEvent::Warning { index } => self
                    .actions
                    .iter()
                    .any(|a| matches!(a,FamiliarAction::ScheduleWarning {index:i,..} if *i==index)),
            }
    }
    pub(crate) fn binding(&self) -> &CastBinding {
        &self.binding
    }
    pub(crate) fn event(&self) -> &FamiliarEvent {
        &self.event
    }
    pub(crate) fn facts(&self) -> &FamiliarFacts {
        &self.facts
    }
    pub(crate) fn owner_facts(&self) -> &FamiliarOwnerFacts {
        &self.owner_facts
    }
    pub(crate) fn cooldown(&self) -> Option<(u32, i32, bool)> {
        self.actions.iter().find_map(|action| match action {
            FamiliarAction::Cooldown {
                reference_spell_id,
                ticks_ms,
                pauses_offline,
            } => Some((*reference_spell_id, *ticks_ms, *pauses_offline)),
            _ => None,
        })
    }
    /// A no-op hook has no durable mutation. A changed snapshot requires its genuine typed
    /// receipt before any physical spawn, timer/cooldown or caster-cost successor is installed.
    pub(crate) fn state_request(
        &self,
        occurrence: FamiliarStateOccurrence,
        content_revision: String,
        policy_revision: String,
        policy_digest: [u8; 32],
    ) -> Option<FamiliarStateRequest> {
        (self.owner_facts.state != self.after).then(|| FamiliarStateRequest {
            occurrence,
            before: self.owner_facts.state.clone(),
            after: self.after.clone(),
            content_revision,
            policy_revision,
            policy_digest,
        })
    }
    pub(crate) fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    pub(crate) fn creates_creature(&self) -> bool {
        self.spawn.is_some()
    }
    pub(crate) fn caster_mana_cost(&self) -> Result<u32, CompanionExecutionError> {
        if self.event != FamiliarEvent::Cast {
            return Ok(0);
        }
        let mana = self.binding.spell.spell()["costs"]["mana"]
            .as_u64()
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
        u32::try_from(mana).map_err(|_| CompanionExecutionError::ArithmeticBounds)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_familiar(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    binding: CastBinding,
    event: FamiliarEvent,
    current: &FamiliarOwnerFacts,
    unix: FamiliarUnixTime,
    death_actor: Option<ExactActorRef>,
) -> Result<PreparedFamiliar, CompanionExecutionError> {
    prepare_familiar_inner(
        runtime,
        Some(cells),
        binding,
        event,
        current,
        unix,
        death_actor,
    )
}
/// Advance/death are actual owner events and need no map-placement capability. This entry
/// cannot recreate or cast a creature, so a hook never fills an absent map with fake cells.
pub(crate) fn prepare_familiar_hook(
    runtime: &ChannelRuntimeV1,
    binding: CastBinding,
    event: FamiliarEvent,
    current: &FamiliarOwnerFacts,
    unix: FamiliarUnixTime,
    death_actor: Option<ExactActorRef>,
) -> Result<PreparedFamiliar, CompanionExecutionError> {
    if !matches!(event, FamiliarEvent::Advance | FamiliarEvent::FamiliarDeath) {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    prepare_familiar_inner(runtime, None, binding, event, current, unix, death_actor)
}
#[allow(clippy::too_many_arguments)]
fn prepare_familiar_inner(
    runtime: &ChannelRuntimeV1,
    cells: Option<&NativeEntryMovementCells>,
    binding: CastBinding,
    event: FamiliarEvent,
    current: &FamiliarOwnerFacts,
    unix: FamiliarUnixTime,
    death_actor: Option<ExactActorRef>,
) -> Result<PreparedFamiliar, CompanionExecutionError> {
    let mut parts = prepare_familiar_parts(
        runtime,
        cells,
        &binding.spell,
        binding.caster,
        binding.command.game_session_id(),
        event,
        current,
        unix,
        death_actor,
    )?;
    if let Some(spawn) = parts.spawn.as_mut() {
        spawn.bind_semantic_creation(binding.cast_at.get())?;
    }
    Ok(PreparedFamiliar {
        binding,
        event: parts.event,
        owner_facts: parts.owner_facts,
        owned: parts.owned,
        death: parts.death,
        spawn: parts.spawn,
        actions: parts.actions,
        facts: parts.facts,
        after: parts.after,
    })
}

#[derive(Debug, Clone)]
struct FamiliarParts {
    event: FamiliarEvent,
    owner_facts: FamiliarOwnerFacts,
    owned: Vec<CompanionSnapshot>,
    death: Option<CompanionSnapshot>,
    spawn: Option<PreparedCompanionSpawn>,
    actions: Vec<FamiliarAction>,
    facts: FamiliarFacts,
    after: DurableFamiliarState,
}
#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
fn prepare_familiar_parts(
    runtime: &ChannelRuntimeV1,
    cells: Option<&NativeEntryMovementCells>,
    spell: &CompiledNativeSpell,
    owner: ExactActorRef,
    session: GameSessionId,
    event: FamiliarEvent,
    current: &FamiliarOwnerFacts,
    unix: FamiliarUnixTime,
    death_actor: Option<ExactActorRef>,
) -> Result<FamiliarParts, CompanionExecutionError> {
    if !matches!(
        event,
        FamiliarEvent::Cast
            | FamiliarEvent::Login
            | FamiliarEvent::Advance
            | FamiliarEvent::FamiliarDeath
    ) || current.state.validate().is_err()
        || current.current_speed < 0
    {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    let p = parameters(spell, "familiar_summon")?;
    runtime.player_control_facts(owner, session)?;
    let owned = runtime.owned_companions(owner, session)?;
    let name = p["creature_name"]
        .as_str()
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    let policy = runtime.companion_policy(name)?;
    if !policy.is_familiar {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    let default_look = p["default_look_type"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    let death = if event == FamiliarEvent::FamiliarDeath {
        let actor = death_actor.ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
        let snapshot = runtime.companion_snapshot_including_dead(actor)?;
        if snapshot.health != 0
            || snapshot.state.master
                != Some(CompanionMaster {
                    actor: owner,
                    session,
                })
            || !snapshot.state.policy.is_familiar
            || snapshot.state.policy.definition_key != policy.definition_key
            || snapshot.state.lifecycle_epoch != current.state.lifecycle_epoch
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        Some(snapshot)
    } else {
        if death_actor.is_some() {
            return Err(CompanionExecutionError::InvalidOwnerFacts);
        }
        None
    };
    let position = if matches!(event, FamiliarEvent::Cast | FamiliarEvent::Login) {
        placement(
            runtime,
            cells.ok_or(CompanionExecutionError::InvalidMap)?,
            runtime.read_actor_position(owner)?.position(),
            p["spawn"]["extended"]
                .as_bool()
                .ok_or(CompanionExecutionError::UnqualifiedProfile)?,
        )?
    } else {
        None
    };
    let facts = FamiliarFacts {
        event: event.clone(),
        vocation: current.vocation.clone(),
        premium: current.premium,
        level: current.level,
        account_at_least_god: current.account_at_least_god,
        owned_summons: owned.len(),
        spawn_room: position.is_some(),
        chosen_look: current.state.selected_look,
        has_vocation_look: current
            .state
            .granted_looks
            .binary_search(&default_look)
            .is_ok(),
        owner_current_speed: current.current_speed,
        familiar_base_speed: policy.base_speed,
        familiar_minutes: current.familiar_minutes,
        vip: current.vip,
        vip_reduction_minutes: current.vip_reduction_minutes,
        cooldown_rate: current.cooldown_rate,
        now_unix: unix.seconds(),
        saved_expiry_unix: current.state.saved_expiry_unix,
        last_logout_unix: current.state.last_logout_unix,
        owner_present: true,
        creature_present: death.is_some(),
        creature_name: name.to_owned(),
        matching_summon_ids: Vec::new(),
        dx: 0,
        dy: 0,
        dz: 0,
        owner_tile_is_teleport: false,
    };
    let actions = match spell
        .plan(
            Facts::Companion(&CompanionFacts::Familiar(facts.clone())),
            &mut |min, _| min,
        )
        .map_err(|e| CompanionExecutionError::Refused(e.0))?
    {
        Plan::Companion(plan) => match *plan {
            super::native_companions::CompanionPlan::Familiar(actions) => actions,
            _ => return Err(CompanionExecutionError::UnqualifiedProfile),
        },
        _ => return Err(CompanionExecutionError::UnqualifiedProfile),
    };
    let mut after = current.state.clone();
    let mut look = None;
    let mut speed_delta = 0;
    let party = actions.contains(&FamiliarAction::RegisterPartyProtectionAllOwnedSummons);
    for action in &actions {
        match action {
            FamiliarAction::SelectLook(value) => after.selected_look = *value,
            FamiliarAction::GrantVocationLook(value) => {
                if let Err(index) = after.granted_looks.binary_search(value) {
                    after.granted_looks.insert(index, *value);
                }
            }
            FamiliarAction::RemoveVocationLook(value) => after.granted_looks.retain(|v| v != value),
            FamiliarAction::StoreExpiry(value) => after.saved_expiry_unix = *value,
            FamiliarAction::Create {
                name: created,
                look: value,
                extended: _,
                force: false,
                owned: true,
            } if created == name => {
                if look.replace(*value).is_some() {
                    return Err(CompanionExecutionError::UnqualifiedProfile);
                }
            }
            FamiliarAction::Create { .. } => {
                return Err(CompanionExecutionError::UnqualifiedProfile);
            }
            FamiliarAction::ChangeSpeed(value) => speed_delta = *value,
            _ => {}
        }
    }
    let spawn = if let Some(look) = look {
        after.lifecycle_epoch = after
            .lifecycle_epoch
            .checked_add(1)
            .ok_or(CompanionExecutionError::ArithmeticBounds)?;
        after.familiar_definition = Some(policy.definition_key.clone());
        after.familiar_revision = Some(policy.definition_revision.clone());
        after.profile_revision = spell.spell()["identity"]["revision"]
            .as_str()
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?
            .to_owned();
        Some(runtime.prepare_companion_spawn(
            owner,
            session,
            &policy.definition_key,
            position.ok_or(CompanionExecutionError::PlacementUnavailable)?,
            Some(look),
            Some(after.saved_expiry_unix),
            after.lifecycle_epoch,
            speed_delta,
            party,
        )?)
    } else {
        None
    };
    if event == FamiliarEvent::Cast {
        let Some((reference_spell_id, ticks_ms)) = actions.iter().find_map(|action| match action {
            FamiliarAction::Cooldown {
                reference_spell_id,
                ticks_ms,
                ..
            } => Some((*reference_spell_id, *ticks_ms)),
            _ => None,
        }) else {
            return Err(CompanionExecutionError::UnqualifiedProfile);
        };
        let snapshot = crate::durability::character_familiar::FamiliarCooldownSnapshot {
            reference_spell_id,
            remaining_micros: if ticks_ms == -1 {
                u64::MAX
            } else {
                u64::from(ticks_ms.max(0) as u32) * 1000
            },
        };
        match after
            .cooldowns
            .binary_search_by_key(&reference_spell_id, |v| v.reference_spell_id)
        {
            Ok(index) => after.cooldowns[index] = snapshot,
            Err(index) => after.cooldowns.insert(index, snapshot),
        }
    }
    after
        .validate()
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    Ok(FamiliarParts {
        event,
        owner_facts: current.clone(),
        owned,
        death,
        spawn,
        actions,
        facts,
        after,
    })
}

/// Typed timer producers retain the actual cast owner/occurrence and exact future fixed-slot
/// incarnation. The common compositor reserves these in the real OwnerTimerLane before commit.
#[derive(Debug, Clone)]
pub(crate) struct FamiliarSchedule {
    pub(crate) phase: u16,
    pub(crate) due: SemanticTimeMicros,
    pub(crate) saved: SavedFamiliarTimer,
}
impl PreparedFamiliar {
    pub(crate) fn schedules(
        &self,
        now: SemanticTimeMicros,
        first_phase: u16,
    ) -> Result<Vec<FamiliarSchedule>, CompanionExecutionError> {
        let Some(spawn) = &self.spawn else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for action in &self.actions {
            let (delay_ms, event) = match action {
                FamiliarAction::ScheduleExpiry { delay_ms } => {
                    (*delay_ms, FamiliarTimerEvent::Expiry)
                }
                FamiliarAction::ScheduleWarning {
                    index, delay_ms, ..
                } => (*delay_ms, FamiliarTimerEvent::Warning { index: *index }),
                _ => continue,
            };
            let phase = first_phase
                .checked_add(
                    u16::try_from(out.len())
                        .map_err(|_| CompanionExecutionError::ArithmeticBounds)?,
                )
                .ok_or(CompanionExecutionError::ArithmeticBounds)?;
            let due = now
                .get()
                .checked_add(
                    u64::from(delay_ms)
                        .checked_mul(1000)
                        .ok_or(CompanionExecutionError::ArithmeticBounds)?,
                )
                .ok_or(CompanionExecutionError::ArithmeticBounds)?;
            out.push(FamiliarSchedule {
                phase,
                due: SemanticTimeMicros::from_micros(due),
                saved: SavedFamiliarTimer {
                    binding: self.binding.clone(),
                    creature: spawn.actor(),
                    lifecycle_epoch: self.after.lifecycle_epoch,
                    event,
                },
            });
        }
        Ok(out)
    }
    /// Validate again while every owner participant remains locked, BEFORE the durable write.
    /// Afterwards the common compositor keeps these locks until all successor installs finish.
    pub(crate) fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        current: &FamiliarOwnerFacts,
    ) -> Result<(), CompanionExecutionError> {
        if &self.owner_facts != current {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        runtime.validate_owned_companions(
            self.binding.caster,
            self.binding.command.game_session_id(),
            &self.owned,
        )?;
        runtime
            .player_control_facts(self.binding.caster, self.binding.command.game_session_id())?;
        if let Some(death) = &self.death {
            runtime.validate_companion_snapshot(death)?;
        }
        if let Some(spawn) = &self.spawn {
            runtime.validate_companion_spawn(spawn)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub(crate) struct FamiliarApplyReceipt {
    creature: Option<ExactActorRef>,
    after: DurableFamiliarState,
    binding: CastBinding,
    /// Only source presentation, hook registration, cooldown and cancellation operations remain;
    /// physical spawn/master/speed/party state and durable appearance/lifetime are already owned.
    owner_operations: Vec<FamiliarAction>,
}
fn same_binding(a: &CastBinding, b: &CastBinding) -> bool {
    a.spell.spell() == b.spell.spell()
        && a.caster == b.caster
        && a.attacker == b.attacker
        && a.command == b.command
        && a.parent_binding == b.parent_binding
        && a.occurrence == b.occurrence
        && a.cast_at == b.cast_at
        && a.cast_position == b.cast_position
        && a.cast_snapshot == b.cast_snapshot
}
impl FamiliarApplyReceipt {
    pub(crate) fn creature(&self) -> Option<ExactActorRef> {
        self.creature
    }
    pub(crate) fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    pub(crate) fn owner_operations(&self) -> &[FamiliarAction] {
        &self.owner_operations
    }
    pub(crate) fn matches_saved(&self, saved: &SavedFamiliarTimer) -> bool {
        self.creature == Some(saved.creature)
            && self.after.lifecycle_epoch == saved.lifecycle_epoch
            && same_binding(&self.binding, &saved.binding) && match saved.event {
            FamiliarTimerEvent::Expiry => true,
            FamiliarTimerEvent::Warning { index } => {
                self.binding.spell.spell()["execution"]["native_behavior"]["parameters"]["warnings"]
                    .as_array()
                    .is_some_and(|v| index < v.len())
            }
        }
    }
}
/// This typed receipt is a durable intent match, not renewed authority. Root must establish
/// fresh Character/scope/session authority and hold the physical/caster/condition/timer owners
/// together. No raw `durable_confirmed` flag can authorize a familiar spawn.
pub(crate) fn commit_familiar(
    runtime: &mut ChannelRuntimeV1,
    prepared: PreparedFamiliar,
    current: &FamiliarOwnerFacts,
    original_fence: &CurrentCharacterGameplayFence,
    request: Option<&FamiliarStateRequest>,
    committed: Option<&CommittedCharacterFamiliar>,
) -> Result<FamiliarApplyReceipt, CompanionExecutionError> {
    prepared.validate_current(runtime, current)?;
    if original_fence.character_id.as_bytes() != prepared.binding.attacker.as_bytes()
        || original_fence.game_session_id != prepared.binding.command.game_session_id()
    {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    if prepared.after != prepared.owner_facts.state {
        let request = request.ok_or(CompanionExecutionError::SnapshotMismatch)?;
        let committed = committed.ok_or(CompanionExecutionError::SnapshotMismatch)?;
        if request.before != prepared.owner_facts.state
            || request.after != prepared.after
            || !committed.matches_request(original_fence, request)
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
    } else if request.is_some() || committed.is_some() {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    // Existing summons receive party protection before new creation. All snapshots were compared
    // above, and immutable policies/master facts cannot change under this exclusive owner turn.
    if prepared
        .actions
        .contains(&FamiliarAction::RegisterPartyProtectionAllOwnedSummons)
    {
        for snapshot in &prepared.owned {
            runtime.mark_companion_party_protection(snapshot)?;
        }
    }
    if let Some(dead) = &prepared.death {
        runtime.despawn_companion(
            prepared.binding.caster,
            original_fence.game_session_id,
            dead,
        )?;
    }
    let creature = prepared
        .spawn
        .map(|spawn| runtime.install_companion_spawn(spawn))
        .transpose()?;
    let mut owner_operations = prepared.actions;
    owner_operations.retain(|a| {
        !matches!(
            a,
            FamiliarAction::Create { .. }
                | FamiliarAction::ChangeSpeed(_)
                | FamiliarAction::StoreExpiry(_)
                | FamiliarAction::SelectLook(_)
                | FamiliarAction::GrantVocationLook(_)
                | FamiliarAction::RemoveVocationLook(_)
                | FamiliarAction::RegisterPartyProtectionAllOwnedSummons
                | FamiliarAction::ScheduleExpiry { .. }
                | FamiliarAction::ScheduleWarning { .. }
        )
    });
    Ok(FamiliarApplyReceipt {
        creature,
        after: prepared.after,
        binding: prepared.binding,
        owner_operations,
    })
}

/// Source corpse and predicted spawn share an exact physical owner binding. Construction
/// requires the Item owner's locked durable corpse reservation, never a planner corpse flag.
#[derive(Debug)]
pub(crate) struct CompanionSpawnReservation {
    spawn: PreparedCompanionSpawn,
    character_id: crate::domain::CharacterId,
    owner: ExactActorRef,
    session: GameSessionId,
    corpse: [u8; 16],
    corpse_revision: u64,
    owned: Vec<ExactActorRef>,
    current: AcquisitionOwnerFacts,
    consume_rune_charge: bool,
}
impl CompanionSpawnReservation {
    pub(crate) fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.current)?;
        runtime.reserve_companion_spawn(&mut self.spawn)?;
        Ok(())
    }
    pub(crate) fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        runtime.rollback_companion_spawn(&self.spawn)?;
        Ok(())
    }
    pub(crate) fn actor(&self) -> ExactActorRef {
        self.spawn.actor()
    }
    pub(crate) fn master_actor(&self) -> ExactActorRef {
        self.owner
    }
    pub(crate) fn character_id(&self) -> crate::domain::CharacterId {
        self.character_id
    }
    pub(crate) fn game_session_id(&self) -> GameSessionId {
        self.session
    }
    pub(crate) fn corpse_item_instance_id(&self) -> &[u8; 16] {
        &self.corpse
    }
    pub(crate) fn corpse_state_revision(&self) -> u64 {
        self.corpse_revision
    }
    pub(crate) fn spawn_cell(&self) -> LogicalCell {
        let p = self.spawn.position();
        LogicalCell {
            x: p.x,
            y: p.y,
            z: i32::from(p.floor),
        }
    }
    pub(crate) fn creature_definition_key(&self) -> &str {
        &self.spawn.policy().definition_key
    }
    pub(crate) fn creature_definition_revision(&self) -> &str {
        &self.spawn.policy().definition_revision
    }
    pub(crate) fn consume_rune_charge(&self) -> bool {
        self.consume_rune_charge
    }
    pub(crate) fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        current: &AcquisitionOwnerFacts,
    ) -> Result<(), CompanionExecutionError> {
        if current != &self.current || owned_ids(runtime, self.owner, self.session)? != self.owned {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        runtime.validate_companion_spawn(&self.spawn)?;
        Ok(())
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_animate_dead(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    owner: ExactActorRef,
    session: GameSessionId,
    character_id: crate::domain::CharacterId,
    spell: &CompiledNativeSpell,
    corpse: &super::world_items_execution::DurableCorpseReservation,
    current: &AcquisitionOwnerFacts,
) -> Result<CompanionSpawnReservation, CompanionExecutionError> {
    let p = parameters(spell, "acquire_summon")?;
    if p["source"] != "corpse_tile" {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    let owned = owned_ids(runtime, owner, session)?;
    let cell = corpse
        .cell()
        .map_err(|_| CompanionExecutionError::InvalidMap)?;
    let anchor = MovementLocalPosition {
        x: cell.x,
        y: cell.y,
        floor: i16::try_from(cell.z).map_err(|_| CompanionExecutionError::InvalidMap)?,
    };
    let name = p["created_creature"]
        .as_str()
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    let policy = runtime.companion_policy(name)?;
    let position = placement(
        runtime,
        cells,
        anchor,
        p["spawn"]["extended"]
            .as_bool()
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?,
    )?;
    // A real reservation exists only for the current top-down, movable corpse with qualified
    // materialization provenance and empty entries, all locked in this same physical transaction.
    let facts = AcquisitionFacts {
        kind: AcquisitionKind::CorpseTile,
        creature_name: name.into(),
        type_found: true,
        summonable: policy.summonable,
        target_is_monster: false,
        convinceable: policy.convinceable,
        target_master_name: None,
        tile_present: true,
        top_down_item_present: true,
        is_corpse: true,
        corpse_movable: true,
        owned_summons: owned.len(),
        can_summon_all: current.can_summon_all,
        can_convince_all: current.can_convince_all,
        black_skull: current.black_skull,
        mana: current.mana,
        creature_mana_cost: 0,
        has_infinite_mana: current.has_infinite_mana,
        spawn_room: position.is_some(),
    };
    let plan = match spell
        .plan(
            Facts::Companion(&CompanionFacts::Acquisition(facts)),
            &mut |min, _| min,
        )
        .map_err(|e| CompanionExecutionError::Refused(e.0))?
    {
        Plan::Companion(plan) => match *plan {
            super::native_companions::CompanionPlan::Acquisition(plan) => plan,
            _ => return Err(CompanionExecutionError::UnqualifiedProfile),
        },
        _ => return Err(CompanionExecutionError::UnqualifiedProfile),
    };
    let spawn = runtime.prepare_companion_spawn(
        owner,
        session,
        &policy.definition_key,
        position.ok_or(CompanionExecutionError::PlacementUnavailable)?,
        None,
        None,
        0,
        0,
        false,
    )?;
    Ok(CompanionSpawnReservation {
        spawn,
        character_id,
        owner,
        session,
        corpse: *corpse.item_instance_id(),
        corpse_revision: corpse.state_revision(),
        owned,
        current: current.clone(),
        consume_rune_charge: plan.consume_rune_charge_on_success,
    })
}

/// Only the common compositor's successful deadline-bounded COMMIT upgrades the Item writer's
/// pending acquisition into this sealed proof. A source corpse cannot disappear before a real
/// spawn reservation is recorded in the same physical transaction. Replay never reallocates.
pub(crate) fn install_reserved_companion(
    runtime: &mut ChannelRuntimeV1,
    reservation: CompanionSpawnReservation,
    current: &AcquisitionOwnerFacts,
    receipt: &crate::durability::spell_item_transaction::CommittedCompanionAcquisition,
) -> Result<AcquisitionReceipt, CompanionExecutionError> {
    // The sole common-compositor caller has already compared the independently
    // current full summon census/restrictions before consuming its training
    // witness. It holds the same exclusive owner turn through this install.
    // Recheck the exact live owner/reserved slot and simple source restrictions
    // without allocating a second owned-companion census after mutation.
    if current != &reservation.current {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    runtime.validate_companion_spawn(&reservation.spawn)?;
    if !receipt.matches_spawn(&reservation) {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    let creature = runtime.install_companion_spawn(reservation.spawn)?;
    Ok(AcquisitionReceipt {
        creature,
        mana_cost: 0,
        mana_spent: 0,
        consume_rune_charge: reservation.consume_rune_charge,
    })
}

impl crate::durability::spell_item_transaction::companion_source_seal::Sealed
    for CompanionSpawnReservation
{
}
impl crate::durability::spell_item_transaction::CompanionAcquisitionSource
    for CompanionSpawnReservation
{
    fn actor(&self) -> ExactActorRef {
        self.actor()
    }
    fn master_actor(&self) -> ExactActorRef {
        self.master_actor()
    }
    fn character_id(&self) -> crate::domain::CharacterId {
        self.character_id()
    }
    fn game_session_id(&self) -> GameSessionId {
        self.game_session_id()
    }
    fn corpse_item_instance_id(&self) -> &[u8; 16] {
        self.corpse_item_instance_id()
    }
    fn corpse_state_revision(&self) -> u64 {
        self.corpse_state_revision()
    }
    fn spawn_cell(&self) -> crate::durability::spell_items_abi::SpellItemCell {
        let cell = self.spawn_cell();
        crate::durability::spell_items_abi::SpellItemCell {
            x: cell.x,
            y: cell.y,
            z: cell.z,
        }
    }
    fn creature_definition_key(&self) -> &str {
        self.creature_definition_key()
    }
    fn creature_definition_revision(&self) -> &str {
        self.creature_definition_revision()
    }
}

/// Replace the source familiar's shared reference-ID cooldown in the genuinely paid successor.
/// Every represented alias of that reference shares its deadline; support-group payment remains
/// from the common cast. Negative nonpermanent ticks are already expired source conditions.
pub(crate) fn finish_familiar_payment(
    before: &super::cast::PlayerSpellState,
    paid: &mut super::cast::PlayerSpellState,
    prepared: &mut PreparedFamiliar,
    book: &super::SpellBook,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> Result<super::combat_batch::SpellAnchor, CompanionExecutionError> {
    let (reference, ticks, _) = prepared
        .cooldown()
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    let deadline = if ticks == -1 {
        u64::MAX
    } else {
        now.get()
            .checked_add(
                u64::try_from(ticks.max(0))
                    .map_err(|_| CompanionExecutionError::ArithmeticBounds)?
                    .checked_mul(1000)
                    .ok_or(CompanionExecutionError::ArithmeticBounds)?,
            )
            .ok_or(CompanionExecutionError::ArithmeticBounds)?
    };
    let mut keys = Vec::new();
    let mut existing = None;
    for spell in &book.spells {
        if let super::Execution::NativeProfile(profile) = &spell.execution
            && profile.spell()["reference_spell_id"].as_u64() == Some(u64::from(reference))
        {
            existing = existing.max(
                before
                    .cooldowns
                    .spell_ready_at(&spell.key)
                    .map(|due| due.get()),
            );
            keys.push(spell.key.clone());
        }
    }
    if keys.is_empty() {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    let deadline = merge_familiar_cooldown_deadline(existing, deadline, ticks, now.get());
    let snapshot = crate::durability::character_familiar::FamiliarCooldownSnapshot {
        reference_spell_id: reference,
        remaining_micros: if deadline == u64::MAX {
            u64::MAX
        } else {
            deadline.saturating_sub(now.get())
        },
    };
    match prepared
        .after
        .cooldowns
        .binary_search_by_key(&reference, |v| v.reference_spell_id)
    {
        Ok(index) => prepared.after.cooldowns[index] = snapshot,
        Err(index) => prepared.after.cooldowns.insert(index, snapshot),
    }
    for key in keys {
        paid.cooldowns.spells.insert(
            key,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(deadline),
        );
    }
    paid.payment_anchor_from(before)
        .ok_or(CompanionExecutionError::SnapshotMismatch)
}
// Canary Condition::updateCondition: infinite blocks only a positive replacement;
// nonnegative replacements preserve a longer finite end time. Negative nonpermanent
// ticks replace/expire even an infinite condition; this is not a universal max().
fn merge_familiar_cooldown_deadline(
    existing: Option<u64>,
    proposed: u64,
    ticks: i32,
    now: u64,
) -> u64 {
    match existing {
        Some(u64::MAX) if ticks > 0 => u64::MAX,
        Some(due) if due != u64::MAX && ticks >= 0 && due > proposed && due > now => due,
        _ => proposed,
    }
}
/// The durable common receipt binds this complete typed cost projection. Independently compare
/// the actual full PlayerSpellState predecessor in the physical compositor; this hash is data,
/// never current authority or a substitute for owner state equality.
pub(crate) fn familiar_cost_binding(
    before: &super::cast::PlayerSpellState,
    paid: &super::cast::PlayerSpellState,
    anchor: &super::combat_batch::SpellAnchor,
) -> Result<crate::durability::spell_items_abi::CastCostBinding, CompanionExecutionError> {
    use sha2::{Digest, Sha256};
    if !paid.paid_successor_of(before, anchor) {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    let digest = |state: &super::cast::PlayerSpellState| -> [u8; 32] {
        let mut h = Sha256::new().chain_update(b"oteryn:spell-cost-projection:v1");
        for n in [
            state.revision,
            u64::from(state.health),
            u64::from(state.mana),
            u64::from(state.soul),
        ] {
            h.update(n.to_be_bytes());
        }
        for (key, due) in state.cooldowns.canonical_deadlines() {
            h.update((key.len() as u64).to_be_bytes());
            h.update(key.as_bytes());
            h.update(due.to_be_bytes());
        }
        h.finalize().into()
    };
    Ok(crate::durability::spell_items_abi::CastCostBinding {
        vitals_revision_before: before.revision,
        vitals_revision_after: paid.revision,
        mana_before: before.mana,
        mana_after: paid.mana,
        soul_before: before.soul,
        soul_after: paid.soul,
        cooldowns_before: before.cooldowns.canonical_deadlines(),
        cooldowns_after: paid.cooldowns.canonical_deadlines(),
        caster_digest_before: digest(before),
        caster_digest_after: digest(paid),
    })
}

/// Noncast event identity. These data values never act as current admission authority.
/// Actual source callers must supply the independently current Character fence at issuance
/// and again before applying a durable result. No lifecycle event invents a client CommandRef.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FamiliarLifecycleOrigin {
    Login {
        session: GameSessionId,
    },
    Advance {
        occurrence: crate::durability::character_progression::ExperienceRewardOccurrence,
        committed_character_revision: u64,
        level_after: u32,
    },
    Death {
        occurrence: crate::foundation::CreatureDeathOccurrenceRef,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamiliarLifecycleBinding {
    spell: CompiledNativeSpell,
    owner: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterGameplayFence,
    stamp: crate::foundation::RuntimeWorkStamp,
    at: SemanticTimeMicros,
    origin: FamiliarLifecycleOrigin,
}
impl FamiliarLifecycleBinding {
    fn issue(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        origin: FamiliarLifecycleOrigin,
    ) -> Result<Self, CompanionExecutionError> {
        parameters(&spell, "familiar_summon")?;
        let binding = runtime.binding();
        if !runtime.owner_fence()?.accepts_stamp(stamp)
            || fence.runtime_scope
                != crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                )
            || fence.scope_ownership_generation != binding.scope_generation()
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        runtime.player_control_facts(owner, fence.game_session_id)?;
        Ok(Self {
            spell,
            owner,
            session: fence.game_session_id,
            fence,
            stamp,
            at,
            origin,
        })
    }
    /// The actual first-entry caller invokes this only after reconciling this admission
    /// operation and comparing its independently current session/root/owner fence.
    pub(crate) fn from_admission(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        admission: &crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
    ) -> Result<Self, CompanionExecutionError> {
        if admission.binding().candidate_session != fence.game_session_id {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        Self::issue(
            runtime,
            spell,
            owner,
            fence,
            stamp,
            at,
            FamiliarLifecycleOrigin::Login {
                session: fence.game_session_id,
            },
        )
    }
    /// Invoked by the actual fenced XP writer result caller, before a different root successor.
    pub(crate) fn from_level_advance(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        experience: &crate::durability::character_progression::CommittedExperienceAward,
    ) -> Result<Self, CompanionExecutionError> {
        if experience.character_id != fence.character_id
            || experience.committed_character_revision != fence.expected_character_revision
            || experience.level_after <= experience.level_before
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        Self::issue(
            runtime,
            spell,
            owner,
            fence,
            stamp,
            at,
            FamiliarLifecycleOrigin::Advance {
                occurrence: experience.occurrence,
                committed_character_revision: experience.committed_character_revision.get(),
                level_after: experience.level_after,
            },
        )
    }
    /// This source occurrence can be constructed only by the real physical lethal owner.
    pub(crate) fn from_committed_lethal(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        corpse: &crate::foundation::RuntimeCorpseProjection,
    ) -> Result<Self, CompanionExecutionError> {
        let dead = runtime.companion_snapshot_including_dead(corpse.occurrence().actor())?;
        if dead.health != 0
            || dead.state.master
                != Some(CompanionMaster {
                    actor: owner,
                    session: fence.game_session_id,
                })
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        Self::issue(
            runtime,
            spell,
            owner,
            fence,
            stamp,
            at,
            FamiliarLifecycleOrigin::Death {
                occurrence: corpse.occurrence().clone(),
            },
        )
    }
    pub(crate) fn spell(&self) -> &CompiledNativeSpell {
        &self.spell
    }
    pub(crate) fn owner(&self) -> ExactActorRef {
        self.owner
    }
    pub(crate) fn session(&self) -> GameSessionId {
        self.session
    }
    pub(crate) fn fence(&self) -> &CurrentCharacterGameplayFence {
        &self.fence
    }
    pub(crate) fn stamp(&self) -> crate::foundation::RuntimeWorkStamp {
        self.stamp
    }
    pub(crate) fn at(&self) -> SemanticTimeMicros {
        self.at
    }
    pub(crate) fn origin(&self) -> &FamiliarLifecycleOrigin {
        &self.origin
    }
}
#[derive(Debug, Clone)]
pub(crate) struct SavedLifecycleFamiliarTimer {
    pub(crate) binding: FamiliarLifecycleBinding,
    pub(crate) creature: ExactActorRef,
    pub(crate) lifecycle_epoch: u64,
    pub(crate) event: FamiliarTimerEvent,
}
/// Same real physical owner/epoch checks as Cast timers; the source identity has no client command.
pub(crate) fn stage_lifecycle_familiar_timer(
    runtime: &ChannelRuntimeV1,
    saved: &SavedLifecycleFamiliarTimer,
) -> Result<Option<StagedFamiliarOperation>, CompanionExecutionError> {
    stage_familiar_timer_parts(
        runtime,
        saved.binding.spell(),
        saved.binding.owner(),
        saved.binding.session(),
        saved.creature,
        saved.lifecycle_epoch,
        saved.event,
    )
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedLifecycleFamiliar {
    binding: FamiliarLifecycleBinding,
    parts: FamiliarParts,
}
#[derive(Debug, Clone)]
pub(crate) struct LifecycleFamiliarSchedule {
    pub(crate) phase: u16,
    pub(crate) due: SemanticTimeMicros,
    pub(crate) saved: SavedLifecycleFamiliarTimer,
}
#[derive(Debug, Clone)]
pub(crate) struct LifecycleFamiliarApplyReceipt {
    binding: FamiliarLifecycleBinding,
    creature: Option<ExactActorRef>,
    after: DurableFamiliarState,
    operations: Vec<FamiliarAction>,
}
impl LifecycleFamiliarApplyReceipt {
    pub(crate) fn binding(&self) -> &FamiliarLifecycleBinding {
        &self.binding
    }
    pub(crate) fn creature(&self) -> Option<ExactActorRef> {
        self.creature
    }
    pub(crate) fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    pub(crate) fn owner_operations(&self) -> &[FamiliarAction] {
        &self.operations
    }
    pub(crate) fn matches_saved(&self, saved: &SavedLifecycleFamiliarTimer) -> bool {
        self.binding == saved.binding
            && self.creature == Some(saved.creature)
            && self.after.lifecycle_epoch == saved.lifecycle_epoch
            && valid_familiar_timer_event(self.binding.spell(), saved.event)
    }
}
fn valid_familiar_timer_event(spell: &CompiledNativeSpell, event: FamiliarTimerEvent) -> bool {
    let Ok(p) = parameters(spell, "familiar_summon") else {
        return false;
    };
    match event {
        FamiliarTimerEvent::Expiry => true,
        FamiliarTimerEvent::Warning { index } => p["warnings"]
            .as_array()
            .is_some_and(|values| index < values.len()),
    }
}
impl PreparedLifecycleFamiliar {
    pub(crate) fn reservation_actors(&self) -> impl Iterator<Item = ExactActorRef> + '_ {
        self.parts
            .owned
            .iter()
            .map(|s| s.actor)
            .chain(self.parts.death.iter().map(|s| s.actor))
    }
    pub(crate) fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.parts.owner_facts)?;
        for owned in &self.parts.owned {
            runtime.assert_actor_spell_unreserved(owned.actor)?;
        }
        if let Some(dead) = &self.parts.death {
            runtime.assert_actor_spell_unreserved(dead.actor)?;
        }
        if let Some(spawn) = self.parts.spawn.as_mut() {
            runtime.reserve_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub(crate) fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        if let Some(spawn) = self.parts.spawn.as_ref() {
            runtime.rollback_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub(crate) fn binding(&self) -> &FamiliarLifecycleBinding {
        &self.binding
    }
    pub(crate) fn owner_facts(&self) -> &FamiliarOwnerFacts {
        &self.parts.owner_facts
    }
    pub(crate) fn after(&self) -> &DurableFamiliarState {
        &self.parts.after
    }
    pub(crate) fn state_request(
        &self,
        occurrence: FamiliarStateOccurrence,
        content_revision: String,
        policy_revision: String,
        policy_digest: [u8; 32],
    ) -> Option<FamiliarStateRequest> {
        (self.parts.after != self.parts.owner_facts.state).then(|| FamiliarStateRequest {
            occurrence,
            before: self.parts.owner_facts.state.clone(),
            after: self.parts.after.clone(),
            content_revision,
            policy_revision,
            policy_digest,
        })
    }
    pub(crate) fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        current: &FamiliarOwnerFacts,
    ) -> Result<(), CompanionExecutionError> {
        if current != &self.parts.owner_facts {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        runtime.validate_owned_companions(
            self.binding.owner(),
            self.binding.session(),
            &self.parts.owned,
        )?;
        runtime.player_control_facts(self.binding.owner(), self.binding.session())?;
        if let Some(dead) = &self.parts.death {
            runtime.validate_companion_snapshot(dead)?;
        }
        if let Some(spawn) = &self.parts.spawn {
            runtime.validate_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub(crate) fn matches_saved(&self, saved: &SavedLifecycleFamiliarTimer) -> bool {
        self.binding == saved.binding
            && self
                .parts
                .spawn
                .as_ref()
                .is_some_and(|spawn| spawn.actor() == saved.creature)
            && self.parts.after.lifecycle_epoch == saved.lifecycle_epoch
            && match saved.event {
                FamiliarTimerEvent::Expiry => self
                    .parts
                    .actions
                    .iter()
                    .any(|a| matches!(a, FamiliarAction::ScheduleExpiry { .. })),
                FamiliarTimerEvent::Warning { index } => self.parts.actions.iter().any(
                    |a| matches!(a, FamiliarAction::ScheduleWarning {index:i,..} if *i==index),
                ),
            }
    }
    pub(crate) fn schedules(
        &self,
        first_phase: u16,
    ) -> Result<Vec<LifecycleFamiliarSchedule>, CompanionExecutionError> {
        let Some(spawn) = &self.parts.spawn else {
            return Ok(Vec::new());
        };
        let mut schedules = Vec::new();
        for action in &self.parts.actions {
            let (delay, event) = match action {
                FamiliarAction::ScheduleExpiry { delay_ms } => {
                    (*delay_ms, FamiliarTimerEvent::Expiry)
                }
                FamiliarAction::ScheduleWarning {
                    index, delay_ms, ..
                } => (*delay_ms, FamiliarTimerEvent::Warning { index: *index }),
                _ => continue,
            };
            let phase = first_phase
                .checked_add(
                    u16::try_from(schedules.len())
                        .map_err(|_| CompanionExecutionError::ArithmeticBounds)?,
                )
                .ok_or(CompanionExecutionError::ArithmeticBounds)?;
            let due = self
                .binding
                .at()
                .get()
                .checked_add(
                    u64::from(delay)
                        .checked_mul(1000)
                        .ok_or(CompanionExecutionError::ArithmeticBounds)?,
                )
                .ok_or(CompanionExecutionError::ArithmeticBounds)?;
            schedules.push(LifecycleFamiliarSchedule {
                phase,
                due: SemanticTimeMicros::from_micros(due),
                saved: SavedLifecycleFamiliarTimer {
                    binding: self.binding.clone(),
                    creature: spawn.actor(),
                    lifecycle_epoch: self.parts.after.lifecycle_epoch,
                    event,
                },
            });
        }
        Ok(schedules)
    }
}
pub(crate) fn prepare_lifecycle_familiar(
    runtime: &ChannelRuntimeV1,
    cells: Option<&NativeEntryMovementCells>,
    binding: FamiliarLifecycleBinding,
    current: &FamiliarOwnerFacts,
    unix: FamiliarUnixTime,
) -> Result<PreparedLifecycleFamiliar, CompanionExecutionError> {
    if let FamiliarLifecycleOrigin::Advance { level_after, .. } = binding.origin()
        && current.level != *level_after
    {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    let (event, dead) = match binding.origin() {
        FamiliarLifecycleOrigin::Login { .. } => (FamiliarEvent::Login, None),
        FamiliarLifecycleOrigin::Advance { .. } => (FamiliarEvent::Advance, None),
        FamiliarLifecycleOrigin::Death { occurrence } => {
            (FamiliarEvent::FamiliarDeath, Some(occurrence.actor()))
        }
    };
    let mut parts = prepare_familiar_parts(
        runtime,
        cells,
        binding.spell(),
        binding.owner(),
        binding.session(),
        event,
        current,
        unix,
        dead,
    )?;
    if let Some(spawn) = parts.spawn.as_mut() {
        spawn.bind_semantic_creation(binding.at().get())?;
    }
    Ok(PreparedLifecycleFamiliar { binding, parts })
}
/// Real projection/write receipt is mandatory for a changed snapshot. No no-op receipt,
/// caster cost, training payment or client command is created for a lifecycle event.
pub(crate) fn commit_lifecycle_familiar(
    runtime: &mut ChannelRuntimeV1,
    prepared: PreparedLifecycleFamiliar,
    current: &FamiliarOwnerFacts,
    fresh_fence: &CurrentCharacterGameplayFence,
    request: Option<&FamiliarStateRequest>,
    committed: Option<&CommittedCharacterFamiliar>,
) -> Result<LifecycleFamiliarApplyReceipt, CompanionExecutionError> {
    prepared.validate_current(runtime, current)?;
    let original = prepared.binding.fence();
    let expected_revision = committed.map_or(original.expected_character_revision, |value| {
        value.committed_character_revision()
    });
    let mut expected = *original;
    expected.expected_character_revision = expected_revision;
    if fresh_fence != &expected {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    if prepared.parts.after != current.state {
        let (Some(request), Some(receipt)) = (request, committed) else {
            return Err(CompanionExecutionError::SnapshotMismatch);
        };
        if request.before != current.state
            || request.after != prepared.parts.after
            || !receipt.matches_request(original, request)
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
    } else if request.is_some() || committed.is_some() {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    // The genuine lethal receipt and independently current full dead snapshot were
    // checked above. Source death callbacks complete before the ordinary physical
    // creature removal; leaving the health-zero slot would repeat the callback and
    // retain its master indefinitely. The lethal projection remains replayable.
    if let Some(dead) = &prepared.parts.death {
        runtime.despawn_companion(prepared.binding.owner(), prepared.binding.session(), dead)?;
    }
    let creature = prepared
        .parts
        .spawn
        .map(|spawn| runtime.install_companion_spawn(spawn))
        .transpose()?;
    let mut operations = prepared.parts.actions;
    operations.retain(|a| {
        !matches!(
            a,
            FamiliarAction::Create { .. }
                | FamiliarAction::ChangeSpeed(_)
                | FamiliarAction::StoreExpiry(_)
                | FamiliarAction::SelectLook(_)
                | FamiliarAction::GrantVocationLook(_)
                | FamiliarAction::RemoveVocationLook(_)
                | FamiliarAction::ScheduleExpiry { .. }
                | FamiliarAction::ScheduleWarning { .. }
        )
    });
    Ok(LifecycleFamiliarApplyReceipt {
        binding: prepared.binding,
        creature,
        after: prepared.parts.after,
        operations,
    })
}

/// Snapshot the actual shared reference-ID condition at a genuine logout decision.
/// Caller writes this via the fenced familiar writer before releasing control. No timer fires
/// while offline and no periodic/logout-stances writer is introduced.
pub(crate) fn familiar_logout_snapshot(
    current: &DurableFamiliarState,
    player: &super::cast::PlayerSpellState,
    profile: &CompiledNativeSpell,
    book: &super::SpellBook,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    unix: FamiliarUnixTime,
) -> Result<DurableFamiliarState, CompanionExecutionError> {
    parameters(profile, "familiar_summon")?;
    let mut references = std::collections::BTreeMap::new();
    for spell in &book.spells {
        let super::Execution::NativeProfile(profile) = &spell.execution else {
            continue;
        };
        if profile.spell()["execution"]["native_behavior"]["key"] != "familiar_summon" {
            continue;
        }
        let reference = profile.spell()["reference_spell_id"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
        let value = player
            .cooldowns
            .spell_ready_at(&spell.key)
            .map(|v| v.get())
            .filter(|due| *due > now.get());
        if let Some(previous) = references.insert(reference, value)
            && previous != value
        {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
    }
    if references.is_empty() {
        return Err(CompanionExecutionError::UnqualifiedProfile);
    }
    let mut next = current.clone();
    next.last_logout_unix = unix.seconds();
    next.cooldowns = references
        .into_iter()
        .filter_map(|(reference_spell_id, deadline)| {
            deadline.map(
                |due| crate::durability::character_familiar::FamiliarCooldownSnapshot {
                    reference_spell_id,
                    remaining_micros: if due == u64::MAX {
                        u64::MAX
                    } else {
                        due - now.get()
                    },
                },
            )
        })
        .collect();
    next.validate()
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    Ok(next)
}
/// Apply saved remaining reference-ID durations to an actual cloned Login player successor.
/// A vocation change does not delete a different familiar's genuine cooldown condition.
pub(crate) fn restore_familiar_cooldown(
    player: &mut super::cast::PlayerSpellState,
    prepared: &PreparedLifecycleFamiliar,
    book: &super::SpellBook,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> Result<(), CompanionExecutionError> {
    if !matches!(
        prepared.binding.origin(),
        FamiliarLifecycleOrigin::Login { .. }
    ) {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    let mut writes = Vec::new();
    for saved in &prepared.parts.after.cooldowns {
        let deadline = if saved.remaining_micros == u64::MAX {
            u64::MAX
        } else {
            now.get()
                .checked_add(saved.remaining_micros)
                .ok_or(CompanionExecutionError::ArithmeticBounds)?
        };
        let mut known = false;
        for spell in &book.spells {
            if let super::Execution::NativeProfile(profile) = &spell.execution
                && profile.spell()["execution"]["native_behavior"]["key"] == "familiar_summon"
                && profile.spell()["reference_spell_id"].as_u64()
                    == Some(u64::from(saved.reference_spell_id))
            {
                known = true;
                writes.push((spell.key.clone(), deadline));
            }
        }
        if !known {
            return Err(CompanionExecutionError::UnqualifiedProfile);
        }
    }
    for (key, deadline) in writes {
        player.cooldowns.spells.insert(
            key,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(deadline),
        );
    }
    Ok(())
}

#[cfg(test)]
mod cooldown_source_tests {
    use super::*;
    #[test]
    fn actual_condition_update_preserves_longer_only_for_source_allowed_ticks() {
        assert_eq!(
            merge_familiar_cooldown_deadline(Some(u64::MAX), 2000, 1000, 1000),
            u64::MAX
        );
        assert_eq!(
            merge_familiar_cooldown_deadline(Some(u64::MAX), 1000, 0, 1000),
            1000
        );
        assert_eq!(
            merge_familiar_cooldown_deadline(Some(u64::MAX), 1000, -2, 1000),
            1000
        );
        assert_eq!(
            merge_familiar_cooldown_deadline(Some(5000), 2000, 1000, 1000),
            5000
        );
        assert_eq!(
            merge_familiar_cooldown_deadline(Some(5000), u64::MAX, -1, 1000),
            u64::MAX
        );
        assert_eq!(
            merge_familiar_cooldown_deadline(Some(5000), 1000, -2, 1000),
            1000
        );
    }
}

#[cfg(test)]
mod direct_acquisition_owner_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::foundation::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags};
    use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
    use crate::spell::{CasterState, Vocation};
    use std::collections::BTreeSet;

    fn profile(source: &str) -> CompiledNativeSpell {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .expect("actual source profiles");
        let row = document["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|r| {
                r["spell"]["execution"]["native_behavior"]["key"] == "acquire_summon"
                    && r["spell"]["execution"]["native_behavior"]["parameters"]["source"] == source
            })
            .expect("actual source family");
        super::super::native::spell_from_bundle(
            &serde_json::json!({"spell":row["spell"]}),
            &row["dependencies"],
        )
        .expect("complete canonical qualification")
    }
    fn target_fixture(
        mana: u64,
        infinite: bool,
    ) -> (
        ChannelRuntimeV1,
        crate::content::QualifiedNativeEntryRoom,
        DirectCompanionReservation,
        AcquisitionOwnerFacts,
    ) {
        let (mut runtime, owner, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(96);
        runtime
            .initialize_first_entry_position(owner)
            .expect("actual positioned owner");
        // A typed physical owner fixture, not an active source importer or grant.
        let policy = CompiledCreaturePolicy {
            definition_key: "creature:test-direct".into(),
            definition_revision: "test-1".into(),
            display_name: "test direct".into(),
            maximum_health: 812,
            base_speed: 220,
            outfit_look_type: 991,
            object_look_type: None,
            summonable: true,
            convinceable: true,
            mana_cost: Some(250),
            is_familiar: false,
            condition_immunities: vec![],
            armor: Some(10),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
            preferred_distance: Some(1),
            reward_boss: Some(false),
        };
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    runtime.content_pin().server_artifact_digest(),
                    vec![policy],
                )
                .expect("closed fixture policy"),
            )
            .expect("actual owner policy");
        let target = runtime
            .create_companion(
                owner,
                session,
                "creature:test-direct",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .expect("real creature incarnation");
        let room = crate::content::qualify_native_entry_room(runtime.binding().world_id())
            .expect("actual map");
        let restrictions = AcquisitionOwnerFacts {
            mana,
            has_infinite_mana: infinite,
            can_summon_all: false,
            can_convince_all: true,
            black_skull: false,
        };
        let character = crate::domain::CharacterId::from_bytes([
            1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16,
        ])
        .expect("native character");
        let reservation = prepare_direct_acquisition(
            &runtime,
            room.movement_cells(),
            owner,
            session,
            character,
            &profile("target_creature"),
            AcquisitionSource::Target(target),
            &restrictions,
        )
        .expect("real target preparation");
        (runtime, room, reservation, restrictions)
    }
    #[test]
    fn direct_reservation_checks_independent_restrictions_and_current_target() {
        let (mut runtime, _, reservation, current) = target_fixture(500, false);
        assert!(reservation.validate_current(&runtime, &current).is_ok());
        for field in 0..4 {
            let mut changed = current.clone();
            match field {
                0 => changed.mana -= 1,
                1 => changed.has_infinite_mana = true,
                2 => changed.can_summon_all = true,
                _ => changed.can_convince_all = false,
            }
            assert_eq!(
                reservation.validate_current(&runtime, &changed),
                Err(CompanionExecutionError::SnapshotMismatch)
            );
        }
        let target = runtime
            .companion_snapshot(reservation.actor())
            .expect("independent actual target");
        let mut changed = target.state.clone();
        changed.creation_speed_delta = 1;
        runtime
            .compare_companion_state(&target, changed)
            .expect("actual unrelated owner mutation");
        assert!(reservation.validate_current(&runtime, &current).is_err());
        assert_eq!(
            runtime
                .companion_snapshot(reservation.actor())
                .expect("target remains live")
                .state
                .master,
            target.state.master
        );
    }
    #[test]
    fn infinite_mana_keeps_exact_source_training_cost_without_debit_or_profile_substitution() {
        let (runtime, _, reservation, current) = target_fixture(0, true);
        assert_eq!(reservation.mana_to_deduct(), 0);
        assert_eq!(reservation.mana_spent(), 250);
        assert!(reservation.consume_rune_charge());
        assert!(reservation.qualifies_payment(&profile("target_creature"), 0));
        assert!(!reservation.qualifies_payment(&profile("named_creature"), 0));
        assert!(!reservation.qualifies_payment(&profile("target_creature"), 1));
        assert!(reservation.validate_current(&runtime, &current).is_ok());
    }
    #[test]
    fn bounded_exclusions_narrow_static_placement_without_granting_absent_cells() {
        let (reference, _, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(97);
        let world = reference.binding().world_id();
        let room =
            crate::content::qualify_native_entry_room(world).expect("qualified immutable map");
        let start = room.entry_start();
        let node = crate::foundation::NodeId::decode(&[
            1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 17,
        ])
        .expect("native test node");
        // A test runtime uses the actual qualified map/frame digests. This is
        // not a production content activation or a dynamic door/Item grant.
        let pin = crate::foundation::ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            reference.binding().channel_id(),
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .expect("actual test carrier");
        let reserved = runtime
            .reserve_fresh_session(session)
            .expect("actual source session");
        let owner = runtime
            .commit_fresh_session(reserved)
            .expect("actual actor");
        runtime
            .initialize_first_entry_position(owner)
            .expect("qualified positioned actor");
        let anchor = runtime
            .read_actor_position(owner)
            .expect("actual physical source")
            .position();
        let first = placement_excluding(&runtime, room.movement_cells(), anchor, true, &[])
            .expect("bounded static search")
            .expect("first free qualified cell");
        let second = placement_excluding(&runtime, room.movement_cells(), anchor, true, &[first])
            .expect("bounded exclusions")
            .expect("later qualified cell");
        assert_ne!(first, second);
        assert_eq!(
            placement_excluding(
                &runtime,
                room.movement_cells(),
                anchor,
                true,
                &[first, second]
            )
            .expect("blocked candidates"),
            None
        );
        assert_eq!(
            placement_excluding(
                &runtime,
                room.movement_cells(),
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: anchor.floor
                },
                true,
                &[]
            )
            .expect("unknown source terrain"),
            None
        );
    }
    #[test]
    fn dynamic_payment_is_once_and_cooldown_rejection_does_not_mutate_the_owner() {
        let (runtime, room, reservation, mut current) = target_fixture(500, false);
        let p = profile("target_creature");
        let spell = super::super::authoring::spell_from_bundle(
            &serde_json::json!({"spell":p.spell()}),
            p.dependencies(),
        )
        .expect("actual complete header");
        let state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Druid,
                level: 100,
                magic_level: 50,
                max_health: 500,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("actual caster state");
        let caster = CasterState {
            vocation: Vocation::Druid,
            level: 100,
            magic_level: 50,
            premium: true,
            mana: 500,
            max_mana: 500,
            soul: 100,
            learned: BTreeSet::new(),
            attack_skill: 0,
            attack_value: 0,
            attack_factor: 1.0,
            shielding_skill: 0,
            melee_weapon: false,
            shield_defense: None,
            harmony_multiplier: super::super::harmony::HarmonyMultiplier::ONE,
        };
        let operational = super::super::OperationalCastFacts {
            caster_position: super::super::chain::TilePosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            target_position: Some(super::super::chain::TilePosition {
                x: 1,
                y: 0,
                floor: 0,
            }),
            target: Some(super::super::target::CastTarget {
                caster: 1,
                creature: 2,
                actor: "actual:creature:2".into(),
                master: Some(1),
            }),
            line_of_sight_clear: Some(true),
            direction_available: false,
            wheel_unlocked: None,
            in_protection_zone: false,
            target_tile_solid: Some(false),
            target_tile_creature: Some(true),
        };
        let paid = super::super::cast::prepare_direct_acquisition_owner_cast_with_caster(
            &state,
            &spell,
            &operational,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
            &caster,
            &reservation,
        )
        .expect("source dynamic payment");
        assert_eq!(paid.anchor.paid_mana, 250);
        assert_eq!(paid.next.vitals().mana, 250);
        assert_eq!(state.vitals().mana, 500);
        let mut occupied_solid = operational.clone();
        occupied_solid.target_tile_solid = Some(true);
        assert!(
            super::super::cast::prepare_direct_acquisition_owner_cast_with_caster(
                &state,
                &spell,
                &occupied_solid,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
                &caster,
                &reservation
            )
            .is_ok()
        );
        let mut empty_solid = occupied_solid.clone();
        empty_solid.target_tile_creature = Some(false);
        assert!(
            super::super::cast::prepare_direct_acquisition_owner_cast_with_caster(
                &state,
                &spell,
                &empty_solid,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
                &caster,
                &reservation
            )
            .is_err()
        );
        occupied_solid.target_tile_solid = None;
        assert!(
            super::super::cast::prepare_direct_acquisition_owner_cast_with_caster(
                &state,
                &spell,
                &occupied_solid,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
                &caster,
                &reservation
            )
            .is_err()
        );
        let mut retry = caster.clone();
        retry.mana = 250;
        current.mana = 250;
        let current_reservation = prepare_direct_acquisition(
            &runtime,
            room.movement_cells(),
            reservation.master_actor(),
            reservation.game_session_id(),
            reservation.character_id(),
            &p,
            AcquisitionSource::Target(reservation.actor()),
            &current,
        )
        .expect("independently current target/cost qualification");
        assert_eq!(
            super::super::cast::prepare_direct_acquisition_owner_cast_with_caster(
                &paid.next,
                &spell,
                &operational,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(101),
                &retry,
                &current_reservation
            )
            .err(),
            Some(oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::CoolingDown)
        );
        assert_eq!(paid.next.vitals().mana, 250);
    }
}
