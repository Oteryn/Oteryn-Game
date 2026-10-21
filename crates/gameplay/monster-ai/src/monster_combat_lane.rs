//! Native AI combat boundary; the composition root supplies encounter behavior.
use crate::ability::RevisionSet;
use crate::ai_think::{ThinkOccurrence, profile_schedule::MonsterSummonFacts};
use crate::content::{ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef as Ref, ProjectV2Draft};
use crate::creature_auto_attack::CurrentCombatFactsReader;
use crate::creature_damage_spell::{CreatureCombatFacts, SpellWorldReader};
use crate::foundation::owner_timer::OwnerClock;
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp,
    ScopeRuntimeFence, TickFacts,
};
use crate::spell::chain::TilePosition;
mod states;
pub use states::MonsterCombatStates;
pub mod native_conditions;
pub mod native_invisibility;
pub mod native_ticks;
use native_invisibility::InvisibleTileCombatPolicy;
use native_ticks::NativeRegenerationRegistry;

#[derive(Clone, Copy)]
pub struct SelfConditionFacts {
    pub base_speed: u16,
    pub mana_shield_capacity: u32,
    pub target_reentry_protected: bool,
    pub source_reentry_protected: bool,
}
#[derive(Debug, Clone)]
pub enum SourceSpawnPlacement {
    Subject,
    Offset(u32),
    Relative(i32, i32),
    Anchor {
        key: String,
        definition: serde_json::Value,
    },
    Room {
        key: String,
        definition: serde_json::Value,
    },
}
/// Selection must come from the existing current AI/perception owner under this same lock.
/// Outer None means facts unavailable; inner None is an actual current no-target decision.
/// This adapter never invents nearest target, PZ, visibility or condition immunity values.
pub trait MonsterCombatReader:
    CurrentCombatFactsReader + SpellWorldReader + InvisibleTileCombatPolicy
{
    fn source_callback_spawn_position(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: &SourceSpawnPlacement,
        _: u64,
        _: usize,
        _: RuntimeWorkStamp,
    ) -> Option<crate::foundation::MovementLocalPosition> {
        None
    }
    fn source_callback_spawn_tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: crate::foundation::MovementLocalPosition,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn source_callback_spawn_anchor_contains(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: &serde_json::Value,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn selected_player(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        occurrence: ThinkOccurrence,
        behavior: &ProjectV2BehaviorAuthoring,
    ) -> Option<Option<(ExactActorRef, GameSessionId)>>;
    /// Player periodic ticks use95's actual PlayerSpellState condition owner.
    /// Standing-field facts are independently current; PZ is read from qualified tiles.
    fn source_player_condition_tick_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        stamp: RuntimeWorkStamp,
    ) -> Option<oteryn_gameplay_foundation::foundation::condition::TickFacts> {
        let current = self.condition_tick_facts(runtime, actor, session, stamp)?;
        let standing = current.standing_on_field.map(|field| match field {
            crate::foundation::DotElement::Poison => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Poison
            }
            crate::foundation::DotElement::Fire => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Fire
            }
            crate::foundation::DotElement::Energy => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Energy
            }
            crate::foundation::DotElement::Bleeding => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Bleeding
            }
            crate::foundation::DotElement::Drown => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Drown
            }
            crate::foundation::DotElement::Freezing => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Freezing
            }
            crate::foundation::DotElement::Dazzled => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Dazzled
            }
            crate::foundation::DotElement::Cursed => {
                oteryn_gameplay_foundation::foundation::condition::DotElement::Cursed
            }
        });
        let (cells, _) = self.native_condition_tick_inputs(runtime, stamp)?;
        qualified_source_player_tick_spatial_facts(runtime, cells, actor, session, Some(standing))
    }
    /// Trusted immutable loader artifacts for target-owned native ticks.
    /// Missing current map or source registry refuses before HP/cursor publication.
    fn native_condition_tick_inputs<'a>(
        &'a mut self,
        _runtime: &ChannelRuntimeV1,
        _stamp: RuntimeWorkStamp,
    ) -> Option<(
        &'a crate::content::NativeEntryMovementCells,
        &'a NativeRegenerationRegistry,
    )> {
        None
    }
    /// Independent source field observation. None refuses; Some(None) means
    /// the actual current dynamic map owner observed no field on this tile.
    fn source_creature_standing_field(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _actor: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<Option<crate::foundation::DotElement>> {
        None
    }
    fn source_creature_condition_tick_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<TickFacts> {
        let standing = self.source_creature_standing_field(runtime, actor, stamp)?;
        let (cells, _) = self.native_condition_tick_inputs(runtime, stamp)?;
        if cells.scope().world_id != runtime.binding().world_id()
            || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        let position = runtime.read_actor_position(actor).ok()?;
        if position.context() != runtime.pinned_movement_context() {
            return None;
        }
        let at = position.position();
        let cell = crate::content::LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        };
        if cells.index().lookup(cells.scope(), cell).ok()?
            != crate::content::CollisionClass::Walkable
        {
            return None;
        }
        let tile = cells.spell_tiles().lookup(cells.scope(), cell).ok()?;
        Some(TickFacts {
            in_protection_zone: tile.flags().protection_zone,
            standing_on_field: standing,
        })
    }
    fn condition_tick_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        stamp: RuntimeWorkStamp,
    ) -> Option<TickFacts>;

    fn self_condition_facts(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _actor: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<SelfConditionFacts> {
        None
    }
    /// Independently current map facts; unavailable means refuse, never inferred allow.
    fn chain_sight(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _from: TilePosition,
        _to: TilePosition,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn chain_path(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _from: TilePosition,
        _to: TilePosition,
        _stamp: RuntimeWorkStamp,
    ) -> Option<Vec<TilePosition>> {
        None
    }
    /// Source/AI movement selection and current map facts are independently provided.
    /// No map/ground/PZ fallback is manufactured by the aggregate.
    fn regular_creature_movement<'a>(
        &'a mut self,
        _runtime: &ChannelRuntimeV1,
        _actor: ExactActorRef,
        _target: Option<(ExactActorRef, GameSessionId)>,
        _creature: &Ref,
        _stamp: RuntimeWorkStamp,
    ) -> Option<RegularCreatureMovementFacts<'a>> {
        None
    }
    fn summon_follow_movement<'a>(
        &'a mut self,
        _runtime: &ChannelRuntimeV1,
        _child: ExactActorRef,
        _parent: ExactActorRef,
        _creature: &Ref,
        _stamp: RuntimeWorkStamp,
    ) -> Option<SummonFollowFacts<'a>> {
        None
    }
    fn summon_inherited_target_eligible(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _child: ExactActorRef,
        _target: ExactActorRef,
        _session: GameSessionId,
        _stamp: RuntimeWorkStamp,
        _behavior: &ProjectV2BehaviorAuthoring,
    ) -> Option<bool> {
        None
    }
    fn summon_target_reachable(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _parent: ExactActorRef,
        _target: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn summon_cell_admits(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _parent: ExactActorRef,
        _child: &Ref,
        _position: crate::foundation::MovementLocalPosition,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    /// Source-specific current Combat eligibility, not inferred from HP metadata.
    fn area_heal_target_allowed(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _caster: ExactActorRef,
        _target: ExactActorRef,
        _ability: &Ref,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn area_heal_non_player_side(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn area_heal_masterless(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn area_heal_top_creature(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    /// Missing current Creature Combat/role policy refuses the source composed cast.
    fn composed_creature_combat(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _issuer: ExactActorRef,
        _target: ExactActorRef,
        _element: Option<&str>,
        _stamp: RuntimeWorkStamp,
    ) -> Option<CreatureCombatFacts> {
        None
    }
    fn composed_callback_allowed(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _issuer: ExactActorRef,
        _target: ExactActorRef,
        _ability: &Ref,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn summon_facts(
        &mut self,
        _runtime: &ChannelRuntimeV1,
        _stamp: RuntimeWorkStamp,
        _occurrence: ThinkOccurrence,
        _behavior: &ProjectV2BehaviorAuthoring,
    ) -> Option<MonsterSummonFacts> {
        None
    }
}

pub struct SummonFollowFacts<'a> {
    /// Independently supplied current map provider; absence refuses cadence/movement.
    pub ground: Option<&'a dyn crate::movement::speed::GroundSpeedSource>,
    pub direction: crate::movement::CardinalStep,
    pub selection: crate::movement::MovementEngineeringSelection<'a>,
    pub index: &'a crate::content::static_cell_engine::EngineeringStaticCellIndex,
    /// Independently current source restrictions: no PZ/floor-change/teleport admission.
    pub tile_admits: Option<bool>,
}
#[derive(Debug)]
pub enum SummonFollowError {
    MissingMap,
    StaleOwner,
    StaleRelation,
    WrongDirection,
    MissingSpeed,
    ClockOverflow,
    Carrier(CarrierError),
    Movement(crate::movement::MovementError),
}
pub struct RegularCreatureMovementFacts<'a> {
    pub direction: crate::movement::CardinalStep,
    pub scope: &'a crate::content::static_cell_engine::EngineeringStaticCellScope,
    pub index: &'a crate::content::static_cell_engine::EngineeringStaticCellIndex,
    pub ground: Option<&'a dyn crate::movement::speed::GroundSpeedSource>,
    pub blocking: &'a [crate::content::LogicalCell],
    /// Current source movement/PZ/teleport/floor-change restrictions, not default allow.
    pub tile_admits: Option<bool>,
}
/// Immutable trusted-loader artifacts; canonical runtime slots retain all tick cursors.
/// Construction accepts the native compiler's qualified room, never raw collision claims.
pub struct TrustedMonsterConditionTickInputs {
    cells: crate::content::NativeEntryMovementCells,
    pub regeneration: NativeRegenerationRegistry,
    world: crate::foundation::WorldId,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl TrustedMonsterConditionTickInputs {
    pub fn from_trusted_native_loader(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        draft: &ProjectV2Draft,
        native: &crate::content::native_gameplay::NativeGameplayState,
    ) -> Option<Self> {
        let digest = runtime.content_pin().server_artifact_digest();
        if !native.qualifies_current_project_definitions(digest, draft) {
            return None;
        }
        Self::from_project_definitions(runtime, room, draft, digest)
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn from_test_native_room(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        draft: &ProjectV2Draft,
        verified_loader_digest: [u8; 32],
    ) -> Option<Self> {
        Self::from_project_definitions(runtime, room, draft, verified_loader_digest)
    }
    fn from_project_definitions(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        draft: &crate::content::ProjectV2Draft,
        verified_loader_digest: [u8; 32],
    ) -> Option<Self> {
        let pin = runtime.content_pin();
        if room.movement_cells().scope().world_id != pin.world_id()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || room.compiled().client_digest() != pin.client_artifact_digest()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
        {
            return None;
        }
        let regeneration = NativeRegenerationRegistry::from_project_definitions(
            runtime,
            draft,
            verified_loader_digest,
        )?;
        Some(Self {
            cells: room.movement_cells().clone(),
            regeneration,
            world: pin.world_id(),
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
        })
    }
    pub fn current<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Option<(
        &'a crate::content::NativeEntryMovementCells,
        &'a NativeRegenerationRegistry,
    )> {
        let pin = runtime.content_pin();
        if pin.world_id() != self.world
            || pin.activation_sequence() != self.activation
            || pin.server_artifact_digest() != self.server
            || pin.client_artifact_digest() != self.client
            || pin.frame_binding_digest() != self.frame
            || pin.map_revision_digest() != self.map
        {
            return None;
        }
        Some((&self.cells, &self.regeneration))
    }
}

impl TrustedMonsterConditionTickInputs {
    /// Static PZ comes only from95's qualified native spell tile owner.
    /// The current standing-field reader supplies an explicit known result:
    /// outerNone is unavailable; Some(None) is independently observed no field.
    /// Native Player periodic state remains exclusively PlayerSpellState.conditions.
    pub fn source_player_tick_spatial_facts(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        current_standing_field: Option<
            Option<oteryn_gameplay_foundation::foundation::condition::DotElement>,
        >,
    ) -> Option<oteryn_gameplay_foundation::foundation::condition::TickFacts> {
        let (cells, _) = self.current(runtime)?;
        qualified_source_player_tick_spatial_facts(
            runtime,
            cells,
            actor,
            session,
            current_standing_field,
        )
    }
}

fn qualified_source_player_tick_spatial_facts(
    runtime: &ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    current_standing_field: Option<
        Option<oteryn_gameplay_foundation::foundation::condition::DotElement>,
    >,
) -> Option<oteryn_gameplay_foundation::foundation::condition::TickFacts> {
    let standing_on_field = current_standing_field?;
    if cells.scope().world_id != runtime.binding().world_id()
        || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
    {
        return None;
    }
    let control = runtime.player_control_facts(actor, session).ok()?;
    if control.control_loss.is_some() {
        return None;
    }
    let position = runtime.read_actor_position(actor).ok()?;
    if position.context() != runtime.pinned_movement_context() {
        return None;
    }
    let p = position.position();
    let cell = crate::content::LogicalCell {
        x: p.x,
        y: p.y,
        z: i32::from(p.floor),
    };
    if cells.index().lookup(cells.scope(), cell).ok()? != crate::content::CollisionClass::Walkable {
        return None;
    }
    let tile = cells.spell_tiles().lookup(cells.scope(), cell).ok()?;
    Some(
        oteryn_gameplay_foundation::foundation::condition::TickFacts {
            in_protection_zone: tile.flags().protection_zone,
            standing_on_field,
        },
    )
}

/// Source-specific combat composition supplied by the owning encounter crate.
/// The AI layer knows only current owners, readers and this call boundary.
pub trait EncounterHooks {
    type Pulse;
    type Error;

    fn run_encounter<R: MonsterCombatReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut impl MonsterCombatStates,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        reader: &mut R,
        revisions: RevisionSet,
    ) -> Result<Self::Pulse, Self::Error>;
}

/// Dispatch a native combat turn through the composition root's encounter hooks.
pub fn run_encounters<H: EncounterHooks, R: MonsterCombatReader>(
    hooks: &mut H,
    runtime: &mut ChannelRuntimeV1,
    states: &mut impl MonsterCombatStates,
    current: &mut ScopeRuntimeFence,
    clock: &impl OwnerClock,
    reader: &mut R,
    revisions: RevisionSet,
) -> Result<H::Pulse, H::Error> {
    hooks.run_encounter(runtime, states, current, clock, reader, revisions)
}
