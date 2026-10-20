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
use crate::spell::native::CompiledNativeSpellPlanExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionExecutionError {
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
pub struct AcquisitionOwnerFacts {
    pub mana: u64,
    pub has_infinite_mana: bool,
    pub can_summon_all: bool,
    pub can_convince_all: bool,
    pub black_skull: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquisitionSource {
    Named(String),
    Target(ExactActorRef),
}
#[derive(Debug, Clone)]
pub struct PreparedAcquisition {
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
    pub fn mana_cost(&self) -> u32 {
        self.mana_cost
    }
    pub fn mana_to_deduct(&self) -> u32 {
        if self.owner_facts.has_infinite_mana {
            0
        } else {
            self.mana_cost
        }
    }
    pub fn mana_spent(&self) -> u32 {
        self.mana_cost
    }
    pub fn consume_rune_charge(&self) -> bool {
        self.plan.consume_rune_charge_on_success
    }
}

/// A direct acquisition joins a genuine physical reservation to the source
/// mana decision. It has no corpse or Familiar-state placeholder.
#[derive(Debug)]
pub struct DirectCompanionReservation {
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
    pub fn qualifies_payment(&self, profile: &CompiledNativeSpell, current_mana: u32) -> bool {
        &self.profile == profile && self.current.mana == u64::from(current_mana)
    }
    pub fn restrictions(&self) -> &AcquisitionOwnerFacts {
        &self.current
    }
    pub fn mana_to_deduct(&self) -> u32 {
        if self.current.has_infinite_mana {
            0
        } else {
            self.mana_cost
        }
    }
    /// Both source scripts call addManaSpent(cost), including InfiniteMana.
    pub fn mana_spent(&self) -> u32 {
        self.mana_cost
    }
    pub fn consume_rune_charge(&self) -> bool {
        self.consume_rune_charge
    }
    pub fn assignment(&self) -> Option<&crate::foundation::PreparedCompanionAssignment> {
        match &self.physical {
            DirectCompanionPhysical::Target(value) => Some(value),
            _ => None,
        }
    }
    pub fn actor(&self) -> ExactActorRef {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => p.actor(),
            DirectCompanionPhysical::Target(p) => p.actor(),
        }
    }
    pub fn master_actor(&self) -> ExactActorRef {
        self.owner
    }
    pub fn character_id(&self) -> crate::domain::CharacterId {
        self.character
    }
    pub fn game_session_id(&self) -> GameSessionId {
        self.session
    }
    pub fn position(&self) -> MovementLocalPosition {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => p.position(),
            DirectCompanionPhysical::Target(p) => p.position(),
        }
    }
    pub fn creature_definition_key(&self) -> &str {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => &p.policy().definition_key,
            DirectCompanionPhysical::Target(p) => &p.policy().definition_key,
        }
    }
    pub fn creature_definition_revision(&self) -> &str {
        match &self.physical {
            DirectCompanionPhysical::Named(p) => &p.policy().definition_revision,
            DirectCompanionPhysical::Target(p) => &p.policy().definition_revision,
        }
    }
    pub fn validate_current(
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
    pub fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.current)?;
        if let DirectCompanionPhysical::Named(p) = &mut self.physical {
            runtime.reserve_companion_spawn(p)?;
        }
        Ok(())
    }
    pub fn rollback_physical(
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
    pub fn install_prevalidated(
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
pub fn prepare_direct_acquisition(
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
pub fn prepare_direct_acquisition_excluding(
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
pub struct AcquisitionReceipt {
    pub creature: ExactActorRef,
    pub mana_cost: u32,
    pub mana_spent: u32,
    pub consume_rune_charge: bool,
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
pub fn placement_excluding(
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

pub fn prepare_acquisition(
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
pub fn commit_acquisition(
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
pub enum FamiliarTimerEvent {
    Expiry,
    Warning { index: usize },
}
#[derive(Debug, Clone)]
pub struct SavedFamiliarTimer {
    pub binding: CastBinding,
    pub creature: ExactActorRef,
    pub lifecycle_epoch: u64,
    pub event: FamiliarTimerEvent,
}
#[derive(Debug, Clone)]
pub struct StagedFamiliarOperation {
    owner: ExactActorRef,
    session: GameSessionId,
    expected: CompanionSnapshot,
    event: FamiliarTimerEvent,
    message: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamiliarTimerReceipt {
    pub creature: ExactActorRef,
    pub message: Option<String>,
}

pub fn stage_familiar_timer(
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
/// #2199 Codex P2 4302737307: only a warning commits in the timer lane. An expiry ends the
/// durable lifecycle, so the lane never removes the familiar: the owner cadence hands it to
/// the fenced lifecycle writer (`FamiliarLifecycleOrigin::Expiry`), which closes the row
/// before the physical despawn, as a familiar death does.
pub fn commit_familiar_timer(
    runtime: &mut ChannelRuntimeV1,
    staged: StagedFamiliarOperation,
) -> Result<FamiliarTimerReceipt, CompanionExecutionError> {
    if staged.event == FamiliarTimerEvent::Expiry {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    runtime.player_control_facts(staged.owner, staged.session)?;
    runtime.assert_actor_spell_unreserved(staged.expected.actor)?;
    runtime.validate_companion_snapshot(&staged.expected)?;
    Ok(FamiliarTimerReceipt {
        creature: staged.expected.actor,
        message: staged.message,
    })
}

use super::native_companions::{FamiliarAction, FamiliarEvent, FamiliarFacts};
use crate::durability::character_familiar::{
    CommittedCharacterFamiliar, DurableFamiliarState, FamiliarLifecycleWriter,
    FamiliarStateOccurrence, FamiliarStateRequest, FamiliarUnixTime,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::PreparedCompanionSpawn;
use crate::foundation::owner_timer::SemanticTimeMicros;

/// These values come from the current Character/account/config owners. The source's Unix
/// lifetime is a separately sealed database-clock sample, never the monotonic timer clock.
#[derive(Debug, Clone, PartialEq)]
pub struct FamiliarOwnerFacts {
    pub vocation: String,
    pub premium: bool,
    pub level: u32,
    pub account_at_least_god: bool,
    pub current_speed: i32,
    pub familiar_minutes: i64,
    pub vip: bool,
    pub vip_reduction_minutes: i64,
    pub cooldown_rate: f32,
    pub state: DurableFamiliarState,
    pub checkpoint_committed_at_ms: Option<i64>,
    pub lifecycle_writer: FamiliarLifecycleWriter,
    /// FAMILIARS0 §4 check 3 / §7, resolved from the owner's current session scope.
    pub lever_boss_room: LeverBossRoomMembership,
}

/// #2199 Codex P1 4302326064: whether the owner is inside a boss room admitted by lever
/// (FAMILIARS0 §4 check 3, BOSS-RAID-0 §6). BOSS-RAID-0 §6.1 makes such a room an activity
/// instance (scope kind 2) while the lever room stays ordinary Channel map, so the current
/// session's runtime scope is the authority: a Channel scope is outside, and the fenced
/// familiar write re-proves that scope (`runtime_scope_kind = 1`) in its transaction. An
/// instance scope may be a house or a boss room; without the BOSS-ROOM-1 owner it is
/// unresolved and refuses, never guessed. `Inside` is the fact BOSS-ROOM-1 supplies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeverBossRoomMembership {
    Outside,
    Inside,
    Unresolved,
}
impl LeverBossRoomMembership {
    pub fn of_current_scope(scope: &crate::foundation::RuntimeScopeRefV1) -> Self {
        match scope {
            crate::foundation::RuntimeScopeRefV1::Channel { .. } => Self::Outside,
            crate::foundation::RuntimeScopeRefV1::Instance { .. } => Self::Unresolved,
        }
    }
}

/// CREATURE-AI-0 `RL-19`: player summons per character, a familiar included (FAMILIARS0 §4).
pub const CREATUREAI0_PLAYER_SUMMONS_PER_CHARACTER_MAX: usize = 2;

/// #2199 Codex P1 4302326065: a stored return replays the source Login recreation, whose
/// planner skips the cast's owned-summon limit, so its admission checks the complete owned
/// census against the aggregate cap itself. Over the cap the row keeps waiting.
pub fn familiar_return_within_summon_cap(owned_summons: usize) -> bool {
    owned_summons < CREATUREAI0_PLAYER_SUMMONS_PER_CHARACTER_MAX
}

/// FAMILIARS0 §7: a returning familiar, immediate (Login) or delayed (Return), first passes
/// cast check 3. Inside a lever boss room the return is refused and the familiar ends: its
/// stored time becomes 0 while the row stays closed and the cooldown is kept. An unresolved
/// room refuses the event, so the row stays waiting. Returns whether the familiar ended.
fn end_familiar_return_in_lever_boss_room(
    lifecycle: &mut crate::durability::character_familiar::FamiliarLifecycleV2,
    room: LeverBossRoomMembership,
) -> Result<bool, CompanionExecutionError> {
    if !familiar_waits_to_return(lifecycle) {
        return Ok(false);
    }
    match room {
        LeverBossRoomMembership::Outside => Ok(false),
        LeverBossRoomMembership::Inside => {
            lifecycle.familiar_remaining_ms = 0;
            Ok(true)
        }
        LeverBossRoomMembership::Unresolved => Err(CompanionExecutionError::InvalidOwnerFacts),
    }
}
#[derive(Debug, Clone)]
pub struct PreparedFamiliar {
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
    pub fn reservation_snapshots(&self) -> &[CompanionSnapshot] {
        &self.owned
    }
    pub fn reserve_physical(
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
    pub fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        if let Some(spawn) = self.spawn.as_ref() {
            runtime.rollback_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub fn matches_saved(&self, saved: &SavedFamiliarTimer) -> bool {
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
    pub fn binding(&self) -> &CastBinding {
        &self.binding
    }
    pub fn event(&self) -> &FamiliarEvent {
        &self.event
    }
    pub fn facts(&self) -> &FamiliarFacts {
        &self.facts
    }
    pub fn owner_facts(&self) -> &FamiliarOwnerFacts {
        &self.owner_facts
    }
    pub fn cooldown(&self) -> Option<(u32, i32, bool)> {
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
    pub fn state_request(
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
    pub fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    pub fn creates_creature(&self) -> bool {
        self.spawn.is_some()
    }
    pub fn caster_mana_cost(&self) -> Result<u32, CompanionExecutionError> {
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
pub fn prepare_familiar(
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
pub fn prepare_familiar_hook(
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
    // An expiry is a lifecycle event of the owner cadence only (`FamiliarLifecycleOrigin`).
    if event == FamiliarEvent::Expiry {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
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
        death: parts.removal,
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
    /// The familiar this event removes: the dead one of a death, or the live one whose
    /// lifetime expired. It is despawned only after the durable lifecycle write.
    removal: Option<CompanionSnapshot>,
    spawn: Option<PreparedCompanionSpawn>,
    actions: Vec<FamiliarAction>,
    facts: FamiliarFacts,
    after: DurableFamiliarState,
    ended_in_lever_boss_room: bool,
}
/// Normalize a stored lifecycle for Login after current owner facts were independently resolved.
/// CP D1296 accepts a foreign still-open lifecycle as a crash: discard its remaining
/// lifetime and close it, fail-closed against duplicating value. A same-lifecycle
/// reconnect must retain its live physical/timer state instead of restoring storage.
fn prepare_familiar_login_lifecycle(
    lifecycle: &mut crate::durability::character_familiar::FamiliarLifecycleV2,
    current_writer: &FamiliarLifecycleWriter,
) -> Result<(), CompanionExecutionError> {
    if lifecycle.clean_end_game_session_id.as_ref() == Some(&current_writer.game_session_id) {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    if lifecycle.open {
        if lifecycle.writer.same_lifecycle(current_writer) {
            return Err(CompanionExecutionError::InvalidOwnerFacts);
        }
        lifecycle.familiar_remaining_ms = 0;
        lifecycle.open = false;
    }
    Ok(())
}

/// FAMILIARS0 §7: a clean row with stored time and no committed return acquisition is a
/// familiar waiting to return (no free tile yet). An open row is a committed acquisition:
/// its familiar is live or already ended in this runtime, and it never waits.
pub fn familiar_waits_to_return(
    lifecycle: &crate::durability::character_familiar::FamiliarLifecycleV2,
) -> bool {
    !lifecycle.open && lifecycle.familiar_remaining_ms > 0
}

/// #2199 Codex P2 4302041461: a Login that created no familiar because no tile was free (the
/// source spawn-room refusal) leaves its stored familiar to a later runtime return.
pub fn login_left_familiar_waiting(
    creature: Option<ExactActorRef>,
    operations: &[FamiliarAction],
) -> bool {
    creature.is_none()
        && operations
            .iter()
            .any(|a| matches!(a, FamiliarAction::CreationRefusal { .. }))
}

/// #2199 Codex P2 4302041461: a runtime return restores only the row this session's Login
/// already reconciled and left waiting: a clean row (after the D1126 conversion of the next
/// write) with stored time, written by this lifecycle, at the epoch the return was issued for,
/// before this session's clean end. An open, foreign or newer row is never adopted, so the
/// stored familiar is created at most once with its fenced remaining lifetime.
pub fn familiar_return_admissible(
    state: &DurableFamiliarState,
    checkpoint_committed_at_ms: Option<i64>,
    current_writer: &FamiliarLifecycleWriter,
    waiting_epoch: u64,
) -> bool {
    state.lifecycle_epoch == waiting_epoch
        && state
            .for_next_write(checkpoint_committed_at_ms, current_writer)
            .ok()
            .and_then(|next| next.lifecycle)
            .is_some_and(|lifecycle| {
                familiar_waits_to_return(&lifecycle)
                    && lifecycle.writer.same_lifecycle(current_writer)
                    && lifecycle.clean_end_game_session_id.as_ref()
                        != Some(&current_writer.game_session_id)
            })
}

/// Owner-turn precheck for a stored return: whether the owner's current cell has a free
/// familiar tile now. It reserves and writes nothing; the prepared return proves it again.
pub fn familiar_return_placement_available(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    spell: &CompiledNativeSpell,
    owner: ExactActorRef,
) -> bool {
    let Some(extended) = parameters(spell, "familiar_summon")
        .ok()
        .and_then(|p| p["spawn"]["extended"].as_bool())
    else {
        return false;
    };
    let Ok(position) = runtime.read_actor_position(owner) else {
        return false;
    };
    matches!(
        placement(runtime, cells, position.position(), extended),
        Ok(Some(_))
    )
}

/// #2199 Codex P1 4301955863: an open row another session or runtime wrote is a committed
/// acquisition only Login crash classification (D1296) may reconcile. Until that reconciliation
/// commits, its lifetime and the cooldown it restores at Login are unaccounted for here, so a
/// Cast or level Advance must not adopt and overwrite it under the current writer.
fn familiar_lifecycle_unreconciled(
    lifecycle: &crate::durability::character_familiar::FamiliarLifecycleV2,
    current_writer: &FamiliarLifecycleWriter,
) -> bool {
    lifecycle.open && !lifecycle.writer.same_lifecycle(current_writer)
}

/// Settle the lifecycle of a non-actor-end familiar write under the current writer.
/// A committed acquisition (cast or return) opens the row with its scheduled lifetime and
/// clears any clean-end marker; a familiar death or its expiry ends it (#2199 Codex P2
/// 4302737307: an expired familiar never leaves an open row with positive time). Any other write (a Login with no
/// free tile, Advance) carries the clean row, its stored waiting time and a closed foreign
/// session's clean-end marker unchanged. F-I11: after this session's own clean end it writes
/// nothing more.
fn settle_familiar_event_lifecycle(
    lifecycle: &mut crate::durability::character_familiar::FamiliarLifecycleV2,
    current_writer: &FamiliarLifecycleWriter,
    event: &FamiliarEvent,
    acquired_remaining_ms: Option<u64>,
) -> Result<(), CompanionExecutionError> {
    if lifecycle.clean_end_game_session_id.as_ref() == Some(&current_writer.game_session_id) {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    lifecycle.writer = current_writer.clone();
    if let Some(remaining) = acquired_remaining_ms {
        lifecycle.open = true;
        lifecycle.clean_end_game_session_id = None;
        lifecycle.familiar_remaining_ms = remaining;
    } else if matches!(event, FamiliarEvent::FamiliarDeath | FamiliarEvent::Expiry) {
        lifecycle.open = false;
        lifecycle.familiar_remaining_ms = 0;
    }
    Ok(())
}

/// Close the lifecycle at a clean session end (logout, or the terminal release at the end of
/// the in-fight deadline; FAMILIARS0 §5). A physically present familiar stores its actual
/// timer remaining. With none present, a clean row still waiting to return keeps its stored
/// time, while an open row whose familiar already ended in this runtime stores zero.
pub fn close_familiar_lifecycle_at_actor_end(
    lifecycle: &mut crate::durability::character_familiar::FamiliarLifecycleV2,
    writer: &FamiliarLifecycleWriter,
    physical_remaining_ms: Option<u64>,
) -> Result<(), CompanionExecutionError> {
    if lifecycle.clean_end_game_session_id.as_ref() == Some(&writer.game_session_id) {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    lifecycle.familiar_remaining_ms = match physical_remaining_ms {
        Some(remaining) => remaining,
        None if lifecycle.open => 0,
        None => lifecycle.familiar_remaining_ms,
    };
    lifecycle.open = false;
    lifecycle.writer = writer.clone();
    lifecycle.clean_end_game_session_id = Some(writer.game_session_id.clone());
    Ok(())
}

/// #2199 Codex P2 4301955868: D1126 converts an absent or legacy projection only for the NEXT
/// write. When the event changes nothing but that manufactured v2 lifecycle, it is a no-op: the
/// projection stays exactly as loaded, so `state_request` writes no row and no revision. A Cast
/// always mutates (cooldown, and later its payment), so it keeps the converted state.
fn keep_unconverted_projection_on_no_op(
    before: &DurableFamiliarState,
    after: &mut DurableFamiliarState,
    event: &FamiliarEvent,
) {
    if *event == FamiliarEvent::Cast || before.lifecycle.is_some() {
        return;
    }
    let mut unconverted = after.clone();
    unconverted.lifecycle = None;
    if unconverted == *before {
        *after = before.clone();
    }
}

/// #2199 Codex P2 4302542011 (FAMILIARS0 §5, §7): a stored familiar returns through the
/// return checks only (the lever boss room, the aggregate summon cap and placement), never
/// through the fresh-cast eligibility of §4 check 1. The source Login gate (Premium and its
/// minimum level) still decides the vocation-look grants and removals, but a lapsed Premium
/// or a lower level never drops a stored return: a clean row with positive time would then
/// neither return nor admit a replacement cast. The return plan contributes only the
/// creation (or its placement refusal, which keeps the row waiting); the looks stay the
/// source Login's.
fn plan_familiar_event_actions(
    spell: &CompiledNativeSpell,
    facts: &FamiliarFacts,
    stored_return: bool,
) -> Result<Vec<FamiliarAction>, CompanionExecutionError> {
    let plan = |facts: &FamiliarFacts| -> Result<Vec<FamiliarAction>, CompanionExecutionError> {
        match spell
            .plan(
                Facts::Companion(&CompanionFacts::Familiar(facts.clone())),
                &mut |min, _| min,
            )
            .map_err(|e| CompanionExecutionError::Refused(e.0))?
        {
            Plan::Companion(plan) => match *plan {
                super::native_companions::CompanionPlan::Familiar(actions) => Ok(actions),
                _ => Err(CompanionExecutionError::UnqualifiedProfile),
            },
            _ => Err(CompanionExecutionError::UnqualifiedProfile),
        }
    };
    let mut actions = plan(facts)?;
    let creation_decided = actions.iter().any(|a| {
        matches!(
            a,
            FamiliarAction::Create { .. } | FamiliarAction::CreationRefusal { .. }
        )
    });
    if !stored_return || facts.event != FamiliarEvent::Login || creation_decided {
        return Ok(actions);
    }
    let minimum_level = parameters(spell, "familiar_summon")?["login"]["minimum_level"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    let returning = FamiliarFacts {
        premium: true,
        level: facts.level.max(minimum_level),
        ..facts.clone()
    };
    actions.extend(plan(&returning)?.into_iter().filter(|a| {
        !matches!(
            a,
            FamiliarAction::RegisterAdvance
                | FamiliarAction::SelectLook(_)
                | FamiliarAction::GrantVocationLook(_)
                | FamiliarAction::RemoveVocationLook(_)
        )
    }));
    Ok(actions)
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
    ended_actor: Option<ExactActorRef>,
) -> Result<FamiliarParts, CompanionExecutionError> {
    if !matches!(
        event,
        FamiliarEvent::Cast
            | FamiliarEvent::Login
            | FamiliarEvent::Advance
            | FamiliarEvent::FamiliarDeath
            | FamiliarEvent::Expiry
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
    let removal = if matches!(event, FamiliarEvent::FamiliarDeath | FamiliarEvent::Expiry) {
        let actor = ended_actor.ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
        let dying = event == FamiliarEvent::FamiliarDeath;
        // A death removes the health-zero slot; an expiry removes the live familiar.
        let snapshot = if dying {
            runtime.companion_snapshot_including_dead(actor)?
        } else {
            runtime.companion_snapshot(actor)?
        };
        if (snapshot.health == 0) != dying
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
        if ended_actor.is_some() {
            return Err(CompanionExecutionError::InvalidOwnerFacts);
        }
        None
    };
    let mut after = current
        .state
        .for_next_write(
            current.checkpoint_committed_at_ms,
            &current.lifecycle_writer,
        )
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    let lifecycle = after
        .lifecycle
        .as_mut()
        .ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
    let unreconciled = event != FamiliarEvent::Login
        && familiar_lifecycle_unreconciled(lifecycle, &current.lifecycle_writer);
    let mut ended_in_lever_boss_room = false;
    if event == FamiliarEvent::Login {
        prepare_familiar_login_lifecycle(lifecycle, &current.lifecycle_writer)?;
        ended_in_lever_boss_room =
            end_familiar_return_in_lever_boss_room(lifecycle, current.lever_boss_room)?;
    } else if unreconciled && event == FamiliarEvent::Advance {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    // FAMILIARS0 §4 check 3: no cast inside a lever boss room or with an unresolved room,
    // before any cost.
    if event == FamiliarEvent::Cast && current.lever_boss_room != LeverBossRoomMembership::Outside {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    // FAMILIARS0 §4 check 4 / §5.2: a stored familiar waiting to return refuses the cast
    // before any cost, so a stored and a new familiar never exist together. An unreconciled
    // foreign open lifecycle refuses it the same way, fail closed, until Login reconciles it.
    if event == FamiliarEvent::Cast && (unreconciled || familiar_waits_to_return(lifecycle)) {
        return Err(CompanionExecutionError::Refused(
            p["failure_messages"]["summon_limit"]
                .as_str()
                .ok_or(CompanionExecutionError::UnqualifiedProfile)?
                .to_owned(),
        ));
    }
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
    let return_ms = (event == FamiliarEvent::Login).then_some(lifecycle.familiar_remaining_ms);
    let return_seconds = return_ms
        .map(|ms| i64::try_from(ms.div_ceil(1000)))
        .transpose()
        .map_err(|_| CompanionExecutionError::ArithmeticBounds)?;
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
        saved_expiry_unix: match return_seconds {
            Some(seconds) => unix
                .seconds()
                .checked_add(seconds)
                .ok_or(CompanionExecutionError::ArithmeticBounds)?,
            None => current.state.saved_expiry_unix,
        },
        last_logout_unix: if return_seconds.is_some() {
            unix.seconds()
        } else {
            current.state.last_logout_unix
        },
        owner_present: true,
        creature_present: removal.is_some(),
        creature_name: name.to_owned(),
        matching_summon_ids: Vec::new(),
        dx: 0,
        dy: 0,
        dz: 0,
        owner_tile_is_teleport: false,
    };
    let mut actions =
        plan_familiar_event_actions(spell, &facts, return_ms.is_some_and(|ms| ms > 0))?;
    if let Some(ms) = return_ms {
        let exact = i64::try_from(ms).map_err(|_| CompanionExecutionError::ArithmeticBounds)?;
        let rounded = return_seconds
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?
            .checked_mul(1000)
            .ok_or(CompanionExecutionError::ArithmeticBounds)?;
        let minimum = p["warning_dispatch"]["scheduler_minimum_ms"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
        for action in &mut actions {
            match action {
                FamiliarAction::ScheduleExpiry { delay_ms } => {
                    *delay_ms =
                        u32::try_from(ms).map_err(|_| CompanionExecutionError::ArithmeticBounds)?
                }
                FamiliarAction::ScheduleWarning {
                    raw_delay_ms,
                    delay_ms,
                    ..
                } => {
                    *raw_delay_ms = raw_delay_ms
                        .checked_sub(rounded - exact)
                        .ok_or(CompanionExecutionError::ArithmeticBounds)?;
                    *delay_ms = u32::try_from((*raw_delay_ms).max(0))
                        .map_err(|_| CompanionExecutionError::ArithmeticBounds)?
                        .max(minimum);
                }
                _ => {}
            }
        }
    }
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
                let duplicate = look.replace(*value).is_some();
                if duplicate {
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
    let acquired_remaining_ms = if spawn.is_some() {
        Some(u64::from(
            actions
                .iter()
                .find_map(|a| match a {
                    FamiliarAction::ScheduleExpiry { delay_ms } => Some(*delay_ms),
                    _ => None,
                })
                .ok_or(CompanionExecutionError::UnqualifiedProfile)?,
        ))
    } else {
        None
    };
    settle_familiar_event_lifecycle(
        after
            .lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?,
        &current.lifecycle_writer,
        &event,
        acquired_remaining_ms,
    )?;
    after
        .validate()
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    keep_unconverted_projection_on_no_op(&current.state, &mut after, &event);
    Ok(FamiliarParts {
        event,
        owner_facts: current.clone(),
        owned,
        removal,
        spawn,
        actions,
        facts,
        after,
        ended_in_lever_boss_room,
    })
}

/// Typed timer producers retain the actual cast owner/occurrence and exact future fixed-slot
/// incarnation. The common compositor reserves these in the real OwnerTimerLane before commit.
#[derive(Debug, Clone)]
pub struct FamiliarSchedule {
    pub phase: u16,
    pub due: SemanticTimeMicros,
    pub saved: SavedFamiliarTimer,
}
impl PreparedFamiliar {
    pub fn schedules(
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
    pub fn validate_current(
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
pub struct FamiliarApplyReceipt {
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
    pub fn creature(&self) -> Option<ExactActorRef> {
        self.creature
    }
    pub fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    pub fn owner_operations(&self) -> &[FamiliarAction] {
        &self.owner_operations
    }
    pub fn matches_saved(&self, saved: &SavedFamiliarTimer) -> bool {
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
pub fn commit_familiar(
    runtime: &mut ChannelRuntimeV1,
    prepared: PreparedFamiliar,
    current: &FamiliarOwnerFacts,
    original_fence: &CurrentCharacterGameplayFence,
    request: Option<&FamiliarStateRequest>,
    committed: Option<&CommittedCharacterFamiliar>,
) -> Result<FamiliarApplyReceipt, CompanionExecutionError> {
    let checked = check_commit_familiar(
        runtime,
        &prepared,
        current,
        original_fence,
        request,
        committed,
        None,
    )?;
    Ok(install_familiar(runtime, prepared, checked))
}

/// Proof that [`check_commit_familiar`] passed under the held guards. It carries the exact
/// receipt [`install_familiar`] returns, so later phase-1 checks can read it before any write.
#[derive(Debug)]
pub struct CheckedFamiliarApply {
    receipt: FamiliarApplyReceipt,
    session: GameSessionId,
}
impl CheckedFamiliarApply {
    pub fn receipt(&self) -> &FamiliarApplyReceipt {
        &self.receipt
    }
}

/// The borrowing check of [`commit_familiar`] (ARCH-SPELL-LOCK-2 §1.6). With `staged`, the
/// caller releases that batch's companion touches first in the same owner turn, and every
/// touched slot is read as released. It changes nothing and allocates only the receipt.
pub fn check_commit_familiar(
    runtime: &ChannelRuntimeV1,
    prepared: &PreparedFamiliar,
    current: &FamiliarOwnerFacts,
    original_fence: &CurrentCharacterGameplayFence,
    request: Option<&FamiliarStateRequest>,
    committed: Option<&CommittedCharacterFamiliar>,
    staged: Option<&crate::foundation::StagedSpellBatch>,
) -> Result<CheckedFamiliarApply, CompanionExecutionError> {
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
    let marks: &[CompanionSnapshot] = if prepared
        .actions
        .contains(&FamiliarAction::RegisterPartyProtectionAllOwnedSummons)
    {
        &prepared.owned
    } else {
        &[]
    };
    runtime.check_familiar_companion_apply(
        staged.map_or(&[][..], |staged| staged.companion_touch_indices()),
        prepared.binding.caster,
        original_fence.game_session_id,
        marks,
        prepared.death.as_ref(),
        prepared.spawn.as_ref(),
    )?;
    let mut owner_operations = prepared.actions.clone();
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
    Ok(CheckedFamiliarApply {
        receipt: FamiliarApplyReceipt {
            creature: prepared.spawn.as_ref().map(|spawn| spawn.actor()),
            after: prepared.after.clone(),
            binding: prepared.binding.clone(),
            owner_operations,
        },
        session: original_fence.game_session_id,
    })
}

/// The infallible apply, only after [`check_commit_familiar`] passed under the same held guards
/// (and, with its `staged` batch, after that batch's companion touch release).
#[allow(
    clippy::expect_used,
    reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
)]
pub fn install_familiar(
    runtime: &mut ChannelRuntimeV1,
    prepared: PreparedFamiliar,
    checked: CheckedFamiliarApply,
) -> FamiliarApplyReceipt {
    // Existing summons receive party protection before new creation. All snapshots were compared
    // in phase 1, and immutable policies/master facts cannot change under this exclusive owner turn.
    if prepared
        .actions
        .contains(&FamiliarAction::RegisterPartyProtectionAllOwnedSummons)
    {
        for snapshot in &prepared.owned {
            runtime
                .mark_companion_party_protection(snapshot)
                .expect("checked companion party protection");
        }
    }
    if let Some(dead) = &prepared.death {
        runtime
            .despawn_companion(prepared.binding.caster, checked.session, dead)
            .expect("checked companion despawn");
    }
    if let Some(spawn) = prepared.spawn {
        runtime
            .install_companion_spawn(spawn)
            .expect("checked reserved companion spawn");
    }
    checked.receipt
}

/// Source corpse and predicted spawn share an exact physical owner binding. Construction
/// requires the Item owner's locked durable corpse reservation, never a planner corpse flag.
#[derive(Debug)]
pub struct CompanionSpawnReservation {
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
    pub fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.current)?;
        runtime.reserve_companion_spawn(&mut self.spawn)?;
        Ok(())
    }
    pub fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        runtime.rollback_companion_spawn(&self.spawn)?;
        Ok(())
    }
    pub fn actor(&self) -> ExactActorRef {
        self.spawn.actor()
    }
    pub fn master_actor(&self) -> ExactActorRef {
        self.owner
    }
    pub fn character_id(&self) -> crate::domain::CharacterId {
        self.character_id
    }
    pub fn game_session_id(&self) -> GameSessionId {
        self.session
    }
    pub fn corpse_item_instance_id(&self) -> &[u8; 16] {
        &self.corpse
    }
    pub fn corpse_state_revision(&self) -> u64 {
        self.corpse_revision
    }
    pub fn spawn_cell(&self) -> LogicalCell {
        let p = self.spawn.position();
        LogicalCell {
            x: p.x,
            y: p.y,
            z: i32::from(p.floor),
        }
    }
    pub fn creature_definition_key(&self) -> &str {
        &self.spawn.policy().definition_key
    }
    pub fn creature_definition_revision(&self) -> &str {
        &self.spawn.policy().definition_revision
    }
    pub fn consume_rune_charge(&self) -> bool {
        self.consume_rune_charge
    }
    pub fn validate_current(
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
pub fn prepare_animate_dead(
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
pub fn install_reserved_companion(
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
pub fn finish_familiar_payment(
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
pub fn familiar_cost_binding(
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
pub enum FamiliarLifecycleOrigin {
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
    /// #2199 Codex P2 4302041461: the Channel owner's later return of the familiar this
    /// session's Login stored because no tile was free. `waiting_epoch` is the stored row's
    /// lifecycle epoch, so one waiting row has one occurrence and its committed return (a new
    /// epoch, an open row) ends it.
    Return {
        session: GameSessionId,
        waiting_epoch: u64,
    },
    /// #2199 Codex P2 4302737307: the due expiry of this live familiar at this lifecycle
    /// epoch. The owner cadence issues it from the actual timer lane deadline; the fenced
    /// lifecycle writer closes the row (remaining 0) before the familiar is despawned.
    Expiry {
        creature: ExactActorRef,
        lifecycle_epoch: u64,
    },
    /// #2199 Codex P1 4303261095 (FAMILIARS0 §7: owner dies, removed, 0): the owner's death
    /// `occurrence` ends this live familiar at this lifecycle epoch. The owner cadence issues
    /// it before the respawn; the fenced lifecycle writer closes the row (remaining 0) before
    /// the familiar is despawned, as an expiry does.
    OwnerDeath {
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
        creature: ExactActorRef,
        lifecycle_epoch: u64,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamiliarLifecycleBinding {
    spell: CompiledNativeSpell,
    owner: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterGameplayFence,
    stamp: crate::foundation::RuntimeWorkStamp,
    at: SemanticTimeMicros,
    origin: FamiliarLifecycleOrigin,
}
/// The live `creature` is `owner`'s familiar in `session` at `lifecycle_epoch`.
fn live_owned_familiar(
    runtime: &ChannelRuntimeV1,
    owner: ExactActorRef,
    session: GameSessionId,
    creature: ExactActorRef,
    lifecycle_epoch: u64,
) -> Result<(), CompanionExecutionError> {
    let live = runtime.companion_snapshot(creature)?;
    if live.state.master
        != Some(CompanionMaster {
            actor: owner,
            session,
        })
        || !live.state.policy.is_familiar
        || live.state.lifecycle_epoch != lifecycle_epoch
    {
        return Err(CompanionExecutionError::SnapshotMismatch);
    }
    Ok(())
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
    pub fn from_admission(
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
    pub fn from_level_advance(
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
    pub fn from_committed_lethal(
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
    /// Invoked by the owner-cadence retry for a stored return of this session. The stored row
    /// is re-proved against current owner facts in `prepare_lifecycle_familiar`.
    pub fn from_waiting_return(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        waiting_epoch: u64,
    ) -> Result<Self, CompanionExecutionError> {
        Self::issue(
            runtime,
            spell,
            owner,
            fence,
            stamp,
            at,
            FamiliarLifecycleOrigin::Return {
                session: fence.game_session_id,
                waiting_epoch,
            },
        )
    }
    /// Invoked by the owner cadence for a lifecycle expiry the timer lane found due. The live
    /// familiar must still be this session's familiar at that lifecycle epoch.
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    pub fn from_due_expiry(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        creature: ExactActorRef,
        lifecycle_epoch: u64,
    ) -> Result<Self, CompanionExecutionError> {
        live_owned_familiar(
            runtime,
            owner,
            fence.game_session_id,
            creature,
            lifecycle_epoch,
        )?;
        Self::issue(
            runtime,
            spell,
            owner,
            fence,
            stamp,
            at,
            FamiliarLifecycleOrigin::Expiry {
                creature,
                lifecycle_epoch,
            },
        )
    }
    /// #2199 Codex P1 4303261095: invoked by the owner cadence for the owner's settled death
    /// `occurrence` (the caller re-proves it under the owner lock) before the respawn. The live
    /// familiar must still be this session's familiar at that lifecycle epoch.
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    pub fn from_owner_death(
        runtime: &ChannelRuntimeV1,
        spell: CompiledNativeSpell,
        owner: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        stamp: crate::foundation::RuntimeWorkStamp,
        at: SemanticTimeMicros,
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
        creature: ExactActorRef,
        lifecycle_epoch: u64,
    ) -> Result<Self, CompanionExecutionError> {
        live_owned_familiar(
            runtime,
            owner,
            fence.game_session_id,
            creature,
            lifecycle_epoch,
        )?;
        Self::issue(
            runtime,
            spell,
            owner,
            fence,
            stamp,
            at,
            FamiliarLifecycleOrigin::OwnerDeath {
                occurrence,
                creature,
                lifecycle_epoch,
            },
        )
    }
    pub fn spell(&self) -> &CompiledNativeSpell {
        &self.spell
    }
    pub fn owner(&self) -> ExactActorRef {
        self.owner
    }
    pub fn session(&self) -> GameSessionId {
        self.session
    }
    pub fn fence(&self) -> &CurrentCharacterGameplayFence {
        &self.fence
    }
    pub fn stamp(&self) -> crate::foundation::RuntimeWorkStamp {
        self.stamp
    }
    pub fn at(&self) -> SemanticTimeMicros {
        self.at
    }
    pub fn origin(&self) -> &FamiliarLifecycleOrigin {
        &self.origin
    }
}
#[derive(Debug, Clone)]
pub struct SavedLifecycleFamiliarTimer {
    pub binding: FamiliarLifecycleBinding,
    pub creature: ExactActorRef,
    pub lifecycle_epoch: u64,
    pub event: FamiliarTimerEvent,
}
/// Same real physical owner/epoch checks as Cast timers; the source identity has no client command.
pub fn stage_lifecycle_familiar_timer(
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
pub struct PreparedLifecycleFamiliar {
    binding: FamiliarLifecycleBinding,
    parts: FamiliarParts,
}
#[derive(Debug, Clone)]
pub struct LifecycleFamiliarSchedule {
    pub phase: u16,
    pub due: SemanticTimeMicros,
    pub saved: SavedLifecycleFamiliarTimer,
}
#[derive(Debug, Clone)]
pub struct LifecycleFamiliarApplyReceipt {
    binding: FamiliarLifecycleBinding,
    creature: Option<ExactActorRef>,
    after: DurableFamiliarState,
    operations: Vec<FamiliarAction>,
}
impl LifecycleFamiliarApplyReceipt {
    pub fn binding(&self) -> &FamiliarLifecycleBinding {
        &self.binding
    }
    pub fn creature(&self) -> Option<ExactActorRef> {
        self.creature
    }
    pub fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    pub fn owner_operations(&self) -> &[FamiliarAction] {
        &self.operations
    }
    pub fn matches_saved(&self, saved: &SavedLifecycleFamiliarTimer) -> bool {
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
    pub fn reservation_actors(&self) -> impl Iterator<Item = ExactActorRef> + '_ {
        self.parts
            .owned
            .iter()
            .map(|s| s.actor)
            .chain(self.parts.removal.iter().map(|s| s.actor))
    }
    pub fn reserve_physical(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        self.validate_current(runtime, &self.parts.owner_facts)?;
        for owned in &self.parts.owned {
            runtime.assert_actor_spell_unreserved(owned.actor)?;
        }
        if let Some(ended) = &self.parts.removal {
            runtime.assert_actor_spell_unreserved(ended.actor)?;
        }
        if let Some(spawn) = self.parts.spawn.as_mut() {
            runtime.reserve_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub fn rollback_physical(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), CompanionExecutionError> {
        if let Some(spawn) = self.parts.spawn.as_ref() {
            runtime.rollback_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub fn binding(&self) -> &FamiliarLifecycleBinding {
        &self.binding
    }
    pub fn owner_facts(&self) -> &FamiliarOwnerFacts {
        &self.parts.owner_facts
    }
    pub fn after(&self) -> &DurableFamiliarState {
        &self.parts.after
    }
    pub fn state_request(
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
    pub fn validate_current(
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
        if let Some(ended) = &self.parts.removal {
            runtime.validate_companion_snapshot(ended)?;
        }
        if let Some(spawn) = &self.parts.spawn {
            runtime.validate_companion_spawn(spawn)?;
        }
        Ok(())
    }
    pub fn matches_saved(&self, saved: &SavedLifecycleFamiliarTimer) -> bool {
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
    pub fn schedules(
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
pub fn prepare_lifecycle_familiar(
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
    let returning = matches!(binding.origin(), FamiliarLifecycleOrigin::Return { .. });
    if let FamiliarLifecycleOrigin::Return { waiting_epoch, .. } = binding.origin()
        && !familiar_return_admissible(
            &current.state,
            current.checkpoint_committed_at_ms,
            &current.lifecycle_writer,
            *waiting_epoch,
        )
    {
        return Err(CompanionExecutionError::InvalidOwnerFacts);
    }
    // A stored return replays the source Login recreation with the stored remaining
    // lifetime; the cooldown was restored by that Login and is never restored again. The
    // fresh-cast eligibility (Premium, level) never drops it (`plan_familiar_event_actions`).
    let (event, dead) = match binding.origin() {
        FamiliarLifecycleOrigin::Login { .. } | FamiliarLifecycleOrigin::Return { .. } => {
            (FamiliarEvent::Login, None)
        }
        FamiliarLifecycleOrigin::Advance { .. } => (FamiliarEvent::Advance, None),
        FamiliarLifecycleOrigin::Death { occurrence } => {
            (FamiliarEvent::FamiliarDeath, Some(occurrence.actor()))
        }
        FamiliarLifecycleOrigin::Expiry {
            creature,
            lifecycle_epoch,
        } => {
            if *lifecycle_epoch != current.state.lifecycle_epoch {
                return Err(CompanionExecutionError::SnapshotMismatch);
            }
            (FamiliarEvent::Expiry, Some(*creature))
        }
        // The owner's death removes its live familiar with remaining 0 (FAMILIARS0 §7), the
        // same removal and closed row as an expiry; the source OwnerRemoval callback invokes
        // no familiar death.
        FamiliarLifecycleOrigin::OwnerDeath {
            creature,
            lifecycle_epoch,
            ..
        } => {
            if *lifecycle_epoch != current.state.lifecycle_epoch {
                return Err(CompanionExecutionError::SnapshotMismatch);
            }
            (FamiliarEvent::Expiry, Some(*creature))
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
    // A return writes only its creation, or its end inside a lever boss room: no free tile,
    // or a full owned-summon census leaves the row waiting unwritten,
    // and a familiar already present is never doubled.
    if returning {
        if parts.owned.iter().any(|s| s.state.policy.is_familiar) {
            return Err(CompanionExecutionError::SnapshotMismatch);
        }
        if parts.spawn.is_some() && !familiar_return_within_summon_cap(parts.owned.len()) {
            return Err(CompanionExecutionError::PlacementUnavailable);
        }
        if parts.spawn.is_none() && !parts.ended_in_lever_boss_room {
            return Err(CompanionExecutionError::PlacementUnavailable);
        }
    }
    if let Some(spawn) = parts.spawn.as_mut() {
        spawn.bind_semantic_creation(binding.at().get())?;
    }
    Ok(PreparedLifecycleFamiliar { binding, parts })
}
/// Real projection/write receipt is mandatory for a changed snapshot. No no-op receipt,
/// caster cost, training payment or client command is created for a lifecycle event.
pub fn commit_lifecycle_familiar(
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
    // #2199 Codex P2 4302737307: an expiry likewise removes its live familiar only after
    // the closed row (remaining 0) is durable; a failed write leaves it present.
    if let Some(ended) = &prepared.parts.removal {
        runtime.despawn_companion(prepared.binding.owner(), prepared.binding.session(), ended)?;
    }
    let creature = prepared
        .parts
        .spawn
        .map(|spawn| runtime.install_companion_spawn(spawn))
        .transpose()?;
    let mut operations = prepared.parts.actions;
    // The removal above performed an expiry's RemoveCreature.
    operations.retain(|a| {
        !matches!(
            a,
            FamiliarAction::Create { .. }
                | FamiliarAction::RemoveCreature
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
#[allow(
    clippy::too_many_arguments,
    reason = "actual owner snapshot and independently validated actor-end identity"
)]
pub fn familiar_logout_snapshot(
    current: &DurableFamiliarState,
    player: &super::cast::PlayerSpellState,
    profile: &CompiledNativeSpell,
    book: &super::SpellBook,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    unix: FamiliarUnixTime,
    checkpoint_committed_at_ms: Option<i64>,
    writer: &FamiliarLifecycleWriter,
    physical_remaining_ms: Option<u64>,
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
    let mut next = current
        .for_next_write(checkpoint_committed_at_ms, writer)
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    close_familiar_lifecycle_at_actor_end(
        next.lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?,
        writer,
        physical_remaining_ms,
    )?;
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
pub fn restore_familiar_cooldown(
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
mod familiar_login_lifecycle_tests {
    use super::*;
    use crate::durability::character_familiar::{FamiliarCooldownSnapshot, FamiliarLifecycleV2};

    #[test]
    fn foreign_open_login_discards_crash_remaining_and_preserves_writer_and_cooldown()
    -> Result<(), CompanionExecutionError> {
        let old_writer = FamiliarLifecycleWriter {
            game_session_id: "0199aabb-1234-7000-8000-000000000001".into(),
            connection_generation: 1,
            world_id: "0199aabb-1234-7000-8000-000000000002".into(),
            channel_id: "0199aabb-1234-7000-8000-000000000003".into(),
            scope_ownership_generation: 4,
        };
        let current_writer = FamiliarLifecycleWriter {
            game_session_id: "0199aabb-1234-7000-8000-000000000004".into(),
            ..old_writer.clone()
        };
        let mut state = DurableFamiliarState {
            lifecycle: Some(FamiliarLifecycleV2 {
                version: 2,
                familiar_remaining_ms: 90_000,
                open: true,
                writer: old_writer.clone(),
                clean_end_game_session_id: None,
            }),
            cooldowns: vec![FamiliarCooldownSnapshot {
                reference_spell_id: 197,
                remaining_micros: 1_234,
            }],
            ..DurableFamiliarState::default()
        };
        let before = state.clone();
        let lifecycle = state
            .lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
        prepare_familiar_login_lifecycle(lifecycle, &current_writer)?;
        assert_eq!(lifecycle.familiar_remaining_ms, 0);
        assert!(!lifecycle.open);
        assert_eq!(lifecycle.writer, old_writer);
        assert_eq!(lifecycle.clean_end_game_session_id, None);
        assert_eq!(state.cooldowns, before.cooldowns);
        let mut expected = before;
        expected
            .lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?
            .familiar_remaining_ms = 0;
        expected
            .lifecycle
            .as_mut()
            .ok_or(CompanionExecutionError::InvalidOwnerFacts)?
            .open = false;
        assert_eq!(state, expected);
        Ok(())
    }

    /// #2199 Codex P1 4302326064: the owner's current session scope resolves lever boss room
    /// membership. A Channel scope is outside (the fenced write re-proves the Channel scope);
    /// an instance scope stays unresolved without the BOSS-ROOM-1 owner and is never guessed.
    #[test]
    fn lever_boss_room_membership_comes_from_the_current_scope()
    -> Result<(), CompanionExecutionError> {
        let uuid = |seed: u8| {
            let mut bytes = [0_u8; 16];
            bytes[0] = 1;
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes[15] = seed;
            bytes
        };
        let world = crate::foundation::WorldId::decode(&uuid(1))
            .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
        let channel = crate::foundation::ChannelId::decode(&uuid(2))
            .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
        let channel_scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
        let instance_scope = crate::foundation::RuntimeScopeRefV1::instance(world, uuid(3))
            .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
        assert_eq!(
            LeverBossRoomMembership::of_current_scope(&channel_scope),
            LeverBossRoomMembership::Outside
        );
        assert_eq!(
            LeverBossRoomMembership::of_current_scope(&instance_scope),
            LeverBossRoomMembership::Unresolved
        );
        Ok(())
    }

    /// #2199 Codex P1 4302326064 (FAMILIARS0 §7): a waiting return inside a lever boss room
    /// ends the stored familiar (time 0, row closed, writer and marker kept); outside it is
    /// untouched, and an unresolved room refuses so the row stays waiting. A row that does not
    /// wait is never changed by the room check.
    #[test]
    fn lever_boss_room_return_ends_the_stored_familiar() -> Result<(), CompanionExecutionError> {
        let (old_writer, _) = writers();
        let waiting = lifecycle(&old_writer, 120_000, false, Some(&old_writer));

        let mut inside = waiting.clone();
        assert!(end_familiar_return_in_lever_boss_room(
            &mut inside,
            LeverBossRoomMembership::Inside
        )?);
        let mut ended = waiting.clone();
        ended.familiar_remaining_ms = 0;
        assert_eq!(inside, ended);
        assert!(!familiar_waits_to_return(&inside));

        let mut outside = waiting.clone();
        assert!(!end_familiar_return_in_lever_boss_room(
            &mut outside,
            LeverBossRoomMembership::Outside
        )?);
        assert_eq!(outside, waiting);

        let mut unresolved = waiting.clone();
        assert_eq!(
            end_familiar_return_in_lever_boss_room(
                &mut unresolved,
                LeverBossRoomMembership::Unresolved
            ),
            Err(CompanionExecutionError::InvalidOwnerFacts)
        );
        assert_eq!(unresolved, waiting);

        for not_waiting in [
            lifecycle(&old_writer, 120_000, true, None),
            lifecycle(&old_writer, 0, false, Some(&old_writer)),
        ] {
            for room in [
                LeverBossRoomMembership::Outside,
                LeverBossRoomMembership::Inside,
                LeverBossRoomMembership::Unresolved,
            ] {
                let mut row = not_waiting.clone();
                assert!(!end_familiar_return_in_lever_boss_room(&mut row, room)?);
                assert_eq!(row, not_waiting);
            }
        }
        Ok(())
    }

    /// #2199 Codex P1 4302326065 (CREATURE-AI-0 RL-19): a stored return checks the complete
    /// owned-summon census against the aggregate cap of two; at the cap it keeps waiting.
    #[test]
    fn familiar_return_respects_the_aggregate_summon_cap() {
        assert!(familiar_return_within_summon_cap(0));
        assert!(familiar_return_within_summon_cap(1));
        assert!(!familiar_return_within_summon_cap(
            CREATUREAI0_PLAYER_SUMMONS_PER_CHARACTER_MAX
        ));
        assert!(!familiar_return_within_summon_cap(3));
    }

    fn knight_familiar_profile() -> Result<CompiledNativeSpell, CompanionExecutionError> {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .map_err(|_| CompanionExecutionError::UnqualifiedProfile)?;
        let row = document["profiles"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|r| {
                    let behavior = &r["spell"]["execution"]["native_behavior"];
                    behavior["key"] == "familiar_summon"
                        && behavior["parameters"]["vocation"] == "knight"
                        && behavior["parameters"]["login"]["lifetime_basis"] == "current_unix_time"
                })
            })
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
        super::super::native::spell_from_bundle(
            &serde_json::json!({"spell": row["spell"]}),
            &row["dependencies"],
        )
        .map_err(|_| CompanionExecutionError::UnqualifiedProfile)
    }

    /// #2199 Codex P2 4302542011 (FAMILIARS0 §5, §7): a clean stored familiar returns at login
    /// after Premium lapsed or the level dropped, because a return passes only the lever boss
    /// room, the summon cap and placement. Without a free tile it reports the placement refusal
    /// that keeps the row waiting for the in-session retry. The source Login look handling
    /// (removing the vocation look without Premium) is kept, no cooldown is restarted, and a
    /// login without stored time still follows the source Premium gate.
    #[test]
    fn stored_familiar_returns_at_login_without_fresh_cast_eligibility()
    -> Result<(), CompanionExecutionError> {
        let spell = knight_familiar_profile()?;
        let p = parameters(&spell, "familiar_summon")?;
        let look = p["default_look_type"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
        let lapsed = FamiliarFacts {
            event: FamiliarEvent::Login,
            vocation: "knight".into(),
            premium: false,
            level: 250,
            account_at_least_god: false,
            owned_summons: 0,
            spawn_room: true,
            chosen_look: 0,
            has_vocation_look: true,
            owner_current_speed: 220,
            familiar_base_speed: 200,
            familiar_minutes: 30,
            vip: false,
            vip_reduction_minutes: 0,
            cooldown_rate: 1.0,
            now_unix: 10_000,
            saved_expiry_unix: 10_090,
            last_logout_unix: 10_000,
            owner_present: true,
            creature_present: false,
            creature_name: p["creature_name"]
                .as_str()
                .ok_or(CompanionExecutionError::UnqualifiedProfile)?
                .to_owned(),
            matching_summon_ids: Vec::new(),
            dx: 0,
            dy: 0,
            dz: 0,
            owner_tile_is_teleport: false,
        };
        let creates = |actions: &[FamiliarAction]| {
            actions
                .iter()
                .filter(|a| matches!(a, FamiliarAction::Create { .. }))
                .count()
        };

        // The source Login alone drops the creation without Premium: the stuck row.
        let source = plan_familiar_event_actions(&spell, &lapsed, false)?;
        assert_eq!(creates(&source), 0);
        assert!(source.contains(&FamiliarAction::RemoveVocationLook(look)));

        // A stored return comes back with its stored lifetime and keeps the source looks.
        let restored = plan_familiar_event_actions(&spell, &lapsed, true)?;
        assert_eq!(creates(&restored), 1);
        assert!(restored.iter().any(|a| matches!(
            a,
            FamiliarAction::Create { name, look: created, owned: true, .. }
                if *name == lapsed.creature_name && *created == look
        )));
        assert!(restored.contains(&FamiliarAction::StoreExpiry(10_090)));
        assert!(
            restored
                .iter()
                .any(|a| matches!(a, FamiliarAction::ScheduleExpiry { .. }))
        );
        assert!(restored.contains(&FamiliarAction::RemoveVocationLook(look)));
        assert!(!restored.iter().any(|a| matches!(
            a,
            FamiliarAction::SelectLook(_)
                | FamiliarAction::GrantVocationLook(_)
                | FamiliarAction::Cooldown { .. }
        )));
        assert_eq!(
            restored
                .iter()
                .filter(|a| **a == FamiliarAction::RegisterAdvance)
                .count(),
            1
        );

        // Below the source login level the stored familiar returns the same way.
        let low_level = FamiliarFacts {
            premium: true,
            level: 120,
            ..lapsed.clone()
        };
        assert_eq!(
            creates(&plan_familiar_event_actions(&spell, &low_level, false)?),
            0
        );
        assert_eq!(
            creates(&plan_familiar_event_actions(&spell, &low_level, true)?),
            1
        );

        // No free tile: the placement refusal keeps the row waiting for the in-session retry.
        let no_tile = FamiliarFacts {
            spawn_room: false,
            ..lapsed.clone()
        };
        let waiting = plan_familiar_event_actions(&spell, &no_tile, true)?;
        assert_eq!(creates(&waiting), 0);
        assert!(login_left_familiar_waiting(None, &waiting));

        // An eligible owner's plan is exactly the source Login's.
        let eligible = FamiliarFacts {
            premium: true,
            has_vocation_look: false,
            ..lapsed.clone()
        };
        assert_eq!(
            plan_familiar_event_actions(&spell, &eligible, true)?,
            plan_familiar_event_actions(&spell, &eligible, false)?
        );
        Ok(())
    }

    fn writers() -> (FamiliarLifecycleWriter, FamiliarLifecycleWriter) {
        let old_writer = FamiliarLifecycleWriter {
            game_session_id: "0199aabb-1234-7000-8000-000000000001".into(),
            connection_generation: 1,
            world_id: "0199aabb-1234-7000-8000-000000000002".into(),
            channel_id: "0199aabb-1234-7000-8000-000000000003".into(),
            scope_ownership_generation: 4,
        };
        let current_writer = FamiliarLifecycleWriter {
            game_session_id: "0199aabb-1234-7000-8000-000000000004".into(),
            ..old_writer.clone()
        };
        (old_writer, current_writer)
    }

    fn lifecycle(
        writer: &FamiliarLifecycleWriter,
        remaining: u64,
        open: bool,
        marker: Option<&FamiliarLifecycleWriter>,
    ) -> FamiliarLifecycleV2 {
        FamiliarLifecycleV2 {
            version: 2,
            familiar_remaining_ms: remaining,
            open,
            writer: writer.clone(),
            clean_end_game_session_id: marker.map(|w| w.game_session_id.clone()),
        }
    }

    /// #2199 Codex P1 4301237612 / 4301237626: both actor ends (logout and the end of the
    /// in-fight deadline) close the row with this session's clean-end marker, which the durable
    /// clean-end proof accepts; with no physical familiar a waiting clean row keeps its stored
    /// time and only an open row that already ended here stores zero.
    #[test]
    fn actor_end_close_marks_clean_end_and_keeps_waiting_time()
    -> Result<(), CompanionExecutionError> {
        use crate::durability::character_familiar::{
            FamiliarCleanEndProof, clean_end_marker_requirement,
        };
        let (old_writer, current) = writers();
        let transport = crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
            .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
        let cases = [
            // a live familiar: its actual timer remaining wins
            (
                lifecycle(&current, 900_000, true, None),
                Some(41_000),
                41_000,
            ),
            // waiting to return (no free tile at login): stored time survives the end
            (
                lifecycle(&current, 90_000, false, Some(&old_writer)),
                None,
                90_000,
            ),
            // an acquisition whose familiar already ended in this runtime
            (lifecycle(&current, 900_000, true, None), None, 0),
        ];
        for (before, physical, expected) in cases {
            let mut after = before.clone();
            close_familiar_lifecycle_at_actor_end(&mut after, &current, physical)?;
            assert_eq!(after.familiar_remaining_ms, expected);
            assert!(!after.open);
            assert_eq!(after.writer, current);
            assert_eq!(
                after.clean_end_game_session_id.as_deref(),
                Some(current.game_session_id.as_str())
            );
            for proof in [
                FamiliarCleanEndProof::GraceExpiry(1),
                FamiliarCleanEndProof::Logout(transport),
            ] {
                let required = clean_end_marker_requirement(
                    before.clean_end_game_session_id.as_deref(),
                    after.clean_end_game_session_id.as_deref(),
                    &current.game_session_id,
                    Some(proof),
                );
                assert!(matches!(required, Ok(Some(p)) if p == proof));
            }
            // F-I11: after its own clean end the session writes nothing more.
            let mut again = after.clone();
            assert_eq!(
                close_familiar_lifecycle_at_actor_end(&mut again, &current, physical),
                Err(CompanionExecutionError::InvalidOwnerFacts)
            );
            assert_eq!(again, after);
        }
        Ok(())
    }

    /// #2199 Codex P1 4301237622: a Login that finds no free tile carries the previous
    /// session's clean row (marker, stored waiting time) as the ordinary write accepts it, so
    /// the cooldown and the delayed return are kept; this session's own marker is refused.
    #[test]
    fn no_room_login_carries_foreign_clean_row() -> Result<(), CompanionExecutionError> {
        use crate::durability::character_familiar::clean_end_marker_requirement;
        let (old_writer, current) = writers();
        let before = lifecycle(&old_writer, 90_000, false, Some(&old_writer));
        let mut after = before.clone();
        prepare_familiar_login_lifecycle(&mut after, &current)?;
        settle_familiar_event_lifecycle(&mut after, &current, &FamiliarEvent::Login, None)?;
        assert_eq!(
            after,
            FamiliarLifecycleV2 {
                writer: current.clone(),
                ..before.clone()
            }
        );
        assert!(familiar_waits_to_return(&after));
        assert!(matches!(
            clean_end_marker_requirement(
                before.clean_end_game_session_id.as_deref(),
                after.clean_end_game_session_id.as_deref(),
                &current.game_session_id,
                None,
            ),
            Ok(None)
        ));
        // The later delayed return (Advance) acquires: open, marker cleared, new lifetime.
        let mut returned = after.clone();
        settle_familiar_event_lifecycle(
            &mut returned,
            &current,
            &FamiliarEvent::Advance,
            Some(90_000),
        )?;
        assert!(returned.open);
        assert_eq!(returned.clean_end_game_session_id, None);
        assert!(matches!(
            clean_end_marker_requirement(
                after.clean_end_game_session_id.as_deref(),
                None,
                &current.game_session_id,
                None,
            ),
            Ok(None)
        ));
        let mut own = lifecycle(&current, 90_000, false, Some(&current));
        assert_eq!(
            settle_familiar_event_lifecycle(&mut own, &current, &FamiliarEvent::Login, None),
            Err(CompanionExecutionError::InvalidOwnerFacts)
        );
        Ok(())
    }

    /// #2199 Codex P2 4302737307: a due expiry closes the open row with no time left, as a
    /// familiar death does, so an expired familiar never leaves an open row with positive
    /// time. A write that ends nothing (Advance) keeps the open row and its time.
    #[test]
    fn familiar_expiry_closes_the_open_lifecycle_like_death() -> Result<(), CompanionExecutionError>
    {
        let (_, current) = writers();
        for event in [FamiliarEvent::Expiry, FamiliarEvent::FamiliarDeath] {
            let mut row = lifecycle(&current, 900_000, true, None);
            settle_familiar_event_lifecycle(&mut row, &current, &event, None)?;
            assert_eq!(row, lifecycle(&current, 0, false, None));
        }
        let mut row = lifecycle(&current, 900_000, true, None);
        settle_familiar_event_lifecycle(&mut row, &current, &FamiliarEvent::Advance, None)?;
        assert_eq!(row, lifecycle(&current, 900_000, true, None));
        Ok(())
    }

    /// #2199 Codex P1 4301955863: an open row from a prior session or runtime that Login has
    /// not reconciled blocks Cast and Advance; this lifecycle's own open row (a same-runtime
    /// reconnect at another connection generation) and any closed row do not.
    #[test]
    fn foreign_open_lifecycle_is_unreconciled_until_login() -> Result<(), CompanionExecutionError> {
        let (old_writer, current) = writers();
        assert!(familiar_lifecycle_unreconciled(
            &lifecycle(&old_writer, 90_000, true, None),
            &current
        ));
        let moved = FamiliarLifecycleWriter {
            scope_ownership_generation: 5,
            ..current.clone()
        };
        assert!(familiar_lifecycle_unreconciled(
            &lifecycle(&moved, 90_000, true, None),
            &current
        ));
        let reconnect = FamiliarLifecycleWriter {
            connection_generation: 2,
            ..current.clone()
        };
        assert!(!familiar_lifecycle_unreconciled(
            &lifecycle(&reconnect, 90_000, true, None),
            &current
        ));
        assert!(!familiar_lifecycle_unreconciled(
            &lifecycle(&old_writer, 90_000, false, Some(&old_writer)),
            &current
        ));
        assert!(!familiar_lifecycle_unreconciled(
            &lifecycle(&old_writer, 0, false, None),
            &current
        ));
        // The waiting-to-return check alone does not catch it; the unreconciled check
        // refuses that cast before payment.
        assert!(!familiar_waits_to_return(&lifecycle(
            &old_writer,
            90_000,
            true,
            None
        )));
        // Login is the reconciliation: afterwards the row is closed and no longer blocks.
        let mut reconciled = lifecycle(&old_writer, 90_000, true, None);
        prepare_familiar_login_lifecycle(&mut reconciled, &current)?;
        assert!(!familiar_lifecycle_unreconciled(&reconciled, &current));
        assert!(!familiar_waits_to_return(&reconciled));
        Ok(())
    }

    /// #2199 Codex P2 4301955868: a Login or Advance that changes nothing but the D1126
    /// conversion of an absent projection keeps it absent, so no row or revision is written.
    #[test]
    fn no_op_event_keeps_absent_projection_absent() -> Result<(), CompanionExecutionError> {
        let (_, current) = writers();
        let absent = DurableFamiliarState::default();
        for event in [FamiliarEvent::Login, FamiliarEvent::Advance] {
            let mut after = absent
                .for_next_write(None, &current)
                .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
            let converted = after
                .lifecycle
                .as_mut()
                .ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
            if event == FamiliarEvent::Login {
                prepare_familiar_login_lifecycle(converted, &current)?;
            }
            settle_familiar_event_lifecycle(converted, &current, &event, None)?;
            assert_ne!(after, absent);
            let mut kept = after.clone();
            keep_unconverted_projection_on_no_op(&absent, &mut kept, &event);
            assert_eq!(kept, absent);
            // A real mutation keeps the conversion, which every new write requires.
            let mut changed = after.clone();
            changed.selected_look = 7;
            let expected = changed.clone();
            keep_unconverted_projection_on_no_op(&absent, &mut changed, &event);
            assert_eq!(changed, expected);
            assert!(changed.lifecycle.is_some());
        }
        // A Cast always writes, and a stored v2 row is never reverted.
        let mut cast = absent
            .for_next_write(None, &current)
            .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
        let expected = cast.clone();
        keep_unconverted_projection_on_no_op(&absent, &mut cast, &FamiliarEvent::Cast);
        assert_eq!(cast, expected);
        let (old_writer, _) = writers();
        let stored = DurableFamiliarState {
            lifecycle: Some(lifecycle(&old_writer, 0, false, None)),
            ..DurableFamiliarState::default()
        };
        let mut next = stored.clone();
        settle_familiar_event_lifecycle(
            next.lifecycle
                .as_mut()
                .ok_or(CompanionExecutionError::InvalidOwnerFacts)?,
            &current,
            &FamiliarEvent::Login,
            None,
        )?;
        let expected = next.clone();
        keep_unconverted_projection_on_no_op(&stored, &mut next, &FamiliarEvent::Login);
        assert_eq!(next, expected);
        Ok(())
    }

    /// #2199 Codex P1 4301237630 (FAMILIARS0 §4 check 4): a stored familiar waiting to return
    /// refuses a cast; an open acquisition or a row with no stored time does not wait.
    #[test]
    fn waiting_stored_familiar_blocks_cast() {
        let (old_writer, current) = writers();
        assert!(familiar_waits_to_return(&lifecycle(
            &current,
            90_000,
            false,
            Some(&old_writer)
        )));
        assert!(familiar_waits_to_return(&lifecycle(
            &current, 1, false, None
        )));
        assert!(!familiar_waits_to_return(&lifecycle(
            &current, 90_000, true, None
        )));
        assert!(!familiar_waits_to_return(&lifecycle(
            &current,
            0,
            false,
            Some(&old_writer)
        )));
    }

    /// #2199 Codex P2 4302041461: a runtime return admits only the row this session's Login
    /// left waiting, at its epoch, so the stored familiar returns once and is never doubled
    /// or adopted from another lifecycle.
    #[test]
    fn stored_return_admits_only_this_lifecycles_waiting_row() {
        let (old_writer, current) = writers();
        let row = |lifecycle: Option<FamiliarLifecycleV2>, epoch: u64| DurableFamiliarState {
            lifecycle,
            lifecycle_epoch: epoch,
            ..DurableFamiliarState::default()
        };
        // Login left it waiting: clean, stored time, this writer, the previous session's marker.
        let waiting = row(
            Some(lifecycle(&current, 90_000, false, Some(&old_writer))),
            3,
        );
        assert!(familiar_return_admissible(&waiting, None, &current, 3));
        // A reconnect of the same lifecycle at another connection generation still returns it.
        let reconnect = FamiliarLifecycleWriter {
            connection_generation: 2,
            ..current.clone()
        };
        assert!(familiar_return_admissible(&waiting, None, &reconnect, 3));
        // A different epoch: the return already committed (or another row replaced it).
        assert!(!familiar_return_admissible(&waiting, None, &current, 2));
        assert!(!familiar_return_admissible(&waiting, None, &current, 4));
        // The return committed: the row is open with its scheduled lifetime.
        let open = row(Some(lifecycle(&current, 90_000, true, None)), 4);
        assert!(!familiar_return_admissible(&open, None, &current, 4));
        // No stored time, a row Login has not reconciled, or this session's own clean end.
        let empty = row(Some(lifecycle(&current, 0, false, None)), 3);
        assert!(!familiar_return_admissible(&empty, None, &current, 3));
        let foreign = row(
            Some(lifecycle(&old_writer, 90_000, false, Some(&old_writer))),
            3,
        );
        assert!(!familiar_return_admissible(&foreign, None, &current, 3));
        let ended = row(Some(lifecycle(&current, 90_000, false, Some(&current))), 3);
        assert!(!familiar_return_admissible(&ended, None, &current, 3));
        // A legacy row that Login kept unconverted returns through the same D1126 conversion
        // as its next write; without its checkpoint it is unknown and refused.
        let legacy = DurableFamiliarState {
            saved_expiry_unix: 2_000,
            lifecycle_epoch: 3,
            ..DurableFamiliarState::default()
        };
        assert!(familiar_return_admissible(
            &legacy,
            Some(1_000_000),
            &current,
            3
        ));
        assert!(!familiar_return_admissible(&legacy, None, &current, 3));
        assert!(!familiar_return_admissible(
            &legacy,
            Some(2_000_000),
            &current,
            3
        ));
    }

    /// #2199 Codex P2 4302041461: only a Login stopped by the source spawn-room refusal leaves
    /// a return waiting; a created familiar or a Login without that refusal does not.
    #[test]
    fn only_a_no_room_login_leaves_a_return_waiting() {
        let refusal = FamiliarAction::CreationRefusal {
            message: "no room".into(),
            poff: true,
        };
        assert!(login_left_familiar_waiting(
            None,
            &[FamiliarAction::RegisterAdvance, refusal]
        ));
        assert!(!login_left_familiar_waiting(
            None,
            &[FamiliarAction::RegisterAdvance]
        ));
        assert!(!login_left_familiar_waiting(None, &[]));
    }
}
