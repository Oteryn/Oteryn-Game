//! The single test constructor for the composed gameplay owner.

use super::{ComposedFreshAdmission, SeamRooms, channel_states::ChannelSessionStates};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, ChannelRuntimeV1, WorldId};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Borrow each test's existing harness and content; callers override deliberate deviations.
pub struct ComposedTestOwner<'a, 'f, 's> {
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
    pub fn build(self) -> ComposedFreshAdmission<'a, 'f, 's> {
        ComposedFreshAdmission {
            proficiency_catalogue: None,
            server_motd: "",
            reset_epoch: 0,
            emit: |_, _| {},
            root: self.root,
            character: self.character,
            holder: self.holder,
            evidence: self.evidence,
            world_id: self.world_id,
            channel_id: self.channel_id,
            premium_only: false,
            required_capabilities: self.required_capabilities,
            runtime: (self.runtime).into(),
            movement_cells: self.room.movement_cells().into(),
            door: (self.door).into(),
            chest: self.room.door(),
            spells: self.spells,
            active_generation: None,
            premium_coordinator: None,
            rooms: SeamRooms::new(None, None),
            achievements: self.achievements,
            imported_charms: self.imported_charms,
            states: (ChannelSessionStates::default()).into(),
            clock_origin: std::time::Instant::now(),
            revision_sequencer:
                (crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new())
                    .into(),
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
            store: None,
            store_purchases: Default::default(),
            activity_runtimes: Default::default(),
        }
    }
}

// The dedicated PG target uses the same caller and constructor as in-crate tests.
pub use super::connection::{FreshAdmissionAuthority, UseCommand};
pub use super::item_view::ItemKey;

pub fn empty_capabilities(
    world: WorldId,
    channel: ChannelId,
) -> Result<Arc<super::world_required_capabilities::BoundRequiredCapabilities>, String> {
    super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[],
        crate::foundation::RuntimeScopeRefV1::channel(world, channel),
        1,
        "policy-1",
        super::world_required_capabilities::offered_for_scope(false, None),
    )
    .map_err(|error| format!("{error:?}"))
}

pub async fn load_quests(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> Result<crate::durability::character_progression::CurrentCharacterGameplayFence, String> {
    owner.refresh_quest_session(session, true).await;
    if owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest copy lock")?
        .get(&session)
        .and_then(|entry| entry.copy.as_ref())
        .is_none()
    {
        return Err("quest copy unavailable".into());
    }
    owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current quest fence")?
        .ok_or("terminal quest session".into())
}

// Retain the existing typed admission fixture inside its owning crate, where
// its canonical UUID binding helper is available without widening production API.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
mod fixture {
    pub(super) fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
        format!("{error:?}").into()
    }
    pub(super) fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }
}
#[allow(
    dead_code,
    reason = "shared fixture includes activity and appearance producers unused by this consumer"
)]
#[path = "monk_save_fresh_fixture.rs"]
pub mod fresh_fixture;

/// Exercise the dispatch guard after the fixture's real Root capture transaction.
pub fn finish_capture(
    runtime: &mut ChannelRuntimeV1,
    target: crate::foundation::ExactActorRef,
    key: &str,
    outcome: &crate::durability::quest_state::QuestTransitionOutcome,
) -> Result<(), String> {
    let mut guard =
        super::cosmetic_item_use::capture_commit::CaptureCommitGuard::new(runtime, true);
    guard.arm();
    guard
        .finish(outcome, Some(target), Some(key))
        .map_err(|_| "capture retirement failed".into())
}

/// Dropping an armed commit is the same path used by cancellation or an unknown DB result.
pub fn cancel_capture(runtime: &mut ChannelRuntimeV1) {
    let guard = super::cosmetic_item_use::capture_commit::CaptureCommitGuard::new(runtime, true);
    guard.arm();
}
