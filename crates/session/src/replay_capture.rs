//! Raw world, appearance and cooldown payloads observed at validated session state boundaries.
use super::*;
use oteryn_protocol_oteryn::{
    StateDeltaView, actor_motion, appearance, entity_names, guild_emblem, inventory_capacity,
    party_skull, world_clock,
};
use std::sync::Arc;
use world_spatial_entities::{EntityRef, decode_world_spatial_entities_delta_with_sidecar};

/// Maximum number of retained deltas per replay domain.
pub const REPLAY_DELTA_LIMIT: usize = 32;

/// Original domain payload and revision metadata of one applied delta.
#[derive(Clone, Debug)]
pub struct RawReplayDelta {
    pub server_sequence: u64,
    pub base_revision: u64,
    pub new_revision: u64,
    pub delta_type: u32,
    pub payload: Arc<[u8]>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ReplayRetention {
    pub(super) world_snapshot: Option<Arc<GatedSnapshot>>,
    pub(super) world_sequence: u64,
    pub(super) world_deltas: VecDeque<RawReplayDelta>,
    pub(super) appearance_sequence: Option<u64>,
    pub(super) appearance_deltas: VecDeque<RawReplayDelta>,
    pub(super) cooldown_deltas: VecDeque<RawReplayDelta>,
}

pub(super) fn retain_delta(
    history: &mut VecDeque<RawReplayDelta>,
    server_sequence: u64,
    delta: &StateDeltaView<'_>,
) {
    if history.len() == REPLAY_DELTA_LIMIT {
        history.pop_front();
    }
    history.push_back(RawReplayDelta {
        server_sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        delta_type: delta.delta_type,
        payload: Arc::from(delta.payload),
    });
}

impl<S> Session<S> {
    /// Most recent validated standalone actor cooldown deltas, oldest first.
    pub fn raw_actor_cooldown_deltas(&self) -> &VecDeque<RawReplayDelta> {
        &self.replay.cooldown_deltas
    }

    /// Most recent applied world entity deltas, oldest first, since the latest world snapshot.
    /// The oldest entries are evicted at [`REPLAY_DELTA_LIMIT`].
    pub fn raw_world_entity_deltas(&self) -> &VecDeque<RawReplayDelta> {
        &self.replay.world_deltas
    }

    /// Most recent applied appearance deltas, oldest first, since the latest appearance snapshot.
    /// The oldest entries are evicted at [`REPLAY_DELTA_LIMIT`]. Read the snapshot through
    /// `gated_snapshot(STATE_DOMAIN_APPEARANCE)`.
    pub fn raw_appearance_deltas(&self) -> &VecDeque<RawReplayDelta> {
        &self.replay.appearance_deltas
    }
}

pub(super) const C16C_FILES: [&str; 3] =
    ["c16c-snapshot.bin", "c16c-deltas.bin", "c16c-expected.txt"];
pub(super) const P19_FILES: [&str; 3] = [
    "p19-appearance-snapshot.bin",
    "p19-appearance-deltas.bin",
    "p19-expected.txt",
];

const ENTITY_CAPABILITIES: &[u32] = &[
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    world_spatial::CAPABILITY_PACED_MOVEMENT_V1,
    world_clock::CAPABILITY_WORLD_LIGHT_V1,
    party_skull::CAPABILITY_PARTY_SKULL_V1,
    entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2,
    world_spatial_entities::CAPABILITY_NATIVE_CREATURE_ICONS_V1,
    world_spatial_entities::CAPABILITY_NATIVE_OBJECT_APPEARANCE_V1,
    actor_motion::CAPABILITY_ACTOR_MOTION_V1,
    inventory_capacity::CAPABILITY_INVENTORY_CAPACITY_V1,
    guild_emblem::CAPABILITY_GUILD_EMBLEM_V1,
];

impl<S: SessionStream> Session<S> {
    /// Serialize the three C16c capture files as (file name, bytes) pairs.
    /// Call after observing the summoned creature enter and while it is still visible.
    /// Returns `None` if the summon or a complete replay chain is unavailable.
    pub fn c16c_capture_files(&self, summon: EntityRef) -> Option<[(&'static str, Vec<u8>); 3]> {
        let entity = self.world_entities()?.get(&summon)?;
        let fields = world_spatial_entities::EntityFields::selected(self.selected_capabilities());
        if entity.kind != EntityKind::Creature
            || !self.raw_world_entity_deltas().iter().any(|delta| {
                decode_world_spatial_entities_delta_with_sidecar(&delta.payload, fields)
                    .is_ok_and(|(value, _, _)| value.enter.iter().any(|e| e.entity == summon))
            })
        {
            return None;
        }
        self.replay_files(C16C_FILES, summon, entity.kind as u8, false)
    }

    /// Serialize the three P19 capture files as (file name, bytes) pairs.
    /// Call while the creature's forge marker is visible. The transcript includes
    /// the spatial context needed to project the appearance binding.
    /// Returns `None` if the marker is unavailable, the snapshots start at
    /// different server sequences, or a delta needed for replay was evicted.
    pub fn p19_capture_files(&self, creature: EntityRef) -> Option<[(&'static str, Vec<u8>); 3]> {
        let stacks = self
            .visible_appearances()
            .into_iter()
            .find(|entry| entry.entity.entity == creature)?
            .forge_stacks?;
        self.replay_files(P19_FILES, creature, stacks, true)
    }

    fn replay_files(
        &self,
        names: [&'static str; 3],
        entity: EntityRef,
        expected: u8,
        forge: bool,
    ) -> Option<[(&'static str, Vec<u8>); 3]> {
        let world = self.replay.world_snapshot.as_deref()?;
        let mut snapshots = vec![world];
        let mut selected = self
            .selected_capabilities()
            .iter()
            .copied()
            .filter(|id| ENTITY_CAPABILITIES.contains(id))
            .collect::<Vec<_>>();
        let mut deltas = Vec::new();
        append_chain(&mut deltas, world, self.raw_world_entity_deltas())?;
        let mut refs = Vec::new();
        let mut primary = world;
        if forge || selected.contains(&actor_motion::CAPABILITY_ACTOR_MOTION_V1) {
            let appearance = self.gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)?;
            // Cross-domain validation needs a common starting state.
            if self.replay.appearance_sequence != Some(self.replay.world_sequence) {
                return None;
            }
            selected.push(appearance::CAPABILITY_APPEARANCE_V1);
            snapshots.push(appearance);
            append_chain(&mut deltas, appearance, self.raw_appearance_deltas())?;
            refs = appearance_state::item_references(self.appearance.as_ref()?).unwrap_or_default();
            if forge {
                selected.push(appearance::CAPABILITY_FORGE_CREATURE_V1);
                primary = appearance;
            }
        }
        if selected.contains(&world_clock::CAPABILITY_WORLD_LIGHT_V1) {
            snapshots.push(self.gated_snapshot(world_clock::STATE_DOMAIN_WORLD_CLOCK)?);
        }
        selected.sort_unstable();
        deltas.sort_unstable_by_key(|(_, delta)| delta.server_sequence);

        let mut transcript = b"OTR1".to_vec();
        transcript.extend(self.replay.world_sequence.to_le_bytes());
        for list in [&selected, &refs] {
            transcript.extend(u32::try_from(list.len()).ok()?.to_le_bytes());
            for value in list {
                transcript.extend(value.to_le_bytes());
            }
        }
        transcript.extend(u32::try_from(snapshots.len()).ok()?.to_le_bytes());
        for snapshot in snapshots {
            transcript.extend(snapshot.domain_id.to_le_bytes());
            transcript.extend(snapshot.revision.to_le_bytes());
            transcript.extend(snapshot.snapshot_type.to_le_bytes());
            append_bytes(
                &mut transcript,
                if snapshot.domain_id == primary.domain_id {
                    &[]
                } else {
                    &snapshot.payload
                },
            )?;
        }
        for (domain, delta) in deltas {
            let frame = oteryn_protocol_oteryn::encode_state_delta(
                1,
                delta.server_sequence,
                domain,
                delta.base_revision,
                delta.new_revision,
                delta.delta_type,
                &delta.payload,
            )
            .ok()?;
            append_bytes(&mut transcript, &frame)?;
        }
        let mut line = String::new();
        use std::fmt::Write as _;
        for byte in entity.identity {
            write!(line, "{byte:02x}").ok()?;
        }
        writeln!(line, " {} {expected}", entity.generation).ok()?;
        Some([
            (names[0], primary.payload.clone()),
            (names[1], transcript),
            (names[2], line.into_bytes()),
        ])
    }
}

fn append_chain<'a>(
    output: &mut Vec<(u32, &'a RawReplayDelta)>,
    snapshot: &GatedSnapshot,
    deltas: &'a VecDeque<RawReplayDelta>,
) -> Option<()> {
    if deltas
        .front()
        .is_some_and(|first| first.base_revision != snapshot.revision)
    {
        return None;
    }
    output.extend(deltas.iter().map(|delta| (snapshot.domain_id, delta)));
    Some(())
}

fn append_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> Option<()> {
    output.extend(u32::try_from(bytes.len()).ok()?.to_le_bytes());
    output.extend(bytes);
    Some(())
}
