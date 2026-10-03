//! VSL-COMBAT-01 Combat D2b/D3-2: compose one committed creature death into
//! its two independent reward descendants (§24.1): the death's corpse MINT
//! followed by a bounded loot MINT batch into that corpse's container
//! (D3 §4.1) through `durability::item_mint`, and one XP award through
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
//!
//! CHARM-2 adds one more independent descendant after XP
//! ([`settle_creature_death_rewards_with_bestiary`]): the principal's
//! Bestiary kill through `durability::bestiary_progress`, under the same
//! gameplay fence and the same memoized (death, character) occurrence.
//!
//! Both revision-advancing descendants run in the principal's revision slot
//! (CHAR-REV-SEQ-1), which the caller acquires before it takes the runtime
//! lock and holds for the whole chain: XP commits at the slot's cursor, and
//! Bestiary takes the revision XP committed, so no other Character write can
//! commit between them.

use super::loot_plan::{
    LootDefinitionRef, LootPlan, LootPlanDeathKey, LootPlanError, LootTableDefinition,
    plan_creature_loot,
};
use crate::domain::CharacterRevision;
use crate::domain::bestiary::{BestiaryError, BestiaryKillCredit, BestiaryRace};
use crate::domain::progression::{FiniteProgressionPolicy, ProgressionRevisionContext};
use crate::durability::DurabilityRoot;
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest, BestiaryProgressError,
};
use crate::durability::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority,
};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence, ProgressionInitializationOutcome,
    ProgressionInitializationRequest,
};
use crate::durability::character_revision_sequencer::RevisionSlot;
use crate::durability::item_mint::{
    CORPSE_CONTAINER_ENTRIES_MAX, CORPSE_MATERIALIZATION_PURPOSE_KEY, CommittedItemMint,
    CorpseContainerPlacement, CorpseLootMintRequest, GroundPlacement, ItemMintCause, ItemMintError,
    ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
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
/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): a corpse container
/// holds at most 16 entries, equal by construction to
/// `COMBAT01-LOOT-PLAN-ITEMS`. Checked once, against the whole accepted plan,
/// before the corpse or any entry is frozen.
pub(crate) const GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX: usize =
    CORPSE_CONTAINER_ENTRIES_MAX as usize;
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
#[allow(
    clippy::enum_variant_names,
    reason = "each variant names its registered ceiling that was exceeded"
)]
pub(crate) enum CombatResourceLimitError {
    InflightLootMintsPerScopeExceeded,
    RewardPrincipalsExceeded,
    CorpseContainerEntriesExceeded,
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
            Self::CorpseContainerEntriesExceeded => {
                formatter.write_str("GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX exceeded")
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

/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX`: reject the whole plan, before
/// the corpse or any entry is frozen, if its accepted entry count exceeds the
/// corpse container's capacity. This prevents only a capacity-caused partial
/// commit, never a generation-ending one (D52).
pub(crate) fn check_corpse_container_capacity(
    plan_entries: usize,
) -> Result<(), CombatResourceLimitError> {
    if plan_entries > GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX {
        return Err(CombatResourceLimitError::CorpseContainerEntriesExceeded);
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
    /// The rewarded character is the fence's own `character_id`: there is no
    /// second identity field that could disagree with the XP writer's fence.
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
    /// The corpse `ItemInstance`'s definition, supplied by the caller exactly
    /// as the loot definitions are (Content binding of `i00005801` waits on
    /// the Content revision, D3-7).
    pub(crate) corpse_item: LootDefinitionRef,
    pub(crate) loot_table_ref: LootDefinitionRef,
    pub(crate) loot_table: LootTableDefinition,
    pub(crate) ground: DeathGroundContext,
    pub(crate) inflight_loot_mints_before_this_death: usize,
    pub(crate) reward_principals: Vec<RewardPrincipal>,
    pub(crate) xp_amount: ExactI64,
    pub(crate) progression: RewardProgressionBinding<N>,
}

/// Refusals before either descendant runs.
#[derive(Debug)]
pub(crate) enum CreatureDeathRewardAdmissionError {
    Limit(CombatResourceLimitError),
    /// `actor` is not this generation's projected committed death.
    Death(CarrierError),
}

#[derive(Debug)]
pub(crate) enum CombatDeathRewardLootError {
    Plan(LootPlanError),
    Capacity(CombatResourceLimitError),
    /// The corpse's own MINT was refused (including the per-scope corpse cap,
    /// `CapacityExceeded`): no corpse and no loot exist for this death.
    Corpse(ItemMintError),
    /// A loot entry's MINT into the committed corpse was refused; already
    /// committed entries stay committed and the remainder is dropped (D52).
    Mint(ItemMintError),
}

/// One death's committed corpse and the loot entries minted into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedCorpseLoot {
    pub(crate) corpse: CommittedItemMint,
    /// In plan order; entry `i` occupies corpse container ordinal `i + 1`.
    pub(crate) entries: Vec<CommittedItemMint>,
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
    pub(crate) loot: Result<CommittedCorpseLoot, CombatDeathRewardLootError>,
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

/// Runs the loot descendant (D3 §4.1): plans and preflights the whole plan,
/// mints the corpse first (an ordinary Ground MINT at the corpse's position,
/// carrying the owner's top-damage winner), then mints each loot entry, in
/// table order, into `Container(parent = that corpse)` ordinal `i + 1`,
/// stopping at the first MINT failure (a fence failure mid batch would reject
/// every remaining entry too; nothing is gained by continuing past it).
/// Already-committed entries stay committed: this never retries or undoes a
/// prior `freeze`/`commit`, so a generation that ends mid-plan leaves the
/// corpse holding only the entries that did commit (D52).
#[allow(clippy::too_many_arguments)]
async fn settle_loot(
    session: &DurabilitySession<'_, '_, '_>,
    death: CreatureDeathOccurrenceKey,
    corpse: MovementLocalPosition,
    top_damage_character_id: [u8; 16],
    ground: &DeathGroundContext,
    corpse_item: &LootDefinitionRef,
    loot_table_ref: &LootDefinitionRef,
    loot_table: &LootTableDefinition,
    inflight_before_this_death: usize,
) -> Result<CommittedCorpseLoot, CombatDeathRewardLootError> {
    let plan: LootPlan = plan_creature_loot(loot_plan_seed(death), loot_table_ref, loot_table)
        .map_err(CombatDeathRewardLootError::Plan)?;
    check_inflight_loot_mint_capacity(inflight_before_this_death, plan.entries.len())
        .map_err(CombatDeathRewardLootError::Capacity)?;
    check_corpse_container_capacity(plan.entries.len())
        .map_err(CombatDeathRewardLootError::Capacity)?;

    // The corpse's cause reuses the loot-cause tuple with the reserved
    // sentinel purpose key, draw ordinal 0 and the corpse's own definition in
    // place of a loot table (D3 §4.1).
    let corpse_definition = to_typed_definition(corpse_item);
    let corpse_request = ItemMintRequest {
        cause: ItemMintCause::from_creature_death(
            death,
            corpse_definition.clone(),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.to_owned(),
            0,
        ),
        item: corpse_definition,
        quantity: 1,
        ground: ground_placement(death, corpse, ground),
        content_revision: ground.content_revision.clone(),
        ruleset_revision: ground.ruleset_revision.clone(),
        sim_revision: ground.sim_revision.clone(),
    };
    let mut corpse_candidate = session
        .root
        .freeze_item_mint(session.authority, session.node, corpse_request)
        .await
        .map_err(CombatDeathRewardLootError::Corpse)?;
    let corpse_outcome = session
        .root
        .commit_corpse_mint(
            session.authority,
            session.node,
            &mut corpse_candidate,
            top_damage_character_id,
        )
        .await
        .map_err(CombatDeathRewardLootError::Corpse)?;
    let corpse_mint = committed(corpse_outcome);

    let mut entries = Vec::with_capacity(plan.entries.len());
    for (index, entry) in plan.entries.iter().enumerate() {
        let request = CorpseLootMintRequest {
            cause: ItemMintCause::from_creature_death(
                death,
                to_typed_definition(&plan.loot_table),
                entry.purpose_key.clone(),
                entry.draw_ordinal,
            ),
            item: to_typed_definition(&entry.item),
            quantity: entry.quantity,
            placement: CorpseContainerPlacement {
                corpse_item_instance_id: corpse_mint.item_instance_id,
                // The preflight bounds `index` by the container capacity.
                placement_ordinal: u32::try_from(index + 1)
                    .map_err(|_| CombatDeathRewardLootError::Mint(ItemMintError::InvalidInput))?,
            },
            content_revision: ground.content_revision.clone(),
            ruleset_revision: ground.ruleset_revision.clone(),
            sim_revision: ground.sim_revision.clone(),
        };
        let mut candidate = session
            .root
            .freeze_corpse_loot_mint(session.authority, session.node, request)
            .await
            .map_err(CombatDeathRewardLootError::Mint)?;
        let outcome = session
            .root
            .commit_corpse_loot_mint(session.authority, session.node, &mut candidate)
            .await
            .map_err(CombatDeathRewardLootError::Mint)?;
        entries.push(committed(outcome));
    }
    Ok(CommittedCorpseLoot {
        corpse: corpse_mint,
        entries,
    })
}

fn committed(outcome: ItemMintOutcome) -> CommittedItemMint {
    match outcome {
        ItemMintOutcome::Committed(result) | ItemMintOutcome::AlreadyCommitted(result) => result,
    }
}

/// Runs the single XP descendant: the memoized (death, character)
/// occurrence, the R7 P03 award, and the D88 initializer only when the
/// progression row is missing.
async fn settle_experience<const N: usize>(
    session: &DurabilitySession<'_, '_, '_>,
    slot: &mut RevisionSlot,
    owner: &mut CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
    principal: &RewardPrincipal,
    progression: &RewardProgressionBinding<N>,
    amount: ExactI64,
) -> Result<ExperienceCommitOutcome, CombatDeathRewardXpError> {
    // The occurrence is memoized per (death, character), so a retry replays
    // the same award: a retained receipt is replayed at its original revision
    // (the award's binding includes it), a new award commits at the slot's
    // cursor. Only a missing progression row runs the D88 initializer, and
    // the award is then attempted once more. A failed earlier initialization
    // therefore never strands the award (a retry re-initializes), and an
    // already advanced revision never rejects a legitimate replay.
    let (occurrence_bytes, _) = owner
        .reward_occurrence(actor, *principal.gameplay_fence.character_id.as_bytes())
        .map_err(CombatDeathRewardXpError::Occurrence)?;
    let occurrence = ExperienceRewardOccurrence::from_bytes(occurrence_bytes)
        .map_err(|_| CombatDeathRewardXpError::InvalidOccurrence)?;

    let original = session
        .root
        .reconcile_character_experience(session.authority, occurrence)
        .await
        .map_err(CombatDeathRewardXpError::Progression)?
        .map(|award| award.original_character_revision);

    match commit_experience(
        session,
        slot,
        principal,
        progression,
        occurrence,
        amount,
        original,
    )
    .await
    {
        Err(CharacterProgressionError::MissingProgressionState) => {}
        result => return result.map_err(CombatDeathRewardXpError::Progression),
    }
    // The initializer writes no revision; it is fenced at the slot's cursor.
    let expected_character_revision =
        slot.cursor(session.root, session.authority)
            .await
            .map_err(|error| {
                CombatDeathRewardXpError::Progression(match error {
                    CharacterAuthorityError::Unavailable(error) => {
                        CharacterProgressionError::Unavailable(error)
                    }
                    _ => CharacterProgressionError::AuthorityRejected,
                })
            })?;
    match session
        .root
        .initialize_character_progression(
            session.authority,
            session.node,
            CurrentCharacterGameplayFence {
                expected_character_revision,
                ..principal.gameplay_fence
            },
            progression.initialization_request(),
        )
        .await
    {
        Ok(ProgressionInitializationOutcome::Initialized(_))
        | Ok(ProgressionInitializationOutcome::AlreadyInitialized(_)) => {}
        Err(error) => return Err(CombatDeathRewardXpError::Progression(error)),
    }
    commit_experience(
        session,
        slot,
        principal,
        progression,
        occurrence,
        amount,
        None,
    )
    .await
    .map_err(CombatDeathRewardXpError::Progression)
}

async fn commit_experience<const N: usize>(
    session: &DurabilitySession<'_, '_, '_>,
    slot: &mut RevisionSlot,
    principal: &RewardPrincipal,
    progression: &RewardProgressionBinding<N>,
    occurrence: ExperienceRewardOccurrence,
    amount: ExactI64,
    original: Option<CharacterRevision>,
) -> Result<ExperienceCommitOutcome, CharacterProgressionError> {
    slot.commit_experience(
        session.root,
        session.authority,
        session.node,
        principal.gameplay_fence,
        progression.award_request(occurrence, amount),
        original,
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
/// regardless of whether the other fails. `slot` is the reward principal's
/// revision slot, acquired before the runtime lock.
pub(crate) async fn settle_creature_death_rewards<const N: usize>(
    actor: ExactActorRef,
    owner: &mut CurrentOwnerCombatDeath<'_>,
    session: &DurabilitySession<'_, '_, '_>,
    slot: &mut RevisionSlot,
    input: CreatureDeathRewardInput<N>,
) -> Result<CreatureDeathRewardOutcome, CreatureDeathRewardAdmissionError> {
    check_reward_principal_count(input.reward_principals.len())
        .map_err(CreatureDeathRewardAdmissionError::Limit)?;
    // The death key and corpse position come only from the owner's own
    // projection of `actor`, so loot and XP always settle one bound death.
    let (death, corpse) = owner
        .projected_death(actor)
        .map_err(CreatureDeathRewardAdmissionError::Death)?;
    let principal = input.reward_principals[0];
    // D132/§4.3: the owner's already tie-broken top-damage winner, read at the
    // same moment as `(death, corpse)`. A death with no tracked contributor
    // (damage-free, or every hit from an untracked attacker) names the death's
    // single reward principal, the only character with a claim on it in this
    // single-principal slice (`COMBAT01-REWARD-PRINCIPALS`).
    let top_damage_character_id = match owner
        .top_damage_character(actor)
        .map_err(CreatureDeathRewardAdmissionError::Death)?
    {
        Some(character) => *character.as_bytes(),
        None => *principal.gameplay_fence.character_id.as_bytes(),
    };

    let loot = settle_loot(
        session,
        death,
        corpse,
        top_damage_character_id,
        &input.ground,
        &input.corpse_item,
        &input.loot_table_ref,
        &input.loot_table,
        input.inflight_loot_mints_before_this_death,
    )
    .await;

    let xp = settle_experience(
        session,
        slot,
        owner,
        actor,
        &principal,
        &input.progression,
        input.xp_amount,
    )
    .await;

    Ok(CreatureDeathRewardOutcome { death, loot, xp })
}

/// CHARM-2 input of the Bestiary descendant: the dead creature's race as
/// bound from its Creature definition, and the credit evidence of the death's
/// single reward principal (CHARM-0 answer 6a).
#[derive(Debug, Clone)]
pub(crate) struct CreatureDeathBestiaryInput {
    /// `None` when the creature's definition carries no Bestiary block: such
    /// a creature is never counted.
    pub(crate) race: Option<BestiaryRace>,
    pub(crate) credit: BestiaryKillCredit,
}

/// A kill the Bestiary descendant settled without an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CombatBestiaryOutcome {
    /// The creature is not a Bestiary race. Nothing was written.
    NotABestiaryRace,
    /// The principal's last damage is older than the credit window. Nothing
    /// was written.
    OutsideCreditWindow,
    Recorded(BestiaryKillOutcome),
}

#[derive(Debug)]
pub(crate) enum CombatDeathRewardBestiaryError {
    Credit(BestiaryError),
    Occurrence(CarrierError),
    InvalidOccurrence,
    Progress(BestiaryProgressError),
}

/// Loot and XP exactly as [`settle_creature_death_rewards`] returns them,
/// plus the independent Bestiary descendant's own terminal result.
#[derive(Debug)]
pub(crate) struct CreatureDeathRewardWithBestiaryOutcome {
    pub(crate) rewards: CreatureDeathRewardOutcome,
    pub(crate) bestiary: Result<CombatBestiaryOutcome, CombatDeathRewardBestiaryError>,
}

/// CHARM-2 entry point: [`settle_creature_death_rewards`] followed by one
/// more independent descendant, the Bestiary kill of the death's single
/// reward principal. It runs after loot and XP have reached their terminal
/// results, in its own Character transaction, so it never blocks, retries or
/// rolls back either of them; an admission refusal refuses all three. The
/// same held `slot` covers XP and Bestiary.
pub(crate) async fn settle_creature_death_rewards_with_bestiary<const N: usize>(
    actor: ExactActorRef,
    owner: &mut CurrentOwnerCombatDeath<'_>,
    session: &DurabilitySession<'_, '_, '_>,
    slot: &mut RevisionSlot,
    input: CreatureDeathRewardInput<N>,
    bestiary: CreatureDeathBestiaryInput,
) -> Result<CreatureDeathRewardWithBestiaryOutcome, CreatureDeathRewardAdmissionError> {
    let binding = BestiaryProgressionBinding {
        context: input.progression.context.clone(),
        policy_revision: input.progression.policy_revision.clone(),
        reward_revision: input.progression.reward_revision.clone(),
    };
    // The admission check inside refuses anything but exactly one principal
    // before any descendant runs, so `first` is that principal on success.
    let principal = input.reward_principals.first().copied();
    let rewards = settle_creature_death_rewards(actor, owner, session, slot, input).await?;
    let principal = principal.ok_or(CreatureDeathRewardAdmissionError::Limit(
        CombatResourceLimitError::RewardPrincipalsExceeded,
    ))?;
    let bestiary =
        settle_bestiary(session, slot, owner, actor, &principal, &binding, bestiary).await;
    Ok(CreatureDeathRewardWithBestiaryOutcome { rewards, bestiary })
}

/// The progression binding the death's XP award used; the Bestiary receipt
/// must carry the same revisions as the progression state it advances.
struct BestiaryProgressionBinding {
    context: ProgressionRevisionContext<String>,
    policy_revision: String,
    reward_revision: String,
}

/// The Bestiary write takes the slot's cursor: the revision the XP award
/// committed when it did, else the Character's current one. Its binding
/// excludes the revision, so a replay resolves at either.
async fn settle_bestiary(
    session: &DurabilitySession<'_, '_, '_>,
    slot: &mut RevisionSlot,
    owner: &mut CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
    principal: &RewardPrincipal,
    binding: &BestiaryProgressionBinding,
    input: CreatureDeathBestiaryInput,
) -> Result<CombatBestiaryOutcome, CombatDeathRewardBestiaryError> {
    let Some(race) = input.race else {
        return Ok(CombatBestiaryOutcome::NotABestiaryRace);
    };
    if !input
        .credit
        .is_credited()
        .map_err(CombatDeathRewardBestiaryError::Credit)?
    {
        return Ok(CombatBestiaryOutcome::OutsideCreditWindow);
    }
    // The same (death, character) occurrence the XP award is keyed by, so a
    // repeated composition call replays this kill instead of adding one.
    let (occurrence_bytes, _) = owner
        .reward_occurrence(actor, *principal.gameplay_fence.character_id.as_bytes())
        .map_err(CombatDeathRewardBestiaryError::Occurrence)?;
    let occurrence = BestiaryKillOccurrence::from_bytes(occurrence_bytes)
        .map_err(|_| CombatDeathRewardBestiaryError::InvalidOccurrence)?;
    slot.commit_bestiary(
        session.root,
        session.authority,
        session.node,
        principal.gameplay_fence,
        BestiaryKillRequest {
            occurrence,
            race,
            context: binding.context.clone(),
            policy_revision: binding.policy_revision.clone(),
            reward_revision: binding.reward_revision.clone(),
        },
    )
    .await
    .map(CombatBestiaryOutcome::Recorded)
    .map_err(CombatDeathRewardBestiaryError::Progress)
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
    fn corpse_container_capacity_accepts_the_full_plan_and_rejects_one_over() {
        // Equal by construction to the accepted loot-plan ceiling.
        assert_eq!(
            GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX,
            super::super::loot_plan::COMBAT01_LOOT_PLAN_ITEMS_MAX
        );
        assert!(check_corpse_container_capacity(0).is_ok());
        assert!(check_corpse_container_capacity(GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX).is_ok());
        assert_eq!(
            check_corpse_container_capacity(GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX + 1),
            Err(CombatResourceLimitError::CorpseContainerEntriesExceeded)
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
