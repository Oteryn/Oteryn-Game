//! Current Map/Item/LocalObject facts, borrowed for one physical Item-owner transaction.
#[cfg(test)]
mod movement_script_tests;
mod prepared;
#[cfg(test)]
mod source_solid_tests;
#[cfg(test)]
mod source_spawn_surface_tests;
#[cfg(test)]
mod source_teleport_tests;
#[cfg(test)]
mod source_walkable_tests;
mod special_movement;
pub use prepared::{PreparedMonsterMapRead, ReadPreparedMonsterMap};
pub use special_movement::step_in as apply_committed_step_in;
#[cfg(any(test, feature = "test-support"))]
mod transaction_test_support;

use super::source_placement::{SourcePlacementCell, SourcePlacementTerrain};
use super::*;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{NativeEntryMovementCells, QualifiedNativeEntryRoom};
use crate::durability::spell_item_transaction::{
    SpellItemScopeAuthority, read_creature_scope_tile_in_transaction,
};
use crate::map::boot::BundleCollisionIndexExt;
use crate::world_runtime::LocalObjectRuntime;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Tile {
    admits: bool,
    solid: bool,
    source_spawn_admits: bool,
    projectile: bool,
    speed: u16,
    field: Option<oteryn_gameplay_domain::domain::condition::DotElement>,
    source_placement: SourcePlacementTerrain,
    source_placement_proven: bool,
}

pub struct MonsterMap {
    valid: bool,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    context: crate::foundation::MovementPositionContext,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    tiles: BTreeMap<(i32, i32, i16), Option<Tile>>,
}
impl MonsterMap {
    pub fn matches(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.valid
            && runtime
                .owner_fence()
                .is_ok_and(|f| f.is_current_for_scope(self.scope, self.generation))
            && runtime.pinned_movement_context() == self.context
            && runtime.content_pin().activation_sequence() == self.activation
            && runtime.content_pin().server_artifact_digest() == self.server
            && runtime.content_pin().client_artifact_digest() == self.client
            && runtime.content_pin().frame_binding_digest() == self.frame
            && runtime.content_pin().map_revision_digest() == self.map
    }
    /// Current child queryAdd DATA from a fully qualified cached tile. Detached
    /// readers retain their SQL row locks; occupancy is always read anew here.
    pub fn source_placement_cell(
        &self,
        runtime: &ChannelRuntimeV1,
        content: &NativeGameplayState,
        child: &str,
        at: MovementLocalPosition,
        removed: Option<ExactActorRef>,
    ) -> Option<SourcePlacementCell> {
        if !self.matches(runtime)
            || content.source_digest() != runtime.content_pin().server_artifact_digest()
            || removed.is_some_and(|actor| !runtime.contains_live_creature(actor))
        {
            return None;
        }
        let tile = self.tile(at)?;
        if !tile.source_placement_proven {
            return None;
        }
        let occupancy = runtime
            .current_native_source_occupancy(content, child, at, removed)
            .ok()?;
        let mut cell = tile
            .source_placement
            .for_masterless_monster(occupancy.familiar, false, &[]);
        cell.query_add &= occupancy.admits_occupants;
        Some(cell)
    }
    pub fn admits(&self, at: MovementLocalPosition) -> bool {
        self.tile(at).is_some_and(|tile| tile.admits)
    }
    /// Raw current SOLID, independently of movement's PZ, floor-change,
    /// teleport and occupancy restrictions. Absent/unknown tiles have no flags.
    pub fn blocks_solid(&self, at: MovementLocalPosition) -> Option<bool> {
        self.tile(at)
            .filter(|tile| tile.source_placement.exists)
            .map(|tile| tile.solid)
    }
    /// data/libs/functions/tile.lua:39-54 with all optional arguments omitted.
    /// This is map DATA, not ordinary movement or source placement authority.
    /// An absent sentinel alone does not prove the source capture covers a cell.
    pub fn source_is_walkable(&self, at: MovementLocalPosition) -> Option<bool> {
        let tile = self.tile(at)?;
        if !tile.source_placement.exists || !tile.source_placement_proven {
            return None;
        }
        Some(
            tile.source_placement.ground
                && !tile.solid
                && !tile.projectile
                && !tile.source_placement.immovable_solid,
        )
    }
    /// Literal source spawn-surface DATA, preserving Unknown separately from
    /// a known absent tile or an actual IMMOVABLEBLOCKSOLID barrier. This does
    /// not impose ground/PZ/projectile/occupancy restrictions or grant a spawn.
    pub fn source_spawn_surface_admits(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> Option<bool> {
        if !self.matches(runtime) {
            return None;
        }
        let tile = self.tile(at)?;
        if !tile.source_placement_proven {
            return None;
        }
        Some(tile.source_placement.exists && !tile.source_placement.immovable_solid)
    }
    /// Existing conservative source predicate, unchanged for its current consumers.
    pub fn source_spawn_admits(&self, at: MovementLocalPosition) -> bool {
        self.tile(at).is_some_and(|tile| tile.source_spawn_admits)
    }
    pub fn known(&self, at: MovementLocalPosition) -> bool {
        self.tile(at).is_some()
    }
    /// Source Combat permits a clear groundless target tile. Movement
    /// admissibility alone cannot distinguish that tile from a solid wall.
    pub fn blocks_projectile(&self, at: MovementLocalPosition) -> Option<bool> {
        self.tile(at).map(|tile| tile.projectile)
    }
    /// Crystal Combat::canDoCombat and getCombatArea: these CURRENT cached
    /// tile facts are independent of walking, occupancy, solid and ground.
    pub fn source_combat_tile_admits(&self, at: MovementLocalPosition) -> Option<bool> {
        let tile = self.tile(at)?;
        Some(
            tile.source_placement.exists
                && !tile.projectile
                && !tile.source_placement.protection_zone
                && !tile.source_placement.floor_change
                && !tile.source_placement.teleport,
        )
    }
    /// CREATURE-AI-0 §5.2: a harmful field costs 180 for a profile that
    /// avoids its element. It remains traversable when no cheaper route exists.
    pub fn field_path_cost(&self, at: MovementLocalPosition, walks_on: [bool; 3]) -> Option<u32> {
        use oteryn_gameplay_domain::domain::condition::DotElement;
        let avoided = match self.tile(at)?.field {
            Some(DotElement::Fire) => !walks_on[0],
            Some(DotElement::Energy) => !walks_on[1],
            Some(DotElement::Poison) => !walks_on[2],
            _ => false,
        };
        Some(if avoided { 180 } else { 0 })
    }
    fn tile(&self, at: MovementLocalPosition) -> Option<Tile> {
        self.tiles.get(&(at.x, at.y, at.floor)).copied().flatten()
    }
    pub fn sight(&self, from: MovementLocalPosition, to: MovementLocalPosition) -> bool {
        crate::native_world_read::source_sight_steps(from, to).is_ok_and(|steps| {
            steps
                .into_iter()
                .all(|(at, exempt)| exempt || self.tile(at).is_some_and(|tile| !tile.projectile))
        })
    }
    pub fn step(
        &self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        actor: ExactActorRef,
        before: MovementPositionSnapshot,
        direction: CardinalStep,
    ) -> Result<MovementPositionSnapshot, MovementError> {
        // Check the independently pinned context before any Map lookup.
        if !self.matches(runtime) || before.context() != runtime.pinned_movement_context() {
            return Err(MovementError::ContextMismatch);
        }
        if runtime
            .native_callback_movement_locked(actor)
            .map_err(|_| MovementError::NotQualified)?
        {
            return Err(MovementError::NotQualified);
        }
        let selection = MovementEngineeringSelection {
            owner_context: self.context,
            content_scope: cells.scope(),
        };
        let mut owner = runtime.borrow_movement_position();
        let mut decision = MovementDecision::begin(&owner, actor, before, &selection, direction)?;
        if !self.admits(decision.destination) {
            return Err(MovementError::NotQualified);
        }
        decision.attempted = 1;
        decision.real_content_lookups = 1;
        decision.qualified = true;
        let moved = decision.commit(&mut owner)?;
        special_movement::step_in(runtime, cells, actor, moved.position());
        if let NativeMovementCollisionIndex::Bundle(collision) = cells.index() {
            collision
                .map()
                .facts
                .record_monster_step(crate::map::facts::MonsterStep {
                    actor,
                    from: before.position(),
                    to: moved.position(),
                });
        }
        Ok(moved)
    }
    /// Crystal game.cpp:3167-3195 and tile.cpp:601-608: an explicit
    /// source teleport uses FLAG_NOLIMIT, after proving the destination Tile
    /// exists. Callback/source/occurrence authority remains with the caller.
    pub fn relocate_source_creature(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !self.matches(runtime)
            || !self
                .tile(destination)
                .is_some_and(|tile| tile.source_placement.exists)
            || !runtime.contains_live_creature(actor)
        {
            return Err(CarrierError::WrongScope);
        }
        let before = runtime.read_actor_position(actor)?;
        if before.position() == destination {
            return runtime
                .borrow_movement_position()
                .commit_turn(before, crate::foundation::MovementFacing::North);
        }
        let moved = runtime
            .borrow_movement_position()
            .commit_creature_relocation(actor, before, destination)?;
        runtime.borrow_movement_position().commit_turn(
            moved,
            source_teleport_facing(before.position(), destination),
        )
    }
    /// A source-side teleport uses the same exact-snapshot position owner.
    /// It preserves committed input loss; this is no client movement grant.
    pub fn relocate_source_player(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        destination: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !self.matches(runtime)
            || !self
                .tile(destination)
                .is_some_and(|tile| tile.source_placement.exists)
        {
            return Err(CarrierError::WrongScope);
        }
        runtime.player_control_facts(actor, session)?;
        let before = runtime.read_actor_position(actor)?;
        if before.position() == destination {
            return runtime
                .borrow_movement_position()
                .commit_turn(before, crate::foundation::MovementFacing::North);
        }
        let moved =
            runtime
                .borrow_movement_position()
                .commit_spell_relocation(SourcePlayerRelocation {
                    actor,
                    before,
                    destination,
                })?;
        runtime.borrow_movement_position().commit_turn(
            moved,
            source_teleport_facing(before.position(), destination),
        )
    }
    pub fn relocate(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !self.matches(runtime)
            || !self.admits(destination)
            || runtime.native_callback_movement_locked(actor)?
            || !runtime.contains_live_creature(actor)
            || runtime.native_summon_role(actor)?.is_some()
            || runtime.position_occupied_by_other(actor, destination)?
        {
            return Err(CarrierError::WrongScope);
        }
        let before = runtime.read_actor_position(actor)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(actor, before, destination)
    }
}
impl speed::GroundSpeedSource for MonsterMap {
    fn ground_speed(&self, cell: LogicalCell) -> u16 {
        i16::try_from(cell.z)
            .ok()
            .and_then(|floor| {
                self.tile(MovementLocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor,
                })
            })
            .map_or(0, |tile| tile.speed)
    }
}

pub struct CurrentMonsterMap<'a, 'db> {
    tx: &'a mut sqlx::Transaction<'db, sqlx::Postgres>,
    authority: &'a SpellItemScopeAuthority,
    room: &'a QualifiedNativeEntryRoom,
    objects: &'a LocalObjectRuntime,
    content: &'a NativeGameplayState,
    view: MonsterMap,
    source_observed_seconds: Option<u64>,
}
impl<'a, 'db> CurrentMonsterMap<'a, 'db> {
    pub fn new(
        tx: &'a mut sqlx::Transaction<'db, sqlx::Postgres>,
        authority: &'a SpellItemScopeAuthority,
        runtime: &ChannelRuntimeV1,
        room: &'a QualifiedNativeEntryRoom,
        objects: &'a LocalObjectRuntime,
        content: &'a NativeGameplayState,
    ) -> Self {
        let binding = runtime.binding();
        Self {
            tx,
            authority,
            room,
            objects,
            content,
            source_observed_seconds: None,
            view: MonsterMap {
                valid: true,
                scope: binding.runtime_scope(),
                generation: binding.scope_generation(),
                context: runtime.pinned_movement_context(),
                activation: runtime.content_pin().activation_sequence(),
                server: runtime.content_pin().server_artifact_digest(),
                client: runtime.content_pin().client_artifact_digest(),
                frame: runtime.content_pin().frame_binding_digest(),
                map: runtime.content_pin().map_revision_digest(),
                tiles: BTreeMap::new(),
            },
        }
    }
    /// Lua `os.time()` DATA sampled once in this actual fenced owner pass.
    /// No player presence, inferred monotonic epoch or callback catch-up supplies
    /// this clock. Even a cached sample requires the same current transaction.
    pub async fn source_observed_seconds(&mut self, runtime: &ChannelRuntimeV1) -> Option<u64> {
        if !self.validate_transaction(runtime).await {
            return None;
        }
        let observed = if let Some(observed) = self.source_observed_seconds {
            observed
        } else {
            let observed: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&mut **self.tx)
                    .await
                    .ok()?;
            u64::try_from(observed).ok()?
        };
        if !self.validate_transaction(runtime).await {
            return None;
        }
        self.source_observed_seconds = Some(observed);
        Some(observed)
    }
    /// Recheck the actual transaction after awaited reads, before a synchronous consumer.
    /// An aborted/lost transaction invalidates even its formerly known empty tiles.
    pub async fn validate_transaction(&mut self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        let valid = self.view.matches(runtime)
            && self.authority.runtime_scope() == binding.runtime_scope()
            && self.authority.generation() == binding.scope_generation().get()
            && crate::durability::spell_item_transaction::check_scope_transaction(
                self.tx,
                self.authority,
            )
            .await
            .is_ok();
        if !valid {
            self.view.valid = false;
            self.view.tiles.clear();
        }
        valid
    }
    /// Read current Quest DATA once for a bounded callback batch, retaining this
    /// exact Map/Item transaction and the canonical admission-journal actor proof.
    pub async fn current_soul_war_taints(
        &mut self,
        root: &crate::durability::DurabilityRoot,
        runtime: &ChannelRuntimeV1,
        actors: &[(
            crate::foundation::ExactActorRef,
            crate::foundation::GameSessionId,
        )],
    ) -> Option<Vec<crate::durability::quest_state::soul_war::CurrentSoulWarTaintFacts>> {
        if !self.validate_transaction(runtime).await {
            return None;
        }
        let facts =
            crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                self.tx,
                root,
                self.authority,
                runtime,
                actors,
            )
            .await
            .ok()?;
        self.validate_transaction(runtime).await.then_some(facts)
    }
    pub async fn current_rotten_blood_taints(
        &mut self,
        root: &crate::durability::DurabilityRoot,
        runtime: &ChannelRuntimeV1,
        actors: &[(
            crate::foundation::ExactActorRef,
            crate::foundation::GameSessionId,
        )],
    ) -> Option<Vec<crate::durability::quest_state::rotten_blood::CurrentRottenBloodTaintFacts>>
    {
        if !self.validate_transaction(runtime).await {
            return None;
        }
        let facts =
            crate::durability::quest_state::rotten_blood::read_current_rotten_blood_taints_in_transaction(
                self.tx,
                root,
                self.authority,
                runtime,
                actors,
            )
            .await
            .ok()?;
        self.validate_transaction(runtime).await.then_some(facts)
    }
    /// Actual visible player/lease and three persistent KV predecessors from
    /// this same current Map transaction. These facts grant no mutation alone.
    pub async fn current_rotten_area_players(
        &mut self,
        root: &crate::durability::DurabilityRoot,
        runtime: &ChannelRuntimeV1,
        actors: &[(
            crate::foundation::ExactActorRef,
            crate::foundation::GameSessionId,
        )],
    ) -> Option<Vec<crate::durability::quest_state::rotten_area::CurrentRottenAreaPlayerFacts>>
    {
        if !self.validate_transaction(runtime).await {
            return None;
        }
        let facts=crate::durability::quest_state::rotten_area::read_current_rotten_area_players_in_transaction(
            self.tx,root,self.authority,runtime,actors).await.ok()?;
        self.validate_transaction(runtime).await.then_some(facts)
    }
    /// The same existing scope read supplies the immutable Bakragore handoff
    /// preparation. Outer None is unavailable; inner None is a different room.
    pub async fn current_bak_preparation(
        &mut self,
        runtime: &ChannelRuntimeV1,
    ) -> Option<Option<crate::durability::quest_state::rotten_flame::PreparedBakEssences>> {
        if !self.validate_transaction(runtime).await {
            return None;
        }
        if self.room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Bakragore)
        {
            return Some(None);
        }
        let facts = crate::durability::quest_state::rotten_flame::read_prepared_bak_in_transaction(
            self.tx,
            self.authority,
            runtime,
        )
        .await
        .ok()?;
        self.validate_transaction(runtime).await.then_some(facts)
    }
    pub fn view(&self) -> &MonsterMap {
        &self.view
    }
    /// Tile existence is independent of queryAdd and current creature/item
    /// blockers. An absent tile is proved only inside the sealed capture.
    pub fn source_tile_exists(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> Option<bool> {
        if !self.view.matches(runtime)
            || self.content.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        crate::native_world_read::qualified_static_owner(self.room, runtime, self.objects).ok()?;
        if matches!(
            self.room.movement_cells().index(),
            NativeMovementCollisionIndex::Bundle(_)
        ) {
            match self.room.movement_cells().spell_tiles().lookup(
                self.room.movement_cells().scope(),
                LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            ) {
                Ok(_) | Err(crate::content::SpellTileLookupError::Unknown) => return Some(true),
                Err(crate::content::SpellTileLookupError::ScopeMismatch) => return None,
                Err(crate::content::SpellTileLookupError::Absent) => {}
            }
        } else if self.view.tile(at)?.source_placement.exists {
            return Some(true);
        }
        self.room
            .pinned_activity_source_room()
            .filter(|source| source.source_capture_covers([at.x, at.y, i32::from(at.floor)]))
            .map(|_| false)
    }
    /// Default Lua Tile:isWalkable, with independently current map identity and
    /// exact capture coverage for a genuinely missing source tile.
    pub fn source_is_walkable(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> Option<bool> {
        match self.source_tile_exists(runtime, at)? {
            false => Some(false),
            true => self.view.source_is_walkable(at),
        }
    }
    /// The active child's source queryAdd policy, with occupancy read anew for
    /// each creation. A missing captured tile differs from unavailable coverage.
    pub fn source_placement_cell(
        &self,
        runtime: &ChannelRuntimeV1,
        child: &str,
        at: MovementLocalPosition,
    ) -> Option<SourcePlacementCell> {
        self.source_placement_cell_bound(runtime, child, at, None, self.private_capture_covers(at))
    }
    fn private_capture_covers(&self, at: MovementLocalPosition) -> bool {
        self.room
            .pinned_activity_source_room()
            .is_some_and(|source| source.source_capture_covers([at.x, at.y, i32::from(at.floor)]))
    }
    pub fn source_placement_cell_in_callback(
        &self,
        runtime: &ChannelRuntimeV1,
        child: &str,
        at: MovementLocalPosition,
        context: &crate::content::QualifiedCallbackSourceContext,
    ) -> Option<SourcePlacementCell> {
        if !context.matches(self.room, runtime.content_pin()) {
            return None;
        }
        let source = context
            .source_frame()
            .source_position((at.x, at.y, at.floor))?;
        let covered =
            self.private_capture_covers(at) || context.source().source_capture_covers(source);
        self.source_placement_cell_bound(runtime, child, at, None, covered)
    }
    pub fn source_placement_cell_in_world(
        &self,
        runtime: &ChannelRuntimeV1,
        child: &str,
        at: MovementLocalPosition,
        context: &crate::content::QualifiedPublishedWorldContext,
    ) -> Option<SourcePlacementCell> {
        if !context.matches(self.room, runtime.content_pin()) {
            return None;
        }
        self.source_placement_cell_bound(runtime, child, at, None, true)
    }
    pub fn source_tile_exists_in_callback(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
        context: &crate::content::QualifiedCallbackSourceContext,
    ) -> Option<bool> {
        if !context.matches(self.room, runtime.content_pin()) {
            return None;
        }
        if let Some(exists) = self.source_tile_exists(runtime, at) {
            return Some(exists);
        }
        let source = context
            .source_frame()
            .source_position((at.x, at.y, at.floor))?;
        (context.source().source_capture_covers(source)
            && self.current_map_proves_absent(runtime, at))
        .then_some(false)
    }
    pub fn source_tile_exists_in_world(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
        context: &crate::content::QualifiedPublishedWorldContext,
    ) -> Option<bool> {
        if !context.matches(self.room, runtime.content_pin()) {
            return None;
        }
        self.source_tile_exists(runtime, at)
            .or_else(|| self.current_map_proves_absent(runtime, at).then_some(false))
    }
    fn current_map_proves_absent(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> bool {
        self.view.matches(runtime)
            && self.content.source_digest() == runtime.content_pin().server_artifact_digest()
            && crate::native_world_read::qualified_static_owner(self.room, runtime, self.objects)
                .is_ok()
            && self.room.movement_cells().spell_tiles().lookup(
                self.room.movement_cells().scope(),
                LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            ) == Err(crate::content::SpellTileLookupError::Absent)
    }
    /// Read-only preflight for the source's remove-old/create-new sequence.
    /// Physical admission still rechecks the actual census after removal.
    pub fn source_placement_cell_after_removal(
        &self,
        runtime: &ChannelRuntimeV1,
        child: &str,
        at: MovementLocalPosition,
        removed: ExactActorRef,
    ) -> Option<SourcePlacementCell> {
        if !runtime.contains_live_creature(removed) {
            return None;
        }
        self.source_placement_cell_bound(
            runtime,
            child,
            at,
            Some(removed),
            self.private_capture_covers(at),
        )
    }
    fn source_placement_cell_bound(
        &self,
        runtime: &ChannelRuntimeV1,
        child: &str,
        at: MovementLocalPosition,
        removed: Option<ExactActorRef>,
        absence_proven: bool,
    ) -> Option<SourcePlacementCell> {
        if !self.view.matches(runtime)
            || self.content.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        let tile = match self.view.tile(at) {
            Some(tile) => tile,
            None if absence_proven && self.current_map_proves_absent(runtime, at) => {
                return Some(SourcePlacementCell {
                    exists: false,
                    query_add: false,
                    protection_zone: false,
                    teleport: false,
                });
            }
            None => return None,
        };
        if !tile.source_placement_proven {
            return None;
        }
        if !tile.source_placement.exists && !absence_proven {
            return None;
        }
        self.view
            .source_placement_cell(runtime, self.content, child, at, removed)
    }

    pub async fn read(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> Option<bool> {
        let key = (at.x, at.y, at.floor);
        if !self.view.matches(runtime)
            || self.content.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        if let Some(tile) = self.view.tiles.get(&key) {
            return tile.map(|tile| tile.admits);
        }
        let tile = self.read_tile(runtime, at).await;
        self.view.tiles.insert(key, tile);
        tile.map(|tile| tile.admits)
    }
    /// Bounded source `Tile:getItemById` predicate. Read the current static
    /// source stack and canonical scoped Item rows under this same transaction;
    /// this grants no portable Item, placement or collision authority.
    pub async fn source_has_any_item(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
        ids: &[u16],
    ) -> Option<bool> {
        if ids.len() > 16
            || !self.view.matches(runtime)
            || self.content.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        crate::native_world_read::qualified_static_owner(self.room, runtime, self.objects).ok()?;
        let target = crate::native_world_items::SpellGroundTarget::for_native_tile_read(
            self.room, runtime, at,
        )
        .ok()?;
        let items = read_creature_scope_tile_in_transaction(self.tx, self.authority, &target)
            .await
            .ok()?;
        let facts = crate::native_world_read::qualified_creature_source_tile_in_transaction(
            self.tx,
            self.authority,
            self.room,
            runtime,
            self.objects,
            at,
        )
        .await
        .ok()?;
        let static_present = facts
            .qualified_tile()
            .top_source_ids()
            .iter()
            .any(|id| ids.iter().any(|wanted| u32::from(*wanted) == *id));
        let mut present = static_present;
        if !present {
            for item in &items.items {
                let policy = self.content.item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )?;
                // Same qualified numeric source binding used by
                // NativeGameplayState::item_policy_for_source_id, never a key guess.
                if ids
                    .iter()
                    .any(|id| policy.record().production_binding.external_id == id.to_string())
                {
                    present = true;
                    break;
                }
            }
        }
        self.validate_transaction(runtime).await.then_some(present)
    }

    async fn read_tile(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> Option<Tile> {
        let target = crate::native_world_items::SpellGroundTarget::for_native_tile_read(
            self.room, runtime, at,
        )
        .ok()?;
        let items = read_creature_scope_tile_in_transaction(self.tx, self.authority, &target)
            .await
            .ok()?;
        crate::native_world_read::qualified_static_owner(self.room, runtime, self.objects).ok()?;
        // A qualified empty source cell is distinct from unavailable metadata.
        if self.room.movement_cells().spell_tiles().lookup(
            self.room.movement_cells().scope(),
            LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) == Err(crate::content::SpellTileLookupError::Absent)
            && items.items.is_empty()
        {
            return Some(Tile {
                admits: false,
                solid: true,
                source_spawn_admits: false,
                projectile: true,
                speed: 0,
                field: None,
                source_placement: SourcePlacementTerrain {
                    exists: false,
                    ground: false,
                    immovable_solid: false,
                    protection_zone: false,
                    floor_change: false,
                    teleport: false,
                },
                source_placement_proven: true,
            });
        }
        let fact = crate::native_world_read::qualified_creature_source_tile_in_transaction(
            self.tx,
            self.authority,
            self.room,
            runtime,
            self.objects,
            at,
        )
        .await
        .ok()?;
        let tile = fact.qualified_tile();
        merge_current_tile(self.room, self.content, at, tile, &items)
    }
    pub async fn read_sight(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) {
        if let Ok(steps) = crate::native_world_read::source_sight_steps(from, to) {
            for (at, exempt) in steps {
                if !exempt {
                    self.read(runtime, at).await;
                }
            }
        }
    }
}

struct SourcePlayerRelocation {
    actor: ExactActorRef,
    before: MovementPositionSnapshot,
    destination: MovementLocalPosition,
}
impl oteryn_gameplay_entities::relocation_seal::Sealed for SourcePlayerRelocation {}
impl oteryn_gameplay_entities::SpellRelocationProof for SourcePlayerRelocation {
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
    ) {
        (self.actor, self.before, self.destination)
    }
}

// Crystal creature_functions.cpp:897-909: default teleport uses x priority,
// then y, and otherwise north. These are actual committed pose facts.
fn source_teleport_facing(
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> crate::foundation::MovementFacing {
    use crate::foundation::MovementFacing;
    if from.x < to.x {
        MovementFacing::East
    } else if from.x > to.x {
        MovementFacing::West
    } else if from.y < to.y {
        MovementFacing::South
    } else {
        MovementFacing::North
    }
}

/// One compositor for borrowed and detached reads of the same current Item rows.
fn merge_current_tile(
    room: &QualifiedNativeEntryRoom,
    content: &NativeGameplayState,
    at: MovementLocalPosition,
    tile: &crate::content::QualifiedSpellTile,
    items: &crate::durability::spell_items_abi::DurableTileItems,
) -> Option<Tile> {
    let mut solid = tile.flags().block_solid;
    let mut immovable_solid = tile.flags().immovable_block_solid;
    let mut source_placement_proven = true;
    let mut projectile = tile.flags().block_projectile;
    let mut field = None;
    let mut teleport = crate::map::floor::is_teleport(
        room.movement_cells(),
        LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        },
    );
    for item in &items.items {
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        // Dynamically created source portals have no static floor-table
        // record. Their actual admitted native Item type still participates
        // in normal C++ monster queryAdd/movement (not Lua:isWalkable).
        teleport |= matches!(&policy.record().semantics.classification,
                crate::content::ReferenceItemField::Known(classification)
                if classification.item_type==crate::content::ReferenceItemField::Known(crate::content::ReferenceItemType::Teleport));
        let attributes = &policy.record().attributes;
        if field.is_none()
            && let Some(recipe) = &attributes.field_condition
            && recipe.condition_values().ok()?.is_some()
        {
            field = recipe.element().ok()?;
        }
        let (blocks_movement, blocks_projectile) =
            match oteryn_gameplay_items::native_world_items::source_barrier_collision(policy) {
                Some(flags) => flags,
                None => (attributes.blocks_movement?, attributes.blocks_projectile?),
            };
        if item.blocks_movement != blocks_movement || item.blocks_projectile != blocks_projectile {
            return None;
        }
        // Missing or mismatched immutable-solid evidence blocks only the
        // source spawn predicate; it does not change normal movement rules.
        source_placement_proven &=
            attributes.immovable_block_solid == Some(item.immovable_block_solid);
        immovable_solid |= attributes.immovable_block_solid != Some(item.immovable_block_solid)
            || item.immovable_block_solid;
        solid |= item.blocks_movement;
        projectile |= item.blocks_projectile;
    }
    if field.is_none() {
        for id in tile.top_source_ids() {
            // Static terrain/decorations are qualified by the Map owner and
            // need not have a portable Item profile. Only an admitted field
            // recipe adds a path cost; collision facts above still apply.
            let Some(policy) = content.item_policy_for_source_id(*id) else {
                continue;
            };
            if let Some(recipe) = &policy.record().attributes.field_condition {
                if recipe.condition_values().ok()?.is_some() {
                    field = recipe.element().ok()?;
                }
                break;
            }
        }
    }
    Some(Tile {
        solid,
        source_placement_proven,
        source_placement: SourcePlacementTerrain {
            exists: true,
            ground: tile.ground_present(),
            immovable_solid,
            protection_zone: tile.flags().protection_zone,
            floor_change: tile.flags().floor_change,
            teleport,
        },
        source_spawn_admits: !immovable_solid,
        admits: tile.ground_present()
            && !solid
            && !tile.flags().protection_zone
            && !tile.flags().floor_change
            && !teleport,
        projectile,
        speed: speed::qualified_ground_speed(tile.ground_present(), tile.ground_speed()),
        field,
    })
}
