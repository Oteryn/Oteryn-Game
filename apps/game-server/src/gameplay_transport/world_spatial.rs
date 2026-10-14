//! `FIRST-CONTROL-WIRE-V1` typed payload codecs (#642 owner acceptance 5853424280).
//!
//! Moved to `oteryn-protocol-oteryn::world_spatial` (ADR-0011 §2, #162 A6, task
//! `OTV2-20260928-protocol-oteryn-crate-c1a`): a client also needs these codecs, and that crate
//! has no dependency on this one. This module re-exports the whole original `pub(crate)` surface
//! unchanged, so nothing else in this crate needed to change.

use std::collections::BTreeMap;

use crate::movement::interest::{
    InterestChange, InterestDiff, InterestEntity, InterestIndex, VisibilityPosition,
    VisibilityQuery, VisibilitySettings, diff_interest,
};
pub(crate) use oteryn_protocol_oteryn::world_spatial::*;
pub(crate) use oteryn_protocol_oteryn::world_spatial_entities::*;

/// WORLD-INTERACTION-0 §10.2 (`WORLDINT0-RL-20`): a shown entity's `light` changes on the wire
/// at most once in this many milliseconds.
pub(crate) const LIGHT_UPDATE_INTERVAL_MS: u64 = 1_000;

/// The lights of `entities` among `lights`, or none for a session without capability 19.
fn lights_of<'a>(
    entities: impl Iterator<Item = &'a WorldSpatialEntity>,
    fields: EntityFields,
    lights: &EntityLights,
) -> EntityLights {
    if !fields.light {
        return EntityLights::new();
    }
    entities
        .filter_map(|entity| {
            let identity = entity.entity.identity;
            lights.get(&identity).map(|light| (identity, *light))
        })
        .collect()
}

/// [`encode_visibility_snapshot_lit`] without lights.
#[cfg(test)]
pub(crate) fn encode_visibility_snapshot(
    selected_capabilities: &[u32],
    snapshot: &WorldSpatialEntitiesSnapshot,
) -> Result<(u32, Vec<u8>), WorldSpatialError> {
    encode_visibility_snapshot_lit(selected_capabilities, snapshot, &EntityLights::new())
}

/// Server side: the payload type and bytes one session receives for a full snapshot. A session
/// that selected capability 6 gets the entity revision; every other session keeps the v1 type
/// with its own actor only. With capability 4 every object also carries the session's item
/// handle (ITEM-VIEW-1b, attached by `item_view::SessionItemView`). With capability 19 its lit
/// players and creatures carry the `lights` the session is shown ([`SessionVisibility::lights`]).
pub(crate) fn encode_visibility_snapshot_lit(
    selected_capabilities: &[u32],
    snapshot: &WorldSpatialEntitiesSnapshot,
    lights: &EntityLights,
) -> Result<(u32, Vec<u8>), WorldSpatialError> {
    if selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let fields = EntityFields::selected(selected_capabilities);
        let lights = lights_of(snapshot.entities.iter(), fields, lights);
        let payload =
            encode_world_spatial_entities_snapshot_with_fields(snapshot, fields, &lights)?;
        Ok((SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload))
    } else {
        validate_snapshot(snapshot)?;
        Ok((
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            encode_world_spatial(&WorldSpatialObservation {
                content_generation: snapshot.content_generation,
                actor_position: snapshot.actor_position,
            }),
        ))
    }
}

/// [`encode_visibility_delta_lit`] without lights.
#[cfg(test)]
pub(crate) fn encode_visibility_delta(
    selected_capabilities: &[u32],
    delta: &WorldSpatialEntitiesDelta,
) -> Result<(u32, Vec<u8>), WorldSpatialError> {
    encode_visibility_delta_lit(selected_capabilities, delta, &EntityLights::new())
}

/// Server side: the payload type and bytes one session receives for a delta, gated as above;
/// only the entered and updated entries carry their light.
pub(crate) fn encode_visibility_delta_lit(
    selected_capabilities: &[u32],
    delta: &WorldSpatialEntitiesDelta,
    lights: &EntityLights,
) -> Result<(u32, Vec<u8>), WorldSpatialError> {
    if selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let fields = EntityFields::selected(selected_capabilities);
        let lights = lights_of(delta.enter.iter().chain(&delta.update), fields, lights);
        let payload = encode_world_spatial_entities_delta_with_fields(delta, fields, &lights)?;
        Ok((DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload))
    } else {
        validate_delta(delta)?;
        Ok((
            DELTA_TYPE_WORLD_SPATIAL_V1,
            encode_world_spatial(&WorldSpatialObservation {
                content_generation: delta.content_generation,
                actor_position: delta.actor_position,
            }),
        ))
    }
}

/// VIS-3 known gap: the runtime carries no facing yet, so every actor faces south.
pub(crate) const PLACEHOLDER_ACTOR_DIRECTION: StepDirection = StepDirection::South;
/// VIS-3 known gap: the runtime carries no appearance yet.
pub(crate) const PLACEHOLDER_APPEARANCE_REF: u32 = 0;
/// VIS-3 known gap: the runtime keeps no maximum health beside the slot, so a live actor shows
/// full health.
pub(crate) const PLACEHOLDER_HEALTH_PERCENT: u8 = 100;
/// D85: an NPC entity always carries full health.
const NPC_HEALTH_PERCENT: u8 = 100;

/// VIS-3: the kind of one Channel entity, as the Channel owner reads it.
///
/// MAP-ITEM-REF-1: a corpse is shown only once its corpse Item is bound and its definition maps
/// to an `item_definition_ref` of the session's generation; it is a D85 object of one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisibleKind {
    Player,
    Creature,
    /// NPC-RT-1: a placed NPC actor (NPC-BEHAVIOUR-0 §3.1). It has no health; the wire always
    /// carries 100 for it.
    Npc,
    Corpse {
        item_definition_ref: std::num::NonZeroU32,
    },
}

/// VIS-3: one entity of the session's Channel, before the interest query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelEntity {
    pub(crate) kind: VisibleKind,
    pub(crate) identity: [u8; ENTITY_IDENTITY_BYTES],
    /// The actor generation.
    pub(crate) generation: u64,
    pub(crate) position: ActorPosition,
    /// The position revision: a changed revision is an update.
    pub(crate) revision: u64,
}

/// VIS-3: every entity of the session's Channel and the observer's identity among them, read in
/// one Channel-owner work item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelEntities {
    pub(crate) content_generation: [u8; 32],
    pub(crate) observer: [u8; ENTITY_IDENTITY_BYTES],
    pub(crate) entities: Vec<ChannelEntity>,
    /// CLOCK-1 (§11.4): the light of each lit player and creature, by identity, in the same read.
    pub(crate) lights: EntityLights,
    /// The owner time of the read in milliseconds; it paces `light` updates.
    pub(crate) observed_at_ms: u64,
}

fn wire_entity(entity: &ChannelEntity) -> WorldSpatialEntity {
    let actor = EntityDetail::Actor {
        direction: PLACEHOLDER_ACTOR_DIRECTION,
        appearance_ref: PLACEHOLDER_APPEARANCE_REF,
        health_percent: PLACEHOLDER_HEALTH_PERCENT,
    };
    let (kind, detail) = match entity.kind {
        VisibleKind::Player => (EntityKind::Player, actor),
        VisibleKind::Creature => (EntityKind::Creature, actor),
        VisibleKind::Npc => (
            EntityKind::Npc,
            EntityDetail::Actor {
                direction: PLACEHOLDER_ACTOR_DIRECTION,
                appearance_ref: PLACEHOLDER_APPEARANCE_REF,
                health_percent: NPC_HEALTH_PERCENT,
            },
        ),
        // The handle is attached per session under capability 4 (`attach`).
        VisibleKind::Corpse {
            item_definition_ref,
        } => (
            EntityKind::Corpse,
            EntityDetail::Object {
                item_definition_ref: item_definition_ref.get(),
                quantity: 1,
                item_handle: None,
            },
        ),
    };
    WorldSpatialEntity {
        kind,
        entity: EntityRef {
            identity: entity.identity,
            generation: entity.generation,
        },
        position: entity.position,
        detail,
    }
}

/// VIS-3: what one visibility refresh sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VisibilityUpdate {
    /// Nothing the session sees changed.
    Unchanged,
    /// At most `MOVE-RL-08` changes.
    Delta(WorldSpatialEntitiesDelta),
    /// A larger change: the full current view (the resync disposition, MOVE-RL-11 §4.3).
    Snapshot(WorldSpatialEntitiesSnapshot),
}

/// VIS-3 (MOVE-RL-11 §4): the domain 1 view of one session with capability 6, composed from the
/// VIS-1 interest set of its Channel. The query keeps the own actor and then at most 255 others
/// in canonical order (the degrade disposition); each refresh is diffed against what the
/// session was last sent. It lives with the connection: a resumed connection starts from a new
/// snapshot.
///
/// MAP-ITEM-REF-1 (CP D885): objects are shown only to a session that selected capability 4;
/// without it an object's `item_definition_ref` names nothing the session can resolve, so the
/// default view drops every object before the interest query.
///
/// CLOCK-1 (WORLD-INTERACTION-0 §10.2): with capability 19 the view also shows each player's and
/// creature's `light`. A shown entity whose light changes is updated at most once per
/// [`LIGHT_UPDATE_INTERVAL_MS`]; until then it keeps the light last sent, even in an entry sent
/// for a move.
#[derive(Debug, Default)]
pub(crate) struct SessionVisibility {
    shown: Option<VisibilityQuery>,
    entities: BTreeMap<[u8; ENTITY_IDENTITY_BYTES], WorldSpatialEntity>,
    objects: bool,
    light: bool,
    /// The light last sent for each shown lit entity.
    lights: EntityLights,
    /// When each shown entity's light last changed on the wire, in owner milliseconds.
    light_sent_at: BTreeMap<[u8; ENTITY_IDENTITY_BYTES], u64>,
}

impl SessionVisibility {
    /// A view that shows objects (corpses) when `objects`, the session's capability 4.
    pub(crate) fn with_objects(objects: bool) -> Self {
        Self {
            objects,
            ..Self::default()
        }
    }

    /// The same view that also shows lights when `light`, the session's capability 19.
    pub(crate) fn lit(self, light: bool) -> Self {
        Self { light, ..self }
    }

    /// The light last sent for each shown lit entity, for the encoders.
    pub(crate) fn lights(&self) -> &EntityLights {
        &self.lights
    }

    /// The light `channel` gives the entity now, or `None` without capability 19.
    fn current_light(
        &self,
        channel: &ChannelEntities,
        identity: &[u8; ENTITY_IDENTITY_BYTES],
    ) -> Option<EntityLight> {
        self.light
            .then(|| channel.lights.get(identity).copied())
            .flatten()
            .filter(|light| light.level != 0)
    }

    /// Records `light` as sent for `identity` at `now`. Showing an entity unlit is a sent light
    /// too: its first shown time starts the interval, so a light it gains next waits its turn.
    fn send_light(
        &mut self,
        identity: [u8; ENTITY_IDENTITY_BYTES],
        light: Option<EntityLight>,
        now: u64,
    ) {
        if self.lights.get(&identity).copied() == light {
            self.light_sent_at.entry(identity).or_insert(now);
            return;
        }
        self.light_sent_at.insert(identity, now);
        match light {
            Some(light) => self.lights.insert(identity, light),
            None => self.lights.remove(&identity),
        };
    }

    /// Whether `identity`'s light may change on the wire at `now`.
    fn light_due(&self, identity: &[u8; ENTITY_IDENTITY_BYTES], now: u64) -> bool {
        self.light_sent_at
            .get(identity)
            .is_none_or(|sent| now.saturating_sub(*sent) >= LIGHT_UPDATE_INTERVAL_MS)
    }

    /// The full view's lights: every selected entity is shown its current light.
    fn show_lights(&mut self, channel: &ChannelEntities, selected: &[WorldSpatialEntity]) {
        for entity in selected {
            let identity = entity.entity.identity;
            let light = self.current_light(channel, &identity);
            self.send_light(identity, light, channel.observed_at_ms);
        }
        self.forget_lights(selected);
    }

    /// Drops the light state of entities no longer shown.
    fn forget_lights(&mut self, selected: &[WorldSpatialEntity]) {
        let shown: std::collections::BTreeSet<_> = selected
            .iter()
            .map(|entity| entity.entity.identity)
            .collect();
        self.lights.retain(|identity, _| shown.contains(identity));
        self.light_sent_at
            .retain(|identity, _| shown.contains(identity));
    }

    /// The query of `channel` from the observer, and each selected entity in its wire form, in
    /// the query's order (own actor first). `attach` gives each object its item handle under
    /// capability 4. An observer that is not in `channel`, or not on a valid floor, fails.
    fn select(
        &self,
        channel: &ChannelEntities,
        attach: &mut dyn FnMut(&mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError>,
    ) -> Result<(VisibilityQuery, Vec<WorldSpatialEntity>), WorldSpatialError> {
        let mut index = InterestIndex::new();
        let mut by_identity = BTreeMap::new();
        for entity in &channel.entities {
            if !self.objects && matches!(entity.kind, VisibleKind::Corpse { .. }) {
                continue;
            }
            // An entity off the 0..=15 floors cannot be shown; the observer then fails below.
            let Ok(position) = VisibilityPosition::new(
                entity.position.x,
                entity.position.y,
                entity.position.floor,
            ) else {
                continue;
            };
            index.upsert(InterestEntity {
                identity: entity.identity,
                position,
                revision: entity.revision,
            });
            by_identity.insert(entity.identity, entity);
        }
        let query = index
            .query(&channel.observer, VisibilitySettings::REFERENCE)
            .map_err(|_| WorldSpatialError::Malformed)?;
        let mut selected = query
            .entities()
            .iter()
            .map(|entity| {
                by_identity
                    .get(&entity.identity)
                    .map(|entity| wire_entity(entity))
                    .ok_or(WorldSpatialError::Malformed)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if selected
            .first()
            .is_none_or(|own| own.kind != EntityKind::Player)
        {
            return Err(WorldSpatialError::Malformed);
        }
        attach(&mut selected)?;
        Ok((query, selected))
    }

    fn snapshot_of(
        channel: &ChannelEntities,
        selected: &[WorldSpatialEntity],
    ) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
        let own = selected.first().ok_or(WorldSpatialError::Malformed)?;
        Ok(WorldSpatialEntitiesSnapshot {
            content_generation: channel.content_generation,
            actor_position: own.position,
            own_identity: own.entity.identity,
            entities: selected.to_vec(),
        })
    }

    fn show(&mut self, query: VisibilityQuery, selected: Vec<WorldSpatialEntity>) {
        self.shown = Some(query);
        self.entities = selected
            .into_iter()
            .map(|entity| (entity.entity.identity, entity))
            .collect();
    }

    /// The full current view, for the join, resync or resume snapshot.
    pub(crate) fn snapshot(
        &mut self,
        channel: &ChannelEntities,
        attach: &mut dyn FnMut(&mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError>,
    ) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
        let (query, selected) = self.select(channel, attach)?;
        let snapshot = Self::snapshot_of(channel, &selected)?;
        self.show_lights(channel, &selected);
        self.show(query, selected);
        Ok(snapshot)
    }

    /// What changed since the session was last sent its view: a delta of at most 256 entries, or
    /// a snapshot when the change is larger (or nothing was shown yet).
    pub(crate) fn refresh(
        &mut self,
        channel: &ChannelEntities,
        attach: &mut dyn FnMut(&mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError>,
    ) -> Result<VisibilityUpdate, WorldSpatialError> {
        let (query, selected) = self.select(channel, attach)?;
        let Some(shown) = &self.shown else {
            let snapshot = Self::snapshot_of(channel, &selected)?;
            self.show_lights(channel, &selected);
            self.show(query, selected);
            return Ok(VisibilityUpdate::Snapshot(snapshot));
        };
        let changes = match diff_interest(shown, &query) {
            InterestDiff::Resync(_) => {
                let snapshot = Self::snapshot_of(channel, &selected)?;
                self.show_lights(channel, &selected);
                self.show(query, selected);
                return Ok(VisibilityUpdate::Snapshot(snapshot));
            }
            InterestDiff::Delta(changes) => changes,
        };
        let current: BTreeMap<_, _> = selected
            .iter()
            .map(|entity| (entity.entity.identity, *entity))
            .collect();
        let own = selected.first().ok_or(WorldSpatialError::Malformed)?;
        let mut delta = WorldSpatialEntitiesDelta {
            content_generation: channel.content_generation,
            actor_position: own.position,
            enter: Vec::new(),
            update: Vec::new(),
            leave: Vec::new(),
        };
        for change in &changes {
            match change {
                InterestChange::Enter(entity) => delta.enter.push(
                    *current
                        .get(&entity.identity)
                        .ok_or(WorldSpatialError::Malformed)?,
                ),
                InterestChange::Update(entity) => delta.update.push(
                    *current
                        .get(&entity.identity)
                        .ok_or(WorldSpatialError::Malformed)?,
                ),
                InterestChange::Leave(identity) => delta.leave.push(
                    self.entities
                        .get(identity)
                        .ok_or(WorldSpatialError::Malformed)?
                        .entity,
                ),
            }
        }
        // CLOCK-1: an entered entity shows its current light; a moved one its current light if
        // due, else the light last sent; any other shown entity whose light changed is updated
        // once due, within the delta ceiling (a deferred one follows on a later refresh).
        let now = channel.observed_at_ms;
        for entity in &delta.enter {
            let identity = entity.entity.identity;
            self.light_sent_at.remove(&identity);
            self.lights.remove(&identity);
            let light = self.current_light(channel, &identity);
            self.send_light(identity, light, now);
        }
        for entity in &delta.update {
            let identity = entity.entity.identity;
            if self.light_due(&identity, now) {
                let light = self.current_light(channel, &identity);
                self.send_light(identity, light, now);
            }
        }
        let mut room = MAX_DELTA_ENTRIES.saturating_sub(changes.len());
        for entity in &selected {
            let identity = entity.entity.identity;
            let light = self.current_light(channel, &identity);
            if room == 0
                || !self.entities.contains_key(&identity)
                || delta
                    .update
                    .iter()
                    .any(|shown| shown.entity.identity == identity)
                || self.lights.get(&identity).copied() == light
                || !self.light_due(&identity, now)
            {
                continue;
            }
            delta.update.push(*entity);
            self.send_light(identity, light, now);
            room -= 1;
        }
        self.forget_lights(&selected);
        self.show(query, selected);
        if delta.enter.is_empty() && delta.update.is_empty() && delta.leave.is_empty() {
            Ok(VisibilityUpdate::Unchanged)
        } else {
            Ok(VisibilityUpdate::Delta(delta))
        }
    }
}

/// Server side (SPEED-1, CONDITIONS-0 §4.3; STEP-PACE-WIRE-1): the command type 1 result one
/// session receives. `TOO_EARLY` and the `step_wait_ms` pacing receipt (field 2) reach only a
/// session that selected capability 13 `PACED_MOVEMENT_V1`; every other session gets `REJECTED`
/// for the same refusal and the exact receipt-less bytes it always got. `step_wait_ms` is 0 for
/// no receipt; a receipt outside the STEP-PACE-WIRE-0 contract (on `BLOCKED` or `REJECTED`, a
/// `MOVED` duration off the 50 ms beat, above [`MAX_STEP_WAIT_MS`]) is dropped, never sent.
pub(crate) fn encode_step_outcome(
    selected_capabilities: &[u32],
    disposition: StepDisposition,
    step_wait_ms: u32,
) -> Vec<u8> {
    if !selected_capabilities.contains(&CAPABILITY_PACED_MOVEMENT_V1) {
        return encode_step_result(match disposition {
            StepDisposition::TooEarly => StepDisposition::Rejected,
            disposition => disposition,
        });
    }
    let result = StepResult {
        disposition,
        step_wait_ms,
    };
    encode_step_result_paced(result, true).unwrap_or_else(|_| encode_step_result(disposition))
}

/// STEP-PACE-WIRE-1 rollout switch, the environment variable of the `serve` process. Capability 13
/// clients built before STEP-PACE-WIRE-0 (#2034) refuse field 2, so the server writes no receipt
/// until the operator sets it to exactly `1`, after the updated capability 13 decoder client is
/// released. Unset or any other value: off, every step result keeps its former bytes.
pub(crate) const STEP_WAIT_RECEIPTS_ENV: &str = "OTERYN_STEP_WAIT_RECEIPTS";

/// Whether the switch value turns the receipt on: only exactly `1` does.
fn step_wait_receipts_switch(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| value == "1")
}

/// The process-wide [`STEP_WAIT_RECEIPTS_ENV`] switch, read once and fixed for the process.
pub(crate) fn step_wait_receipts_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        step_wait_receipts_switch(std::env::var_os(STEP_WAIT_RECEIPTS_ENV).as_deref())
    })
}

/// STEP-PACE-WIRE-1: the `MOVED` receipt, the exact step duration the pacing clock recorded, in
/// milliseconds; 0 (no receipt) without a duration or for one that is not a whole number of
/// milliseconds within `u32`.
pub(crate) fn moved_step_wait_ms(duration: Option<std::time::Duration>) -> u32 {
    duration
        .filter(|duration| duration.subsec_nanos() % 1_000_000 == 0)
        .and_then(|duration| u32::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

/// STEP-PACE-WIRE-1: the `TOO_EARLY` receipt, the time `left` until the pacing clock is due,
/// rounded up to whole milliseconds (saturating); 0 (no receipt) when no deadline runs.
pub(crate) fn too_early_step_wait_ms(left: Option<std::time::Duration>) -> u32 {
    left.map_or(0, |left| {
        let millis = left.as_nanos().div_ceil(1_000_000);
        u32::try_from(millis).unwrap_or(u32::MAX)
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
    use oteryn_protocol_oteryn::world_clock::CAPABILITY_WORLD_LIGHT_V1;

    fn entity(n: u8, kind: EntityKind) -> WorldSpatialEntity {
        let mut identity = [0_u8; ENTITY_IDENTITY_BYTES];
        identity[15] = n;
        WorldSpatialEntity {
            kind,
            entity: EntityRef {
                identity,
                generation: u64::from(n),
            },
            position: ActorPosition {
                x: i32::from(n),
                y: 0,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 3,
                health_percent: 50,
            },
        }
    }

    fn snapshot() -> WorldSpatialEntitiesSnapshot {
        let own = entity(0, EntityKind::Player);
        WorldSpatialEntitiesSnapshot {
            content_generation: [7; 32],
            actor_position: own.position,
            own_identity: own.entity.identity,
            entities: vec![own, entity(1, EntityKind::Creature)],
        }
    }

    #[test]
    fn only_a_session_that_selected_capability_6_receives_the_entity_revision() {
        let snapshot = snapshot();
        // Old client against a new server: v1 type and payload, own actor only.
        let (kind, payload) = encode_visibility_snapshot(&[], &snapshot).expect("v1");
        assert_eq!(kind, SNAPSHOT_TYPE_WORLD_SPATIAL_V1);
        assert_eq!(
            decode_world_spatial(&payload),
            Ok(WorldSpatialObservation {
                content_generation: snapshot.content_generation,
                actor_position: snapshot.actor_position,
            })
        );
        // A session that selected another capability still gets v1.
        assert_eq!(
            encode_visibility_snapshot(&[1], &snapshot).expect("v1").0,
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1
        );
        // New client against a new server: type 2 carries every entity.
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&selected, &snapshot).expect("v2");
        assert_eq!(kind, SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2);
        assert_eq!(
            decode_world_spatial_snapshot_view(&selected, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(snapshot))
        );
        // New client against an old server: it selected nothing and decodes the v1 type.
        let (kind, payload) = encode_visibility_snapshot(&[], &self::snapshot()).expect("v1");
        assert!(matches!(
            decode_world_spatial_snapshot_view(&[], kind, &payload),
            Ok(WorldSpatialSnapshotView::OwnActor(_))
        ));
    }

    #[test]
    fn delta_follows_the_same_gate_and_refuses_an_over_bound_change() {
        let delta = WorldSpatialEntitiesDelta {
            content_generation: [7; 32],
            actor_position: ActorPosition {
                x: 2,
                y: 0,
                floor: 7,
            },
            enter: vec![entity(2, EntityKind::Creature)],
            update: vec![entity(0, EntityKind::Player)],
            leave: vec![entity(1, EntityKind::Creature).entity],
        };
        let (kind, payload) = encode_visibility_delta(&[], &delta).expect("v1");
        assert_eq!(kind, DELTA_TYPE_WORLD_SPATIAL_V1);
        assert!(decode_world_spatial(&payload).is_ok());
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_delta(&selected, &delta).expect("v2");
        assert_eq!(kind, DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2);
        assert_eq!(
            decode_world_spatial_delta_view(&selected, kind, &payload),
            Ok(WorldSpatialDeltaView::Entities(delta.clone()))
        );
        // The bound applies to v1 sessions too: the change is one the server must resync.
        let mut over = delta;
        over.leave = (0..=255_u8)
            .map(|n| entity(n, EntityKind::Creature).entity)
            .collect();
        assert_eq!(
            encode_visibility_delta(&[], &over),
            Err(WorldSpatialError::LimitExceeded)
        );
    }

    #[test]
    fn too_early_is_sent_only_to_a_session_with_capability_13() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        assert_eq!(
            decode_step_result_paced(
                &encode_step_outcome(&paced, StepDisposition::TooEarly, 0),
                true
            ),
            Ok(StepDisposition::TooEarly)
        );
        assert_eq!(
            decode_step_result(&encode_step_outcome(&[], StepDisposition::TooEarly, 0)),
            Ok(StepDisposition::Rejected)
        );
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            for selected in [&paced[..], &[]] {
                assert_eq!(
                    encode_step_outcome(selected, disposition, 0),
                    encode_step_result(disposition)
                );
            }
        }
    }

    /// A proto3 reader that knows only field 1 and skips every unknown varint field, as the
    /// protobuf rules require of an older client: what such a client makes of the result.
    fn field_1_only(payload: &[u8]) -> Option<u64> {
        fn varint(input: &[u8], cursor: &mut usize) -> Option<u64> {
            let mut value = 0_u64;
            for shift in (0..64).step_by(7) {
                let byte = *input.get(*cursor)?;
                *cursor += 1;
                value |= u64::from(byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    return Some(value);
                }
            }
            None
        }
        let (mut cursor, mut disposition) = (0, None);
        while cursor < payload.len() {
            let key = varint(payload, &mut cursor)?;
            if key & 7 != 0 {
                return None;
            }
            let value = varint(payload, &mut cursor)?;
            if key >> 3 == 1 {
                disposition = Some(value);
            }
        }
        disposition
    }

    /// STEP-PACE-WIRE-1 field-2 compatibility gate: golden bytes with and without the receipt,
    /// field 2 only under capability 13, a round trip through the shared decoder, and a reader
    /// that ignores field 2 still decodes every result.
    #[test]
    fn step_wait_ms_is_sent_only_under_capability_13_and_old_readers_still_decode() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        let cases: [(StepDisposition, u32, &[u8], &[u8]); 7] = [
            // (disposition, wait, capability 13 bytes, bytes without capability 13)
            (
                StepDisposition::Moved,
                50,
                &[0x08, 0x01, 0x10, 0x32],
                &[0x08, 0x01],
            ),
            (
                StepDisposition::Moved,
                100,
                &[0x08, 0x01, 0x10, 0x64],
                &[0x08, 0x01],
            ),
            (
                StepDisposition::Moved,
                MAX_STEP_WAIT_MS,
                &[0x08, 0x01, 0x10, 0xae, 0xe4, 0x06],
                &[0x08, 0x01],
            ),
            (StepDisposition::Moved, 0, &[0x08, 0x01], &[0x08, 0x01]),
            (
                StepDisposition::TooEarly,
                37,
                &[0x08, 0x04, 0x10, 0x25],
                &[0x08, 0x03],
            ),
            (StepDisposition::TooEarly, 0, &[0x08, 0x04], &[0x08, 0x03]),
            (StepDisposition::Blocked, 0, &[0x08, 0x02], &[0x08, 0x02]),
        ];
        for (disposition, wait, with, without) in cases {
            let sent = encode_step_outcome(&paced, disposition, wait);
            assert_eq!(sent, with, "{disposition:?} {wait}");
            assert!(sent.len() <= MAX_PACED_STEP_RESULT_BYTES);
            assert_eq!(
                decode_step_result_receipt(&sent, true),
                Ok(StepResult {
                    disposition,
                    step_wait_ms: wait
                })
            );
            assert_eq!(field_1_only(&sent), Some(disposition as u64));
            // Without capability 13: never field 2, the exact receipt-less bytes, which the
            // strict pre-receipt decoder accepts within its 4-byte bound.
            let legacy = encode_step_outcome(&[], disposition, wait);
            assert_eq!(legacy, without, "{disposition:?} {wait}");
            assert!(legacy.len() <= MAX_STEP_RESULT_BYTES);
            assert!(decode_step_result(&legacy).is_ok());
            assert_eq!(field_1_only(&legacy), field_1_only(without));
        }
        // The strict decoder of a session without capability 13 refuses field 2: that is why
        // the receipt is never sent to one.
        assert!(decode_step_result(&[0x08, 0x01, 0x10, 0x64]).is_err());
    }

    /// A receipt outside the STEP-PACE-WIRE-0 contract is dropped, never sent: the result keeps
    /// its disposition and goes out without field 2.
    #[test]
    fn an_out_of_contract_step_wait_ms_is_dropped() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        for (disposition, wait) in [
            (StepDisposition::Moved, 80),
            (StepDisposition::Moved, 1),
            (StepDisposition::Moved, MAX_STEP_WAIT_MS + STEP_WAIT_BEAT_MS),
            (StepDisposition::TooEarly, MAX_STEP_WAIT_MS + 1),
            (StepDisposition::TooEarly, u32::MAX),
            (StepDisposition::Blocked, 50),
            (StepDisposition::Rejected, 50),
        ] {
            assert_eq!(
                encode_step_outcome(&paced, disposition, wait),
                encode_step_result(disposition),
                "{disposition:?} {wait}"
            );
        }
    }

    /// The rollout switch is off unless it is exactly `1`.
    #[test]
    fn the_step_wait_receipt_switch_is_off_unless_exactly_1() {
        use std::ffi::OsStr;
        assert!(!step_wait_receipts_switch(None));
        for off in ["", "0", "true", "on", "yes", " 1", "1 ", "01", "TRUE"] {
            assert!(!step_wait_receipts_switch(Some(OsStr::new(off))), "{off:?}");
        }
        assert!(step_wait_receipts_switch(Some(OsStr::new("1"))));
        assert_eq!(STEP_WAIT_RECEIPTS_ENV, "OTERYN_STEP_WAIT_RECEIPTS");
    }

    #[test]
    fn the_receipt_is_the_recorded_duration_or_the_time_left_rounded_up() {
        use std::time::Duration;
        assert_eq!(moved_step_wait_ms(Some(Duration::from_millis(100))), 100);
        assert_eq!(
            moved_step_wait_ms(Some(Duration::from_millis(111_150))),
            111_150
        );
        assert_eq!(moved_step_wait_ms(Some(Duration::from_micros(100_500))), 0);
        assert_eq!(moved_step_wait_ms(Some(Duration::from_secs(1 << 40))), 0);
        assert_eq!(moved_step_wait_ms(None), 0);
        assert_eq!(too_early_step_wait_ms(None), 0);
        assert_eq!(too_early_step_wait_ms(Some(Duration::ZERO)), 0);
        assert_eq!(too_early_step_wait_ms(Some(Duration::from_nanos(1))), 1);
        assert_eq!(
            too_early_step_wait_ms(Some(Duration::from_micros(99_001))),
            100
        );
        assert_eq!(
            too_early_step_wait_ms(Some(Duration::from_millis(100))),
            100
        );
        assert_eq!(
            too_early_step_wait_ms(Some(Duration::from_secs(1 << 40))),
            u32::MAX
        );
    }

    /// ITEM-VIEW-1b: with capability 4 every object carries its item handle on the snapshot and
    /// the delta; without it the handle is never sent.
    #[test]
    fn an_object_carries_its_item_handle_only_with_capability_4() {
        let corpse = |handle: Option<u64>| WorldSpatialEntity {
            detail: EntityDetail::Object {
                item_definition_ref: 4240,
                quantity: 1,
                item_handle: handle.and_then(oteryn_protocol_oteryn::item_view::ItemHandle::new),
            },
            entity: EntityRef {
                identity: [9; ENTITY_IDENTITY_BYTES],
                generation: 0,
            },
            ..entity(9, EntityKind::Corpse)
        };
        let mut handled = snapshot();
        handled.entities.push(corpse(Some(5)));
        let with = [
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_ITEM_VIEW_MOVE_V1,
        ];
        let (kind, payload) = encode_visibility_snapshot(&with, &handled).expect("handles");
        assert_eq!(
            decode_world_spatial_snapshot_view(&with, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(handled.clone()))
        );
        // Without capability 4 the handle is not sent.
        let without = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&without, &handled).expect("no handles");
        let mut unhandled = handled.clone();
        unhandled.entities[2] = corpse(None);
        assert_eq!(
            decode_world_spatial_snapshot_view(&without, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(unhandled.clone()))
        );
        // With capability 4 an object without its handle fails closed.
        assert!(encode_visibility_snapshot(&with, &unhandled).is_err());
        let delta = WorldSpatialEntitiesDelta {
            content_generation: [7; 32],
            actor_position: handled.actor_position,
            enter: vec![corpse(Some(5))],
            update: Vec::new(),
            leave: Vec::new(),
        };
        let (kind, payload) = encode_visibility_delta(&with, &delta).expect("handles");
        assert_eq!(
            decode_world_spatial_delta_view(&with, kind, &payload),
            Ok(WorldSpatialDeltaView::Entities(delta))
        );
    }

    // VIS-3: the session's view composed from the interest set.

    fn id(n: u32) -> [u8; ENTITY_IDENTITY_BYTES] {
        let mut identity = [0_u8; ENTITY_IDENTITY_BYTES];
        identity[12..].copy_from_slice(&n.to_be_bytes());
        identity
    }

    fn at(n: u32, kind: VisibleKind, x: i32, y: i32, floor: i16) -> ChannelEntity {
        ChannelEntity {
            kind,
            identity: id(n),
            generation: 1,
            position: ActorPosition { x, y, floor },
            revision: 0,
        }
    }

    /// The observer, `id(0)`, stands at (100, 100, 7).
    fn channel(others: impl IntoIterator<Item = ChannelEntity>) -> ChannelEntities {
        let mut entities = vec![at(0, VisibleKind::Player, 100, 100, 7)];
        entities.extend(others);
        ChannelEntities {
            content_generation: [7; 32],
            observer: id(0),
            entities,
            lights: EntityLights::new(),
            observed_at_ms: 0,
        }
    }

    fn lit(mut channel: ChannelEntities, at_ms: u64, lights: &[(u32, u8)]) -> ChannelEntities {
        channel.observed_at_ms = at_ms;
        channel.lights = lights
            .iter()
            .map(|(n, level)| {
                (
                    id(*n),
                    EntityLight {
                        level: *level,
                        color: 215,
                    },
                )
            })
            .collect();
        channel
    }

    fn update_ids(update: &VisibilityUpdate) -> Vec<[u8; ENTITY_IDENTITY_BYTES]> {
        match update {
            VisibilityUpdate::Delta(delta) => identities(&delta.update),
            _ => panic!("expected a delta"),
        }
    }

    #[test]
    fn light_reaches_only_a_session_with_capability_19_and_changes_at_most_once_a_second() {
        let others = || [at(1, VisibleKind::Player, 101, 100, 7)];
        let mut unlit = SessionVisibility::default();
        unlit
            .snapshot(&lit(channel(others()), 0, &[(1, 100)]), &mut no_handles)
            .expect("snapshot");
        assert!(unlit.lights().is_empty());

        let mut view = SessionVisibility::default().lit(true);
        let snapshot = view
            .snapshot(&lit(channel(others()), 0, &[(1, 100)]), &mut no_handles)
            .expect("snapshot");
        let expected = EntityLight {
            level: 100,
            color: 215,
        };
        assert_eq!(view.lights().get(&id(1)), Some(&expected));
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES, CAPABILITY_WORLD_LIGHT_V1];
        let (_, payload) =
            encode_visibility_snapshot_lit(&selected, &snapshot, view.lights()).expect("lit");
        let (_, lights) = decode_world_spatial_entities_snapshot_with_fields(
            &payload,
            EntityFields::selected(&selected),
        )
        .expect("decode");
        assert_eq!(lights.get(&id(1)), Some(&expected));
        let (_, unlit_payload) =
            encode_visibility_snapshot_lit(&selected[..1], &snapshot, view.lights())
                .expect("unlit");
        assert_eq!(
            unlit_payload,
            encode_visibility_snapshot(&selected[..1], &snapshot)
                .expect("plain")
                .1
        );

        // A change within 1,000 ms of the last one waits; once due it is one update entry.
        let changed = view
            .refresh(&lit(channel(others()), 999, &[(1, 50)]), &mut no_handles)
            .expect("refresh");
        assert!(matches!(changed, VisibilityUpdate::Unchanged));
        assert_eq!(view.lights().get(&id(1)), Some(&expected));
        let changed = view
            .refresh(&lit(channel(others()), 1_000, &[(1, 50)]), &mut no_handles)
            .expect("refresh");
        assert_eq!(update_ids(&changed), [id(1)]);
        assert_eq!(view.lights().get(&id(1)).map(|light| light.level), Some(50));

        // A light that goes out is omitted; an unchanged one sends nothing.
        let changed = view
            .refresh(&lit(channel(others()), 2_000, &[]), &mut no_handles)
            .expect("refresh");
        assert_eq!(update_ids(&changed), [id(1)]);
        assert!(view.lights().is_empty());
        let same = view
            .refresh(&lit(channel(others()), 5_000, &[]), &mut no_handles)
            .expect("refresh");
        assert!(matches!(same, VisibilityUpdate::Unchanged));
    }

    #[test]
    fn a_move_within_the_interval_keeps_the_light_last_sent_and_a_leaver_forgets_it() {
        let mut view = SessionVisibility::default().lit(true);
        view.snapshot(
            &lit(
                channel([at(1, VisibleKind::Creature, 101, 100, 7)]),
                0,
                &[(1, 30)],
            ),
            &mut no_handles,
        )
        .expect("snapshot");
        let mut moved = at(1, VisibleKind::Creature, 102, 100, 7);
        moved.revision += 1;
        let changed = view
            .refresh(&lit(channel([moved]), 500, &[(1, 90)]), &mut no_handles)
            .expect("refresh");
        assert_eq!(update_ids(&changed), [id(1)]);
        assert_eq!(view.lights().get(&id(1)).map(|light| light.level), Some(30));
        view.refresh(&lit(channel([]), 600, &[]), &mut no_handles)
            .expect("leave");
        assert!(view.lights().is_empty());
    }

    #[test]
    fn an_entity_shown_unlit_gains_a_light_only_once_due() {
        let others = || [at(1, VisibleKind::Player, 101, 100, 7)];
        let mut view = SessionVisibility::default().lit(true);
        view.snapshot(&lit(channel(others()), 0, &[]), &mut no_handles)
            .expect("snapshot");
        assert!(view.lights().is_empty());
        let early = view
            .refresh(&lit(channel(others()), 250, &[(1, 40)]), &mut no_handles)
            .expect("refresh");
        assert!(matches!(early, VisibilityUpdate::Unchanged));
        assert!(view.lights().is_empty());
        let due = view
            .refresh(&lit(channel(others()), 1_000, &[(1, 40)]), &mut no_handles)
            .expect("refresh");
        assert_eq!(update_ids(&due), [id(1)]);
        assert_eq!(view.lights().get(&id(1)).map(|light| light.level), Some(40));

        // An entity entering unlit starts its interval at the enter as well.
        let mut view = SessionVisibility::default().lit(true);
        view.snapshot(&lit(channel([]), 0, &[]), &mut no_handles)
            .expect("snapshot");
        view.refresh(&lit(channel(others()), 2_000, &[]), &mut no_handles)
            .expect("enter");
        let early = view
            .refresh(&lit(channel(others()), 2_250, &[(1, 40)]), &mut no_handles)
            .expect("refresh");
        assert!(matches!(early, VisibilityUpdate::Unchanged));
        let due = view
            .refresh(&lit(channel(others()), 3_000, &[(1, 40)]), &mut no_handles)
            .expect("refresh");
        assert_eq!(update_ids(&due), [id(1)]);
    }

    fn no_handles(_: &mut [WorldSpatialEntity]) -> Result<(), WorldSpatialError> {
        Ok(())
    }

    fn identities(entities: &[WorldSpatialEntity]) -> Vec<[u8; ENTITY_IDENTITY_BYTES]> {
        entities
            .iter()
            .map(|entity| entity.entity.identity)
            .collect()
    }

    #[test]
    fn the_snapshot_is_the_interest_set_own_actor_first_in_canonical_order() {
        let channel = channel([
            at(5, VisibleKind::Creature, 102, 100, 7),
            at(4, VisibleKind::Player, 99, 101, 7),
            at(3, VisibleKind::Player, 101, 100, 7),
            // Outside the 18 x 14 area, and on a floor a ground observer does not see.
            at(6, VisibleKind::Creature, 120, 100, 7),
            at(7, VisibleKind::Creature, 100, 100, 8),
            // Above ground: floor 6 is offset by one tile, so (101, 101, 6) is at ring 0.
            at(8, VisibleKind::Player, 101, 101, 6),
        ]);
        let mut view = SessionVisibility::default();
        let snapshot = view.snapshot(&channel, &mut no_handles).expect("snapshot");
        assert_eq!(
            identities(&snapshot.entities),
            [id(0), id(3), id(4), id(5), id(8)]
        );
        assert_eq!(snapshot.own_identity, id(0));
        assert_eq!(snapshot.actor_position, channel.entities[0].position);
        assert_eq!(snapshot.content_generation, [7; 32]);
        assert_eq!(snapshot.entities[0].kind, EntityKind::Player);
        assert_eq!(snapshot.entities[2].kind, EntityKind::Player);
        assert_eq!(snapshot.entities[3].kind, EntityKind::Creature);
        assert_eq!(
            snapshot.entities[3].detail,
            EntityDetail::Actor {
                direction: PLACEHOLDER_ACTOR_DIRECTION,
                appearance_ref: PLACEHOLDER_APPEARANCE_REF,
                health_percent: PLACEHOLDER_HEALTH_PERCENT,
            }
        );
        // It is a valid type 2 snapshot, and a v1 session still gets its own actor only.
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&selected, &snapshot).expect("v2");
        assert_eq!(
            decode_world_spatial_snapshot_view(&selected, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(snapshot.clone()))
        );
        assert_eq!(
            encode_visibility_snapshot(&[], &snapshot).expect("v1").0,
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1
        );
    }

    #[test]
    fn an_npc_is_an_npc_actor_entity_with_full_health() {
        let channel = channel([at(1, VisibleKind::Npc, 101, 100, 7)]);
        let snapshot = SessionVisibility::default()
            .snapshot(&channel, &mut no_handles)
            .expect("snapshot");
        assert_eq!(identities(&snapshot.entities), [id(0), id(1)]);
        assert_eq!(snapshot.entities[1].kind, EntityKind::Npc);
        assert_eq!(
            snapshot.entities[1].detail,
            EntityDetail::Actor {
                direction: PLACEHOLDER_ACTOR_DIRECTION,
                appearance_ref: PLACEHOLDER_APPEARANCE_REF,
                health_percent: 100,
            }
        );
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_snapshot(&selected, &snapshot).expect("v2");
        assert_eq!(
            decode_world_spatial_snapshot_view(&selected, kind, &payload),
            Ok(WorldSpatialSnapshotView::Entities(snapshot))
        );
    }

    #[test]
    fn an_observer_outside_its_channel_fails_closed() {
        let mut lost = channel([at(1, VisibleKind::Creature, 101, 100, 7)]);
        lost.observer = id(9);
        assert!(
            SessionVisibility::default()
                .snapshot(&lost, &mut no_handles)
                .is_err()
        );
        // The observer must be a player.
        let mut creature = channel([]);
        creature.entities[0].kind = VisibleKind::Creature;
        assert!(
            SessionVisibility::default()
                .refresh(&creature, &mut no_handles)
                .is_err()
        );
    }

    /// `count` others in the cell east of the observer, then one entity five tiles west.
    fn crowd(count: u32) -> ChannelEntities {
        channel(
            (1..=count)
                .map(|n| at(n, VisibleKind::Creature, 101, 100, 7))
                .chain([at(9_999, VisibleKind::Creature, 95, 100, 7)]),
        )
    }

    #[test]
    fn the_256_ceiling_keeps_the_nearest_and_the_rest_enter_as_others_leave() {
        // 256 entities, own actor included: all of them.
        let mut view = SessionVisibility::default();
        let all = view.snapshot(&crowd(254), &mut no_handles).expect("256");
        assert_eq!(all.entities.len(), MAX_SNAPSHOT_ENTITIES);
        assert_eq!(
            all.entities.last().map(|e| e.entity.identity),
            Some(id(9_999))
        );
        // 257: the farthest is cut (degrade), never an error.
        let mut view = SessionVisibility::default();
        let cut = view.snapshot(&crowd(255), &mut no_handles).expect("257");
        assert_eq!(cut.entities.len(), MAX_SNAPSHOT_ENTITIES);
        assert!(!identities(&cut.entities).contains(&id(9_999)));
        assert!(encode_visibility_snapshot(&[CAPABILITY_WORLD_SPATIAL_ENTITIES], &cut).is_ok());
        // One near entity leaves: the farthest enters in the same delta.
        let mut fewer = crowd(255);
        fewer.entities.retain(|entity| entity.identity != id(7));
        let VisibilityUpdate::Delta(delta) = view.refresh(&fewer, &mut no_handles).expect("delta")
        else {
            panic!("expected a delta");
        };
        assert_eq!(identities(&delta.enter), [id(9_999)]);
        assert_eq!(
            delta.leave.iter().map(|e| e.identity).collect::<Vec<_>>(),
            [id(7)]
        );
        assert!(delta.update.is_empty());
    }

    /// The observer and `count` creatures from `first`, east of it in the area.
    fn line(first: u32, count: u32) -> ChannelEntities {
        channel((0..count).map(|n| {
            let x = 101 + i32::try_from(n % 9).expect("x");
            at(first + n, VisibleKind::Creature, x, 100, 7)
        }))
    }

    #[test]
    fn a_change_of_up_to_256_entries_is_a_delta_and_a_larger_one_a_snapshot() {
        // 128 leave and 128 enter: 256 entries, one delta.
        let mut view = SessionVisibility::default();
        view.snapshot(&line(1, 128), &mut no_handles)
            .expect("snapshot");
        let VisibilityUpdate::Delta(delta) = view
            .refresh(&line(1_001, 128), &mut no_handles)
            .expect("refresh")
        else {
            panic!("expected a delta");
        };
        assert_eq!((delta.enter.len(), delta.leave.len()), (128, 128));
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let (kind, payload) = encode_visibility_delta(&selected, &delta).expect("bounded");
        assert_eq!(
            decode_world_spatial_delta_view(&selected, kind, &payload),
            Ok(WorldSpatialDeltaView::Entities(delta))
        );
        // 129 leave and 129 enter: 258 entries, a new snapshot of the current view (resync).
        let mut view = SessionVisibility::default();
        view.snapshot(&line(1, 129), &mut no_handles)
            .expect("snapshot");
        let next = line(1_001, 129);
        let VisibilityUpdate::Snapshot(snapshot) =
            view.refresh(&next, &mut no_handles).expect("refresh")
        else {
            panic!("expected a snapshot");
        };
        assert_eq!(snapshot.entities.len(), 130);
        assert_eq!(snapshot.entities[0].entity.identity, id(0));
        // The view now shows the snapshot: nothing changed since.
        assert_eq!(
            view.refresh(&next, &mut no_handles),
            Ok(VisibilityUpdate::Unchanged)
        );
    }

    #[test]
    fn a_moved_or_restamped_entity_is_an_update_and_the_own_step_moves_the_header() {
        let mut view = SessionVisibility::default();
        let first = channel([at(1, VisibleKind::Creature, 101, 100, 7)]);
        view.snapshot(&first, &mut no_handles).expect("snapshot");
        assert_eq!(
            view.refresh(&first, &mut no_handles),
            Ok(VisibilityUpdate::Unchanged)
        );
        let mut stepped = first.clone();
        stepped.entities[0].position.x = 101;
        stepped.entities[0].revision = 1;
        stepped.entities[1].revision = 4;
        let VisibilityUpdate::Delta(delta) =
            view.refresh(&stepped, &mut no_handles).expect("delta")
        else {
            panic!("expected a delta");
        };
        assert_eq!(delta.actor_position, stepped.entities[0].position);
        assert_eq!(identities(&delta.update), [id(0), id(1)]);
        assert!(delta.enter.is_empty() && delta.leave.is_empty());
    }

    /// MAP-ITEM-REF-1 (CP D885): a session without capability 4 is sent no object; with it, the
    /// corpse is an entry carrying its item handle.
    #[test]
    fn objects_are_shown_only_to_a_session_with_capability_4() {
        let corpse = VisibleKind::Corpse {
            item_definition_ref: std::num::NonZeroU32::new(4240).expect("ref"),
        };
        let mut corpse_at = at(2, corpse, 101, 100, 7);
        corpse_at.generation = 0;
        let channel = channel([at(1, VisibleKind::Creature, 101, 100, 7), corpse_at]);
        let mut handles = |entities: &mut [WorldSpatialEntity]| {
            for entity in entities {
                if let EntityDetail::Object { item_handle, .. } = &mut entity.detail {
                    *item_handle = oteryn_protocol_oteryn::item_view::ItemHandle::new(5);
                }
            }
            Ok(())
        };
        let without = SessionVisibility::default()
            .snapshot(&channel, &mut handles)
            .expect("without 4");
        assert_eq!(identities(&without.entities), [id(0), id(1)]);
        assert!(
            without
                .entities
                .iter()
                .all(|entity| !matches!(entity.detail, EntityDetail::Object { .. }))
        );
        assert_eq!(
            SessionVisibility::with_objects(false)
                .refresh(&channel, &mut handles)
                .map(|update| match update {
                    VisibilityUpdate::Snapshot(snapshot) => snapshot.entities.len(),
                    _ => usize::MAX,
                }),
            Ok(2)
        );
        let with = SessionVisibility::with_objects(true)
            .snapshot(&channel, &mut handles)
            .expect("with 4");
        assert_eq!(identities(&with.entities), [id(0), id(1), id(2)]);
        assert_eq!(with.entities[2].kind, EntityKind::Corpse);
        assert_eq!(with.entities[2].entity.generation, 0);
        assert_eq!(
            with.entities[2].detail,
            EntityDetail::Object {
                item_definition_ref: 4240,
                quantity: 1,
                item_handle: oteryn_protocol_oteryn::item_view::ItemHandle::new(5),
            }
        );
        let selected = [
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_ITEM_VIEW_MOVE_V1,
        ];
        assert!(encode_visibility_snapshot(&selected, &with).is_ok());
    }
}
