//! Immutable native source tick definitions and owner receipts shared with transport.
use crate::durability::character_death::PlayerDeathOccurrence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};

/// One present player's death (§4.2): the occurrence minted at the lethal write, the idempotency
/// key of every consequence, and the cell the player died on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDeath {
    pub occurrence: PlayerDeathOccurrence,
    pub cell: MovementLocalPosition,
}

/// Retained source payload reader ABI; source monster regeneration is N/A (zero captured definitions).
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
pub trait NativeRegenerationTickSource {
    fn gains(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        definition: &str,
        revision: u32,
        due: u64,
    ) -> Option<(u64, u64)>;
}
/// Immutable source payload retained from the trusted, already parsed native loader.
/// Runtime condition cursors remain exclusively in canonical actor slots.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
pub struct NativeRegenerationRegistry {
    world: crate::foundation::WorldId,
    server: [u8; 32],
    loader_digest: [u8; 32],
    entries: Vec<NativeRegenerationEntry>,
}
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
struct NativeRegenerationEntry {
    key: String,
    revision: u32,
    source: crate::content::ProjectV2DefinitionRef,
    values: crate::content::ProjectV2ConditionRegeneration,
}
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
impl NativeRegenerationRegistry {
    /// Immutable definitions only; the lane independently checks loaded membership before
    /// constructing a current tick capability. This registry alone grants no tick authority.
    pub fn from_project_definitions(
        runtime: &ChannelRuntimeV1,
        draft: &crate::content::ProjectV2Draft,
        loader_digest: [u8; 32],
    ) -> Option<Self> {
        use crate::content::{
            ProjectV2AbilityEffect, ProjectV2AuthoringProfileData, ProjectV2InlineEffectOperation,
        };
        let world = runtime.binding().world_id();
        let text: String = world
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if draft.core.world_id != text || loader_digest == [0; 32] {
            return None;
        }
        let mut entries = Vec::new();
        for profile in &draft.state.authoring_profiles {
            let ProjectV2AuthoringProfileData::Ability(ability) = &profile.data else {
                continue;
            };
            let Some(details) = &ability.details else {
                continue;
            };
            let revision = profile
                .target
                .revision
                .strip_prefix("definition-r")?
                .parse::<u32>()
                .ok()?;
            if revision == 0 {
                return None;
            }
            for effect in &details.effects {
                let ProjectV2AbilityEffect::Inline(inline) = effect else {
                    continue;
                };
                let ProjectV2InlineEffectOperation::Condition { condition, .. } = &inline.operation
                else {
                    continue;
                };
                let Some(values) = condition.regeneration else {
                    continue;
                };
                let definition = crate::creature_condition_content::lower_condition_definition(
                    inline, revision, None,
                )
                .ok()?;
                if entries.iter().any(|e: &NativeRegenerationEntry| {
                    e.key == definition.key() && e.revision == revision
                }) {
                    return None;
                }
                entries.try_reserve(1).ok()?;
                entries.push(NativeRegenerationEntry {
                    key: definition.key().to_owned(),
                    revision,
                    source: profile.target.clone(),
                    values,
                });
            }
        }
        Some(Self {
            world,
            server: runtime.content_pin().server_artifact_digest(),
            loader_digest,
            entries,
        })
    }
    #[allow(
        clippy::len_without_is_empty,
        reason = "retained registry cardinality probe for qualification"
    )]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn loader_digest(&self) -> [u8; 32] {
        self.loader_digest
    }
}
impl NativeRegenerationTickSource for NativeRegenerationRegistry {
    fn gains(
        &self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _definition: &str,
        _revision: u32,
        _due: u64,
    ) -> Option<(u64, u64)> {
        None
    }
}
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
pub struct NativeOwnedPlayerTickReceipt {
    pub ticks: Vec<crate::ability::condition::ConditionTick<String>>,
    pub revision: u64,
    pub damage: u32,
    pub death: Option<PlayerDeath>,
}
