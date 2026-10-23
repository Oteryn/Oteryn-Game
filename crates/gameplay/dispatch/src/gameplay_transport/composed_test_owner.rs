//! The single test constructor for the composed gameplay owner.

use super::{ComposedFreshAdmission, SeamRooms, channel_states::ChannelSessionStates};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, ChannelRuntimeV1, WorldId};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Borrow each test's existing harness and content; callers override deliberate deviations.
pub(super) struct ComposedTestOwner<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub holder: &'a NodeIncarnationProof,
    pub evidence: &'a super::FreshEvidenceSource,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub required_capabilities: Arc<super::world_required_capabilities::BoundRequiredCapabilities>,
    pub runtime: &'a Mutex<ChannelRuntimeV1>,
    pub room: &'a crate::content::QualifiedNativeEntryRoom,
    pub door: &'a Mutex<crate::world_runtime::LocalObjectRuntime>,
    pub spells: &'a crate::spell::SpellBook,
    pub achievements: &'a crate::achievement_catalogue::AchievementCatalogue,
    pub imported_charms: &'a crate::content::charm_source::CanonicalCharmCatalogue,
}

impl<'a, 'f, 's> ComposedTestOwner<'a, 'f, 's> {
    pub(super) fn build(self) -> ComposedFreshAdmission<'a, 'f, 's> {
        ComposedFreshAdmission {
            reset_epoch: 0,
            emit: |_, _| {},
            root: self.root,
            character: self.character,
            holder: self.holder,
            evidence: self.evidence,
            world_id: self.world_id,
            channel_id: self.channel_id,
            required_capabilities: self.required_capabilities,
            runtime: self.runtime,
            movement_cells: self.room.movement_cells(),
            door: self.door,
            chest: self.room.door(),
            spells: self.spells,
            active_generation: None,
            premium_coordinator: None,
            rooms: SeamRooms::new(None, None),
            achievements: self.achievements,
            imported_charms: self.imported_charms,
            states: ChannelSessionStates::default(),
            clock_origin: std::time::Instant::now(),
            revision_sequencer:
                crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
            quest_catalogue: None,
            quest_log_content: None,
            wheel_ruleset: None,
            premium: crate::premium::refresh::PremiumRefresher::new(
                Default::default(),
                self.root.clone(),
                None,
            ),
            spell_lane: crate::durability::spell_owner_commit::SpellLane::new(
                self.world_id,
                self.channel_id,
            ),
        }
    }
}
