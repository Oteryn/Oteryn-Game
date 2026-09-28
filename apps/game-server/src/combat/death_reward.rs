//! VSL-COMBAT-01 Combat D2b: compose one committed creature death into its
//! two independent reward descendants (§24.1): a bounded loot MINT batch
//! through `durability::item_mint`, and one XP award through
//! `durability::character_progression`. They are never one distributed
//! transaction (~:455): each runs to its own terminal `Result`, and a
//! failure in one never blocks, retries or rolls back the other.
//!
//! The composition point is right after Combat's own [`super::
//! project_fixed_one_creature_death`] projects the committed lethal
//! occurrence: its caller extracts the owned, `Copy`
//! `(CreatureDeathOccurrenceKey, MovementLocalPosition)` pair (as the D1
//! `CombatDeathFixture::project_death` test fixture already does) and passes
//! them in here together with the still-borrowed owner handle, so this
//! module never re-derives death facts from caller bytes.

use super::loot_plan::{
    LootDefinitionRef, LootPlan, LootPlanDeathKey, LootPlanError, LootTableDefinition,
    plan_creature_loot,
};
use crate::domain::CharacterId;
use crate::domain::progression::{FiniteProgressionPolicy, ProgressionRevisionContext};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence, ProgressionInitializationOutcome,
    ProgressionInitializationRequest,
};
use crate::durability::item_mint::{
    CommittedItemMint, GroundPlacement, ItemMintCause, ItemMintError, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{
    CarrierError, CreatureDeathOccurrenceKey, CurrentOwnerCombatDeath, ExactActorRef,
    MovementLocalPosition,
};
use oteryn_simulation_determinism::ExactI64;

/// `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE` (D78, resource rows decision
/// §4.1, row 7): at most 64 in-flight loot MINTs per scope at once. Checked
/// before any entry of a death's plan is minted, against the caller-tracked
/// count of MINTs already in flight elsewhere in the same scope.
pub(crate) const COMBAT01_INFLIGHT_LOOT_MINTS_PER_SCOPE_MAX: usize = 64;
/// `COMBAT01-XP-DESCENDANTS-PER-DEATH` (§4.1, row 9): exactly one. Enforced
/// structurally: [`settle_creature_death_rewards`] issues at most one
/// `commit_character_experience` call per invocation, keyed by an occurrence
/// the physical Channel owner memoizes once per (death, character)
/// ([`CurrentOwnerCombatDeath::reward_occurrence`]), so a repeated
/// composition call reuses it rather than creating a second descendant.
pub(crate) const COMBAT01_XP_DESCENDANTS_PER_DEATH_MAX: usize = 1;
/// `COMBAT01-REWARD-PRINCIPALS` (§4.1, row 10): exactly one reward principal
/// per death in this single-principal slice.
pub(crate) const COMBAT01_REWARD_PRINCIPALS_MAX: usize = 1;

/// `COMBAT01-DEATH-WORKFLOWS-PER-SCOPE` (row 3) is intentionally not
/// registered by this module: its own decision text records the failure
/// mode as "structurally unreachable" (bounded by the creature-count
/// envelope, a different, still-unregistered row), and this composition has
/// no scope-wide active-workflow counter to check. Registering an
/// unenforced ceiling here would be a fake guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatResourceLimitError {
    InflightLootMintsPerScopeExceeded,
    RewardPrincipalsExceeded,
}

impl std::fmt::Display for CombatResourceLimitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InflightLootMintsPerScopeExceeded => {
                formatter.write_str("COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE exceeded")
            }
            Self::RewardPrincipalsExceeded => {
                formatter.write_str("COMBAT01-REWARD-PRINCIPALS exceeded")
            }
        }
    }
}

impl std::error::Error for CombatResourceLimitError {}

/// `COMBAT01-REWARD-PRINCIPALS`: reject up front, before planning or
/// minting anything, unless exactly one reward principal was supplied.
pub(crate) fn check_reward_principal_count(count: usize) -> Result<(), CombatResourceLimitError> {
    if count == 0 || count > COMBAT01_REWARD_PRINCIPALS_MAX {
        return Err(CombatResourceLimitError::RewardPrincipalsExceeded);
    }
    Ok(())
}

/// `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE`: reject the whole plan, before
/// any entry is frozen, if adding it to the scope's already in-flight MINT
/// count would exceed the ceiling.
pub(crate) fn check_inflight_loot_mint_capacity(
    inflight_before_this_death: usize,
    plan_entries: usize,
) -> Result<(), CombatResourceLimitError> {
    let total = inflight_before_this_death
        .checked_add(plan_entries)
        .ok_or(CombatResourceLimitError::InflightLootMintsPerScopeExceeded)?;
    if total > COMBAT01_INFLIGHT_LOOT_MINTS_PER_SCOPE_MAX {
        return Err(CombatResourceLimitError::InflightLootMintsPerScopeExceeded);
    }
    Ok(())
}

/// Ground-placement facts a caller resolves once per death (map/content/
/// ruleset/sim revisions and the room placement context); this module never
/// invents them.
#[derive(Debug, Clone)]
pub(crate) struct DeathGroundContext {
    pub(crate) map_revision: String,
    pub(crate) content_revision: String,
    pub(crate) ruleset_revision: String,
    pub(crate) sim_revision: String,
    pub(crate) native_room_placement_context: Vec<u8>,
}

/// The single reward principal of one death (`COMBAT01-REWARD-PRINCIPALS`):
/// who is awarded XP, and the current gameplay fence R7 P03 requires to
/// award it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RewardPrincipal {
    pub(crate) character_id: CharacterId,
    pub(crate) gameplay_fence: CurrentCharacterGameplayFence,
}

/// The one progression policy binding shared by the D88 initializer and the
/// R7 P03 XP writer (`docs/agents/tasks/archive/OTV2-20260928-char-
/// progression-init.md`: "the same policy binding it passes to the XP
/// writer").
#[derive(Debug, Clone)]
pub(crate) struct RewardProgressionBinding<const N: usize> {
    pub(crate) context: ProgressionRevisionContext<String>,
    pub(crate) policy_revision: String,
    pub(crate) reward_revision: String,
    pub(crate) policy: FiniteProgressionPolicy<String, N>,
}

impl<const N: usize> RewardProgressionBinding<N> {
    fn initialization_request(&self) -> ProgressionInitializationRequest<N> {
        ProgressionInitializationRequest {
            context: self.context.clone(),
            policy_revision: self.policy_revision.clone(),
            reward_revision: self.reward_revision.clone(),
            policy: self.policy.clone(),
        }
    }

    fn award_request(
        &self,
        occurrence: ExperienceRewardOccurrence,
        amount: ExactI64,
    ) -> ExperienceAwardRequest<N> {
        ExperienceAwardRequest {
            occurrence,
            amount,
            context: self.context.clone(),
            policy_revision: self.policy_revision.clone(),
            reward_revision: self.reward_revision.clone(),
            policy: self.policy.clone(),
        }
    }
}

/// Complete semantic input of one death's reward composition.
#[derive(Debug, Clone)]
pub(crate) struct CreatureDeathRewardInput<const N: usize> {
    pub(crate) loot_table_ref: LootDefinitionRef,
    pub(crate) loot_table: LootTableDefinition,
    pub(crate) ground: DeathGroundContext,
    pub(crate) inflight_loot_mints_before_this_death: usize,
    pub(crate) reward_principals: Vec<RewardPrincipal>,
    pub(crate) xp_amount: ExactI64,
    pub(crate) progression: RewardProgressionBinding<N>,
}

#[derive(Debug)]
pub(crate) enum CombatDeathRewardLootError {
    Plan(LootPlanError),
    Capacity(CombatResourceLimitError),
    Mint(ItemMintError),
}

#[derive(Debug)]
pub(crate) enum CombatDeathRewardXpError {
    Occurrence(CarrierError),
    InvalidOccurrence,
    Progression(CharacterProgressionError),
}

/// Both descendants' terminal results. Each is independent: a `loot` error
/// carries no information about `xp` and vice versa, and both fields are
/// always populated (never short-circuited by the other).
#[derive(Debug)]
pub(crate) struct CreatureDeathRewardOutcome {
    pub(crate) death: CreatureDeathOccurrenceKey,
    pub(crate) loot: Result<Vec<CommittedItemMint>, CombatDeathRewardLootError>,
    pub(crate) xp: Result<ExperienceCommitOutcome, CombatDeathRewardXpError>,
}

fn to_typed_definition(definition: &LootDefinitionRef) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: definition.family.clone(),
        production_key: definition.production_key.clone(),
        revision_ref: definition.revision_ref.clone(),
    }
}

/// The death-key seed `plan_creature_loot` plans from: the same
/// `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` fields as the real death key,
/// copied out (D2a's documented, deferred production wiring).
fn loot_plan_seed(death: CreatureDeathOccurrenceKey) -> LootPlanDeathKey {
    LootPlanDeathKey::new(
        *death.world_id().as_bytes(),
        *death.channel_id().as_bytes(),
        death.scope_ownership_generation().get(),
        death.actor_local_id(),
        death.actor_local_generation(),
    )
}

/// A death has no durable placement rule yet (out of scope; owned by a
/// future Ground/room placement decision). This builds the smallest
/// defensible `GroundPlacement`: the corpse's own local position, and a
/// `corpse_ref` that is a canonical big-endian encoding of the exact death
/// key, so two entries of the same death always frame to the same
/// placement and two different deaths never collide.
fn ground_placement(
    death: CreatureDeathOccurrenceKey,
    corpse: MovementLocalPosition,
    ground: &DeathGroundContext,
) -> GroundPlacement {
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&corpse.x.to_be_bytes());
    spatial_position.extend_from_slice(&corpse.y.to_be_bytes());
    spatial_position.extend_from_slice(&corpse.floor.to_be_bytes());
    let mut corpse_ref = Vec::with_capacity(52);
    corpse_ref.extend_from_slice(death.world_id().as_bytes());
    corpse_ref.extend_from_slice(death.channel_id().as_bytes());
    corpse_ref.extend_from_slice(&death.scope_ownership_generation().get().to_be_bytes());
    corpse_ref.extend_from_slice(&death.actor_local_id().to_be_bytes());
    corpse_ref.extend_from_slice(&death.actor_local_generation().to_be_bytes());
    GroundPlacement {
        spatial_position,
        corpse_ref,
        map_revision: ground.map_revision.clone(),
        content_revision: ground.content_revision.clone(),
        native_room_placement_context: ground.native_room_placement_context.clone(),
    }
}

/// Runs the bounded loot descendant: plans, then mints one entry at a time
/// in table order, stopping at the first MINT failure (a fence failure mid
/// batch would reject every remaining entry too; nothing is gained by
/// continuing past it). Already-committed entries stay committed: this
/// never retries or undoes a prior `freeze_item_mint`/`commit_item_mint`.
async fn settle_loot(
    session: &DurabilitySession<'_, '_, '_>,
    death: CreatureDeathOccurrenceKey,
    corpse: MovementLocalPosition,
    ground: &DeathGroundContext,
    loot_table_ref: &LootDefinitionRef,
    loot_table: &LootTableDefinition,
    inflight_before_this_death: usize,
) -> Result<Vec<CommittedItemMint>, CombatDeathRewardLootError> {
    let plan: LootPlan = plan_creature_loot(loot_plan_seed(death), loot_table_ref, loot_table)
        .map_err(CombatDeathRewardLootError::Plan)?;
    check_inflight_loot_mint_capacity(inflight_before_this_death, plan.entries.len())
        .map_err(CombatDeathRewardLootError::Capacity)?;

    let mut results = Vec::with_capacity(plan.entries.len());
    for entry in &plan.entries {
        let request = ItemMintRequest {
            cause: ItemMintCause::from_creature_death(
                death,
                to_typed_definition(&plan.loot_table),
                entry.purpose_key.clone(),
                entry.draw_ordinal,
            ),
            item: to_typed_definition(&entry.item),
            quantity: entry.quantity,
            ground: ground_placement(death, corpse, ground),
            content_revision: ground.content_revision.clone(),
            ruleset_revision: ground.ruleset_revision.clone(),
            sim_revision: ground.sim_revision.clone(),
        };
        let mut candidate = session
            .root
            .freeze_item_mint(session.authority, session.node, request)
            .await
            .map_err(CombatDeathRewardLootError::Mint)?;
        let outcome = session
            .root
            .commit_item_mint(session.authority, session.node, &mut candidate)
            .await
            .map_err(CombatDeathRewardLootError::Mint)?;
        results.push(match outcome {
            ItemMintOutcome::Committed(result) | ItemMintOutcome::AlreadyCommitted(result) => {
                result
            }
        });
    }
    Ok(results)
}

/// Runs the single XP descendant: the memoized (death, character)
/// occurrence, the R7 P03 award, and the D88 initializer only when the
/// progression row is missing.
async fn settle_experience<const N: usize>(
    session: &DurabilitySession<'_, '_, '_>,
    owner: &mut CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
    principal: &RewardPrincipal,
    progression: &RewardProgressionBinding<N>,
    amount: ExactI64,
) -> Result<ExperienceCommitOutcome, CombatDeathRewardXpError> {
    // The occurrence is memoized per (death, character), so a retry replays
    // the same award. The award is attempted first: its own occurrence-keyed
    // replay resolves a retry without re-asserting the caller's revision.
    // Only a missing progression row runs the D88 initializer, and the award
    // is then attempted once more. A failed earlier initialization therefore
    // never strands the award (a retry re-initializes), and an already
    // advanced revision never rejects a legitimate replay.
    let (occurrence_bytes, _) = owner
        .reward_occurrence(actor, *principal.character_id.as_bytes())
        .map_err(CombatDeathRewardXpError::Occurrence)?;
    let occurrence = ExperienceRewardOccurrence::from_bytes(occurrence_bytes)
        .map_err(|_| CombatDeathRewardXpError::InvalidOccurrence)?;

    match commit_experience(session, principal, progression, occurrence, amount).await {
        Err(CharacterProgressionError::MissingProgressionState) => {}
        result => return result.map_err(CombatDeathRewardXpError::Progression),
    }
    match session
        .root
        .initialize_character_progression(
            session.authority,
            session.node,
            principal.gameplay_fence,
            progression.initialization_request(),
        )
        .await
    {
        Ok(ProgressionInitializationOutcome::Initialized(_))
        | Ok(ProgressionInitializationOutcome::AlreadyInitialized(_)) => {}
        Err(error) => return Err(CombatDeathRewardXpError::Progression(error)),
    }
    commit_experience(session, principal, progression, occurrence, amount)
        .await
        .map_err(CombatDeathRewardXpError::Progression)
}

async fn commit_experience<const N: usize>(
    session: &DurabilitySession<'_, '_, '_>,
    principal: &RewardPrincipal,
    progression: &RewardProgressionBinding<N>,
    occurrence: ExperienceRewardOccurrence,
    amount: ExactI64,
) -> Result<ExperienceCommitOutcome, CharacterProgressionError> {
    session
        .root
        .commit_character_experience(
            session.authority,
            session.node,
            principal.gameplay_fence,
            progression.award_request(occurrence, amount),
        )
        .await
}

/// The three durability handles every descendant call needs, bundled to keep
/// each function's own argument count small.
pub(crate) struct DurabilitySession<'a, 'f, 's> {
    pub(crate) root: &'a DurabilityRoot,
    pub(crate) authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(crate) node: &'a NodeIncarnationProof,
}

/// D2b entry point: compose one already-committed, already-projected
/// creature death (`death`, `corpse`) into its loot and XP descendants.
/// `owner` is the same borrowed handle Combat used to project the death
/// (`super::project_fixed_one_creature_death`); the caller extracts `death`/
/// `corpse` as owned `Copy` values first so this call does not conflict with
/// that earlier borrow. `COMBAT01-REWARD-PRINCIPALS` is checked before
/// anything else runs; loot and XP then always both run, independently,
/// regardless of whether the other fails.
pub(crate) async fn settle_creature_death_rewards<const N: usize>(
    death: CreatureDeathOccurrenceKey,
    corpse: MovementLocalPosition,
    actor: ExactActorRef,
    owner: &mut CurrentOwnerCombatDeath<'_>,
    session: &DurabilitySession<'_, '_, '_>,
    input: CreatureDeathRewardInput<N>,
) -> Result<CreatureDeathRewardOutcome, CombatResourceLimitError> {
    check_reward_principal_count(input.reward_principals.len())?;
    let principal = input.reward_principals[0];

    let loot = settle_loot(
        session,
        death,
        corpse,
        &input.ground,
        &input.loot_table_ref,
        &input.loot_table,
        input.inflight_loot_mints_before_this_death,
    )
    .await;

    let xp = settle_experience(
        session,
        owner,
        actor,
        &principal,
        &input.progression,
        input.xp_amount,
    )
    .await;

    Ok(CreatureDeathRewardOutcome { death, loot, xp })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn reward_principal_count_accepts_one_and_rejects_zero_or_two() {
        assert!(check_reward_principal_count(1).is_ok());
        assert_eq!(
            check_reward_principal_count(0),
            Err(CombatResourceLimitError::RewardPrincipalsExceeded)
        );
        assert_eq!(
            check_reward_principal_count(2),
            Err(CombatResourceLimitError::RewardPrincipalsExceeded)
        );
    }

    #[test]
    fn inflight_loot_mint_capacity_accepts_exact_ceiling_and_rejects_one_over() {
        assert!(check_inflight_loot_mint_capacity(48, 16).is_ok());
        assert_eq!(
            check_inflight_loot_mint_capacity(49, 16),
            Err(CombatResourceLimitError::InflightLootMintsPerScopeExceeded)
        );
        assert_eq!(
            check_inflight_loot_mint_capacity(64, 1),
            Err(CombatResourceLimitError::InflightLootMintsPerScopeExceeded)
        );
        assert!(check_inflight_loot_mint_capacity(0, 64).is_ok());
        assert_eq!(
            check_inflight_loot_mint_capacity(usize::MAX, 1),
            Err(CombatResourceLimitError::InflightLootMintsPerScopeExceeded)
        );
    }

    #[test]
    fn ground_placement_differs_between_distinct_deaths_and_matches_for_replay() {
        use crate::foundation::{ChannelId, CombatDeathFixture, ScopeOwnershipGeneration, WorldId};

        fn id(seed: u8) -> [u8; 16] {
            [
                seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
            ]
        }

        fn death_key(channel: u8) -> CreatureDeathOccurrenceKey {
            let mut fixture = CombatDeathFixture::new(
                WorldId::decode(&id(1)).expect("world"),
                ChannelId::decode(&id(channel)).expect("channel"),
                ScopeOwnershipGeneration::new(1).expect("generation"),
            )
            .expect("fixture");
            fixture
                .strike(
                    "fixture:death-reward-tests.strike",
                    CombatDeathFixture::HEALTH,
                )
                .expect("lethal strike");
            fixture.project_death().expect("projected death").0
        }

        let death_a = death_key(2);
        let death_b = death_key(3);
        let corpse = MovementLocalPosition {
            x: 3,
            y: 4,
            floor: 5,
        };
        let ground = DeathGroundContext {
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            native_room_placement_context: b"room".to_vec(),
        };
        let placement_a = ground_placement(death_a, corpse, &ground);
        let placement_b = ground_placement(death_b, corpse, &ground);
        assert_ne!(placement_a.corpse_ref, placement_b.corpse_ref);
        let replayed = ground_placement(death_a, corpse, &ground);
        assert_eq!(placement_a, replayed);
    }
}
