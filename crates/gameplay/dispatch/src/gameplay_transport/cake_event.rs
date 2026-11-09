//! Crystal201189f data-global/scripts/quests/a_piece_of_cake/actions_cake_bite.lua:5-63
//! and data-global/scripts/lib/a_piece_of_cake_config.lua:127-185,508-512:
//! normal fixed-map USE and the existing terrain projection.
use super::world_map::{MapViewSource, resolve_map_target};
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
};
use crate::durability::cake_event::{CakeBite, CakeMapSource};
use crate::durability::item_decay_retire::CorpseDecayFence;
use crate::durability::terrain_tool::{MapMutation, TerrainScope};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, RuntimeScopeRefV1};
use crate::map::overlay::TilePos;
use crate::map::view::{self, MapEntryKey};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[path = "cake_bridge.rs"]
mod bridge;

fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    bytes.into_iter().map(|b| format!("{b:02x}")).collect()
}
fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}
fn item_id(key: &str) -> Option<u16> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}
type CachedCakeMapSources = ([u8; 32], [u8; 32], Vec<CakeMapSource>);

#[derive(Default)]
pub(super) struct SourceCache(std::sync::Mutex<Option<CachedCakeMapSources>>);

impl ComposedFreshAdmission<'_, '_, '_> {
    fn immutable_cake_candidate(&self, target: &WorldObjectTarget) -> bool {
        let Some(map) = self.bundle_map() else {
            return false;
        };
        if target.placement.len() != 40 || target.placement[..32] != map.overlay.base().digest() {
            return false;
        }
        let Ok(bytes) = target.placement[32..].try_into() else {
            return false;
        };
        let key = u64::from_be_bytes(bytes);
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        let facts = map.facts.base_only();
        let Some(tile) = view::compose_tile(&map.overlay, &facts, pos).ok().flatten() else {
            return false;
        };
        let Some(entry) = tile
            .entries
            .into_iter()
            .find(|entry| entry.source.key() == MapEntryKey::Base(key))
        else {
            return false;
        };
        let MapDefinition::Item(reference) = entry.facts.definition else {
            return false;
        };
        let Some(index) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.item_index())
        else {
            return false;
        };
        super::item_look::typed_definition(index, reference)
            .and_then(|definition| item_id(&definition.production_key))
            .is_some_and(|id| matches!(id, 12085 | 12086))
    }

    /// Immutable source facts, never the current hidden/transformed stack, define
    /// the tower's fixed placements (config.lua127-134,162-185,508-512).
    fn cake_sources(&self) -> Option<Vec<CakeMapSource>> {
        let map = self.bundle_map()?;
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let bundle = map.overlay.base().digest();
        let facts = map.facts.base_only();
        let mut cache = self.states.cake_sources.0.lock().ok()?;
        if let Some((cached_bundle, cached_content, entries)) = cache.as_ref()
            && *cached_bundle == bundle
            && *cached_content == native.source_digest()
        {
            return Some(entries.clone());
        }
        let source_ids = [12085_u16, 12086, 12147, 12148, 12152, 12153, 12154, 12155];
        let compact = source_ids
            .into_iter()
            .filter_map(|id| {
                index
                    .definition_ref("Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1")
                    .map(|reference| reference.get() - 1)
            })
            .collect::<std::collections::BTreeSet<_>>();
        let mut sources = Vec::new();
        // Derived geometry cache only: normal bundle palettes and native Item
        // identities remain the sole registry. Edible USE is globally registered;
        // only upper-floor removal is restricted to the donor tower rectangle.
        for (floor, x, y, raw) in map.overlay.base().tiles() {
            if !raw.ids().iter().any(|id| compact.contains(id)) {
                continue;
            }
            let pos = TilePos { x, y, floor };
            let Ok(z) = u8::try_from(-i16::from(floor)) else {
                continue;
            };
            let Some(tile) = view::compose_tile(&map.overlay, &facts, pos).ok()? else {
                continue;
            };
            for entry in tile.entries {
                let MapEntryKey::Base(key) = entry.source.key() else {
                    continue;
                };
                if entry.facts.bound
                    || entry.facts.count != 1
                    || entry.facts.sub_type != 0
                    || map.facts.terrain_tool_action(key) != Some(None)
                {
                    continue;
                }
                let MapDefinition::Item(reference) = entry.facts.definition else {
                    continue;
                };
                let definition = super::item_look::typed_definition(index, reference)?;
                let Some(id) = item_id(&definition.production_key) else {
                    continue;
                };
                let upper = (4..=6).contains(&z)
                    && (33287..=33330).contains(&x)
                    && (32065..=32103).contains(&y)
                    && matches!(id, 12147 | 12148 | 12152..=12155);
                if !((z == 7 && matches!(id, 12085 | 12086)) || upper) {
                    continue;
                }
                sources.push(CakeMapSource {
                    placement: hex(bundle.into_iter().chain(key.to_be_bytes())),
                    spatial: hex(key.to_be_bytes()),
                    source_item: id,
                    x,
                    y,
                    z,
                });
                if sources.len() > 8192 {
                    return None;
                }
            }
        }
        *cache = Some((bundle, native.source_digest(), sources.clone()));
        Some(sources)
    }
    pub(in crate::gameplay_transport) async fn refresh_cake_world_map(&self) -> bool {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return false;
        };
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let fence = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().runtime_scope()
                != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
            {
                return false;
            }
            let Ok(owner) = runtime.owner_fence() else {
                return false;
            };
            CorpseDecayFence {
                runtime_scope: runtime.binding().runtime_scope(),
                scope_ownership_generation: owner.generation(),
            }
        };
        let scope = TerrainScope {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_generation: native.source_digest(),
            reset_epoch: match i64::try_from(self.reset_epoch) {
                Ok(epoch) => epoch,
                Err(_) => return false,
            },
        };
        let Some(sources) = self.cake_sources() else {
            return false;
        };
        let Ok(rows) = self
            .root
            .project_world_cake(
                self.holder,
                fence,
                map.overlay.base().digest(),
                scope,
                sources,
            )
            .await
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        if !runtime
            .owner_fence()
            .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
            || super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none()
        {
            return false;
        }
        let base = map.facts.base_only();
        for row in rows {
            let Ok(bytes) = row.placement.get(32..).unwrap_or_default().try_into() else {
                return false;
            };
            let key = u64::from_be_bytes(bytes);
            let Some((pos, _)) = view::placement_of(key) else {
                return false;
            };
            let Some(tile) = view::compose_tile(&map.overlay, &base, pos).ok().flatten() else {
                return false;
            };
            let Some(entry) = tile
                .entries
                .into_iter()
                .find(|entry| entry.source.key() == MapEntryKey::Base(key))
            else {
                return false;
            };
            let Ok(revision) = u64::try_from(row.revision) else {
                return false;
            };
            if !map.facts.install_gathering_state(
                key,
                revision,
                entry.facts,
                (row.item != 0).then_some(entry.facts),
            ) {
                return false;
            }
        }
        true
    }
    pub(in crate::gameplay_transport) async fn use_cake_world_object(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let fence = command.item_fence?;
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
            return None;
        }
        let map = self.bundle_map()?;
        let native = self.active_generation?.native_gameplay()?;
        let candidate = self.immutable_cake_candidate(target);
        let permit = self.spell_lane_permit().await?;
        let (at, generation) = {
            let runtime = self.runtime.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return Some(outcome(UseDisposition::StaleState));
            }
            (
                runtime.read_actor_position(actor).ok()?.position(),
                runtime.content_pin().client_artifact_digest(),
            )
        };
        let intent = oteryn_protocol_oteryn::world_object::encode_use_intent(target).ok()?;
        let reference = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        match self
            .root
            .replay_world_cake(
                self.character,
                self.holder,
                fence,
                reference,
                intent.clone(),
                native.source_digest(),
                candidate,
            )
            .await
        {
            Ok(Some(disposition)) => return Some(outcome(disposition)),
            Ok(None) => {}
            Err(_) => return Some(outcome(UseDisposition::NotPossible)),
        }
        if !candidate {
            return None;
        }
        if !self.refresh_cake_world_map().await {
            return Some(outcome(UseDisposition::NotPossible));
        }
        let facts = map.facts.pinned();
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation: generation,
            reset_epoch: self.reset_epoch,
        };
        let resolved = resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let entry = view::compose_tile(&map.overlay, &facts, resolved.pos)
            .ok()??
            .entries
            .into_iter()
            .find(|entry| entry.source.key() == MapEntryKey::Base(resolved.placement_key))?;
        let MapDefinition::Item(reference) = entry.facts.definition else {
            return None;
        };
        let definition = super::item_look::typed_definition(native.item_index()?, reference)?;
        let id = item_id(&definition.production_key)?;
        if !matches!(id, 12085 | 12086) {
            return None;
        }
        if i32::from(resolved.pos.floor) != -i32::from(at.floor)
            || i32::from(resolved.pos.x).abs_diff(at.x) > 1
            || i32::from(resolved.pos.y).abs_diff(at.y) > 1
        {
            return Some(outcome(UseDisposition::TooFar));
        }
        let expected_revision = match i64::try_from(target.expected_revision) {
            Ok(revision) => revision,
            Err(_) => return Some(outcome(UseDisposition::StaleState)),
        };
        let request = CakeBite {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).ok()?,
            ),
            intent,
            scope: TerrainScope {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_generation: native.source_digest(),
                reset_epoch: i64::try_from(self.reset_epoch).ok()?,
            },
            mutation: MapMutation {
                placement: target.placement.clone(),
                expected_revision,
                before: u32::from(id),
                after: 0,
                decay: Vec::new(),
            },
            floor: u8::try_from(-resolved.pos.floor).ok()?,
        };
        {
            let runtime = self.runtime.lock().await;
            if !runtime
                .read_actor_position(actor)
                .is_ok_and(|current| current.position() == at)
                || runtime.content_pin().client_artifact_digest() != generation
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || self
                    .states
                    .spell_states
                    .try_lock()
                    .map_or(true, |states| states.is_dead(actor))
            {
                return Some(outcome(UseDisposition::StaleState));
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return Some(outcome(UseDisposition::Exhausted));
            }
        }
        let disposition = self
            .root
            .bite_world_cake(&permit, self.character, self.holder, fence, request)
            .await
            .unwrap_or(UseDisposition::NotPossible);
        if disposition == UseDisposition::Committed {
            let _ = self.refresh_cake_world_map().await;
        }
        Some(outcome(disposition))
    }
}
