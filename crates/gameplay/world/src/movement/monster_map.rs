//! Current Map/Item/LocalObject facts, borrowed for one physical Item-owner transaction.
#[cfg(any(test, feature = "test-support"))]
mod transaction_test_support;

use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{NativeEntryMovementCells, QualifiedNativeEntryRoom};
use crate::durability::spell_item_transaction::{
    SpellItemScopeAuthority, read_creature_scope_tile_in_transaction,
};
use crate::world_runtime::LocalObjectRuntime;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Tile {
    admits: bool,
    projectile: bool,
    speed: u16,
    field: Option<oteryn_gameplay_domain::domain::condition::DotElement>,
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
    pub fn admits(&self, at: MovementLocalPosition) -> bool {
        self.tile(at).is_some_and(|tile| tile.admits)
    }
    pub fn known(&self, at: MovementLocalPosition) -> bool {
        self.tile(at).is_some()
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
        decision.commit(&mut owner)
    }
    pub fn relocate(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !self.matches(runtime)
            || !self.admits(destination)
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
    pub fn view(&self) -> &MonsterMap {
        &self.view
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
                projectile: true,
                speed: 0,
                field: None,
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
        let mut solid = tile.flags().block_solid;
        let mut projectile = tile.flags().block_projectile;
        let mut field = None;
        for item in &items.items {
            let policy = self.content.item_policy(
                &item.definition.production_key,
                &item.definition.revision_ref,
            )?;
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
            if item.blocks_movement != blocks_movement
                || item.blocks_projectile != blocks_projectile
            {
                return None;
            }
            solid |= item.blocks_movement;
            projectile |= item.blocks_projectile;
        }
        if field.is_none() {
            for id in tile.top_source_ids() {
                // Static terrain/decorations are qualified by the Map owner and
                // need not have a portable Item profile. Only an admitted field
                // recipe adds a path cost; collision facts above still apply.
                let Some(policy) = self.content.item_policy_for_source_id(*id) else {
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
            admits: tile.ground_present()
                && !solid
                && !tile.flags().protection_zone
                && !tile.flags().floor_change
                && !crate::map::floor::is_teleport(
                    self.room.movement_cells(),
                    LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ),
            projectile,
            speed: speed::qualified_ground_speed(tile.ground_present(), tile.ground_speed()),
            field,
        })
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
