//! W18: ordinary USE and map presentation of immutable House decorations.
use super::connection::{UseCommand, UseOutcome};
use super::world_map::{DecoratedFacts, HouseBaseMapEntry, MapViewSource, resolve_map_target};
use super::{ComposedFreshAdmission, ExactActorRef};
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::house_base_decorations::{
    HouseBaseDecorationSet, HouseBaseDecorationSource,
};
use crate::map::overlay::TilePos;
use crate::map::view::{self, MapFacts};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde::Deserialize;
use std::{collections::BTreeSet, sync::OnceLock};

#[derive(Deserialize)]
struct Pair {
    item_id: u32,
    target: u32,
    kind: Option<String>,
    #[serde(default)]
    noop: bool,
}
#[derive(Deserialize)]
struct Catalogue {
    schema: String,
    crystal_commit: String,
    windows: Vec<Pair>,
    native_transforms: Vec<Pair>,
}
fn catalogue() -> Option<&'static Catalogue> {
    static VALUE: OnceLock<Option<Catalogue>> = OnceLock::new();
    VALUE
        .get_or_init(|| {
            let value: Catalogue = serde_json::from_str(include_str!(
                "../../../../../tools/content-schema/native-gameplay/house-decoration-actions.json"
            ))
            .ok()?;
            (value.schema == "OTERYN_HOUSE_DECORATION_ACTIONS/v1"
                && value.crystal_commit == "201189f871d23597b82caf88fd611454a2ddefb8"
                && value.windows.len() <= 1024
                && value.native_transforms.len() <= 4096)
                .then_some(value)
        })
        .as_ref()
}
fn source_rule(id: u32, rotate: bool) -> Option<(&'static Pair, bool)> {
    let catalogue = catalogue()?;
    if !rotate && let Some(pair) = catalogue.windows.iter().find(|pair| pair.item_id == id) {
        return Some((pair, true));
    }
    catalogue
        .native_transforms
        .iter()
        .find(|pair| {
            pair.item_id == id
                && pair.kind.as_deref() == Some(if rotate { "ROTATE" } else { "USE" })
        })
        .map(|pair| (pair, false))
}

fn timed_keys(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> Option<BTreeSet<String>> {
    use crate::content::{ReferenceItemField as F, ReferenceMilliseconds, ReferenceTemporalMode};
    let mut pending: Vec<_> = catalogue()?
        .native_transforms
        .iter()
        .flat_map(|pair| [pair.item_id, pair.target])
        .map(|id| format!("oteryn:item.tibia.i{id}"))
        .collect();
    let mut seen = BTreeSet::new();
    let mut timed = BTreeSet::new();
    while let Some(key) = pending.pop() {
        if !seen.insert(key.clone()) {
            continue;
        }
        if seen.len() > 8192 {
            return None;
        }
        let Some(policy) = native.item_policy(&key, "definition-r1") else {
            continue;
        };
        let F::Known(temporal) = &policy.record().semantics.temporal else {
            continue;
        };
        if temporal.consumption_mode != F::Known(ReferenceTemporalMode::DurableAbsoluteDeadline)
            || temporal.stop_duration != F::Known(false)
            || !matches!(temporal.duration,F::Known(ReferenceMilliseconds(duration)) if duration>0)
        {
            return None;
        }
        timed.insert(key);
        if let F::Known(target) = &temporal.decay_target {
            if target.revision != "definition-r1" {
                return None;
            }
            pending.push(target.key.clone());
        }
    }
    Some(timed)
}
fn item_id(key: &str) -> Option<u32> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}
fn outcome(disposition: UseDisposition) -> Option<UseOutcome> {
    Some(UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Capture both possible adjacent BED components from this exact House and bundle.
    /// Crystal bed_modification_kits.lua:21-47 picks the orientation from the actual head.
    pub(in crate::gameplay_transport) async fn prepare_house_bed_changes(
        &self,
        context: &HouseActionContext,
        target: &WorldObjectTarget,
    ) -> Option<serde_json::Value> {
        let map = self.bundle_map()?;
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let bundle = map.overlay.base().digest();
        let changes = self.house_base_decoration_facts(context, bundle).await?;
        let base = map.facts.base_only();
        let facts = DecoratedFacts {
            base: &base,
            changes: &changes,
        };
        let content_generation = self
            .runtime
            .lock()
            .await
            .content_pin()
            .client_artifact_digest();
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation,
            reset_epoch: self.reset_epoch,
        };
        let resolved = resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let house = super::house_query::source_house_id(context.house().house_key())?;
        let at = context.position();
        if i32::from(resolved.pos.floor) != -at[2]
            || i32::from(resolved.pos.x).abs_diff(at[0]) > 1
            || i32::from(resolved.pos.y).abs_diff(at[1]) > 1
        {
            return None;
        }
        let bed = |pos: TilePos, expected_key: Option<u64>| -> Option<serde_json::Value> {
            if map.facts.house_id(pos) != Some(house) {
                return None;
            }
            let tile = view::compose_tile(&map.overlay, &facts, pos).ok()??;
            let mut beds = tile.entries.into_iter().rev().filter_map(|entry| {
                let view::MapEntryKey::Base(key) = entry.source.key() else { return None; };
                if expected_key.is_some_and(|expected| key!=expected) || entry.facts.bound { return None; }
                let MapDefinition::Item(reference)=entry.facts.definition else { return None; };
                let definition = super::item_look::typed_definition(index,reference)?;
                let policy = native.item_policy(&definition.production_key,&definition.revision_ref)?.record();
                if !matches!(&policy.semantics.classification,crate::content::ReferenceItemField::Known(classification)
                    if classification.item_type==crate::content::ReferenceItemField::Known(crate::content::ReferenceItemType::Bed)) {
                    return None;
                }
                Some(serde_json::json!({"placement_key":key.to_string(),
                    "expected_revision":facts.object_revision(key),"before_key":definition.production_key,
                    "position":[i32::from(pos.x),i32::from(pos.y),-i32::from(pos.floor)]}))
            });
            beds.next()
        };
        let first = bed(resolved.pos, Some(resolved.placement_key))?;
        let south = resolved
            .pos
            .y
            .checked_add(1)
            .and_then(|y| bed(TilePos { y, ..resolved.pos }, None));
        let east = resolved
            .pos
            .x
            .checked_add(1)
            .and_then(|x| bed(TilePos { x, ..resolved.pos }, None));
        Some(
            serde_json::json!({"bundle_digest":bundle.as_slice(),"target":first,"south":south,"east":east}),
        )
    }

    pub(in crate::gameplay_transport) async fn house_base_decoration_facts(
        &self,
        context: &HouseActionContext,
        bundle: [u8; 32],
    ) -> Option<Vec<HouseBaseMapEntry>> {
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let mut stored = self
            .root
            .read_house_base_decorations(self.holder, context, bundle)
            .await
            .ok()?;
        if !stored.materialized {
            let map = self.bundle_map()?;
            if map.overlay.base().digest() != bundle {
                return None;
            }
            let house = super::house_query::source_house_id(context.house().house_key())?;
            let timed = timed_keys(native)?;
            let base = map.facts.base_only();
            let mut initial = Vec::new();
            if !timed.is_empty() {
                for pos in map.facts.house_tiles(house) {
                    let mut entries = Vec::new();
                    view::compose_into(map.overlay.base(), None, &base, pos, &mut entries).ok()?;
                    for entry in entries {
                        let view::MapEntryKey::Base(placement_key) = entry.source.key() else {
                            continue;
                        };
                        let MapDefinition::Item(reference) = entry.facts.definition else {
                            continue;
                        };
                        if entry.facts.bound {
                            continue;
                        }
                        let definition = super::item_look::typed_definition(index, reference)?;
                        if timed.contains(&definition.production_key) {
                            initial.push(HouseBaseDecorationSource {
                                placement_key,
                                before_key: definition.production_key,
                                position: [
                                    i32::from(pos.x),
                                    i32::from(pos.y),
                                    -i32::from(pos.floor),
                                ],
                            });
                        }
                    }
                }
            }
            if !self
                .root
                .materialize_house_base_decorations(self.holder, context, bundle, initial)
                .await
                .ok()?
            {
                return None;
            }
            stored = self
                .root
                .read_house_base_decorations(self.holder, context, bundle)
                .await
                .ok()?;
        }
        let mut result = Vec::with_capacity(stored.entries.len());
        for state in stored.entries {
            let reference = index.definition_ref("Item", &state.definition_key, "definition-r1")?;
            let policy = native
                .item_policy(&state.definition_key, "definition-r1")?
                .record();
            result.push(HouseBaseMapEntry {
                placement_key: state.placement_key,
                definition: MapDefinition::Item(reference),
                appearance_id: view::appearance_id(&state.definition_key, None),
                blocks_projectile: policy.attributes.blocks_projectile?,
                revision: state.revision,
            });
        }
        Some(result)
    }

    /// Source membership, current actor, and immutable command receipt are independent checks.
    pub(in crate::gameplay_transport) async fn use_house_base_decoration(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        self.house_base_decoration_action(actor, command, target, false)
            .await
    }

    pub(in crate::gameplay_transport) async fn rotate_house_base_decoration(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        self.house_base_decoration_action(actor, command, target, true)
            .await
    }

    async fn house_base_decoration_action(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
        rotate: bool,
    ) -> Option<UseOutcome> {
        let map = self.bundle_map()?;
        let fence = command.item_fence?;
        let _lane = self.spell_lane_permit().await?;
        let (context, generation) = {
            let runtime = self.runtime.lock().await;
            runtime
                .house_actor_state(actor, command.game_session_id)
                .ok()?;
            if !runtime
                .house_player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return outcome(UseDisposition::Rejected);
            }
            let context = match HouseActionContext::from_native_owner(
                &runtime,
                actor,
                command.game_session_id,
                fence,
            ) {
                Ok(context) => context,
                Err(_) => return outcome(UseDisposition::StaleState),
            };
            if super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none() {
                return outcome(UseDisposition::StaleState);
            }
            (context, runtime.content_pin().client_artifact_digest())
        };
        let intent = if rotate {
            let mut encoded = oteryn_protocol_oteryn::house_rotation::COMMAND
                .to_be_bytes()
                .to_vec();
            encoded.extend(
                oteryn_protocol_oteryn::house_rotation::Request::Rotate(
                    oteryn_protocol_oteryn::house_rotation::Target::Base(target.clone()),
                )
                .encode()?,
            );
            encoded
        } else {
            oteryn_protocol_oteryn::world_object::encode_use_intent(target).ok()?
        };
        let bundle = map.overlay.base().digest();
        match self
            .root
            .replay_house_base_decoration(
                self.holder,
                &context,
                bundle,
                command.command_id,
                &intent,
            )
            .await
        {
            Ok(Some(disposition)) => return outcome(disposition),
            Ok(None) => {}
            Err(_) => return outcome(UseDisposition::NotPossible),
        }
        let Some(changes) = self.house_base_decoration_facts(&context, bundle).await else {
            return outcome(UseDisposition::NotPossible);
        };
        let base = map.facts.base_only();
        let facts = DecoratedFacts {
            base: &base,
            changes: &changes,
        };
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation: generation,
            reset_epoch: self.reset_epoch,
        };
        let resolved = match resolve_map_target(&source, target, |_| None::<()>) {
            Ok(resolved) => resolved,
            Err(_) => return outcome(UseDisposition::StaleState),
        };
        let before = view::compose_tile(&map.overlay, &facts, resolved.pos)
            .ok()??
            .entries
            .into_iter()
            .find(|entry| entry.source.key() == view::MapEntryKey::Base(resolved.placement_key))?
            .facts;
        let MapDefinition::Item(reference) = before.definition else {
            return outcome(UseDisposition::NothingToUse);
        };
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let definition = super::item_look::typed_definition(index, reference)?;
        let (rule, window) =
            item_id(&definition.production_key).and_then(|id| source_rule(id, rotate))?;
        if before.bound {
            return outcome(UseDisposition::NothingToUse);
        }
        let Some(source_house) = super::house_query::source_house_id(context.house().house_key())
        else {
            return outcome(UseDisposition::NotPossible);
        };
        // Crystal windows.lua:13-30 resolves the actual tile, then north, then west.
        let house_cell = std::iter::once(resolved.pos)
            .chain(
                window
                    .then_some([
                        resolved
                            .pos
                            .y
                            .checked_sub(1)
                            .map(|y| TilePos { y, ..resolved.pos }),
                        resolved
                            .pos
                            .x
                            .checked_sub(1)
                            .map(|x| TilePos { x, ..resolved.pos }),
                    ])
                    .into_iter()
                    .flatten()
                    .flatten(),
            )
            .find(|pos| map.facts.house_id(*pos).is_some());
        let Some(house_cell) = house_cell else {
            return outcome(UseDisposition::NothingToUse);
        };
        if map.facts.house_id(house_cell) != Some(source_house) {
            return outcome(UseDisposition::NothingToUse);
        }
        let at = context.position();
        if i32::from(resolved.pos.floor) != -at[2]
            || i32::from(resolved.pos.x).abs_diff(at[0]) > 1
            || i32::from(resolved.pos.y).abs_diff(at[1]) > 1
        {
            return outcome(UseDisposition::TooFar);
        }
        let after_key = format!("oteryn:item.tibia.i{}", rule.target);
        let Some(after) = native.item_policy(&after_key, "definition-r1") else {
            return outcome(UseDisposition::NotPossible);
        };
        let Some(policy) = native.item_policy(&definition.production_key, &definition.revision_ref)
        else {
            return outcome(UseDisposition::NotPossible);
        };
        // House pacing consumes the immutable base collision map. Admit only transformations
        // preserving its walkability; projectile presentation uses the actual target facts.
        if policy.record().attributes.blocks_movement.is_none()
            || policy.record().attributes.blocks_movement
                != after.record().attributes.blocks_movement
        {
            return outcome(UseDisposition::NotPossible);
        }
        {
            let runtime = self.runtime.lock().await;
            let Ok(current) = HouseActionContext::from_native_owner(
                &runtime,
                actor,
                command.game_session_id,
                fence,
            ) else {
                return outcome(UseDisposition::StaleState);
            };
            if current.position() != at
                || runtime.content_pin().client_artifact_digest() != generation
                || !runtime
                    .house_player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || self
                    .states
                    .spell_states
                    .try_lock()
                    .map_or(true, |states| states.is_dead(actor))
            {
                return outcome(UseDisposition::StaleState);
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return outcome(UseDisposition::Exhausted);
            }
        }
        // Crystal game.cpp:3054-3057 keeps the source Item when the requested type is absent.
        if rule.noop || rule.item_id == rule.target {
            return outcome(UseDisposition::Committed);
        }
        let request = HouseBaseDecorationSet {
            bundle,
            command: command.command_id,
            placement: resolved.placement_key,
            expected_revision: target.expected_revision,
            before_key: definition.production_key,
            after_key,
            intent,
            house_cell: [
                i32::from(house_cell.x),
                i32::from(house_cell.y),
                -i32::from(house_cell.floor),
            ],
        };
        outcome(
            self.root
                .set_house_base_decoration(self.holder, &context, request)
                .await
                .unwrap_or(UseDisposition::NotPossible),
        )
    }
}
